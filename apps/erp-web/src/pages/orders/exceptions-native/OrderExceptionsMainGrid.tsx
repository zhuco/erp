import { Alert, Button, Empty, Spin } from 'antd';
import type { Key } from 'react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type {
  CellContextMenuEvent,
  ColDef,
  ICellRendererParams,
  ColumnHeaderContextMenuEvent,
  GetMainMenuItemsParams,
  GetRowIdParams,
  GridApi,
  GridReadyEvent,
  ModelUpdatedEvent,
  RowDoubleClickedEvent,
  SelectionChangedEvent,
} from 'ag-grid-community';
import { ModuleRegistry } from 'ag-grid-community';
import { AllEnterpriseModule } from 'ag-grid-enterprise';
import { AgGridReact } from 'ag-grid-react';

import type { OrderExceptionSummary } from '@/api/orders';
import {
  buildAgGridColumnStateStorageKey,
  ERP_AG_GRID_LOCALE_TEXT_ZH_CN,
  persistAgGridColumnState,
  readPersistedAgGridColumnState,
  resolveAgGridMainMenuItems,
} from '@/components/agGridColumnState';
import { useSessionStore } from '@/features/sessionStore';
import { resolveOrderExceptionRowKey } from '@/pages/orders/exceptions-native/orderExceptionsRowKey';
import {
  OrderReviewSetFloatingFilter,
  OrderReviewTextFloatingFilter,
} from '@/pages/orders/review-native/OrderReviewGridFilters';
import { formatDateTime, money, platformLabel } from '@/pages/orders/review-native/helpers';
import { toggleAgGridHeaderSelection } from '@/pages/orders/shared/agGridSelectionHelpers';
import { AgGridSelectionHeaderCheckbox } from '@/pages/orders/shared/AgGridSelectionHeaderCheckbox';
import { redrawFocusedRows } from '@/pages/orders/shared/agGridFocusRefreshHelpers';
import { useAgGridWorkbenchSelection } from '@/pages/orders/shared/useAgGridWorkbenchSelection';

ModuleRegistry.registerModules([AllEnterpriseModule]);

const mainGridStructuralColumnIds = new Set(['__erpSelection', '__erpRowIndex']);

type FilterOption = {
  label: string;
  value: string;
};

type BuildOrderExceptionsMainGridColDefsOptions = {
  platformOptions: FilterOption[];
  shopOptions: FilterOption[];
  warehouseOptions: FilterOption[];
  auditStatusOptions: FilterOption[];
  exceptionStatusOptions: FilterOption[];
  assigneeOptions: FilterOption[];
  onToggleAllSelection: () => void;
  resolveAuditLabel: (status?: string | null) => string;
  resolveExceptionLabel: (status?: string | null) => string;
};

type OrderExceptionsMainGridProps = {
  rows: OrderExceptionSummary[];
  quickFilterExpanded: boolean;
  selectedRowKeys: Key[];
  focusedOrderKey: string | null;
  onGridApiChange?: (api: GridApi<OrderExceptionSummary> | null) => void;
  onSelectionChange: (keys: Key[]) => void;
  onRowActive: (record: OrderExceptionSummary) => void;
  onRowDoubleClick: (record: OrderExceptionSummary) => void;
  onFocus: (record: OrderExceptionSummary) => void;
  onContextMenu: (record: OrderExceptionSummary, event: MouseEvent) => void;
  onDisplayedRowsChange?: (count: number) => void;
  resolveAuditLabel: (status?: string | null) => string;
  resolveExceptionLabel: (status?: string | null) => string;
  loading: boolean;
  error: unknown;
  onRetry: () => void;
};

