import type { MouseEvent as ReactMouseEvent, ReactNode } from 'react';
import { useMemo, useState } from 'react';
import type { FilterModel } from 'ag-grid-community';

import {
  ErpDataGrid,
  type ErpDataGridColumn,
  type ErpDataGridProps,
  type ErpDataGridSortState,
  type SummaryField,
  type SummaryResolver,
} from '@/components/ErpDataGrid';
import { IndustrialWorkbenchGrid } from '@/components/IndustrialWorkbenchGrid';
import { resolveErpGridPreset, type ErpGridPresetName } from '@/components/industrialGridPresets';

import './ErpDataGridNext.css';

export type GridRowKey = string | number;

export type ErpDataGridNextColumn<T> = {
  key: string;
  header: ReactNode;
  field?: string;
  width?: number;
  minWidth?: number;
  filter?: ErpDataGridColumn<T>['filter'];
  filterParams?: ErpDataGridColumn<T>['filterParams'];
  filterValueGetter?: ErpDataGridColumn<T>['filterValueGetter'];
  valueGetter?: ErpDataGridColumn<T>['valueGetter'];
  cellRenderer?: ErpDataGridColumn<T>['cellRenderer'];
  cellStyle?: ErpDataGridColumn<T>['cellStyle'];
  hozAlign?: ErpDataGridColumn<T>['hozAlign'];
  frozen?: boolean;
  headerSort?: boolean;
  render: (row: T, rowIndex: number) => ReactNode;
  sortValue?: (row: T) => string | number | null | undefined;
};

export type ErpDataGridNextProps<T> = {
  rows: T[];
  columns: Array<ErpDataGridNextColumn<T> | ErpDataGridColumn<T>>;
  rowKey: (row: T) => GridRowKey;
  gridPreset?: ErpGridPresetName;
  columnSettingsKey?: string;
  height?: number | string;
  dataTestId?: string;
  testIdPrefix?: string;
  showColumnSettings?: boolean;
  sortState?: ErpDataGridSortState;
  onSortChange?: (state: ErpDataGridSortState) => void;
  selectionMode?: 'single' | 'multi';
  showSelectionColumn?: boolean;
  selectionColumnWidth?: number;
  selectionCellMode?: ErpDataGridProps<T>['selectionCellMode'];
  selectionHeaderMode?: ErpDataGridProps<T>['selectionHeaderMode'];
  showStatusBar?: boolean;
  summaryFields?: SummaryField<T>[];
  summaryResolver?: SummaryResolver<T>;
  rowClassName?: ErpDataGridProps<T>['rowClassName'];
  rowClickSelects?: boolean;
  suppressInitialFocus?: boolean;
  enableRowModifierSelection?: boolean;
  emptyText?: string;
  emptyAction?: ReactNode;
  loading?: boolean;
  loadingTimeoutMs?: number;
  error?: unknown;
  errorTitle?: string;
  onRetry?: () => void;
  headerFilters?: Record<string, ReactNode>;
  selectedRowKeys?: GridRowKey[];
  onSelectionChange?: (keys: GridRowKey[], rows?: T[]) => void;
  activeRowKey?: GridRowKey | null;
  onActiveRowChange?: (row: T) => void;
  onRowClick?: (row: T, event: ReactMouseEvent) => void;
  onRowDoubleClick?: (row: T, event: ReactMouseEvent) => void;
  onRowContextMenu?: (row: T, event: ReactMouseEvent) => void;
  gridFilterModel?: FilterModel;
  onGridFilterModelChange?: (model: FilterModel) => void;
};

const isPrebuiltErpColumn = <T,>(
  column: ErpDataGridNextColumn<T> | ErpDataGridColumn<T>,
): column is ErpDataGridColumn<T> =>
  'title' in column || 'renderCell' in column || 'columns' in column;

