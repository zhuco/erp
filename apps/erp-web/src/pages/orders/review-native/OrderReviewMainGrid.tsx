import { Alert, Button, Empty, Spin } from 'antd';
import type { Key } from 'react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type {
  CellContextMenuEvent,
  ColDef,
  ICellRendererParams,
  ColumnHeaderContextMenuEvent,
  FilterChangedEvent,
  FilterModel,
  GetMainMenuItemsParams,
  GetRowIdParams,
  GridApi,
  GridReadyEvent,
  RowDoubleClickedEvent,
  SelectionChangedEvent,
} from 'ag-grid-community';
import { ModuleRegistry } from 'ag-grid-community';
import { AllEnterpriseModule } from 'ag-grid-enterprise';
import { AgGridReact } from 'ag-grid-react';

import type { OrderSummary } from '@/api/orders';
import { useSessionStore } from '@/features/sessionStore';
import {
  buildAgGridColumnStateStorageKey,
  ERP_AG_GRID_LOCALE_TEXT_ZH_CN,
  persistAgGridColumnState,
  readPersistedAgGridColumnState,
  resolveAgGridMainMenuItems,
} from '@/components/agGridColumnState';

import { formatDateTime, money, platformLabel, resolveOrderRowKey } from './helpers';
import {
  buildMainGridFilterModel,
  extractMainGridColumnFilters,
  hasSameMainGridColumnFilters,
  OrderReviewDateRangeFloatingFilter,
  OrderReviewSetFloatingFilter,
  OrderReviewTextFloatingFilter,
} from './OrderReviewGridFilters';
import { toggleAgGridHeaderSelection } from '../shared/agGridSelectionHelpers';
import { AgGridSelectionHeaderCheckbox } from '../shared/AgGridSelectionHeaderCheckbox';
import { redrawFocusedRows } from '../shared/agGridFocusRefreshHelpers';
import { useAgGridWorkbenchSelection } from '../shared/useAgGridWorkbenchSelection';
import type { ColumnFilters } from './types';

ModuleRegistry.registerModules([AllEnterpriseModule]);

const mainGridStructuralColumnIds = new Set(['__erpSelection', '__erpRowIndex']);

type FilterOption = {
  label: string;
  value: string;
};

type BuildOrderReviewMainGridColDefsOptions = {
  logisticsOptions: FilterOption[];
  onToggleAllSelection: () => void;
  platformFilterOptions: FilterOption[];
  shopOptions: FilterOption[];
};

type OrderReviewMainGridProps = {
  rows: OrderSummary[];
  quickFilterExpanded: boolean;
  columnFilters: ColumnFilters;
  shopOptions: FilterOption[];
  logisticsOptions: FilterOption[];
  selectedRowKeys: Key[];
  focusedOrderKey: string | null;
  onGridApiChange?: (api: GridApi<OrderSummary> | null) => void;
  onColumnFiltersChange: (filters: ColumnFilters) => void;
  onSelectionChange: (keys: Key[]) => void;
  onRowActive: (record: OrderSummary) => void;
  onRowDoubleClick: (record: OrderSummary) => void;
  onFocus: (record: OrderSummary) => void;
  onContextMenu: (record: OrderSummary, event: MouseEvent) => void;
  loading: boolean;
  error: unknown;
  onRetry: () => void;
};

