import './ErpDataGrid.css';

import { Alert, Empty, Spin, Typography } from 'antd';
import type { MenuProps } from 'antd';
import type { SortOrder } from 'antd/es/table/interface';
import type { Key, ReactNode } from 'react';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import type {
  CellClickedEvent,
  CellContextMenuEvent,
  ColDef,
  ColumnHeaderContextMenuEvent,
  ColumnMovedEvent,
  ColumnPinnedEvent,
  ColumnResizedEvent,
  ColumnVisibleEvent,
  GetRowIdParams,
  GridApi,
  GridReadyEvent,
  RowClickedEvent,
  RowDoubleClickedEvent,
  RowSelectionOptions,
  SideBarDef,
  SelectionChangedEvent,
  SortChangedEvent,
} from 'ag-grid-community';
import { ModuleRegistry } from 'ag-grid-community';
import { AgGridReact } from 'ag-grid-react';

import 'ag-grid-community/styles/ag-grid.css';
import 'ag-grid-community/styles/ag-theme-quartz.css';

import {
  buildOrderQuickFilterCells,
  type OrderQuickFilterField,
  type OrderQuickFilterOption,
} from './OrderQuickFilterCells';
import {
  buildAgGridApplyColumnState,
  buildAgGridColumnStateStorageKey,
  ERP_AG_GRID_LOCALE_TEXT_ZH_CN,
  persistAgGridColumnState,
  readPersistedAgGridColumnState,
  resolveAgGridMainMenuItems,
} from './agGridColumnState';
import { ERP_DATA_GRID_ENTERPRISE_MODULES } from './agGridEnterpriseModules';
import { useSessionStore } from '@/features/sessionStore';

ModuleRegistry.registerModules([...ERP_DATA_GRID_ENTERPRISE_MODULES]);

declare global {
  interface Window {
    __ERP_GRID_TEST_APIS?: Record<string, GridApi<unknown>>;
  }
}

type Primitive = string | number | boolean | null | undefined;

type UnknownRow = Record<string, unknown>;

export type ErpDataGridRow<T> = T & {
  __erpRowKey: Key;
  __erpRowIndex: number;
};

export type ErpDataGridColumn<T> = {
  key?: string;
  field?: keyof T | string | number;
  title?: ReactNode;
  width?: number;
  minWidth?: number;
  hozAlign?: 'left' | 'center' | 'right';
  headerHozAlign?: 'left' | 'center' | 'right';
  headerSort?: boolean;
  frozen?: boolean;
  sorter?: string | ((a: unknown, b: unknown) => number);
  editor?: string;
  columns?: ErpDataGridColumn<T>[];
  renderCell?: (value: unknown, record: T, rowIndex: number) => ReactNode;
  [key: string]: unknown;
};

export type ErpDataGridSortState = {
  field?: string;
  order?: SortOrder;
};

export type GridFilterOperator =
  | 'contains'
  | 'equals'
  | 'startsWith'
  | 'endsWith'
  | 'gt'
  | 'gte'
  | 'lt'
  | 'lte'
  | 'in';

export type GridFilterMode = 'local' | 'remote';

export type GridFilterCondition<T = unknown> = {
  key: string;
  label: string;
  value?: unknown;
  displayValue?: string;
  field?: keyof T | string | number;
  operator?: GridFilterOperator;
  disabled?: boolean;
};

export type GridFilterState<T> = GridFilterCondition<T>[];

export type GridFilterTag = {
  key: string;
  label: string;
  value: string;
};

export type ErpDataGridSortStates = Array<{ field: string; order: 'ascend' | 'descend' }>;

export type SummaryField<T> = {
  key: string;
  label: string;
  formatter?: (value: number | null, rows: T[]) => ReactNode;
  resolver?: (rows: T[]) => number | null;
};

export type SummaryResolver<T> = (rows: T[]) => Record<string, number | null>;

export type ErpDataGridClipboardConfig = {
  enabled?: boolean;
  includeHeaders?: boolean;
};

export type ErpDataGridKeyBinding = {
  key: string;
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
  meta?: boolean;
};

export type ErpDataGridKeyboardConfig = {
  enabled?: boolean;
  shortcuts?: Record<string, ErpDataGridKeyBinding[]>;
};

export type BatchEditValidationRule = {
  type: string;
  message?: string;
  validator?: (value: unknown) => boolean;
};

export type ErpDataGridBatchEditField<T> = {
  key: string;
  label: string;
  field: keyof T | string;
  type?: 'text' | 'number' | 'select' | 'date';
  options?: Array<{ label: string; value: unknown }>;
  rules?: BatchEditValidationRule[];
};

export type ErpDataGridBatchEditConfig<T> = {
  enabled?: boolean;
  fields?: ErpDataGridBatchEditField<T>[];
};

export type ErpDataGridSidePanelTab = 'columns' | 'filters' | 'pivot' | 'export' | 'chart';

export type ErpDataGridSidePanelConfig = {
  enabled?: boolean;
  placement?: 'left' | 'right';
  width?: number;
  defaultOpen?: boolean;
  defaultTab?: ErpDataGridSidePanelTab;
  tabs?: Partial<Record<ErpDataGridSidePanelTab, boolean>>;
  suppressRowGroups?: boolean;
  suppressValues?: boolean;
  suppressPivots?: boolean;
  suppressPivotMode?: boolean;
  customTabs?: Array<{ key: string; title: string; render: () => ReactNode }>;
  onOpenChange?: (open: boolean) => void;
};

export type ErpDataGridPivotValue = {
  field: string;
  label: string;
  agg: 'sum' | 'count' | 'avg' | 'min' | 'max';
  format?: (value: number | null) => string;
};

export type ErpDataGridPivotConfig<T> = {
  enabled?: boolean;
  mode?: 'client' | 'server';
  rowGroups: string[];
  columnGroups?: string[];
  values: ErpDataGridPivotValue[];
  showSubTotal?: boolean;
  showGrandTotal?: boolean;
  onPivotChange?: (config: ErpDataGridPivotConfig<T>) => void;
};

export type ErpDataGridGroupConfig<T> = {
  groupBy?: keyof T | string;
  expanded?: boolean;
};

