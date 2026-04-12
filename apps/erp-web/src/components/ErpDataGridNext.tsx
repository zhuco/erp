import type { ReactNode } from 'react';
import { useMemo, useState } from 'react';

import {
  ErpDataGrid,
  type ErpDataGridColumn,
  type ErpDataGridProps,
} from '@/components/ErpDataGrid';

import './ErpDataGridNext.css';

export type GridRowKey = string | number;

export type ErpDataGridNextColumn<T> = {
  key: string;
  header: ReactNode;
  width?: number;
  render: (row: T, rowIndex: number) => ReactNode;
  sortValue?: (row: T) => string | number | null | undefined;
};

export type ErpDataGridNextProps<T> = {
  rows: T[];
  columns: ErpDataGridNextColumn<T>[];
  rowKey: (row: T) => GridRowKey;
  columnSettingsKey?: string;
  height?: number | string;
  dataTestId?: string;
  testIdPrefix?: string;
  showStatusBar?: boolean;
  enableRowModifierSelection?: boolean;
  emptyText?: string;
  loading?: boolean;
  error?: unknown;
  errorTitle?: string;
  onRetry?: () => void;
  headerFilters?: Record<string, ReactNode>;
  selectedRowKeys?: GridRowKey[];
  onSelectionChange?: (keys: GridRowKey[], rows?: T[]) => void;
  activeRowKey?: GridRowKey | null;
  onActiveRowChange?: (row: T) => void;
  onRowClick?: (row: T, event: React.MouseEvent) => void;
  onRowDoubleClick?: (row: T, event: React.MouseEvent) => void;
  onRowContextMenu?: (row: T, event: React.MouseEvent) => void;
};

export const ErpDataGridNext = <T,>({
  rows,
  columns,
  rowKey,
  columnSettingsKey,
  dataTestId,
  showStatusBar,
  enableRowModifierSelection = false,
  emptyText,
  loading,
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
}: ErpDataGridNextProps<T>) => {
  const [internalActiveRowKey, setInternalActiveRowKey] = useState<GridRowKey | null>(null);

  const resolvedActiveRowKey = activeRowKey ?? internalActiveRowKey;

  const mappedColumns = useMemo<ErpDataGridColumn<T>[]>(
    () =>
      columns.map((column) => ({
        key: column.key,
        field: column.key,
        title: column.header,
        width: column.width,
        renderCell: (_value, record, rowIndex) => column.render(record, rowIndex),
        headerSort: true,
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
      })),
    [columns],
  );

  const baseProps = useMemo<ErpDataGridProps<T>>(
    () => ({
      data: rows,
      columns: mappedColumns,
      rowKey: (record) => rowKey(record),
      columnSettingsKey,
      dataTestId,
      showSelectionColumn: enableRowModifierSelection,
      selectionMode: enableRowModifierSelection ? 'multi' : 'single',
      headerFilters,
      selectedRowKeys,
      activeRowKey: resolvedActiveRowKey,
      showStatusBar,
      emptyText,
      loading,
      error,
      errorTitle,
      onRetry,
      variant: 'next',
      rowClickSelects: true,
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
      enableRowModifierSelection,
      error,
      errorTitle,
      headerFilters,
      loading,
      mappedColumns,
      onActiveRowChange,
      onRetry,
      onRowClick,
      onRowContextMenu,
      onRowDoubleClick,
      onSelectionChange,
      resolvedActiveRowKey,
      rowKey,
      rows,
      selectedRowKeys,
      showStatusBar,
      columnSettingsKey,
    ],
  );

  return <ErpDataGrid<T> {...baseProps} />;
};