export const OrderReviewMainGrid = ({
  rows,
  quickFilterExpanded,
  columnFilters,
  shopOptions,
  logisticsOptions,
  selectedRowKeys,
  focusedOrderKey,
  onGridApiChange,
  onColumnFiltersChange,
  onSelectionChange,
  onRowActive,
  onRowDoubleClick,
  onFocus,
  onContextMenu,
  loading,
  error,
  onRetry,
}: OrderReviewMainGridProps) => {
  const shellRef = useRef<HTMLDivElement | null>(null);
  const apiRef = useRef<GridApi<OrderSummary> | null>(null);
  const shouldResetViewportRef = useRef(false);
  const previousFocusedRowKeyRef = useRef<string | null>(null);
  const [gridApi, setGridApi] = useState<GridApi<OrderSummary> | null>(null);
  const syncingSelectionRef = useRef(false);
  const syncingFilterModelRef = useRef(false);
  const columnStatePersistTimerRef = useRef<number | null>(null);
  const applyingPersistedColumnStateRef = useRef(false);
  const tenantId = useSessionStore((state) => state.selectedTenantId);
  const userId = useSessionStore((state) => state.userId);

  const selectedKeySet = useMemo(() => new Set(selectedRowKeys.map((item) => String(item))), [selectedRowKeys]);
  const selectedInGridCount = useMemo(
    () =>
      rows.reduce((count, record) => {
        const rowKey = resolveOrderRowKey(record);
        if (!rowKey) return count;
        return selectedKeySet.has(String(rowKey)) ? count + 1 : count;
      }, 0),
    [rows, selectedKeySet],
  );

  useEffect(() => {
    const api = apiRef.current;
    if (!api) return;
    syncingSelectionRef.current = true;
    api.forEachNode((node) => {
      if (!node.data) return;
      const key = resolveOrderRowKey(node.data);
      const shouldSelect = Boolean(key) && selectedKeySet.has(String(key));
      if (node.isSelected() !== shouldSelect) {
        node.setSelected(shouldSelect);
      }
    });
    syncingSelectionRef.current = false;
  }, [selectedKeySet]);

  useEffect(() => {
    redrawFocusedRows({
      api: apiRef.current,
      previousFocusedRowKey: previousFocusedRowKeyRef.current,
      nextFocusedRowKey: focusedOrderKey,
    });
    previousFocusedRowKeyRef.current = focusedOrderKey;
  }, [focusedOrderKey, rows]);

  const resetViewportToTop = useCallback((api?: GridApi<OrderSummary> | null) => {
    const targetApi = api ?? apiRef.current;
    if (!targetApi) {
      return;
    }

    targetApi.clearFocusedCell?.();
    targetApi.ensureIndexVisible(0, 'top');
    targetApi.resetRowHeights();
    const bodyViewport = shellRef.current?.querySelector<HTMLElement>('.ag-body-viewport');
    if (bodyViewport && bodyViewport.scrollTop !== 0) {
      bodyViewport.scrollTop = 0;
    }
  }, []);

  useEffect(() => {
    const api = apiRef.current;
    if (!api) {
      return;
    }

    shouldResetViewportRef.current = true;
    queueMicrotask(() => {
      if (!shouldResetViewportRef.current) {
        return;
      }
      resetViewportToTop(api);
    });
  }, [resetViewportToTop, rows]);

  useEffect(() => {
    if (!gridApi) return;
    const nextFilterModel = buildMainGridFilterModel(columnFilters);
    const currentFilterModel = gridApi.getFilterModel();
    if (JSON.stringify(currentFilterModel) === JSON.stringify(nextFilterModel)) {
      return;
    }

    syncingFilterModelRef.current = true;
    gridApi.setFilterModel(nextFilterModel);
    queueMicrotask(() => {
      syncingFilterModelRef.current = false;
    });
  }, [columnFilters, gridApi]);

  useEffect(() => {
    onGridApiChange?.(gridApi);
    return () => {
      onGridApiChange?.(null);
    };
  }, [gridApi, onGridApiChange]);

  const platformFilterOptions = useMemo(() => {
    const map = new Map<string, FilterOption>();
    rows.forEach((record) => {
      const value = String(record.platform ?? '').trim();
      if (!value) return;
      map.set(value, { value, label: platformLabel(value) });
    });

    const currentValue = String(columnFilters.platform?.[0] ?? '').trim();
    if (currentValue && !map.has(currentValue)) {
      map.set(currentValue, { value: currentValue, label: platformLabel(currentValue) });
    }

    return Array.from(map.values());
  }, [columnFilters.platform, rows]);

  const emitSelectionKeys = useCallback(
    (api: GridApi<OrderSummary>) => {
      const keys = api
        .getSelectedNodes()
        .map((node) => (node.data ? resolveOrderRowKey(node.data) : ''))
        .filter(Boolean);
      onSelectionChange(keys);
    },
    [onSelectionChange],
  );

  const {
    handleCellClicked,
    handleCellFocused,
    handleCellKeyDown,
    handleGridMouseDownCapture,
    handleRowClicked,
    handleSpaceKeyDownCapture,
    selectionDiagnostic,
    selectionDiagnosticsEnabled,
  } = useAgGridWorkbenchSelection<OrderSummary>({
    apiRef,
    debugKey: 'order-review-data-grid',
    syncingSelectionRef,
    focusedRowKey: focusedOrderKey,
    resolveRowKey: (record) => resolveOrderRowKey(record) || null,
    emitSelectionKeys,
    onRowActive,
    onFocus,
  });

  const colDefs = useMemo<ColDef<OrderSummary>[]>(
    () =>
      buildOrderReviewMainGridColDefs({
        logisticsOptions,
        onToggleAllSelection: () => {
          const api = apiRef.current;
          if (!api) return;
          syncingSelectionRef.current = true;
          toggleAgGridHeaderSelection({
            headerChecked: rows.length > 0 && selectedInGridCount === rows.length,
            deselectAll: () => api.deselectAll(),
            forEachNodeAfterFilter: (callback) => {
              api.forEachNodeAfterFilter(callback);
            },
          });
          syncingSelectionRef.current = false;
          onSelectionChange(
            headerChecked
              ? []
              : api
                  .getSelectedNodes()
                  .map((node) => (node.data ? resolveOrderRowKey(node.data) : ''))
                  .filter(Boolean),
          );
        },
        platformFilterOptions,
        shopOptions,
      }),
    [
      logisticsOptions,
      onSelectionChange,
      platformFilterOptions,
      rows.length,
      selectedInGridCount,
      shopOptions,
    ],
  );

  const defaultColDef = useMemo<ColDef<OrderSummary>>(
    () => buildOrderReviewMainGridDefaultColDef(quickFilterExpanded),
    [quickFilterExpanded],
  );
  const columnStateStorageKey = useMemo(
    () =>
      buildAgGridColumnStateStorageKey({
        columnSettingsKey: 'orders.review.main-grid',
        tenantId,
        userId,
      }),
    [tenantId, userId],
  );

  const getMainMenuItems = useCallback((params: GetMainMenuItemsParams<OrderSummary>) => {
    const colId = params.column?.getColId?.() ?? '';
    if (!colId || mainGridStructuralColumnIds.has(colId)) {
      return [];
    }
    return Array.from(resolveAgGridMainMenuItems(colId));
  }, []);

  const schedulePersistColumnState = useCallback(
    (api?: GridApi<OrderSummary> | null) => {
      if (!columnStateStorageKey || typeof window === 'undefined') {
        return;
      }
      if (applyingPersistedColumnStateRef.current) {
        return;
      }

      const targetApi = api ?? apiRef.current;
      if (!targetApi) {
        return;
      }

      if (columnStatePersistTimerRef.current != null) {
        window.clearTimeout(columnStatePersistTimerRef.current);
      }

      columnStatePersistTimerRef.current = window.setTimeout(() => {
        persistAgGridColumnState(window.localStorage, columnStateStorageKey, targetApi.getColumnState());
        columnStatePersistTimerRef.current = null;
      }, 120);
    },
    [columnStateStorageKey],
  );

  const registerGridApiForTesting = useCallback(
    (api: GridApi<OrderSummary> | null) => {
      if (typeof window === 'undefined') {
        return;
      }

      if (!window.__ERP_GRID_TEST_APIS) {
        window.__ERP_GRID_TEST_APIS = {};
      }

      if (api) {
        window.__ERP_GRID_TEST_APIS['order-review-data-grid'] = api as GridApi<unknown>;
        return;
      }

      delete window.__ERP_GRID_TEST_APIS['order-review-data-grid'];
    },
    [],
  );

  const applyPersistedColumnState = useCallback(
    (api: GridApi<OrderSummary>) => {
      if (!columnStateStorageKey || typeof window === 'undefined') {
        return;
      }

      const persistedState = readPersistedAgGridColumnState(window.localStorage, columnStateStorageKey);
      if (persistedState.length === 0) {
        return;
      }

      applyingPersistedColumnStateRef.current = true;
      api.applyColumnState({
        state: persistedState,
        applyOrder: true,
      });
      queueMicrotask(() => {
        applyingPersistedColumnStateRef.current = false;
      });
    },
    [columnStateStorageKey],
  );

  useEffect(
    () => () => {
      if (columnStatePersistTimerRef.current != null && typeof window !== 'undefined') {
        window.clearTimeout(columnStatePersistTimerRef.current);
      }
      registerGridApiForTesting(null);
    },
    [registerGridApiForTesting],
  );

  const handleFilterChanged = useCallback(
    (event: FilterChangedEvent<OrderSummary>) => {
      if (syncingFilterModelRef.current) return;
      const nextManagedFilters = extractMainGridColumnFilters(event.api.getFilterModel() as FilterModel);
      const nextFilters = {
        ...columnFilters,
        ...nextManagedFilters,
      };
      if (hasSameMainGridColumnFilters(columnFilters, nextFilters)) {
        return;
      }
      schedulePersistColumnState(event.api);
      onColumnFiltersChange(nextFilters);
    },
    [columnFilters, onColumnFiltersChange, schedulePersistColumnState],
  );

  const handleSelectionChanged = useCallback(
    (event: SelectionChangedEvent<OrderSummary>) => {
      if (syncingSelectionRef.current) return;
      const keys = event.api
        .getSelectedNodes()
        .map((node) => (node.data ? resolveOrderRowKey(node.data) : ''))
        .filter(Boolean);
      onSelectionChange(keys);
    },
    [onSelectionChange],
  );

  const handleCellContextMenu = useCallback(
    (event: CellContextMenuEvent<OrderSummary>) => {
      event.event?.preventDefault?.();
      if (!event.data || !event.event) return;
      onContextMenu(event.data, event.event as MouseEvent);
    },
    [onContextMenu],
  );

  const handleRowDoubleClicked = useCallback(
    (event: RowDoubleClickedEvent<OrderSummary>) => {
      if (!event.data) return;
      onRowDoubleClick(event.data);
    },
    [onRowDoubleClick],
  );

  const handleColumnHeaderContextMenu = useCallback((event: ColumnHeaderContextMenuEvent<OrderSummary>) => {
    const nativeEvent = event.event as MouseEvent | undefined;
    nativeEvent?.preventDefault?.();
    nativeEvent?.stopPropagation?.();

    const column = 'getColId' in event.column ? event.column : null;
    const colId = column?.getColId?.() ?? '';
    if (!column || !colId || mainGridStructuralColumnIds.has(colId)) {
      return;
    }

    event.api.hidePopupMenu();
    event.api.showColumnMenu(column);
  }, []);

  const handleColumnStateChanged = useCallback(
    (event: {
      api: GridApi<OrderSummary>;
      finished?: boolean;
    }) => {
      if (typeof event.finished === 'boolean' && !event.finished) {
        return;
      }
      schedulePersistColumnState(event.api);
    },
    [schedulePersistColumnState],
  );

  const getRowId = useCallback(
    (params: GetRowIdParams<OrderSummary>) => resolveOrderRowKey(params.data) || `row-${params.rowIndex ?? 0}`,
    [],
  );

  const handleModelUpdated = useCallback(
    (api: GridApi<OrderSummary>) => {
      if (!shouldResetViewportRef.current) {
        return;
      }
      shouldResetViewportRef.current = false;
      resetViewportToTop(api);
      if (typeof window !== 'undefined') {
        window.requestAnimationFrame(() => {
          resetViewportToTop(api);
          window.requestAnimationFrame(() => {
            resetViewportToTop(api);
          });
        });
      }
    },
    [resetViewportToTop],
  );

  const handleFirstDataRendered = useCallback(
    (api: GridApi<OrderSummary>) => {
      resetViewportToTop(api);
      if (typeof window !== 'undefined') {
        window.requestAnimationFrame(() => {
          resetViewportToTop(api);
          window.requestAnimationFrame(() => {
            resetViewportToTop(api);
          });
        });
      }
    },
    [resetViewportToTop],
  );

  return (
    <div
      ref={shellRef}
      className="ag-theme-quartz order-review-main-grid-shell"
      data-testid="order-review-data-grid"
      onKeyDownCapture={handleSpaceKeyDownCapture}
      onMouseDownCapture={handleGridMouseDownCapture}
    >
      {error ? (
        <div className="order-review-main-grid-state">
          <Alert
            type="error"
            showIcon
            message="订单加载失败"
            description={error instanceof Error ? error.message : '请稍后重试'}
            action={
              <Button size="small" onClick={onRetry}>
                重试
              </Button>
            }
          />
        </div>
      ) : (
        <AgGridReact<OrderSummary>
          rowData={rows}
          columnDefs={colDefs}
          defaultColDef={defaultColDef}
          localeText={ERP_AG_GRID_LOCALE_TEXT_ZH_CN}
          getRowId={getRowId}
          rowSelection={{
            mode: 'multiRow',
            checkboxes: false,
            headerCheckbox: false,
            enableClickSelection: false,
          }}
          suppressRowClickSelection
          suppressClickEdit
          singleClickEdit={false}
          readOnlyEdit
          rowHeight={28}
          headerHeight={32}
          animateRows={false}
          cellSelection
          suppressContextMenu
          enableCellTextSelection={false}
          preventDefaultOnContextMenu
          rowBuffer={10}
          suppressRowVirtualisation={false}
          suppressColumnVirtualisation
          suppressRowTransform
          debounceVerticalScrollbar
          getMainMenuItems={getMainMenuItems}
          onGridReady={(event: GridReadyEvent<OrderSummary>) => {
            apiRef.current = event.api;
            registerGridApiForTesting(event.api);
            applyPersistedColumnState(event.api);
            setGridApi(event.api);
            queueMicrotask(() => {
              resetViewportToTop(event.api);
            });
          }}
          onFirstDataRendered={(event) => {
            handleFirstDataRendered(event.api);
          }}
          onColumnHeaderContextMenu={handleColumnHeaderContextMenu}
          onColumnMoved={handleColumnStateChanged}
          onColumnVisible={handleColumnStateChanged}
          onColumnPinned={handleColumnStateChanged}
          onColumnResized={handleColumnStateChanged}
          onModelUpdated={(event) => {
            handleModelUpdated(event.api);
          }}
          onFilterChanged={handleFilterChanged}
          onSelectionChanged={handleSelectionChanged}
          onCellClicked={handleCellClicked}
          onCellFocused={handleCellFocused}
          onCellKeyDown={handleCellKeyDown}
          onRowClicked={handleRowClicked}
          onRowDoubleClicked={handleRowDoubleClicked}
          onCellContextMenu={handleCellContextMenu}
          getRowClass={(params) => {
            const row = params.data;
            if (!row) return 'erp-data-grid-row';
            const rowKey = resolveOrderRowKey(row);
            if (!rowKey) return 'erp-data-grid-row';
            if (rowKey === focusedOrderKey) {
              return 'erp-data-grid-row erp-data-grid-row-active order-review-main-row-active';
            }
            return 'erp-data-grid-row';
          }}
        />
      )}

      {!error && !loading && rows.length === 0 ? (
        <div className="order-review-main-grid-empty">
          <Empty description="暂无订单" />
        </div>
      ) : null}

      {loading ? (
        <div className="order-review-main-grid-loading-mask">
          <Spin size="large" tip="加载中..." />
        </div>
      ) : null}

      {selectionDiagnosticsEnabled ? (
        <div
          className="order-review-selection-diagnostic"
          data-testid="order-review-selection-diagnostic"
          data-action={selectionDiagnostic?.action ?? 'idle'}
          data-col-id={selectionDiagnostic?.colId ?? ''}
          data-hit-zone={selectionDiagnostic?.hitZone ?? 'idle'}
          data-row-key={selectionDiagnostic?.rowKey ?? ''}
        >
          {selectionDiagnostic?.summary ?? '点击诊断：等待操作'}
        </div>
      ) : null}
    </div>
  );
};

