import './IndustrialWorkbenchGrid.css';

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
  ICellRendererParams,
  RowDoubleClickedEvent,
  SelectionChangedEvent,
  SortChangedEvent,
  SuppressKeyboardEventParams,
} from 'ag-grid-community';
import { ModuleRegistry } from 'ag-grid-community';
import { AgGridReact } from 'ag-grid-react';

import type { ErpDataGridColumn } from '@/components/ErpDataGrid';
import {
  buildAgGridApplyColumnState,
  buildAgGridColumnStateStorageKey,
  ERP_AG_GRID_LOCALE_TEXT_ZH_CN,
  persistAgGridColumnState,
  readPersistedAgGridColumnState,
  resolveAgGridMainMenuItems,
} from '@/components/agGridColumnState';
import { ERP_DATA_GRID_ENTERPRISE_MODULES } from '@/components/agGridEnterpriseModules';
import {
  IndustrialWorkbenchDateRangeFloatingFilter,
  IndustrialWorkbenchSetFloatingFilter,
  IndustrialWorkbenchTextFloatingFilter,
} from '@/components/industrialWorkbenchGridFilters';
import {
  WorkbenchColumnSettingsDialog,
  applyWorkbenchGridColumnPolicy,
  sanitizeWorkbenchColumnState,
  useWorkbenchColumnSettings,
} from '@/components/page-templates';
import { useSessionStore } from '@/features/sessionStore';
import { toggleAgGridHeaderSelection } from '@/pages/orders/shared/agGridSelectionHelpers';
import {
  buildAgGridWorkbenchStructuralColDefs,
  ORDER_WORKBENCH_STRUCTURAL_COLUMN_IDS,
} from '@/pages/orders/shared/agGridWorkbenchStructuralColumns';
import {
  ORDER_WORKBENCH_GRID_HEADER_HEIGHT,
  ORDER_WORKBENCH_GRID_ROW_HEIGHT,
} from '@/pages/orders/shared/orderWorkbenchGridMetrics';
import {
  suppressAgGridSpaceKeyboardEvent,
  useAgGridWorkbenchSelection,
} from '@/pages/orders/shared/useAgGridWorkbenchSelection';

ModuleRegistry.registerModules([...ERP_DATA_GRID_ENTERPRISE_MODULES]);

type IndustrialWorkbenchGridProps<T> = {
  className?: string;
  rows: T[];
  columns: ErpDataGridColumn<T>[];
  rowKey: (record: T) => Key;
  columnSettingsKey?: string;
  dataTestId?: string;
  showColumnSettings?: boolean;
  selectedRowKeys?: Key[];
  activeRowKey?: Key | null;
  onSelectionChange?: (keys: Key[], rows: T[]) => void;
  onActiveRowChange?: (row: T) => void;
  onRowDoubleClick?: (row: T, event: MouseEvent) => void;
  onRowContextMenu?: (row: T, event: MouseEvent) => void;
  emptyText?: string;
  loading?: boolean;
  error?: unknown;
  errorTitle?: string;
  onRetry?: () => void;
  rowClassName?: string | ((record: T, index: number) => string);
  gridFilterModel?: FilterModel;
  onGridFilterModelChange?: (model: FilterModel) => void;
  rowHeight?: number;
  headerHeight?: number;
};

const isPlainObject = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value != null;

const normalizePrimitive = (value: unknown) => {
  if (value == null) {
    return null;
  }
  if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') {
    return value;
  }
  return String(value);
};

const resolveColumnPath = <T,>(column: ErpDataGridColumn<T>, fallback: string) => {
  if (column.field != null) {
    return String(column.field);
  }
  if (column.key) {
    return column.key;
  }
  return fallback;
};

const resolveColumnTitle = <T,>(column: ErpDataGridColumn<T>, fallback: string) => {
  if (typeof column.title === 'string' || typeof column.title === 'number') {
    return String(column.title);
  }
  if (column.field != null) {
    return String(column.field);
  }
  if (column.key) {
    return column.key;
  }
  return fallback;
};

const getValueByPath = (record: unknown, path: string): unknown => {
  if (!path) {
    return undefined;
  }
  const segments = path.split('.').filter(Boolean);
  let current = record;
  for (const segment of segments) {
    if (!isPlainObject(current)) {
      return undefined;
    }
    current = current[segment];
  }
  return current;
};

