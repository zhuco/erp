import { Alert, Button, Empty, Spin } from 'antd';
import type { Key } from 'react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type {
  CellContextMenuEvent,
  ColDef,
  ColumnHeaderContextMenuEvent,
  FilterChangedEvent,
  FilterModel,
  GetMainMenuItemsParams,
  GetRowIdParams,
  GridApi,
  GridReadyEvent,
  ModelUpdatedEvent,
  RowDoubleClickedEvent,
  SelectionChangedEvent,
} from 'ag-grid-community';
import { ModuleRegistry } from 'ag-grid-community';
import { AgGridReact } from 'ag-grid-react';

import type { OrderExceptionSummary } from '@/api/orders';
import { ORDER_WORKBENCH_ENTERPRISE_MODULES } from '@/components/agGridEnterpriseModules';
import {
  buildAgGridApplyColumnState,
  buildAgGridColumnStateStorageKey,
  ERP_AG_GRID_LOCALE_TEXT_ZH_CN,
  persistAgGridColumnState,
  readPersistedAgGridColumnState,
  resolveAgGridMainMenuItems,
} from '@/components/agGridColumnState';
import {
  fulfillmentStatusLabels,
  reserveStatusLabels,
} from '@/features/oms-wms-main-chain/statuses';
import { useSessionStore } from '@/features/sessionStore';
import { resolveOrderExceptionRowKey } from '@/pages/orders/exceptions-native/orderExceptionsRowKey';
import {
  OrderReviewSetFloatingFilter,
  OrderReviewTextFloatingFilter,
} from '@/pages/orders/review-native/OrderReviewGridFilters';
import { formatDateTime, money, platformLabel } from '@/pages/orders/review-native/helpers';
import { toggleAgGridHeaderSelection } from '@/pages/orders/shared/agGridSelectionHelpers';
import { redrawFocusedRows } from '@/pages/orders/shared/agGridFocusRefreshHelpers';
import {
  buildAgGridWorkbenchStructuralColDefs,
  ORDER_WORKBENCH_STRUCTURAL_COLUMN_IDS,
} from '@/pages/orders/shared/agGridWorkbenchStructuralColumns';
import { useAgGridWorkbenchSelection } from '@/pages/orders/shared/useAgGridWorkbenchSelection';
import { resolveSelectionDiagnosticsEnabled } from '@/pages/orders/shared/selectionDiagnostics';
import {
  buildOrderExceptionsMainGridFilterModel,
  extractOrderExceptionsMainGridColumnFilters,
  hasSameOrderExceptionsMainGridColumnFilters,
  mergeOrderExceptionsFilterOptionsWithCurrentValue,
  resolveOrderExceptionSlaStatus,
  resolveOrderExceptionSlaStatusLabel,
  type OrderExceptionsColumnFilters,
} from './OrderExceptionsGridFilters';

ModuleRegistry.registerModules([...ORDER_WORKBENCH_ENTERPRISE_MODULES]);

const mainGridStructuralColumnIds = ORDER_WORKBENCH_STRUCTURAL_COLUMN_IDS;

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
  slaStatusOptions: FilterOption[];
  assigneeOptions: FilterOption[];
  onToggleAllSelection: () => void;
  resolveAuditLabel: (status?: string | null) => string;
  resolveExceptionLabel: (status?: string | null) => string;
};