export type ErpDataGridAdvancedFilterOperator =
  | 'contains'
  | 'equals'
  | 'notEquals'
  | 'in'
  | 'notIn'
  | 'gt'
  | 'gte'
  | 'lt'
  | 'lte'
  | 'between'
  | 'startsWith'
  | 'endsWith'
  | 'blank'
  | 'notBlank';

export type ErpDataGridAdvancedFilterRule = {
  id: string;
  field: string;
  operator: ErpDataGridAdvancedFilterOperator;
  value?: unknown;
};

export type ErpDataGridAdvancedFilterGroup = {
  id: string;
  logic: 'and' | 'or';
  children: Array<ErpDataGridAdvancedFilterRule | ErpDataGridAdvancedFilterGroup>;
};

export type ErpDataGridAdvancedFilterConfig<T> = {
  enabled?: boolean;
  model?: ErpDataGridAdvancedFilterGroup;
  onModelChange?: (model: ErpDataGridAdvancedFilterGroup) => void;
  distinctValues?: Record<string, Array<string | number>>;
  mode?: 'local' | 'remote';
  fields?: Array<{
    field: keyof T | string;
    label: string;
    type?: 'text' | 'number' | 'date' | 'select';
    options?: Array<{ label: string; value: unknown }>;
  }>;
};

export type ErpDataGridTreeConfig<T> = {
  enabled?: boolean;
  childField?: string;
  expandByDefault?: boolean | number;
  lazy?: boolean;
  loadChildren?: (record: T) => Promise<T[]>;
};

export type ErpDataGridDetailConfig<T> = {
  enabled?: boolean;
  render: (record: T) => ReactNode;
  height?: number;
  expandOnRowClick?: boolean;
};

export type ErpDataGridChartConfig = {
  enabled?: boolean;
  defaultType?: 'bar' | 'line' | 'pie';
  dataResolver?: (rows: unknown[]) => { labels: string[]; values: number[] };
};

export type ErpDataGridServerSideParams<T> = {
  startRow: number;
  endRow: number;
  sortStates: Array<{ field: string; dir: 'asc' | 'desc' }>;
  globalFilter: string;
  filterState: GridFilterState<T>;
  advancedFilter?: ErpDataGridAdvancedFilterGroup;
  groupConfig?: ErpDataGridGroupConfig<T>;
  pivotConfig?: ErpDataGridPivotConfig<T>;
  signal?: AbortSignal;
};

export type ErpDataGridServerSideResult<T> = {
  rows: T[];
  total?: number;
  lastRow?: number;
  summary?: Record<string, number | null>;
};

export type ErpDataGridServerSideConfig<T> = {
  getRows: (params: ErpDataGridServerSideParams<T>) => Promise<ErpDataGridServerSideResult<T>>;
  blockSize?: number;
};

export type ErpDataGridExportType = 'csv' | 'excel';

export type ErpDataGridExportConfig = {
  mode?: 'client' | 'server';
  types?: ErpDataGridExportType[];
  fileName?: string | ((type: ErpDataGridExportType) => string);
  includeSummary?: boolean;
};

export type ErpDataGridContextMenuAction = 'select-all' | 'invert' | 'export' | 'print';

export type ErpDataGridInteractionAuditAction =
  | 'side-panel-toggle'
  | 'side-panel-tab-change'
  | 'context-menu-open'
  | 'context-menu-action';

export type ErpDataGridInteractionAuditEvent = {
  action: ErpDataGridInteractionAuditAction;
  status: 'executed' | 'blocked';
  source: 'ui' | 'keyboard';
  contextMenuAction?: ErpDataGridContextMenuAction;
  rowKey?: Key | null;
  tabKey?: string;
  component: 'ErpDataGrid';
  dataTestId: string | null;
  rowModel: 'client' | 'server';
  timestamp: string;
};

type BaseErpDataGridProps<T> = {
  className?: string;
  data: T[];
  columns: ErpDataGridColumn<T>[];
  rowKey: (record: T) => Key;
  headerFilters?: Record<string, ReactNode>;
  sortState?: ErpDataGridSortState;
  onSortChange?: (state: ErpDataGridSortState) => void;
  selectionMode?: 'single' | 'multi';
  showSelectionColumn?: boolean;
  selectionCellMode?: 'ag' | 'custom';
  selectionHeaderMode?: 'default' | 'allInvert';
  selectionColumnWidth?: number;
  columnSettingsKey?: string;
  showColumnSettings?: boolean;
  showRowIndex?: boolean;
  rowIndexTitle?: string;
  rowIndexWidth?: number;
  rowIndexOffset?: number;
  rowHeight?: number;
  headerHeight?: number;
  animateRows?: boolean;
  rowClassName?: string | ((record: T, index: number) => string);
  selectedRowKeys?: Key[];
  activeRowKey?: Key | null;
  onSelectionChange?: (keys: Key[], rows: T[]) => void;
  onSelectionSelectAll?: () => void;
  onSelectionInvert?: () => void;
  onRowClick?: (record: T, row: { getData: () => ErpDataGridRow<T> }, event: UIEvent) => void;
  onRowDoubleClick?: (record: T, row: { getData: () => ErpDataGridRow<T> }, event: UIEvent) => void;
  onRowContextMenu?: (record: T, event: MouseEvent) => void;
  onSelectionCellClick?: (record: T, event: MouseEvent) => void;
  emptyText?: string;
  emptyAction?: ReactNode;
  error?: unknown;
  errorTitle?: string;
  loading?: boolean;
  loadingTimeoutMs?: number;
  onRetry?: () => void;
  dataTestId?: string;
  enableRangeSelection?: boolean;
  enableCellTextSelection?: boolean;
  floatingFilter?: boolean;
  suppressContextMenu?: boolean;
  rowClickSelects?: boolean;
  suppressInitialFocus?: boolean;
};