const toRowId = (key: Key, rowIndex: number) => {
  if (typeof key === 'string' || typeof key === 'number') {
    return String(key);
  }
  return `row-${rowIndex}`;
};

const resolveKeyString = <T,>(rowKey: (record: T) => Key, record: T | null | undefined) => {
  if (!record) {
    return null;
  }
  const normalized = String(rowKey(record) ?? '').trim();
  return normalized || null;
};

const readStaticFilterOptions = (filterParams: unknown) => {
  if (!filterParams || typeof filterParams !== 'object' || !('values' in filterParams)) {
    return [];
  }
  const values = (filterParams as { values?: unknown }).values;
  if (!Array.isArray(values)) {
    return [];
  }
  return values
    .map((value) => {
      const normalized = String(value ?? '').trim();
      return normalized ? { label: normalized, value: normalized } : null;
    })
    .filter((value): value is { label: string; value: string } => Boolean(value));
};

const applyIndustrialFloatingFilter = <T,>(colDef: ColDef<T>): ColDef<T> => {
  const colId = String(colDef.colId ?? '');
  if (ORDER_WORKBENCH_STRUCTURAL_COLUMN_IDS.has(colId)) {
    return colDef;
  }
  if (typeof colDef.floatingFilterComponent === 'function') {
    return colDef;
  }
  const filterType = typeof colDef.filter === 'string' ? colDef.filter : '';
  if (filterType === 'agSetColumnFilter') {
    const options = readStaticFilterOptions(colDef.filterParams);
    return {
      ...colDef,
      floatingFilterComponent: IndustrialWorkbenchSetFloatingFilter,
      floatingFilterComponentParams: {
        emptyLabel: `全部${String(colDef.headerName ?? '')}`.trim(),
        options,
        testId: `industrial-floating-filter-${colId}`,
      },
    };
  }
  if (filterType === 'agDateColumnFilter') {
    return {
      ...colDef,
      floatingFilterComponent: IndustrialWorkbenchDateRangeFloatingFilter,
      floatingFilterComponentParams: {
        triggerTestId: `industrial-floating-filter-${colId}-trigger`,
      },
    };
  }
  if (filterType === 'agTextColumnFilter' || filterType === 'agNumberColumnFilter') {
    return {
      ...colDef,
      floatingFilterComponent: IndustrialWorkbenchTextFloatingFilter,
      floatingFilterComponentParams: {
        placeholder: `筛选${String(colDef.headerName ?? '')}`.trim(),
        testId: `industrial-floating-filter-${colId}`,
      },
    };
  }
  return colDef;
};

const buildBusinessColDefs = <T,>(columns: ErpDataGridColumn<T>[]): ColDef<T>[] =>
  columns.map((source, index) => {
    const path = resolveColumnPath(source, String(source.key ?? `col_${index}`));
    const headerName = resolveColumnTitle(source, `列${index + 1}`);
    const {
      key: _key,
      title: _title,
      renderCell: _renderCell,
      headerSort: _headerSort,
      sorter: _sorter,
      frozen: _frozen,
      hozAlign: _hozAlign,
      headerHozAlign: _headerHozAlign,
      ...restSource
    } = source as ErpDataGridColumn<T>;
    const sourceValueGetter =
      typeof (source as { valueGetter?: unknown }).valueGetter === 'function'
        ? ((source as { valueGetter: ColDef<T>['valueGetter'] }).valueGetter ?? undefined)
        : undefined;
    const sourceCellRenderer =
      typeof (source as { cellRenderer?: unknown }).cellRenderer === 'function'
        ? ((source as { cellRenderer: ColDef<T>['cellRenderer'] }).cellRenderer ?? undefined)
        : undefined;
    const sourceComparator =
      typeof source.sorter === 'function'
        ? ((leftValue: unknown, rightValue: unknown, leftNode, rightNode) =>
            source.sorter?.(leftNode?.data ?? leftValue, rightNode?.data ?? rightValue) ?? 0)
        : undefined;

    return {
      ...(restSource as unknown as ColDef<T>),
      colId: path,
      headerName,
      field: typeof source.field === 'string' ? source.field : undefined,
      width: typeof source.width === 'number' ? source.width : undefined,
      minWidth: typeof source.minWidth === 'number' ? source.minWidth : 100,
      pinned: source.frozen ? 'left' : undefined,
      sortable: source.headerSort !== false,
      comparator: sourceComparator,
      resizable: true,
      cellStyle:
        source.hozAlign === 'right'
          ? { textAlign: 'right' }
          : source.hozAlign === 'center'
            ? { textAlign: 'center' }
            : undefined,
      valueGetter: sourceValueGetter ?? ((params) => getValueByPath(params.data, path)),
      cellRenderer:
        sourceCellRenderer ??
        ((params: ICellRendererParams<T>) => {
          const rowData = params.data as T | undefined;
          const rowIndex = Number(params.node?.rowIndex ?? 0);
          if (!rowData) {
            return '-';
          }
          if (typeof source.renderCell === 'function') {
            return source.renderCell(params.value, rowData, rowIndex);
          }
          const normalized = normalizePrimitive(params.value);
          return normalized == null || normalized === '' ? '-' : String(normalized);
        }),
    } satisfies ColDef<T>;
  });