export type OrderExceptionsMainGridProps = {
  rows: OrderExceptionSummary[];
  filterOptionRows: OrderExceptionSummary[];
  quickFilterExpanded: boolean;
  columnFilters: OrderExceptionsColumnFilters;
  selectedRowKeys: Key[];
  focusedOrderKey: string | null;
  onGridApiChange?: (api: GridApi<OrderExceptionSummary> | null) => void;
  onColumnFiltersChange: (filters: OrderExceptionsColumnFilters) => void;
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

type SelectionColumnSnapshot = {
  columnExists: boolean;
  columnHidden: boolean;
  columnWidth: number;
  displayedRowCount: number;
  headerCheckboxCount: number;
  renderedCheckboxCount: number;
  renderedSelectionCellCount: number;
};

const selectionColumnDiagnosticsEnabled = resolveSelectionDiagnosticsEnabled();

type OrderExceptionSummaryWithOperationalFields = OrderExceptionSummary & {
  latest_action?: string | null;
  sla_deadline_at?: string | null;
  sla_remaining_minutes?: number | null;
};

const resolveFulfillmentStatusLabel = (status?: string | null) => {
  const normalized = String(status ?? '').trim().toUpperCase();
  if (!normalized) {
    return '-';
  }
  return fulfillmentStatusLabels[normalized as keyof typeof fulfillmentStatusLabels] ?? normalized;
};

const resolveReserveStatusLabel = (status?: string | null) => {
  const normalized = String(status ?? '').trim().toUpperCase();
  if (!normalized) {
    return '未预占';
  }
  return reserveStatusLabels[normalized as keyof typeof reserveStatusLabels] ?? normalized;
};

const formatOrderExceptionSlaRemainingMinutes = (record: OrderExceptionSummary): string => {
  const raw = (record as OrderExceptionSummaryWithOperationalFields).sla_remaining_minutes;
  if (!Number.isFinite(raw)) {
    return '-';
  }
  const minutes = Math.trunc(raw as number);
  if (minutes < 0) {
    return `超时 ${Math.abs(minutes)} 分`;
  }
  return `${minutes} 分`;
};

export const OrderExceptionsMainGrid = ({
  rows,
  filterOptionRows,
  quickFilterExpanded,
  columnFilters,
  selectedRowKeys,
  focusedOrderKey,
  onGridApiChange,
  onColumnFiltersChange,
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
  const [selectionColumnSnapshot, setSelectionColumnSnapshot] = useState<SelectionColumnSnapshot | null>(null);
  const syncingFilterModelRef = useRef(false);
  const syncingSelectionRef = useRef(false);
  const columnStatePersistTimerRef = useRef<number | null>(null);
  const applyingPersistedColumnStateRef = useRef(false);
  const recoveringSelectionColumnRef = useRef(false);
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

  const refreshSelectionColumn = useCallback((api?: GridApi<OrderExceptionSummary> | null) => {
    const targetApi = api ?? apiRef.current;
    if (!targetApi) {
      return;
    }

    targetApi.refreshCells({
      force: true,
      columns: ['__erpSelection'],
    });
  }, []);

  const collectSelectionColumnSnapshot = useCallback(
    (api?: GridApi<OrderExceptionSummary> | null): SelectionColumnSnapshot | null => {
      const targetApi = api ?? apiRef.current;
      if (!targetApi) {
        return null;
      }

      const selectionColumn = targetApi.getColumn?.('__erpSelection') ?? null;
      const root = shellRef.current;
      const renderedSelectionCells = root?.querySelectorAll('.ag-cell[col-id="__erpSelection"]') ?? null;
      const renderedCheckboxes =
        root?.querySelectorAll('.ag-cell[col-id="__erpSelection"] .ag-selection-checkbox') ?? null;
      const headerCheckboxes =
        root?.querySelectorAll('[data-testid="erp-selection-header-checkbox"]') ?? null;

      return {
        columnExists: Boolean(selectionColumn),
        columnHidden: selectionColumn?.isVisible?.() === false,
        columnWidth: selectionColumn?.getActualWidth?.() ?? 0,
        displayedRowCount: targetApi.getDisplayedRowCount(),
        headerCheckboxCount: headerCheckboxes?.length ?? 0,
        renderedCheckboxCount: renderedCheckboxes?.length ?? 0,
        renderedSelectionCellCount: renderedSelectionCells?.length ?? 0,
      };
    },
    [],
  );

  const publishSelectionColumnSnapshot = useCallback(
    (api?: GridApi<OrderExceptionSummary> | null) => {
      if (!selectionColumnDiagnosticsEnabled) {
        return null;
      }
      const snapshot = collectSelectionColumnSnapshot(api);
      setSelectionColumnSnapshot(snapshot);
      return snapshot;
    },
    [collectSelectionColumnSnapshot],
  );

  const ensureSelectionColumnRendered = useCallback(
    (api?: GridApi<OrderExceptionSummary> | null) => {
      const targetApi = api ?? apiRef.current;
      if (!targetApi || recoveringSelectionColumnRef.current) {
        return;
      }

      const snapshot = publishSelectionColumnSnapshot(targetApi) ?? collectSelectionColumnSnapshot(targetApi);
      if (!snapshot) {
        return;
      }

      const shouldRecover =
        snapshot.columnExists &&
        !snapshot.columnHidden &&
        snapshot.displayedRowCount > 0 &&
        snapshot.renderedSelectionCellCount > 0 &&
        snapshot.renderedCheckboxCount === 0;

      if (!shouldRecover) {
        return;
      }

      recoveringSelectionColumnRef.current = true;
      targetApi.redrawRows();
      refreshSelectionColumn(targetApi);
      if (typeof window === 'undefined') {
        recoveringSelectionColumnRef.current = false;
        return;
      }

      window.requestAnimationFrame(() => {
        refreshSelectionColumn(targetApi);
        publishSelectionColumnSnapshot(targetApi);
        recoveringSelectionColumnRef.current = false;
      });
    },
    [collectSelectionColumnSnapshot, publishSelectionColumnSnapshot, refreshSelectionColumn],
  );

  const scheduleSelectionColumnRefresh = useCallback((api?: GridApi<OrderExceptionSummary> | null) => {
    const targetApi = api ?? apiRef.current;
    if (!targetApi) {
      return;
    }

    refreshSelectionColumn(targetApi);
    publishSelectionColumnSnapshot(targetApi);
    ensureSelectionColumnRendered(targetApi);
    if (typeof window === 'undefined') {
      return;
    }

    window.requestAnimationFrame(() => {
      refreshSelectionColumn(targetApi);
      publishSelectionColumnSnapshot(targetApi);
      ensureSelectionColumnRendered(targetApi);
      window.requestAnimationFrame(() => {
        refreshSelectionColumn(targetApi);
        publishSelectionColumnSnapshot(targetApi);
        ensureSelectionColumnRendered(targetApi);
      });
    });
  }, [ensureSelectionColumnRendered, publishSelectionColumnSnapshot, refreshSelectionColumn]);

  useEffect(() => {
    const api = gridApi ?? apiRef.current;
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
    scheduleSelectionColumnRefresh(api);
  }, [gridApi, rows, scheduleSelectionColumnRefresh, selectedKeySet]);

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
    if (!gridApi) {
      return;
    }
    const nextFilterModel = buildOrderExceptionsMainGridFilterModel(columnFilters);
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
    () =>
      mergeOrderExceptionsFilterOptionsWithCurrentValue(
        buildUniqueOptions(filterOptionRows.map((record) => record.platform), (value) => platformLabel(value)),
        columnFilters.platform?.[0],
        (value) => platformLabel(value),
      ),
    [buildUniqueOptions, columnFilters.platform, filterOptionRows],
  );
  const shopOptions = useMemo(
    () =>
      mergeOrderExceptionsFilterOptionsWithCurrentValue(
        buildUniqueOptions(filterOptionRows.map((record) => record.shop_id)),
        columnFilters.shopId?.[0],
      ),
    [buildUniqueOptions, columnFilters.shopId, filterOptionRows],
  );
  const warehouseOptions = useMemo(
    () =>
      mergeOrderExceptionsFilterOptionsWithCurrentValue(
        buildUniqueOptions(filterOptionRows.map((record) => record.target_warehouse_id), (value) => value || '未指定仓库'),
        columnFilters.warehouseId?.[0],
        (value) => value || '未指定仓库',
      ),
    [buildUniqueOptions, columnFilters.warehouseId, filterOptionRows],
  );
  const auditStatusOptions = useMemo(
    () =>
      mergeOrderExceptionsFilterOptionsWithCurrentValue(
        buildUniqueOptions(filterOptionRows.map((record) => record.audit_status), (value) => resolveAuditLabel(value)),
        columnFilters.auditStatus?.[0],
        (value) => resolveAuditLabel(value),
      ),
    [buildUniqueOptions, columnFilters.auditStatus, filterOptionRows, resolveAuditLabel],
  );
  const exceptionStatusOptions = useMemo(
    () =>
      mergeOrderExceptionsFilterOptionsWithCurrentValue(
        buildUniqueOptions(filterOptionRows.map((record) => record.exception_status), (value) => resolveExceptionLabel(value)),
        columnFilters.exceptionStatus?.[0],
        (value) => resolveExceptionLabel(value),
      ),
    [buildUniqueOptions, columnFilters.exceptionStatus, filterOptionRows, resolveExceptionLabel],
  );
  const slaStatusOptions = useMemo(
    () =>
      mergeOrderExceptionsFilterOptionsWithCurrentValue(
        buildUniqueOptions(filterOptionRows.map((record) => resolveOrderExceptionSlaStatus(record)), (value) =>
          resolveOrderExceptionSlaStatusLabel(value),
        ),
        columnFilters.slaStatus?.[0],
        (value) => resolveOrderExceptionSlaStatusLabel(value),
      ),
    [buildUniqueOptions, columnFilters.slaStatus, filterOptionRows],
  );
  const assigneeOptions = useMemo(
    () =>
      mergeOrderExceptionsFilterOptionsWithCurrentValue(
        buildUniqueOptions(filterOptionRows.map((record) => record.assignee_name), (value) => value || '未分派'),
        columnFilters.assigneeName?.[0],
        (value) => value || '未分派',
      ),
    [buildUniqueOptions, columnFilters.assigneeName, filterOptionRows],
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
        slaStatusOptions,
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
      slaStatusOptions,
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
        state: buildAgGridApplyColumnState(persistedState),
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
    (event: FilterChangedEvent<OrderExceptionSummary>) => {
      event.api.refreshHeader();
      schedulePersistColumnState(event.api);
      onDisplayedRowsChange?.(event.api.getDisplayedRowCount());
      if (syncingFilterModelRef.current) {
        return;
      }

      const nextManagedFilters = extractOrderExceptionsMainGridColumnFilters(
        event.api.getFilterModel() as FilterModel,
      );
      const nextFilters = {
        ...columnFilters,
        ...nextManagedFilters,
      };
      if (hasSameOrderExceptionsMainGridColumnFilters(columnFilters, nextFilters)) {
        return;
      }
      onColumnFiltersChange(nextFilters);
    },
    [columnFilters, onColumnFiltersChange, onDisplayedRowsChange, schedulePersistColumnState],
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
      scheduleSelectionColumnRefresh(event.api);
      if (typeof window !== 'undefined') {
        window.requestAnimationFrame(() => {
          resetViewportToTop(event.api);
          window.requestAnimationFrame(() => {
            resetViewportToTop(event.api);
          });
        });
      }
    },
    [onDisplayedRowsChange, resetViewportToTop, scheduleSelectionColumnRefresh],
  );

  const handleFirstDataRendered = useCallback(
    (api: GridApi<OrderExceptionSummary>) => {
      resetViewportToTop(api);
      scheduleSelectionColumnRefresh(api);
      if (typeof window !== 'undefined') {
        window.requestAnimationFrame(() => {
          resetViewportToTop(api);
          window.requestAnimationFrame(() => {
            resetViewportToTop(api);
          });
        });
      }
    },
    [resetViewportToTop, scheduleSelectionColumnRefresh],
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
          theme="legacy"
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
          enableFilterHandlers
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
              scheduleSelectionColumnRefresh(event.api);
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
          data-selection-column-exists={selectionColumnSnapshot?.columnExists ? 'true' : 'false'}
          data-selection-column-hidden={selectionColumnSnapshot?.columnHidden ? 'true' : 'false'}
          data-selection-column-width={String(selectionColumnSnapshot?.columnWidth ?? 0)}
          data-selection-column-rendered-cells={String(selectionColumnSnapshot?.renderedSelectionCellCount ?? 0)}
          data-selection-column-rendered-checkboxes={String(selectionColumnSnapshot?.renderedCheckboxCount ?? 0)}
        >
          {`${selectionDiagnostic?.summary ?? '点击诊断：等待操作'} | 选择列: ${
            selectionColumnSnapshot?.columnExists ? '存在' : '缺失'
          }, 隐藏=${selectionColumnSnapshot?.columnHidden ? '是' : '否'}, 宽度=${
            selectionColumnSnapshot?.columnWidth ?? 0
          }, 单元格=${selectionColumnSnapshot?.renderedSelectionCellCount ?? 0}, 复选框=${
            selectionColumnSnapshot?.renderedCheckboxCount ?? 0
          }`}
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
  slaStatusOptions,
  assigneeOptions,
  onToggleAllSelection,
  resolveAuditLabel,
  resolveExceptionLabel,
}: BuildOrderExceptionsMainGridColDefsOptions): ColDef<OrderExceptionSummary>[] => {
  void slaStatusOptions;
  return [
    ...buildAgGridWorkbenchStructuralColDefs<OrderExceptionSummary>({
      onToggleAllSelection,
      pinLeft: true,
    }),
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
    colId: 'fulfillment_status',
    headerName: '履约状态',
    field: 'fulfillment_status',
    width: 126,
    minWidth: 78,
    sortable: true,
    filter: false,
    valueFormatter: (params) => resolveFulfillmentStatusLabel(String(params.value ?? '')),
  },
  {
    colId: 'reserve_status',
    headerName: '库存占用',
    field: 'reserve_status',
    width: 118,
    minWidth: 78,
    sortable: true,
    filter: false,
    valueFormatter: (params) => resolveReserveStatusLabel(params.value as string | null | undefined),
  },
  {
    colId: 'sla_status',
    headerName: 'SLA 状态',
    width: 112,
    minWidth: 72,
    sortable: true,
    filter: false,
    filterValueGetter: (params) =>
      params.data ? resolveOrderExceptionSlaStatus(params.data) : 'NORMAL',
    valueGetter: (params) =>
      params.data ? resolveOrderExceptionSlaStatus(params.data) : 'NORMAL',
    valueFormatter: (params) => resolveOrderExceptionSlaStatusLabel(String(params.value ?? 'NORMAL')),
  },
  {
    colId: 'sla_remaining_minutes',
    headerName: 'SLA 剩余',
    width: 116,
    minWidth: 76,
    sortable: true,
    filter: false,
    valueGetter: (params) => (params.data ? formatOrderExceptionSlaRemainingMinutes(params.data) : '-'),
  },
  {
    colId: 'sla_deadline_at',
    headerName: 'SLA 截止',
    width: 156,
    minWidth: 84,
    sortable: true,
    filter: false,
    valueGetter: (params) =>
      params.data
        ? formatDateTime(
            (params.data as OrderExceptionSummaryWithOperationalFields).sla_deadline_at ?? null,
          )
        : '-',
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
    colId: 'latest_action',
    headerName: '最近处理动作',
    field: 'latest_action',
    width: 168,
    minWidth: 96,
    sortable: true,
    filter: false,
    valueGetter: (params) =>
      String(
        (params.data as OrderExceptionSummaryWithOperationalFields | undefined)?.latest_action ?? '-',
      ),
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
};