export const OrderExceptionsMainGrid = ({
  rows,
  quickFilterExpanded,
  selectedRowKeys,
  focusedOrderKey,
  onGridApiChange,
  onSelectionChange,
  onRowActive,
  onRowDoubleClick,
  onFocus,
  onContextMenu,
  onDisplayedRowsChange,
  resolveAuditLabel,
  resolveExceptionLabel,
  loading,
  error,
  onRetry,
}: OrderExceptionsMainGridProps) => {
  const shellRef = useRef<HTMLDivElement | null>(null);
  const apiRef = useRef<GridApi<OrderExceptionSummary> | null>(null);
  const shouldResetViewportRef = useRef(false);
  const previousFocusedRowKeyRef = useRef<string | null>(null);
  const [gridApi, setGridApi] = useState<GridApi<OrderExceptionSummary> | null>(null);
  const syncingSelectionRef = useRef(false);
  const columnStatePersistTimerRef = useRef<number | null>(null);
  const applyingPersistedColumnStateRef = useRef(false);
  const tenantId = useSessionStore((state) => state.selectedTenantId);
  const userId = useSessionStore((state) => state.userId);

  const selectedKeySet = useMemo(() => new Set(selectedRowKeys.map((item) => String(item))), [selectedRowKeys]);
  const selectedInGridCount = useMemo(
    () =>
      rows.reduce((count, record) => {
        const rowKey = resolveOrderExceptionRowKey(record);
        if (!rowKey) return count;
        return selectedKeySet.has(rowKey) ? count + 1 : count;
      }, 0),
    [rows, selectedKeySet],
  );

  useEffect(() => {
    const api = apiRef.current;
    if (!api) return;
    syncingSelectionRef.current = true;
    api.forEachNode((node) => {
      if (!node.data) return;
      const key = resolveOrderExceptionRowKey(node.data);
      const shouldSelect = Boolean(key) && selectedKeySet.has(key);
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

  const resetViewportToTop = useCallback((api?: GridApi<OrderExceptionSummary> | null) => {
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
    onGridApiChange?.(gridApi);
    return () => {
      onGridApiChange?.(null);
    };
  }, [gridApi, onGridApiChange]);

  const buildUniqueOptions = useCallback((items: Array<string | null | undefined>, formatter?: (value: string) => string) => {
    const map = new Map<string, FilterOption>();
    items.forEach((item) => {
      const value = String(item ?? '').trim();
      if (!value) return;
      map.set(value, {
        value,
        label: formatter ? formatter(value) : value,
      });
    });
    return Array.from(map.values());
  }, []);

  const platformOptions = useMemo(
    () => buildUniqueOptions(rows.map((record) => record.platform), (value) => platformLabel(value)),
    [buildUniqueOptions, rows],
  );
  const shopOptions = useMemo(() => buildUniqueOptions(rows.map((record) => record.shop_id)), [buildUniqueOptions, rows]);
  const warehouseOptions = useMemo(
    () => buildUniqueOptions(rows.map((record) => record.target_warehouse_id), (value) => value || '未指定仓库'),
    [buildUniqueOptions, rows],
  );
  const auditStatusOptions = useMemo(
    () => buildUniqueOptions(rows.map((record) => record.audit_status), (value) => resolveAuditLabel(value)),
    [buildUniqueOptions, resolveAuditLabel, rows],
  );
  const exceptionStatusOptions = useMemo(
    () => buildUniqueOptions(rows.map((record) => record.exception_status), (value) => resolveExceptionLabel(value)),
    [buildUniqueOptions, resolveExceptionLabel, rows],
  );
  const assigneeOptions = useMemo(
    () => buildUniqueOptions(rows.map((record) => record.assignee_name), (value) => value || '未分派'),
    [buildUniqueOptions, rows],
  );

  const emitSelectionKeys = useCallback(
    (api: GridApi<OrderExceptionSummary>) => {
      const keys = api
        .getSelectedNodes()
        .map((node) => (node.data ? resolveOrderExceptionRowKey(node.data) : ''))
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
  } = useAgGridWorkbenchSelection<OrderExceptionSummary>({
    apiRef,
    debugKey: 'order-exceptions-data-grid',
    syncingSelectionRef,
    focusedRowKey: focusedOrderKey,
    resolveRowKey: (record) => resolveOrderExceptionRowKey(record) || null,
    emitSelectionKeys,
    onRowActive,
    onFocus,
  });

  const colDefs = useMemo<ColDef<OrderExceptionSummary>[]>(
    () =>
      buildOrderExceptionsMainGridColDefs({
        platformOptions,
        shopOptions,
        warehouseOptions,
        auditStatusOptions,
        exceptionStatusOptions,
        assigneeOptions,
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
          emitSelectionKeys(api);
        },
        resolveAuditLabel,
        resolveExceptionLabel,
      }),
    [
      assigneeOptions,
      auditStatusOptions,
      exceptionStatusOptions,
      platformOptions,
      resolveAuditLabel,
      resolveExceptionLabel,
      rows.length,
      selectedInGridCount,
      shopOptions,
      warehouseOptions,
    ],
  );

  const defaultColDef = useMemo<ColDef<OrderExceptionSummary>>(
    () => ({
      filter: false,
      floatingFilter: quickFilterExpanded,
      resizable: true,
      minWidth: 52,
      editable: false,
      sortable: true,
      headerClass: 'order-review-main-header-center',
      suppressHeaderMenuButton: true,
      suppressHeaderFilterButton: true,
    }),
    [quickFilterExpanded],
  );

  const columnStateStorageKey = useMemo(
    () =>
      buildAgGridColumnStateStorageKey({
        columnSettingsKey: 'orders.exceptions.main-grid.v3',
        tenantId,
        userId,
      }),
    [tenantId, userId],
  );

  const getMainMenuItems = useCallback((params: GetMainMenuItemsParams<OrderExceptionSummary>) => {
    const colId = params.column?.getColId?.() ?? '';
    if (!colId || mainGridStructuralColumnIds.has(colId)) {
      return [];
    }
    return Array.from(resolveAgGridMainMenuItems(colId));
  }, []);

  const schedulePersistColumnState = useCallback(
    (api?: GridApi<OrderExceptionSummary> | null) => {
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

  const registerGridApiForTesting = useCallback((api: GridApi<OrderExceptionSummary> | null) => {
    if (typeof window === 'undefined') {
      return;
    }

    if (!window.__ERP_GRID_TEST_APIS) {
      window.__ERP_GRID_TEST_APIS = {};
    }

    if (api) {
      window.__ERP_GRID_TEST_APIS['order-exceptions-data-grid'] = api as GridApi<unknown>;
      return;
    }

    delete window.__ERP_GRID_TEST_APIS['order-exceptions-data-grid'];
  }, []);

  const applyPersistedColumnState = useCallback(
    (api: GridApi<OrderExceptionSummary>) => {
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

  const handleSelectionChanged = useCallback(
    (event: SelectionChangedEvent<OrderExceptionSummary>) => {
      if (syncingSelectionRef.current) return;
      emitSelectionKeys(event.api);
    },
    [emitSelectionKeys],
  );

  const handleCellContextMenu = useCallback(
    (event: CellContextMenuEvent<OrderExceptionSummary>) => {
      event.event?.preventDefault?.();
      if (!event.data || !event.event) return;
      onContextMenu(event.data, event.event as MouseEvent);
    },
    [onContextMenu],
  );

  const handleRowDoubleClicked = useCallback(
    (event: RowDoubleClickedEvent<OrderExceptionSummary>) => {
      if (!event.data) return;
      onRowDoubleClick(event.data);
    },
    [onRowDoubleClick],
  );

  const handleColumnHeaderContextMenu = useCallback((event: ColumnHeaderContextMenuEvent<OrderExceptionSummary>) => {
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
    (event: { api: GridApi<OrderExceptionSummary>; finished?: boolean }) => {
      if (typeof event.finished === 'boolean' && !event.finished) {
        return;
      }
      schedulePersistColumnState(event.api);
    },
    [schedulePersistColumnState],
  );

  const handleModelUpdated = useCallback(
    (event: ModelUpdatedEvent<OrderExceptionSummary>) => {
      onDisplayedRowsChange?.(event.api.getDisplayedRowCount());
      if (!shouldResetViewportRef.current) {
        return;
      }
      shouldResetViewportRef.current = false;
      resetViewportToTop(event.api);
      if (typeof window !== 'undefined') {
        window.requestAnimationFrame(() => {
          resetViewportToTop(event.api);
          window.requestAnimationFrame(() => {
            resetViewportToTop(event.api);
          });
        });
      }
    },
    [onDisplayedRowsChange, resetViewportToTop],
  );

  const handleFirstDataRendered = useCallback(
    (api: GridApi<OrderExceptionSummary>) => {
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

  const getRowId = useCallback((params: GetRowIdParams<OrderExceptionSummary>) => {
    const row = params.data as OrderExceptionSummary & { __erpRowId?: string };
    const stableRowId = String(row.__erpRowId ?? '').trim();
    if (stableRowId) {
      return stableRowId;
    }

    return resolveOrderExceptionRowKey(params.data) || `row-${params.rowIndex ?? 0}`;
  }, []);

  return (
    <div
      ref={shellRef}
      className="ag-theme-quartz order-review-main-grid-shell"
      data-testid="order-exceptions-data-grid"
      onKeyDownCapture={handleSpaceKeyDownCapture}
      onMouseDownCapture={handleGridMouseDownCapture}
    >
      {error ? (
        <div className="order-review-main-grid-state">
          <Alert
            type="error"
            showIcon
            message="订单复核加载失败"
            description={error instanceof Error ? error.message : '请稍后重试'}
            action={
              <Button size="small" onClick={onRetry}>
                重试
              </Button>
            }
          />
        </div>
      ) : (
        <AgGridReact<OrderExceptionSummary>
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
          onGridReady={(event: GridReadyEvent<OrderExceptionSummary>) => {
            apiRef.current = event.api;
            registerGridApiForTesting(event.api);
            applyPersistedColumnState(event.api);
            onDisplayedRowsChange?.(event.api.getDisplayedRowCount());
            setGridApi(event.api);
            queueMicrotask(() => {
              resetViewportToTop(event.api);
            });
          }}
          onFirstDataRendered={(event) => {
            handleFirstDataRendered(event.api);
          }}
          onModelUpdated={handleModelUpdated}
          onColumnHeaderContextMenu={handleColumnHeaderContextMenu}
          onColumnMoved={handleColumnStateChanged}
          onColumnVisible={handleColumnStateChanged}
          onColumnPinned={handleColumnStateChanged}
          onColumnResized={handleColumnStateChanged}
          onFilterChanged={(event) => {
            schedulePersistColumnState(event.api);
            onDisplayedRowsChange?.(event.api.getDisplayedRowCount());
          }}
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
            const rowKey = resolveOrderExceptionRowKey(row);
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
          <Empty description="当前没有待处理的复核订单" />
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
          data-testid="order-exceptions-selection-diagnostic"
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

export const buildOrderExceptionsMainGridColDefs = ({
  platformOptions,
  shopOptions,
  warehouseOptions,
  auditStatusOptions,
  exceptionStatusOptions,
  assigneeOptions,
  onToggleAllSelection,
  resolveAuditLabel,
  resolveExceptionLabel,
}: BuildOrderExceptionsMainGridColDefsOptions): ColDef<OrderExceptionSummary>[] => [
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
    cellRenderer: (params: ICellRendererParams<OrderExceptionSummary>) => {
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
    colId: 'order_no',
    headerName: '订单号',
    field: 'order_no',
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
      testId: 'order-exceptions-floating-filter-order-no',
      placeholder: '筛订单号',
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
      values: platformOptions.map((option) => option.value),
    },
    floatingFilterComponent: OrderReviewSetFloatingFilter,
    floatingFilterComponentParams: {
      testId: 'order-exceptions-floating-filter-platform',
      emptyLabel: '全部平台',
      options: platformOptions,
    },
    valueFormatter: (params) => platformLabel(String(params.value ?? '-')),
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
      testId: 'order-exceptions-floating-filter-shop',
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
      testId: 'order-exceptions-floating-filter-receiver-name',
      placeholder: '筛收件人',
    },
  },
  {
    colId: 'total_amount_cent',
    headerName: '金额',
    field: 'total_amount_cent',
    width: 110,
    minWidth: 60,
    sortable: true,
    filter: false,
    cellStyle: { textAlign: 'right' },
    valueFormatter: (params) => money((params.value as number | null | undefined) ?? null),
  },
  {
    colId: 'audit_status',
    headerName: '审核状态',
    field: 'audit_status',
    width: 118,
    minWidth: 72,
    sortable: true,
    filter: 'agSetColumnFilter',
    filterParams: {
      values: auditStatusOptions.map((option) => option.value),
    },
    floatingFilterComponent: OrderReviewSetFloatingFilter,
    floatingFilterComponentParams: {
      testId: 'order-exceptions-floating-filter-audit-status',
      emptyLabel: '全部审核状态',
      options: auditStatusOptions,
    },
    valueFormatter: (params) => resolveAuditLabel(String(params.value ?? '')),
  },
  {
    colId: 'exception_status',
    headerName: '异常状态',
    field: 'exception_status',
    width: 120,
    minWidth: 72,
    sortable: true,
    filter: 'agSetColumnFilter',
    filterParams: {
      values: exceptionStatusOptions.map((option) => option.value),
    },
    floatingFilterComponent: OrderReviewSetFloatingFilter,
    floatingFilterComponentParams: {
      testId: 'order-exceptions-floating-filter-exception-status',
      emptyLabel: '全部异常状态',
      options: exceptionStatusOptions,
    },
    valueFormatter: (params) => resolveExceptionLabel(String(params.value ?? '')),
  },
  {
    colId: 'target_warehouse_id',
    headerName: '发货仓',
    field: 'target_warehouse_id',
    width: 132,
    minWidth: 72,
    sortable: true,
    filter: 'agSetColumnFilter',
    filterParams: {
      values: warehouseOptions.map((option) => option.value),
    },
    floatingFilterComponent: OrderReviewSetFloatingFilter,
    floatingFilterComponentParams: {
      testId: 'order-exceptions-floating-filter-warehouse',
      emptyLabel: '全部仓库',
      options: warehouseOptions,
    },
    valueFormatter: (params) => String(params.value ?? '未指定仓库'),
  },
  {
    colId: 'assignee_name',
    headerName: '责任人',
    field: 'assignee_name',
    width: 110,
    minWidth: 72,
    sortable: true,
    filter: 'agSetColumnFilter',
    filterParams: {
      values: assigneeOptions.map((option) => option.value),
    },
    floatingFilterComponent: OrderReviewSetFloatingFilter,
    floatingFilterComponentParams: {
      testId: 'order-exceptions-floating-filter-assignee',
      emptyLabel: '全部责任人',
      options: assigneeOptions,
    },
    valueFormatter: (params) => String(params.value ?? '未分派'),
  },
  {
    colId: 'latest_action_at',
    headerName: '最近处理时间',
    field: 'latest_action_at',
    width: 156,
    minWidth: 84,
    sortable: true,
    filter: false,
    valueFormatter: (params) => formatDateTime(params.value as string | null | undefined),
  },
  {
    colId: 'blocking_reason',
    headerName: '阻断原因',
    field: 'blocking_reason',
    width: 260,
    minWidth: 100,
    sortable: true,
    filter: 'agTextColumnFilter',
    filterParams: {
      filterOptions: ['contains'],
      defaultOption: 'contains',
      maxNumConditions: 1,
    },
    floatingFilterComponent: OrderReviewTextFloatingFilter,
    floatingFilterComponentParams: {
      testId: 'order-exceptions-floating-filter-blocking-reason',
      placeholder: '筛阻断原因',
    },
  },
];