export type ErpDataGridProps<T> = BaseErpDataGridProps<T> & {
  exportConfig?: ErpDataGridExportConfig;
  showExportEntry?: boolean;
  sortStates?: ErpDataGridSortStates;
  onSortStatesChange?: (states: ErpDataGridSortStates) => void;
  filterMode?: GridFilterMode;
  globalFilter?: string;
  onGlobalFilterChange?: (value: string) => void;
  filterState?: GridFilterState<T>;
  onFilterChange?: (state: GridFilterState<T>) => void;
  filterPanel?: ReactNode;
  filterPanelTitle?: string;
  filterBarPlaceholder?: string;
  showFilterBar?: boolean;
  advancedFilterConfig?: ErpDataGridAdvancedFilterConfig<T>;
  pivotConfig?: ErpDataGridPivotConfig<T>;
  chartConfig?: ErpDataGridChartConfig;
  sidePanelConfig?: ErpDataGridSidePanelConfig;
  summaryFields?: SummaryField<T>[];
  summaryResolver?: SummaryResolver<T>;
  showStatusBar?: boolean;
  showInlineStatusSummary?: boolean;
  groupConfig?: ErpDataGridGroupConfig<T>;
  keyboardConfig?: ErpDataGridKeyboardConfig;
  clipboardConfig?: ErpDataGridClipboardConfig;
  batchEditConfig?: ErpDataGridBatchEditConfig<T>;
  treeConfig?: ErpDataGridTreeConfig<T>;
  detailConfig?: ErpDataGridDetailConfig<T>;
  rowModel?: 'client' | 'server';
  serverSideConfig?: ErpDataGridServerSideConfig<T>;
  contextMenuEnabled?: boolean;
  contextMenuItems?: MenuProps['items'] | ((row: T | null) => MenuProps['items']);
  onContextMenuItemClick?: (key: string, row: T | null) => void;
  onContextMenuOpen?: (row: T) => void;
  contextMenuActionGuard?: (options: {
    action: ErpDataGridContextMenuAction;
    row: T | null;
    visibleRowCount: number;
    selectedRowCount: number;
    selectionEnabled: boolean;
    hasExport: boolean;
  }) => boolean;
  onInteractionAudit?: (event: ErpDataGridInteractionAuditEvent) => void;
  variant?: 'default' | 'next';
};

type FlattenedColumn<T> = {
  key: string;
  source: ErpDataGridColumn<T>;
};

const isPlainObject = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value != null;

const normalizePrimitive = (value: unknown): Primitive => {
  if (value == null) {
    return null;
  }
  if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') {
    return value;
  }
  return String(value);
};

const toFilterText = (value: unknown): string => {
  const normalized = normalizePrimitive(value);
  if (normalized == null) {
    return '';
  }
  return String(normalized).trim().toLowerCase();
};