export const buildOrderReviewMainGridColDefs = ({
  logisticsOptions,
  onToggleAllSelection,
  platformFilterOptions,
  shopOptions,
}: BuildOrderReviewMainGridColDefsOptions): ColDef<OrderSummary>[] =>
  [
      {
        colId: '__erpSelection',
        headerName: '',
        width: 36,
        minWidth: 28,
        maxWidth: 44,
        resizable: true,
        sortable: false,
        filter: false,
        headerClass: 'order-review-main-header-center order-review-main-selection-header-cell',
        cellClass: 'order-review-main-selection-cell',
    cellRenderer: (params: ICellRendererParams<OrderSummary>) => {
      const rowNode = params.node;
      if (!rowNode?.data) return null;

      return (
        <label
          className={`ag-selection-checkbox${rowNode.isSelected() ? ' is-checked' : ''}`}
          title="勾选/取消勾选"
        >
          <input
            type="checkbox"
            className="ag-checkbox-input"
            checked={Boolean(rowNode.isSelected())}
            readOnly
            tabIndex={-1}
            aria-label="勾选/取消勾选"
          />
            </label>
          );
        },
        headerComponent: AgGridSelectionHeaderCheckbox,
        headerComponentParams: {
          onToggleAllSelection,
          testId: 'erp-selection-header-checkbox',
        },
      },
      {
        colId: '__erpRowIndex',
        headerName: '',
        width: 48,
        minWidth: 38,
        maxWidth: 58,
        sortable: false,
        filter: false,
        resizable: true,
        valueGetter: (params) => Number(params.node?.rowIndex ?? 0) + 1,
        headerClass: 'order-review-main-header-center order-review-main-row-index-header',
        cellClass: 'order-review-main-row-index-cell',
      },
      {
        colId: 'id',
        headerName: '订单号',
        field: 'id',
        width: 168,
        minWidth: 62,
        sortable: true,
        filter: 'agTextColumnFilter',
        filterParams: {
          filterOptions: ['contains'],
          defaultOption: 'contains',
          maxNumConditions: 1,
        },
        floatingFilterComponent: OrderReviewTextFloatingFilter,
        floatingFilterComponentParams: {
          testId: 'order-review-floating-filter-id',
          placeholder: '筛订单号',
        },
      },
      {
        colId: 'platform_order_id',
        headerName: '平台单号',
        field: 'platform_order_id',
        width: 168,
        minWidth: 62,
        sortable: true,
        filter: 'agTextColumnFilter',
        filterParams: {
          filterOptions: ['contains'],
          defaultOption: 'contains',
          maxNumConditions: 1,
        },
        floatingFilterComponent: OrderReviewTextFloatingFilter,
        floatingFilterComponentParams: {
          testId: 'order-review-floating-filter-platform-order-id',
          placeholder: '筛平台单号',
        },
      },
      {
        colId: 'platform',
        headerName: '平台',
        field: 'platform',
        width: 110,
        minWidth: 56,
        sortable: true,
        filter: 'agSetColumnFilter',
        filterParams: {
          values: platformFilterOptions.map((option) => option.value),
        },
        floatingFilterComponent: OrderReviewSetFloatingFilter,
        floatingFilterComponentParams: {
          testId: 'order-review-floating-filter-platform',
          emptyLabel: '全部平台',
          options: platformFilterOptions,
        },
        valueFormatter: (params) => platformLabel(String(params.value ?? '-')),
      },
      {
        colId: 'created_at',
        headerName: '下单时间',
        field: 'created_at',
        width: 156,
        minWidth: 68,
        sortable: true,
        filter: 'agDateColumnFilter',
        filterParams: {
          filterOptions: ['inRange', 'greaterThanOrEqual', 'lessThanOrEqual'],
          defaultOption: 'inRange',
          maxNumConditions: 1,
        },
        filterValueGetter: (params) => {
          const raw = String(params.data?.created_at ?? '').trim();
          if (!raw) return null;
          const value = new Date(raw);
          return Number.isNaN(value.getTime()) ? null : value;
        },
        floatingFilterComponent: OrderReviewDateRangeFloatingFilter,
        floatingFilterComponentParams: {
          triggerTestId: 'order-review-floating-filter-created-at-trigger',
        },
        valueFormatter: (params) => formatDateTime(params.value as string | null | undefined),
      },
      {
        colId: 'shop_id',
        headerName: '店铺',
        field: 'shop_id',
        width: 128,
        minWidth: 60,
        sortable: true,
        filter: 'agSetColumnFilter',
        filterParams: {
          values: shopOptions.map((option) => option.value),
        },
        floatingFilterComponent: OrderReviewSetFloatingFilter,
        floatingFilterComponentParams: {
          testId: 'order-review-floating-filter-shop',
          emptyLabel: '全部店铺',
          options: shopOptions,
        },
      },
      {
        colId: 'receiver_name',
        headerName: '收件人',
        field: 'receiver_name',
        width: 120,
        minWidth: 56,
        sortable: true,
        filter: 'agTextColumnFilter',
        filterParams: {
          filterOptions: ['contains'],
          defaultOption: 'contains',
          maxNumConditions: 1,
        },
        floatingFilterComponent: OrderReviewTextFloatingFilter,
        floatingFilterComponentParams: {
          testId: 'order-review-floating-filter-receiver-name',
          placeholder: '筛收件人',
        },
      },
      {
        colId: 'receiver_phone',
        headerName: '手机号',
        field: 'receiver_phone',
        width: 130,
        minWidth: 60,
        sortable: true,
        filter: 'agTextColumnFilter',
        filterParams: {
          filterOptions: ['contains'],
          defaultOption: 'contains',
          maxNumConditions: 1,
        },
        floatingFilterComponent: OrderReviewTextFloatingFilter,
        floatingFilterComponentParams: {
          testId: 'order-review-floating-filter-receiver-phone',
          placeholder: '筛手机号',
        },
      },
      {
        colId: 'logistics_company',
        headerName: '物流公司',
        field: 'logistics_company',
        width: 132,
        minWidth: 60,
        sortable: true,
        filter: 'agSetColumnFilter',
        filterParams: {
          values: logisticsOptions.map((option) => option.value),
        },
        floatingFilterComponent: OrderReviewSetFloatingFilter,
        floatingFilterComponentParams: {
          testId: 'order-review-floating-filter-logistics',
          emptyLabel: '全部物流',
          options: logisticsOptions,
        },
      },
      {
        colId: 'total_amount_cent',
        headerName: '应收合计',
        field: 'total_amount_cent',
        width: 118,
        minWidth: 60,
        sortable: true,
        filter: false,
        cellStyle: { textAlign: 'right' },
        valueFormatter: (params) => money((params.value as number | null | undefined) ?? null),
      },
    ];

export const buildOrderReviewMainGridDefaultColDef = (quickFilterExpanded: boolean): ColDef<OrderSummary> => ({
  filter: false,
  floatingFilter: quickFilterExpanded,
  resizable: true,
  minWidth: 52,
  editable: false,
  sortable: true,
  headerClass: 'order-review-main-header-center',
  suppressHeaderMenuButton: true,
  suppressHeaderFilterButton: true,
});