export const ErpDataGridNext = <T,>({
  rows,
  columns,
  rowKey,
  gridPreset,
  columnSettingsKey,
  dataTestId,
  showColumnSettings,
  sortState,
  onSortChange,
  selectionMode,
  showSelectionColumn,
  selectionColumnWidth,
  selectionCellMode,
  selectionHeaderMode,
  showStatusBar,
  summaryFields,
  summaryResolver,
  rowClassName,
  rowClickSelects = true,
  suppressInitialFocus = false,
  enableRowModifierSelection = false,
  emptyText,
  emptyAction,
  loading,
  loadingTimeoutMs,
  error,
  errorTitle,
  onRetry,
  headerFilters,
  selectedRowKeys,
  onSelectionChange,
  activeRowKey,
  onActiveRowChange,
  onRowClick,
  onRowDoubleClick,
  onRowContextMenu,
  gridFilterModel,
  onGridFilterModelChange,
}: ErpDataGridNextProps<T>) => {
  const [internalActiveRowKey, setInternalActiveRowKey] = useState<GridRowKey | null>(null);
  const resolvedGridPreset = resolveErpGridPreset(gridPreset);

  const resolvedActiveRowKey = activeRowKey ?? internalActiveRowKey;
  const resolvedShowSelectionColumn = showSelectionColumn ?? enableRowModifierSelection;
  const resolvedSelectionMode = selectionMode ?? (resolvedShowSelectionColumn ? 'multi' : 'single');

  const mappedColumns = useMemo<ErpDataGridColumn<T>[]>(
    () =>
      columns.map((column) => {
        if (isPrebuiltErpColumn(column)) {
          return column;
        }
        return {
          key: column.key,
          field: column.field ?? column.key,
          title: column.header,
          width: column.width,
          minWidth: column.minWidth ?? column.width,
          filter: column.filter,
          filterParams: column.filterParams,
          filterValueGetter: column.filterValueGetter,
          valueGetter: column.valueGetter,
          cellRenderer: column.cellRenderer,
          cellStyle: column.cellStyle,
          hozAlign: column.hozAlign,
          frozen: column.frozen,
          headerSort: column.headerSort ?? true,
          renderCell: (_value, record, rowIndex) => column.render(record, rowIndex),
          sorter:
            typeof column.sortValue === 'function'
              ? (left, right) => {
                  const leftValue = column.sortValue?.(left as T);
                  const rightValue = column.sortValue?.(right as T);
                  if (leftValue == null && rightValue == null) {
                    return 0;
                  }
                  if (leftValue == null) {
                    return -1;
                  }
                  if (rightValue == null) {
                    return 1;
                  }
                  if (leftValue === rightValue) {
                    return 0;
                  }
                  return String(leftValue).localeCompare(String(rightValue));
                }
              : undefined,
        };
      }),
    [columns],
  );

  const baseProps = useMemo<ErpDataGridProps<T>>(
    () => ({
      data: rows,
      columns: mappedColumns,
      rowKey: (record) => rowKey(record),
      columnSettingsKey,
      dataTestId,
      className: resolvedGridPreset?.className,
      showColumnSettings,
      sortState,
      onSortChange,
      selectionColumnWidth: resolvedGridPreset?.selectionColumnWidth ?? selectionColumnWidth,
      showSelectionColumn: resolvedShowSelectionColumn,
      selectionMode: resolvedSelectionMode,
      selectionCellMode: resolvedGridPreset?.selectionCellMode ?? selectionCellMode,
      selectionHeaderMode: resolvedGridPreset?.selectionHeaderMode ?? selectionHeaderMode,
      showRowIndex: resolvedGridPreset?.showRowIndex,
      rowIndexWidth: resolvedGridPreset?.rowIndexWidth,
      rowHeight: resolvedGridPreset?.rowHeight,
      headerHeight: resolvedGridPreset?.headerHeight,
      animateRows: resolvedGridPreset?.animateRows,
      headerFilters,
      selectedRowKeys,
      activeRowKey: resolvedActiveRowKey,
      showStatusBar,
      summaryFields,
      summaryResolver,
      rowClassName,
      emptyText,
      emptyAction,
      loading,
      loadingTimeoutMs,
      error,
      errorTitle,
      onRetry,
      variant: 'next',
      rowClickSelects,
      suppressInitialFocus,
      onSelectionChange: (keys, selectedRows) => {
        onSelectionChange?.(keys as GridRowKey[], selectedRows);
      },
      onRowClick: (record, _row, event) => {
        const resolvedKey = rowKey(record);
        setInternalActiveRowKey(resolvedKey);
        onActiveRowChange?.(record);
        onRowClick?.(record, event as unknown as React.MouseEvent);
      },
      onRowDoubleClick: (record, _row, event) => {
        onRowDoubleClick?.(record, event as unknown as React.MouseEvent);
      },
      onRowContextMenu: (record, event) => {
        onRowContextMenu?.(record, event as unknown as React.MouseEvent);
      },
    }),
    [
      dataTestId,
      emptyText,
      emptyAction,
      error,
      errorTitle,
      headerFilters,
      loading,
      loadingTimeoutMs,
      mappedColumns,
      gridPreset,
      onActiveRowChange,
      onRetry,
      onRowClick,
      onRowContextMenu,
      onRowDoubleClick,
      onSortChange,
      onSelectionChange,
      rowClassName,
      rowClickSelects,
      resolvedActiveRowKey,
      resolvedSelectionMode,
      resolvedShowSelectionColumn,
      rowKey,
      rows,
      resolvedGridPreset,
      selectedRowKeys,
      showColumnSettings,
      sortState,
      selectionColumnWidth,
      selectionCellMode,
      selectionHeaderMode,
      showStatusBar,
      summaryFields,
      summaryResolver,
      suppressInitialFocus,
      columnSettingsKey,
    ],
  );

  if (resolvedGridPreset?.structuralColumnStyle === 'order-review') {
    return (
      <IndustrialWorkbenchGrid<T>
        className={resolvedGridPreset.className}
        rows={rows}
        columns={mappedColumns}
        rowKey={(record) => rowKey(record)}
        columnSettingsKey={columnSettingsKey}
        dataTestId={dataTestId}
        showColumnSettings={Boolean(showColumnSettings)}
        selectedRowKeys={selectedRowKeys}
        activeRowKey={resolvedActiveRowKey}
        onSelectionChange={(keys, selectedRows) => {
          onSelectionChange?.(keys as GridRowKey[], selectedRows);
        }}
        onActiveRowChange={onActiveRowChange}
        onRowDoubleClick={(row, event) => {
          onRowDoubleClick?.(row, event as unknown as React.MouseEvent);
        }}
        onRowContextMenu={(row, event) => {
          onRowContextMenu?.(row, event as unknown as React.MouseEvent);
        }}
        emptyText={emptyText}
        loading={loading}
        error={error}
        errorTitle={errorTitle}
        onRetry={onRetry}
        rowClassName={rowClassName}
        gridFilterModel={gridFilterModel}
        onGridFilterModelChange={onGridFilterModelChange}
        rowHeight={resolvedGridPreset.rowHeight}
        headerHeight={resolvedGridPreset.headerHeight}
      />
    );
  }

  return <ErpDataGrid<T> {...baseProps} />;
};