const matchesCondition = (recordValue: unknown, condition: GridFilterCondition<unknown>) => {
  const keyword = toFilterText(condition.value);
  const rawText = toFilterText(recordValue);
  const operator = condition.operator ?? 'contains';
  if (!keyword) {
    return true;
  }
  if (operator === 'equals') {
    return rawText === keyword;
  }
  if (operator === 'startsWith') {
    return rawText.startsWith(keyword);
  }
  if (operator === 'endsWith') {
    return rawText.endsWith(keyword);
  }
  if (operator === 'in' && Array.isArray(condition.value)) {
    return condition.value.map((item) => toFilterText(item)).includes(rawText);
  }
  const recordNumber = Number(recordValue);
  const keywordNumber = Number(condition.value);
  if (operator === 'gt' && Number.isFinite(recordNumber) && Number.isFinite(keywordNumber)) {
    return recordNumber > keywordNumber;
  }
  if (operator === 'gte' && Number.isFinite(recordNumber) && Number.isFinite(keywordNumber)) {
    return recordNumber >= keywordNumber;
  }
  if (operator === 'lt' && Number.isFinite(recordNumber) && Number.isFinite(keywordNumber)) {
    return recordNumber < keywordNumber;
  }
  if (operator === 'lte' && Number.isFinite(recordNumber) && Number.isFinite(keywordNumber)) {
    return recordNumber <= keywordNumber;
  }
  return rawText.includes(keyword);
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

const flattenColumns = <T,>(columns: ErpDataGridColumn<T>[]): FlattenedColumn<T>[] => {
  const result: FlattenedColumn<T>[] = [];
  const walk = (items: ErpDataGridColumn<T>[], parentKey: string) => {
    items.forEach((column, index) => {
      const columnKey = String(column.key ?? column.field ?? `${parentKey}_${index}`);
      if (Array.isArray(column.columns) && column.columns.length > 0) {
        walk(column.columns, columnKey);
        return;
      }
      result.push({
        key: columnKey,
        source: column,
      });
    });
  };
  walk(columns, 'col');
  return result;
};

const resolveSummaryValue = <T,>(field: SummaryField<T>, rows: T[]) => {
  if (field.resolver) {
    return field.resolver(rows);
  }
  return null;
};

const toAgSort = (order: SortOrder | undefined) => {
  if (order === 'ascend') {
    return 'asc';
  }
  if (order === 'descend') {
    return 'desc';
  }
  return null;
};

const toLegacySortOrder = (sort: string | null | undefined): SortOrder | undefined => {
  if (sort === 'asc') {
    return 'ascend';
  }
  if (sort === 'desc') {
    return 'descend';
  }
  return undefined;
};

const toRowId = (key: Key, rowIndex: number) => {
  if (typeof key === 'string' || typeof key === 'number') {
    return String(key);
  }
  return `row-${rowIndex}`;
};

export type { ErpDataGridExportConfig, ErpDataGridExportType };
export type {
  GridFilterCondition as ErpDataGridFilterCondition,
  GridFilterMode as ErpDataGridFilterMode,
  GridFilterState as ErpDataGridFilterState,
  GridFilterTag as ErpDataGridFilterTag,
};
export type { ErpDataGridGroupConfig };
export type { SummaryField, SummaryResolver };
export type { ErpDataGridClipboardConfig, ErpDataGridKeyBinding, ErpDataGridKeyboardConfig };
export type {
  ErpDataGridAdvancedFilterConfig,
  ErpDataGridAdvancedFilterGroup,
  ErpDataGridAdvancedFilterOperator,
  ErpDataGridAdvancedFilterRule,
  ErpDataGridChartConfig,
  ErpDataGridDetailConfig,
  ErpDataGridPivotConfig,
  ErpDataGridPivotValue,
  ErpDataGridServerSideConfig,
  ErpDataGridSidePanelConfig,
  ErpDataGridSidePanelTab,
  ErpDataGridTreeConfig,
};
export {
  buildOrderQuickFilterCells,
  type OrderQuickFilterField,
  type OrderQuickFilterOption,
};

export const ErpDataGrid = <T extends object>({
  className,
  data,
  columns,
  rowKey,
  headerFilters,
  columnSettingsKey,
  sortState,
  onSortChange,
  sortStates,
  onSortStatesChange,
  globalFilter,
  filterState,
  selectionMode = 'multi',
  showSelectionColumn = false,
  selectionCellMode = 'ag',
  selectionHeaderMode = 'default',
  selectionColumnWidth = 48,
  showRowIndex = false,
  rowIndexTitle = '#',
  rowIndexWidth = 56,
  rowIndexOffset = 0,
  rowHeight = 32,
  headerHeight = 34,
  animateRows = true,
  rowClassName,
  selectedRowKeys,
  activeRowKey,
  onSelectionChange,
  onSelectionSelectAll,
  onSelectionInvert,
  onRowClick,
  onRowDoubleClick,
  onRowContextMenu,
  onSelectionCellClick,
  emptyText = '暂无数据',
  emptyAction,
  error,
  errorTitle,
  loading,
  onRetry,
  dataTestId,
  enableRangeSelection,
  enableCellTextSelection = true,
  floatingFilter = false,
  suppressContextMenu = false,
  rowClickSelects = false,
  summaryFields,
  summaryResolver,
  showStatusBar = false,
  showInlineStatusSummary = true,
  sidePanelConfig,
  detailConfig,
  variant = 'default',
}: ErpDataGridProps<T>) => {
  const apiRef = useRef<GridApi<T> | null>(null);
  const syncingSelectionRef = useRef(false);
  const previousSelectedRowKeySetRef = useRef<Set<string>>(new Set());
  const previousActiveRowKeyRef = useRef<Key | null>(activeRowKey ?? null);
  const columnStatePersistTimerRef = useRef<number | null>(null);
  const applyingPersistedColumnStateRef = useRef(false);
  const tenantId = useSessionStore((state) => state.selectedTenantId);
  const userId = useSessionStore((state) => state.userId);

  const flatColumns = useMemo(() => flattenColumns(columns), [columns]);
  const columnStateStorageKey = useMemo(
    () =>
      buildAgGridColumnStateStorageKey({
        columnSettingsKey,
        tenantId,
        userId,
      }),
    [columnSettingsKey, tenantId, userId],
  );

  const filteredRows = useMemo(() => {
    const localFilterState = (filterState ?? []).filter(
      (condition) => condition != null && !condition.disabled,
    );
    const keyword = (globalFilter ?? '').trim().toLowerCase();

    return data.filter((record) => {
      const passLegacyFilter = localFilterState.every((condition) => {
        const field = String(condition.field ?? condition.key ?? '');
        if (!field) {
          return true;
        }
        return matchesCondition(getValueByPath(record, field), condition as GridFilterCondition<unknown>);
      });

      if (!passLegacyFilter) {
        return false;
      }

      if (!keyword) {
        return true;
      }

      return flatColumns.some((entry) => {
        const path = resolveColumnPath(entry.source, entry.key);
        return toFilterText(getValueByPath(record, path)).includes(keyword);
      });
    });
  }, [data, filterState, flatColumns, globalFilter]);

  const filteredRowsRef = useRef<T[]>([]);
  const dataRef = useRef<T[]>(data);
  const selectedRowKeysRef = useRef<Key[]>([]);
  const selectedRowKeySetRef = useRef<Set<string>>(new Set());
  const onSelectionCellClickRef = useRef(onSelectionCellClick);
  const onSelectionSelectAllRef = useRef(onSelectionSelectAll);
  const onSelectionInvertRef = useRef(onSelectionInvert);

  useEffect(() => {
    filteredRowsRef.current = filteredRows;
  }, [filteredRows]);

  useEffect(() => {
    dataRef.current = data;
  }, [data]);

  useEffect(() => {
    selectedRowKeysRef.current = selectedRowKeys ?? [];
    const nextSelectedSet = new Set((selectedRowKeys ?? []).map((key) => String(key)));
    selectedRowKeySetRef.current = nextSelectedSet;
  }, [selectedRowKeys]);

  useEffect(() => {
    onSelectionCellClickRef.current = onSelectionCellClick;
  }, [onSelectionCellClick]);

  useEffect(() => {
    onSelectionSelectAllRef.current = onSelectionSelectAll;
  }, [onSelectionSelectAll]);

  useEffect(() => {
    onSelectionInvertRef.current = onSelectionInvert;
  }, [onSelectionInvert]);

  useEffect(() => {
    if (!apiRef.current || !showSelectionColumn || selectionCellMode !== 'custom') {
      return;
    }
    const previousSet = previousSelectedRowKeySetRef.current;
    const nextSet = new Set((selectedRowKeys ?? []).map((key) => String(key)));
    const changedKeys = new Set<string>();

    previousSet.forEach((key) => {
      if (!nextSet.has(key)) {
        changedKeys.add(key);
      }
    });
    nextSet.forEach((key) => {
      if (!previousSet.has(key)) {
        changedKeys.add(key);
      }
    });

    if (!changedKeys.size) {
      return;
    }

    const changedNodes = Array.from(changedKeys)
      .map((key) => apiRef.current?.getRowNode(key))
      .filter((node): node is NonNullable<typeof node> => node != null);

    if (changedNodes.length > 0) {
      apiRef.current.refreshCells({
        rowNodes: changedNodes,
        columns: ['__erpSelection'],
        force: true,
      });
    } else {
      apiRef.current.refreshCells({
        columns: ['__erpSelection'],
        force: true,
      });
    }

    previousSelectedRowKeySetRef.current = nextSet;
  }, [selectedRowKeys, selectionCellMode, showSelectionColumn]);

  const selectAllFromHeader = useCallback(() => {
    if (!onSelectionChange) {
      return;
    }
    const rowsSnapshot = filteredRowsRef.current;
    const keys = rowsSnapshot
      .map((record) => rowKey(record))
      .filter((key): key is Key => key != null);
    onSelectionChange(keys, rowsSnapshot);
  }, [onSelectionChange, rowKey]);

  const clearFilteredSelectionFromHeader = useCallback(() => {
    if (!onSelectionChange) {
      return;
    }
    const selectedSet = new Set((selectedRowKeysRef.current ?? []).map((key) => String(key)));
    if (!selectedSet.size) {
      return;
    }
    const filteredKeySet = new Set(
      filteredRowsRef.current
        .map((record) => rowKey(record))
        .filter((key): key is Key => key != null)
        .map((key) => String(key)),
    );

    const nextKeys = (selectedRowKeysRef.current ?? []).filter((key) => !filteredKeySet.has(String(key)));
    const nextKeySet = new Set(nextKeys.map((key) => String(key)));
    const nextRows = dataRef.current.filter((record) => nextKeySet.has(String(rowKey(record))));
    onSelectionChange(nextKeys, nextRows);
  }, [onSelectionChange, rowKey]);

  const selectedInFilteredCount = useMemo(() => {
    if (!filteredRows.length) {
      return 0;
    }
    const selectedSet = new Set((selectedRowKeys ?? []).map((key) => String(key)));
    let count = 0;
    filteredRows.forEach((record) => {
      if (selectedSet.has(String(rowKey(record)))) {
        count += 1;
      }
    });
    return count;
  }, [filteredRows, rowKey, selectedRowKeys]);

  const headerSelectionChecked = filteredRows.length > 0 && selectedInFilteredCount === filteredRows.length;
  const headerSelectionIndeterminate =
    selectedInFilteredCount > 0 && selectedInFilteredCount < filteredRows.length;

  const colDefs = useMemo<ColDef<T>[]>(() => {
    const defs: ColDef<T>[] = [];

    if (showSelectionColumn) {
      defs.push({
        colId: '__erpSelection',
        headerName: '',
        headerClass: 'erp-data-grid-selection-col-header',
        cellClass: 'erp-data-grid-selection-col-cell',
        width: selectionColumnWidth,
        minWidth: selectionColumnWidth,
        maxWidth: Math.max(selectionColumnWidth, 52),
        pinned: 'left',
        sortable: false,
        filter: false,
        resizable: false,
        lockPinned: true,
        suppressMovable: true,
        checkboxSelection: selectionCellMode !== 'custom',
        headerCheckboxSelection:
          selectionCellMode !== 'custom' &&
          selectionHeaderMode !== 'allInvert' &&
          selectionMode !== 'single',
        cellRenderer:
          selectionCellMode === 'custom'
            ? (params) => {
                if (!params.data) {
                  return null;
                }
                const key = rowKey(params.data);
                const checked = selectedRowKeySetRef.current.has(String(key));
                return (
                  <input
                    type="checkbox"
                    className="erp-data-grid-selection-input"
                    checked={checked}
                    onClick={(event) => {
                      event.preventDefault();
                      event.stopPropagation();
                      if (!onSelectionCellClickRef.current) {
                        return;
                      }
                      onSelectionCellClickRef.current(params.data as T, event.nativeEvent as MouseEvent);
                    }}
                    readOnly
                  />
                );
              }
            : undefined,
        headerComponent:
          selectionHeaderMode === 'allInvert'
            ? () => (
                <label className="erp-data-grid-selection-header-checkbox" title="全选 / 取消全选">
                  <input
                    type="checkbox"
                    className="erp-data-grid-selection-input"
                    data-testid="erp-selection-header-checkbox"
                    checked={headerSelectionChecked}
                    ref={(node) => {
                      if (node) {
                        node.indeterminate = headerSelectionIndeterminate;
                      }
                    }}
                    onClick={(event) => {
                      event.preventDefault();
                      event.stopPropagation();
                      if (headerSelectionChecked) {
                        if (onSelectionInvertRef.current) {
                          onSelectionInvertRef.current();
                          return;
                        }
                        clearFilteredSelectionFromHeader();
                        return;
                      }
                      if (onSelectionSelectAllRef.current) {
                        onSelectionSelectAllRef.current();
                        return;
                      }
                      selectAllFromHeader();
                    }}
                    readOnly
                    aria-label="全选 / 取消全选"
                  />
                </label>
              )
            : undefined,
      });
    }

    if (showRowIndex) {
      defs.push({
        colId: '__erpRowIndex',
        headerName: rowIndexTitle,
        width: rowIndexWidth,
        minWidth: rowIndexWidth,
        maxWidth: rowIndexWidth + 20,
        pinned: 'left',
        sortable: false,
        filter: false,
        resizable: false,
        lockPinned: true,
        suppressMovable: true,
        valueGetter: (params) => Number(params.node?.rowIndex ?? 0) + rowIndexOffset + 1,
      });
    }

    flatColumns.forEach((entry, index) => {
      const { source } = entry;
      const path = resolveColumnPath(source, entry.key);
      const headerName = resolveColumnTitle(source, `列${index + 1}`);

      defs.push({
        colId: path,
        headerName,
        field: path,
        width: typeof source.width === 'number' ? source.width : undefined,
        minWidth: typeof source.minWidth === 'number' ? source.minWidth : 100,
        pinned: source.frozen ? 'left' : undefined,
        sortable: source.headerSort !== false,
        resizable: true,
        filter: true,
        cellStyle:
          source.hozAlign === 'right'
            ? { textAlign: 'right' }
            : source.hozAlign === 'center'
              ? { textAlign: 'center' }
              : undefined,
        valueGetter: (params) => getValueByPath(params.data, path),
        cellRenderer: (params) => {
          const rowData = params.data as T;
          const rowIndex = Number(params.node?.rowIndex ?? 0);
          if (!rowData) {
            return '-';
          }
          if (typeof source.renderCell === 'function') {
            return source.renderCell(params.value, rowData, rowIndex);
          }
          const normalized = normalizePrimitive(params.value);
          return normalized == null || normalized === '' ? '-' : String(normalized);
        },
      });
    });

    return defs;
  }, [
    flatColumns,
    rowIndexOffset,
    rowIndexTitle,
    rowIndexWidth,
    clearFilteredSelectionFromHeader,
    headerSelectionChecked,
    headerSelectionIndeterminate,
    rowKey,
    selectionCellMode,
    selectAllFromHeader,
    selectionColumnWidth,
    selectionHeaderMode,
    selectionMode,
    showRowIndex,
    showSelectionColumn,
  ]);

  const defaultColDef = useMemo<ColDef<T>>(
    () => ({
      sortable: true,
      filter: true,
      floatingFilter,
      resizable: true,
      editable: false,
      minWidth: 100,
    }),
    [floatingFilter],
  );

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

  const resolvedSideBar = useMemo<SideBarDef | boolean | undefined>(() => {
    if (sidePanelConfig?.enabled === false) {
      return false;
    }

    const tabs = sidePanelConfig?.tabs ?? { columns: true, filters: true };
    const suppressRowGroups = sidePanelConfig?.suppressRowGroups ?? false;
    const suppressValues = sidePanelConfig?.suppressValues ?? false;
    const suppressPivots = sidePanelConfig?.suppressPivots ?? false;
    const suppressPivotMode = sidePanelConfig?.suppressPivotMode ?? false;
    const toolPanels: NonNullable<SideBarDef['toolPanels']> = [];

    if (tabs.columns !== false) {
      toolPanels.push({
        id: 'columns',
        labelDefault: '列',
        labelKey: 'columns',
        iconKey: 'columns',
        toolPanel: 'agColumnsToolPanel',
        toolPanelParams: {
          suppressRowGroups,
          suppressValues,
          suppressPivots,
          suppressPivotMode,
        },
      });
    }
    if (tabs.filters !== false) {
      toolPanels.push({
        id: 'filters',
        labelDefault: '筛选',
        labelKey: 'filters',
        iconKey: 'filter',
        toolPanel: 'agFiltersToolPanel',
      });
    }

    if (!toolPanels.length) {
      return false;
    }

    return {
      position: sidePanelConfig?.placement === 'left' ? 'left' : 'right',
      defaultToolPanel:
        sidePanelConfig?.defaultTab === 'filters' ? 'filters' : 'columns',
      hiddenByDefault: sidePanelConfig?.defaultOpen !== true,
      toolPanels,
    };
  }, [sidePanelConfig]);

  const detailCellRenderer = useMemo(() => {
    if (!detailConfig?.enabled) {
      return undefined;
    }
    const DetailCellRenderer = (params: { data?: T }) => {
      if (!params.data) {
        return null;
      }
      return <div className="erp-data-grid-detail-cell">{detailConfig.render(params.data)}</div>;
    };
    DetailCellRenderer.displayName = 'ErpDataGridDetailCellRenderer';
    return DetailCellRenderer;
  }, [detailConfig]);

  const resolvedRowSelection = useMemo<RowSelectionOptions | undefined>(() => {
    if (!showSelectionColumn && !onSelectionChange && !selectedRowKeys?.length) {
      return undefined;
    }
    if (selectionMode === 'single') {
      return {
        mode: 'singleRow',
        checkboxes: false,
        enableClickSelection: rowClickSelects,
      };
    }
    return {
      mode: 'multiRow',
      checkboxes: false,
      headerCheckbox: false,
      enableClickSelection: rowClickSelects,
    };
  }, [onSelectionChange, rowClickSelects, selectedRowKeys?.length, selectionMode, showSelectionColumn]);

  const resolveRowKey = useCallback(
    (record: T, rowIndex: number) => {
      const key = rowKey(record);
      return {
        key,
        rowId: toRowId(key, rowIndex),
      };
    },
    [rowKey],
  );

  const getRowId = useCallback(
    (params: GetRowIdParams<T>) => {
      const rowIndex = Number(params.node?.rowIndex ?? 0);
      return resolveRowKey(params.data, rowIndex).rowId;
    },
    [resolveRowKey],
  );

  const applyControlledSelection = useCallback(() => {
    if (!apiRef.current || !selectedRowKeys) {
      return;
    }
    syncingSelectionRef.current = true;
    const keySet = new Set(selectedRowKeys.map((key) => String(key)));
    apiRef.current.forEachNode((node) => {
      const row = node.data;
      if (!row) {
        return;
      }
      const rowIndex = Number(node.rowIndex ?? 0);
      const rowMeta = resolveRowKey(row, rowIndex);
      const shouldSelect = keySet.has(String(rowMeta.key));
      if (node.isSelected() !== shouldSelect) {
        node.setSelected(shouldSelect);
      }
    });
    syncingSelectionRef.current = false;
  }, [resolveRowKey, selectedRowKeys]);

  useEffect(() => {
    applyControlledSelection();
  }, [applyControlledSelection]);

  useEffect(() => {
    if (!apiRef.current) {
      return;
    }
    apiRef.current.redrawRows();
  }, [rowClassName, variant]);

  useEffect(() => {
    if (!apiRef.current) {
      return;
    }
    const previousKey = previousActiveRowKeyRef.current;
    const currentKey = activeRowKey ?? null;
    if (String(previousKey ?? '') === String(currentKey ?? '')) {
      return;
    }

    const targetNodes = [previousKey, currentKey]
      .filter((key): key is Key => key != null)
      .map((key) => apiRef.current?.getRowNode(String(key)))
      .filter((node): node is NonNullable<typeof node> => node != null);

    if (targetNodes.length > 0) {
      apiRef.current.redrawRows({ rowNodes: targetNodes });
    } else {
      apiRef.current.redrawRows();
    }
    previousActiveRowKeyRef.current = currentKey;
  }, [activeRowKey]);

  useEffect(() => {
    if (!apiRef.current) {
      return;
    }
    const applied = sortStates?.length
      ? sortStates
          .filter((item) => item.field)
          .map((item) => ({
            colId: item.field,
            sort: item.order === 'ascend' ? 'asc' : 'desc',
          }))
      : sortState?.field
        ? [{ colId: sortState.field, sort: toAgSort(sortState.order) ?? undefined }]
        : [];

    apiRef.current.applyColumnState({
      state: applied,
      defaultState: { sort: null },
    });
  }, [sortState?.field, sortState?.order, sortStates]);

  const emitSelection = useCallback(
    (event: SelectionChangedEvent<T>) => {
      if (syncingSelectionRef.current) {
        return;
      }
      if (!onSelectionChange) {
        return;
      }
      const selectedNodes = event.api.getSelectedNodes();
      const selectedRows = selectedNodes
        .map((node) => node.data)
        .filter((row): row is T => row != null);
      const keys = selectedNodes
        .map((node) => {
          if (!node.data) {
            return null;
          }
          const rowIndex = Number(node.rowIndex ?? 0);
          return resolveRowKey(node.data, rowIndex).key;
        })
        .filter((key): key is Key => key != null);
      onSelectionChange(keys, selectedRows);
    },
    [onSelectionChange, resolveRowKey],
  );

  const buildLegacyRowAdapter = useCallback(
    (record: T, rowIndex: number) => ({
      getData: () => ({
        ...(record as UnknownRow),
        __erpRowKey: resolveRowKey(record, rowIndex).key,
        __erpRowIndex: rowIndex,
      }) as ErpDataGridRow<T>,
    }),
    [resolveRowKey],
  );

  const handleRowClicked = useCallback(
    (event: RowClickedEvent<T>) => {
      const clickedSelectionColumn = event.column?.getColId() === '__erpSelection';
      if (
        detailConfig?.enabled &&
        detailConfig.expandOnRowClick &&
        event.node &&
        !clickedSelectionColumn
      ) {
        event.node.setExpanded(!event.node.expanded);
      }
      if (onRowClick && event.data) {
        const rowIndex = Number(event.rowIndex ?? 0);
        onRowClick(event.data, buildLegacyRowAdapter(event.data, rowIndex), event.event as UIEvent);
      }
    },
    [buildLegacyRowAdapter, detailConfig, onRowClick],
  );

  const handleRowDoubleClicked = useCallback(
    (event: RowDoubleClickedEvent<T>) => {
      if (!onRowDoubleClick || !event.data) {
        return;
      }
      const rowIndex = Number(event.rowIndex ?? 0);
      onRowDoubleClick(
        event.data,
        buildLegacyRowAdapter(event.data, rowIndex),
        event.event as UIEvent,
      );
    },
    [buildLegacyRowAdapter, onRowDoubleClick],
  );

  const handleCellContextMenu = useCallback(
    (event: CellContextMenuEvent<T>) => {
      event.event?.preventDefault?.();
      if (!onRowContextMenu || !event.data) {
        return;
      }
      onRowContextMenu(event.data, event.event as MouseEvent);
    },
    [onRowContextMenu],
  );

  const handleCellClicked = useCallback(
    (event: CellClickedEvent<T>) => {
      if (selectionCellMode === 'custom') {
        return;
      }
      if (event.column?.getColId() !== '__erpSelection') {
        return;
      }
      if (!onSelectionCellClickRef.current || !event.data) {
        return;
      }
      event.event?.preventDefault?.();
      onSelectionCellClickRef.current(event.data, event.event as MouseEvent);
    },
    [selectionCellMode],
  );

  const handleSortChanged = useCallback(
    (event: SortChangedEvent<T>) => {
      schedulePersistColumnState(event.api);

      const sortedColumns = event.api
        .getColumnState()
        .filter((item) => item.sort)
        .map((item) => ({
          field: item.colId,
          order: toLegacySortOrder(item.sort) ?? 'ascend',
        }));

      if (onSortStatesChange) {
        onSortStatesChange(
          sortedColumns.map((item) => ({
            field: item.field,
            order: item.order,
          })),
        );
      }

      if (onSortChange) {
        const first = sortedColumns[0];
        onSortChange(
          first
            ? {
                field: first.field,
                order: first.order,
              }
            : {},
        );
      }
    },
    [onSortChange, onSortStatesChange, schedulePersistColumnState],
  );

  const getMainMenuItems = useCallback(
    (params: { column?: { getColId?: () => string | null } | null }) =>
      resolveAgGridMainMenuItems(params.column?.getColId?.() ?? ''),
    [],
  );

  const handleColumnHeaderContextMenu = useCallback(
    (event: ColumnHeaderContextMenuEvent<T>) => {
      const nativeEvent = event.event as MouseEvent | undefined;
      nativeEvent?.preventDefault?.();
      nativeEvent?.stopPropagation?.();

      const column = 'getColId' in event.column ? event.column : null;
      const colId = column?.getColId?.() ?? '';
      if (!column || !resolveAgGridMainMenuItems(colId).length) {
        return;
      }

      event.api.hidePopupMenu();
      event.api.showColumnMenu(column);
    },
    [],
  );

  const handleColumnMoved = useCallback(
    (event: ColumnMovedEvent<T>) => {
      schedulePersistColumnState(event.api);
    },
    [schedulePersistColumnState],
  );

  const handleColumnVisible = useCallback(
    (event: ColumnVisibleEvent<T>) => {
      schedulePersistColumnState(event.api);
    },
    [schedulePersistColumnState],
  );

  const handleColumnPinned = useCallback(
    (event: ColumnPinnedEvent<T>) => {
      schedulePersistColumnState(event.api);
    },
    [schedulePersistColumnState],
  );

  const handleColumnResized = useCallback(
    (event: ColumnResizedEvent<T>) => {
      if (!event.finished) {
        return;
      }
      schedulePersistColumnState(event.api);
    },
    [schedulePersistColumnState],
  );

  const resolvedClassName = ['erp-data-grid', className, variant === 'next' ? 'erp-data-grid-next' : null]
    .filter(Boolean)
    .join(' ');

  const normalizedHeaderFilters = useMemo(() => headerFilters ?? {}, [headerFilters]);
  const hasHeaderFilters = useMemo(
    () => Object.keys(normalizedHeaderFilters).length > 0,
    [normalizedHeaderFilters],
  );

  const headerFilterCells = useMemo(
    () =>
      flatColumns.map((entry) => {
        const path = resolveColumnPath(entry.source, entry.key);
        const cellNode =
          normalizedHeaderFilters[path] ??
          normalizedHeaderFilters[entry.key] ??
          normalizedHeaderFilters[String(entry.source.field ?? '')];
        const width = typeof entry.source.width === 'number' ? entry.source.width : undefined;
        const minWidth =
          typeof entry.source.minWidth === 'number' && entry.source.minWidth > 0
            ? entry.source.minWidth
            : 100;
        return {
          key: entry.key,
          width,
          minWidth,
          node: cellNode,
        };
      }),
    [flatColumns, normalizedHeaderFilters],
  );

  const hasRows = filteredRows.length > 0;

  const summaryValues = useMemo(() => {
    if (!summaryFields || summaryFields.length === 0) {
      return [];
    }
    return summaryFields.map((field) => {
      const value = resolveSummaryValue(field, filteredRows);
      const renderValue = field.formatter ? field.formatter(value, filteredRows) : value ?? '-';
      return {
        key: field.key,
        label: field.label,
        value: renderValue,
      };
    });
  }, [filteredRows, summaryFields]);

  const extraSummary = useMemo(() => {
    if (!summaryResolver) {
      return null;
    }
    return summaryResolver(filteredRows);
  }, [filteredRows, summaryResolver]);

  return (
    <div className={resolvedClassName} data-testid={dataTestId}>
      {error ? (
        <div className="erp-data-grid-state">
          <Alert
            type="error"
            showIcon
            message={errorTitle ?? '数据加载失败'}
            description={
              <div className="erp-data-grid-error-actions">
                <Typography.Text type="secondary">
                  {error instanceof Error ? error.message : '请检查网络或稍后重试。'}
                </Typography.Text>
                {onRetry ? (
                  <a onClick={() => onRetry()} role="button">
                    重试
                  </a>
                ) : null}
              </div>
            }
          />
        </div>
      ) : null}

      {!error && !loading && !hasRows ? (
        <div className="erp-data-grid-state">
          <Empty description={emptyText}>{emptyAction}</Empty>
        </div>
      ) : null}

      {!error ? (
        <div className={`erp-data-grid-table${variant === 'next' ? ' erp-data-grid-next__table' : ''}`}>
          {hasHeaderFilters ? (
            <div className="erp-data-grid-header-filter-row" role="group" aria-label="列筛选">
              {headerFilterCells.map((cell) => (
                <div
                  key={cell.key}
                  className="erp-data-grid-header-filter-cell workbench-table-query"
                  style={{
                    minWidth: cell.minWidth,
                    width: cell.width,
                    flex: cell.width ? `0 0 ${cell.width}px` : `1 1 ${cell.minWidth}px`,
                  }}
                >
                  {cell.node ?? <span className="workbench-table-query-placeholder">&nbsp;</span>}
                </div>
              ))}
            </div>
          ) : null}
          <div className="ag-theme-quartz erp-data-grid-shell">
            <AgGridReact<T>
              theme="legacy"
              rowData={filteredRows}
              columnDefs={colDefs}
              defaultColDef={defaultColDef}
              localeText={ERP_AG_GRID_LOCALE_TEXT_ZH_CN}
              getRowId={getRowId}
              rowSelection={resolvedRowSelection}
              suppressRowClickSelection={!rowClickSelects}
              animateRows={animateRows}
              rowHeight={rowHeight}
              headerHeight={headerHeight}
              quickFilterText={globalFilter}
              rowBuffer={10}
              sideBar={resolvedSideBar}
              statusBar={
                showStatusBar
                  ? {
                      statusPanels: [
                        { statusPanel: 'agTotalAndFilteredRowCountComponent', align: 'left' },
                        { statusPanel: 'agAggregationComponent', align: 'right' },
                      ],
                    }
                  : undefined
              }
              masterDetail={Boolean(detailConfig?.enabled)}
              detailRowHeight={detailConfig?.height}
              detailCellRenderer={detailCellRenderer}
              isRowMaster={(dataItem) => Boolean(detailConfig?.enabled && dataItem != null)}
              keepDetailRows={Boolean(detailConfig?.enabled)}
              keepDetailRowsCount={1}
              suppressRowVirtualisation={false}
              suppressColumnVirtualisation={false}
              suppressScrollOnNewData
              debounceVerticalScrollbar
              getMainMenuItems={getMainMenuItems}
              cellSelection={Boolean(enableRangeSelection)}
              suppressContextMenu={suppressContextMenu}
              enableCellTextSelection={enableCellTextSelection}
              onGridReady={(event: GridReadyEvent<T>) => {
                apiRef.current = event.api;
                registerGridApiForTesting(event.api);
                applyPersistedColumnState(event.api);
                applyControlledSelection();
                previousSelectedRowKeySetRef.current = new Set(
                  (selectedRowKeys ?? []).map((key) => String(key)),
                );
                if (showSelectionColumn && selectionCellMode === 'custom') {
                  event.api.refreshCells({
                    columns: ['__erpSelection'],
                    force: true,
                  });
                }
              }}
              onColumnHeaderContextMenu={handleColumnHeaderContextMenu}
              onColumnMoved={handleColumnMoved}
              onColumnVisible={handleColumnVisible}
              onColumnPinned={handleColumnPinned}
              onColumnResized={handleColumnResized}
              onSelectionChanged={emitSelection}
              onRowClicked={handleRowClicked}
              onRowDoubleClicked={handleRowDoubleClicked}
              onCellClicked={handleCellClicked}
              onCellContextMenu={handleCellContextMenu}
              onSortChanged={handleSortChanged}
              getRowClass={(params) => {
                const classes: string[] = ['erp-data-grid-row'];
                if (variant === 'next') {
                  classes.push('erp-data-grid-next__row');
                }

                const row = params.data as T | undefined;
                const rowIndex = Number(params.node?.rowIndex ?? 0);
                if (row && rowClassName) {
                  if (typeof rowClassName === 'string') {
                    classes.push(rowClassName);
                  } else {
                    const value = rowClassName(row, rowIndex);
                    if (value) {
                      classes.push(value);
                    }
                  }
                }

                if (row && activeRowKey != null) {
                  const rowMeta = resolveRowKey(row, rowIndex);
                  if (String(rowMeta.key) === String(activeRowKey)) {
                    classes.push('erp-data-grid-row-active');
                    if (variant === 'next') {
                      classes.push('erp-data-grid-next__row--active');
                    }
                  }
                }

                return classes;
              }}
            />

            {loading ? (
              <div className="erp-data-grid-loading-mask">
                <Spin size="large" tip="加载中..." />
              </div>
            ) : null}
          </div>
          <div className={`erp-data-grid-body${variant === 'next' ? ' erp-data-grid-next__body' : ''}`} />
        </div>
      ) : null}

      {showStatusBar && showInlineStatusSummary ? (
        <div className="erp-data-grid-status-inline">
          <span>总行数: {filteredRows.length}</span>
          {summaryValues.map((item) => (
            <span key={item.key}>
              {item.label}: {item.value as ReactNode}
            </span>
          ))}
          {extraSummary
            ? Object.entries(extraSummary).map(([key, value]) => (
                <span key={key}>
                  {key}: {value ?? '-'}
                </span>
              ))
            : null}
        </div>
      ) : null}
    </div>
  );
};