export const IndustrialWorkbenchGrid = <T extends object>({
  className,
  rows,
  columns,
  rowKey,
  columnSettingsKey,
  dataTestId,
  showColumnSettings = false,
  selectedRowKeys = [],
  activeRowKey,
  onSelectionChange,
  onActiveRowChange,
  onRowDoubleClick,
  onRowContextMenu,
  emptyText = '暂无数据',
  loading = false,
  error,
  errorTitle = '数据加载失败',
  onRetry,
  rowClassName,
  gridFilterModel,
  onGridFilterModelChange,
  rowHeight = ORDER_WORKBENCH_GRID_ROW_HEIGHT,
  headerHeight = ORDER_WORKBENCH_GRID_HEADER_HEIGHT,
}: IndustrialWorkbenchGridProps<T>) => {
  const shellRef = useRef<HTMLDivElement | null>(null);
  const apiRef = useRef<GridApi<T> | null>(null);
  const syncingSelectionRef = useRef(false);
  const syncingFilterModelRef = useRef(false);
  const columnStatePersistTimerRef = useRef<number | null>(null);
  const applyingPersistedColumnStateRef = useRef(false);
  const [gridApi, setGridApi] = useState<GridApi<T> | null>(null);
  const tenantId = useSessionStore((state) => state.selectedTenantId);
  const userId = useSessionStore((state) => state.userId);
  const focusedRowKey = activeRowKey == null ? null : String(activeRowKey);
  const columnStateStorageKey = useMemo(
    () =>
      buildAgGridColumnStateStorageKey({
        columnSettingsKey,
        tenantId,
        userId,
      }),
    [columnSettingsKey, tenantId, userId],
  );

  const refreshStructuralColumns = useCallback((api?: GridApi<T> | null) => {
    const targetApi = api ?? apiRef.current;
    if (!targetApi) {
      return;
    }
    targetApi.refreshHeader();
    targetApi.refreshCells({
      force: true,
      columns: ['__erpSelection', '__erpRowIndex'],
    });
  }, []);

  const schedulePersistColumnState = useCallback(
    (api?: GridApi<T> | null) => {
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

  const {
    applyDraft: applyColumnSettingsDraft,
    closeDialog: closeColumnSettingsDialog,
    items: columnSettingsItems,
    open: columnSettingsOpen,
    openDialog: openColumnSettingsDialog,
    resetDraft: resetColumnSettingsDraft,
    resetToDefault: resetColumnSettingsToDefault,
    updateItem: updateColumnSettingsItem,
  } = useWorkbenchColumnSettings<T>({
    columnStateStorageKey,
    gridApi,
    onColumnStateApplied: (api) => {
      refreshStructuralColumns(api);
    },
  });

  const emitSelectionKeys = useCallback(
    (api: GridApi<T>) => {
      if (!onSelectionChange) {
        return;
      }
      const selectedNodes = api.getSelectedNodes();
      const selectedRows = selectedNodes
        .map((node) => node.data)
        .filter((row): row is T => row != null);
      const keys = selectedNodes
        .map((node) => (node.data ? rowKey(node.data) : null))
        .filter((key): key is Key => key != null);
      onSelectionChange(keys, selectedRows);
    },
    [onSelectionChange, rowKey],
  );

  const {
    handleCellClicked,
    handleCellFocused,
    handleCellKeyDown,
    handleGridMouseDownCapture,
    handleRowClicked,
    handleSpaceKeyDownCapture,
    handleSpaceKeyUpCapture,
  } = useAgGridWorkbenchSelection<T>({
    apiRef,
    syncingSelectionRef,
    focusedRowKey,
    resolveRowKey: (record) => resolveKeyString(rowKey, record),
    emitSelectionKeys,
    onRowActive: (record) => {
      onActiveRowChange?.(record);
    },
    onFocus: (record) => {
      onActiveRowChange?.(record);
    },
  });

  const handleToggleAllSelection = useCallback(() => {
    const api = apiRef.current;
    if (!api) {
      return;
    }
    const selectedKeySet = new Set(selectedRowKeys.map((item) => String(item ?? '').trim()).filter(Boolean));
    const visibleRows = rows.filter((record) => {
      const resolvedKey = resolveKeyString(rowKey, record);
      return Boolean(resolvedKey);
    });
    const headerChecked =
      visibleRows.length > 0 &&
      visibleRows.every((record) => {
        const resolvedKey = resolveKeyString(rowKey, record);
        return Boolean(resolvedKey) && selectedKeySet.has(String(resolvedKey));
      });

    syncingSelectionRef.current = true;
    toggleAgGridHeaderSelection({
      headerChecked,
      deselectAll: () => api.deselectAll(),
      forEachNodeAfterFilter: (callback) => {
        api.forEachNodeAfterFilter(callback);
      },
    });
    syncingSelectionRef.current = false;
    emitSelectionKeys(api);
    refreshStructuralColumns(api);
  }, [emitSelectionKeys, refreshStructuralColumns, rowKey, rows, selectedRowKeys]);

  const colDefs = useMemo(() => {
    const businessColDefs = buildBusinessColDefs(columns).map((colDef) => applyIndustrialFloatingFilter(colDef));

    return applyWorkbenchGridColumnPolicy<T>([
      ...buildAgGridWorkbenchStructuralColDefs<T>({
        onOpenColumnSettings: showColumnSettings ? openColumnSettingsDialog : undefined,
        onToggleAllSelection: handleToggleAllSelection,
        pinLeft: true,
      }),
      ...businessColDefs,
    ]);
  }, [columns, handleToggleAllSelection, openColumnSettingsDialog, showColumnSettings]);

  const defaultColDef = useMemo<ColDef<T>>(
    () => ({
      filter: false,
      floatingFilter: true,
      resizable: true,
      minWidth: 52,
      editable: false,
      sortable: true,
      headerClass: 'order-review-main-header-center',
      suppressHeaderMenuButton: true,
      suppressHeaderFilterButton: true,
      suppressKeyboardEvent: (params: SuppressKeyboardEventParams<T>) =>
        suppressAgGridSpaceKeyboardEvent(params.event as KeyboardEvent),
    }),
    [],
  );

  const getMainMenuItems = useCallback((params: GetMainMenuItemsParams<T>) => {
    const colId = params.column?.getColId?.() ?? '';
    if (!colId || ORDER_WORKBENCH_STRUCTURAL_COLUMN_IDS.has(colId)) {
      return [];
    }
    return Array.from(resolveAgGridMainMenuItems(colId));
  }, []);

  const handleFilterChanged = useCallback(
    (event: FilterChangedEvent<T>) => {
      if (syncingFilterModelRef.current) {
        return;
      }
      schedulePersistColumnState(event.api);
      onGridFilterModelChange?.(event.api.getFilterModel() as FilterModel);
      refreshStructuralColumns(event.api);
    },
    [onGridFilterModelChange, refreshStructuralColumns, schedulePersistColumnState],
  );

  const handleSelectionChanged = useCallback(
    (event: SelectionChangedEvent<T>) => {
      if (syncingSelectionRef.current) {
        return;
      }
      emitSelectionKeys(event.api);
      refreshStructuralColumns(event.api);
    },
    [emitSelectionKeys, refreshStructuralColumns],
  );

  const handleColumnHeaderContextMenu = useCallback((event: ColumnHeaderContextMenuEvent<T>) => {
    const column = 'getColId' in event.column ? event.column : null;
    const colId = column?.getColId?.() ?? '';
    if (!column || !colId || ORDER_WORKBENCH_STRUCTURAL_COLUMN_IDS.has(colId)) {
      return;
    }
    event.api.hidePopupMenu();
    event.api.showColumnMenu(column);
  }, []);

  const handleSortChanged = useCallback(
    (event: SortChangedEvent<T>) => {
      schedulePersistColumnState(event.api);
      refreshStructuralColumns(event.api);
    },
    [refreshStructuralColumns, schedulePersistColumnState],
  );

  const applyPersistedColumnState = useCallback(
    (api: GridApi<T>) => {
      if (!columnStateStorageKey || typeof window === 'undefined') {
        return;
      }
      const persistedState = readPersistedAgGridColumnState(window.localStorage, columnStateStorageKey);
      if (persistedState.length === 0) {
        return;
      }
      applyingPersistedColumnStateRef.current = true;
      api.applyColumnState({
        state: sanitizeWorkbenchColumnState(buildAgGridApplyColumnState(persistedState)),
        applyOrder: true,
      });
      queueMicrotask(() => {
        applyingPersistedColumnStateRef.current = false;
      });
    },
    [columnStateStorageKey],
  );

  const registerGridApiForTesting = useCallback(
    (api: GridApi<T> | null) => {
      if (typeof window === 'undefined' || !dataTestId) {
        return;
      }
      if (!window.__ERP_GRID_TEST_APIS) {
        window.__ERP_GRID_TEST_APIS = {};
      }
      if (api) {
        window.__ERP_GRID_TEST_APIS[dataTestId] = api as GridApi<unknown>;
        return;
      }
      delete window.__ERP_GRID_TEST_APIS[dataTestId];
    },
    [dataTestId],
  );

  useEffect(() => {
    const api = gridApi ?? apiRef.current;
    if (!api) {
      return;
    }
    syncingSelectionRef.current = true;
    const selectedKeySet = new Set(selectedRowKeys.map((item) => String(item ?? '').trim()).filter(Boolean));
    api.forEachNode((node) => {
      if (!node.data) {
        return;
      }
      const nextKey = resolveKeyString(rowKey, node.data);
      const shouldSelect = Boolean(nextKey) && selectedKeySet.has(nextKey);
      if (node.isSelected() !== shouldSelect) {
        node.setSelected(shouldSelect);
      }
    });
    syncingSelectionRef.current = false;
    refreshStructuralColumns(api);
  }, [gridApi, refreshStructuralColumns, rowKey, selectedRowKeys]);

  useEffect(() => {
    const api = gridApi ?? apiRef.current;
    if (!api || gridFilterModel === undefined) {
      return;
    }
    const currentFilterModel = api.getFilterModel();
    if (JSON.stringify(currentFilterModel) === JSON.stringify(gridFilterModel)) {
      return;
    }
    syncingFilterModelRef.current = true;
    api.setFilterModel(gridFilterModel);
    queueMicrotask(() => {
      syncingFilterModelRef.current = false;
    });
  }, [gridApi, gridFilterModel]);

  useEffect(
    () => () => {
      if (columnStatePersistTimerRef.current != null && typeof window !== 'undefined') {
        window.clearTimeout(columnStatePersistTimerRef.current);
      }
      registerGridApiForTesting(null);
    },
    [registerGridApiForTesting],
  );

  const getRowId = useCallback(
    (params: GetRowIdParams<T>) => {
      const rowIndex = Number(params.node?.rowIndex ?? 0);
      return toRowId(rowKey(params.data), rowIndex);
    },
    [rowKey],
  );

  const hasRows = rows.length > 0;

  return (
    <div
      ref={shellRef}
      className={['ag-theme-quartz', 'industrial-workbench-grid', className].filter(Boolean).join(' ')}
      data-testid={dataTestId}
      onKeyDownCapture={handleSpaceKeyDownCapture}
      onKeyUpCapture={handleSpaceKeyUpCapture}
      onMouseDownCapture={handleGridMouseDownCapture}
    >
      {error ? (
        <div className="industrial-workbench-grid-state">
          <Alert
            type="error"
            showIcon
            message={errorTitle}
            description={error instanceof Error ? error.message : '请稍后重试'}
            action={
              onRetry ? (
                <Button size="small" onClick={onRetry}>
                  重试
                </Button>
              ) : null
            }
          />
        </div>
      ) : (
        <AgGridReact<T>
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
          suppressClickEdit
          singleClickEdit={false}
          readOnlyEdit
          rowHeight={rowHeight}
          headerHeight={headerHeight}
          animateRows={false}
          cellSelection
          suppressContextMenu
          preventDefaultOnContextMenu
          enableCellTextSelection={false}
          rowBuffer={10}
          enableFilterHandlers
          suppressRowVirtualisation={false}
          suppressColumnVirtualisation
          suppressRowTransform
          getMainMenuItems={getMainMenuItems}
          onGridReady={(event: GridReadyEvent<T>) => {
            apiRef.current = event.api;
            registerGridApiForTesting(event.api);
            applyPersistedColumnState(event.api);
            setGridApi(event.api);
            queueMicrotask(() => {
              refreshStructuralColumns(event.api);
            });
          }}
          onFirstDataRendered={(event) => {
            refreshStructuralColumns(event.api);
          }}
          onModelUpdated={(event) => {
            refreshStructuralColumns(event.api);
          }}
          onColumnHeaderContextMenu={handleColumnHeaderContextMenu}
          onColumnMoved={(event) => {
            schedulePersistColumnState(event.api);
            refreshStructuralColumns(event.api);
          }}
          onColumnVisible={(event) => {
            schedulePersistColumnState(event.api);
            refreshStructuralColumns(event.api);
          }}
          onColumnPinned={(event) => {
            schedulePersistColumnState(event.api);
            refreshStructuralColumns(event.api);
          }}
          onColumnResized={(event) => {
            if (!event.finished) {
              return;
            }
            schedulePersistColumnState(event.api);
            refreshStructuralColumns(event.api);
          }}
          onSortChanged={handleSortChanged}
          onFilterChanged={handleFilterChanged}
          onSelectionChanged={handleSelectionChanged}
          onCellClicked={handleCellClicked}
          onCellFocused={handleCellFocused}
          onCellKeyDown={handleCellKeyDown}
          onRowClicked={handleRowClicked}
          onRowDoubleClicked={(event: RowDoubleClickedEvent<T>) => {
            if (!event.data || !event.event) {
              return;
            }
            onRowDoubleClick?.(event.data, event.event as MouseEvent);
          }}
          onCellContextMenu={(event: CellContextMenuEvent<T>) => {
            event.event?.preventDefault?.();
            if (!event.data || !event.event) {
              return;
            }
            onRowContextMenu?.(event.data, event.event as MouseEvent);
          }}
          getRowClass={(params) => {
            const classes = ['erp-data-grid-row'];
            const row = params.data as T | undefined;
            const rowIndex = Number(params.node?.rowIndex ?? 0);

            if (row && rowClassName) {
              if (typeof rowClassName === 'string') {
                classes.push(rowClassName);
              } else {
                const nextClassName = rowClassName(row, rowIndex);
                if (nextClassName) {
                  classes.push(nextClassName);
                }
              }
            }

            if (row && activeRowKey != null) {
              const currentRowKey = resolveKeyString(rowKey, row);
              if (currentRowKey && currentRowKey === String(activeRowKey)) {
                classes.push('erp-data-grid-row-active');
                classes.push('order-review-main-row-active');
              }
            }

            return classes.join(' ');
          }}
        />
      )}

      {!error && !loading && !hasRows ? (
        <div className="industrial-workbench-grid-empty">
          <Empty description={emptyText} />
        </div>
      ) : null}

      {loading ? (
        <div className="industrial-workbench-grid-loading-mask">
          <Spin size="large" />
        </div>
      ) : null}

      {showColumnSettings ? (
        <WorkbenchColumnSettingsDialog
          open={columnSettingsOpen}
          items={columnSettingsItems}
          onCancel={closeColumnSettingsDialog}
          onApply={applyColumnSettingsDraft}
          onReset={resetColumnSettingsDraft}
          onResetToDefault={resetColumnSettingsToDefault}
          onUpdateItem={updateColumnSettingsItem}
        />
      ) : null}
    </div>
  );
};
