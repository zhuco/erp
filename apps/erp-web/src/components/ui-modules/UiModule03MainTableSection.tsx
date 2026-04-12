import type { HTMLAttributes, MutableRefObject, ReactNode, Ref } from 'react';
import { useCallback, useMemo, useRef } from 'react';
import type { Key } from 'react';
import type { TablePaginationConfig, TableProps } from 'antd';
import type { ColumnsType } from 'antd/es/table';

import {
  ErpDataGrid,
  type ErpDataGridColumn,
  type ErpDataGridRow,
  type SummaryField,
  type SummaryResolver
} from '@/components/ErpDataGrid';
import type { ServerTableSortState } from '@/components/ServerTable';
import { WorkbenchMainTableTemplate } from '@/components/workbench/templates';

import './UiModule03MainTableSection.css';

type UiModule03MainTableSectionProps<T extends { id?: string }> = {
  className?: string;
  tableClassName?: string;
  tableContainerRef?: Ref<HTMLDivElement>;
  dataTestId?: string;
  rowClickSelects?: boolean;
  suppressInitialFocus?: boolean;
  toolbarLeft?: ReactNode;
  toolbarCenter?: ReactNode;
  toolbarRight?: ReactNode;
  header?: ReactNode;
  actionBar?: ReactNode;
  columns: ColumnsType<T>;
  data: T[];
  loading?: boolean;
  error?: unknown;
  errorTitle?: string;
  emptyText?: string;
  onRetry: () => void;
  total: number;
  pagination: TablePaginationConfig;
  onPaginationChange: (pagination: TablePaginationConfig) => void;
  onSortChange?: (sortState: ServerTableSortState) => void;
  sortState?: ServerTableSortState;
  columnSettingsKey?: string;
  showColumnSettings?: boolean;
  emptyAction?: ReactNode;
  loadingTimeoutMs?: number;
  virtual?: boolean;
  scroll?: TableProps<T>['scroll'];
  queryRow?: ReactNode;
  queryCells?: ReactNode[] | Record<string, ReactNode>;
  showQueryRow?: boolean;
  queryRowPlacement?: 'top' | 'bottom';
  showStatusBar?: boolean;
  summaryFields?: SummaryField<T>[];
  summaryResolver?: SummaryResolver<T>;
  rowKey: TableProps<T>['rowKey'];
  rowSelection?: TableProps<T>['rowSelection'];
  onRow?: TableProps<T>['onRow'];
  rowClassName?: TableProps<T>['rowClassName'];
  contextMenuSelectsRow?: boolean;
  onContextMenuSelect?: (record: T, index?: number) => void;
  tableFooter?: ReactNode;
  footerSummary?: ReactNode;
  bottomPanel?: ReactNode;
  enablePanels?: boolean;
  leftPanelWidth?: number;
};

export const UiModule03MainTableSection = <T extends { id?: string }>({
  className,
  tableClassName,
  tableContainerRef,
  dataTestId,
  rowClickSelects = false,
  suppressInitialFocus = false,
  toolbarLeft,
  toolbarCenter,
  toolbarRight,
  header,
  actionBar,
  columns,
  data,
  loading,
  error,
  errorTitle,
  emptyText = '暂无订单',
  onRetry,
  total,
  pagination,
  onPaginationChange,
  onSortChange,
  sortState,
  columnSettingsKey,
  showColumnSettings,
  emptyAction,
  loadingTimeoutMs,
  queryCells,
  showQueryRow = true,
  showStatusBar,
  summaryFields,
  summaryResolver,
  rowKey,
  rowSelection,
  onRow,
  rowClassName,
  contextMenuSelectsRow = true,
  onContextMenuSelect,
  tableFooter,
  footerSummary,
  bottomPanel,
  leftPanelWidth
}: UiModule03MainTableSectionProps<T>) => {
  const internalTableContainerRef = useRef<HTMLDivElement | null>(null);
  const handleTableContainerRef = useCallback(
    (node: HTMLDivElement | null) => {
      internalTableContainerRef.current = node;
      if (typeof tableContainerRef === 'function') {
        tableContainerRef(node);
      } else if (tableContainerRef && typeof tableContainerRef === 'object') {
        (tableContainerRef as MutableRefObject<HTMLDivElement | null>).current = node;
      }
    },
    [tableContainerRef]
  );
  const footerSyncKey = useMemo(
    () => `${pagination.current ?? 1}-${pagination.pageSize ?? 0}-${total}-${data.length}`,
    [data.length, pagination.current, pagination.pageSize, total]
  );
  const resolvedTableFooter = useMemo<ReactNode>(() => {
    if (tableFooter !== undefined) {
      return tableFooter;
    }
    return undefined;
  }, [footerSyncKey, onPaginationChange, pagination, tableFooter, total]);
  const hasTableFooter =
    resolvedTableFooter !== undefined ||
    (tableFooter === undefined && Boolean(pagination && onPaginationChange));
  const resolvedTableClassName = [
    'order-workbench-table-module-03',
    tableClassName,
    hasTableFooter ? 'order-workbench-table-with-footer' : null
  ]
    .filter(Boolean)
    .join(' ');

  const resolveRowKeyValue = (record: T, index?: number): Key | undefined => {
    if (typeof rowKey === 'function') {
      return rowKey(record);
    }
    if (typeof rowKey === 'string') {
      return (record as Record<string, unknown>)[rowKey] as Key | undefined;
    }
    if (rowKey != null) {
      return rowKey as Key;
    }
    return record.id ?? index;
  };

  const resolveColumnKey = useCallback(
    (column: ColumnsType<T>[number], index: number): string => {
      if (column.key != null) {
        return String(column.key);
      }
      if (column.dataIndex != null) {
        return Array.isArray(column.dataIndex)
          ? column.dataIndex.join('.')
          : String(column.dataIndex);
      }
      return `column-${index}`;
    },
    []
  );

  const resolveColumnTitle = useCallback(
    (column: ColumnsType<T>[number], fallback: string): string => {
      if (typeof column.title === 'string' || typeof column.title === 'number') {
        return String(column.title);
      }
      return fallback;
    },
    []
  );

  const resolveRowIndex = useCallback(
    (record: T) => {
      const key = resolveRowKeyValue(record);
      if (key == null) {
        return data.indexOf(record);
      }
      const index = data.findIndex((item, rowIndex) => {
        const itemKey = resolveRowKeyValue(item, rowIndex);
        return itemKey === key;
      });
      return index >= 0 ? index : undefined;
    },
    [data, resolveRowKeyValue]
  );

  const resolveDataGridRowKey = useCallback(
    (record: T) => {
      const key = resolveRowKeyValue(record);
      if (key != null) {
        return key;
      }
      const index = resolveRowIndex(record);
      if (index != null && index >= 0) {
        return index;
      }
      return record.id ?? data.indexOf(record);
    },
    [data, resolveRowIndex, resolveRowKeyValue]
  );

  const resolvedRowSelectionMode =
    rowSelection?.type === 'radio' ? 'single' : rowSelection ? 'multi' : undefined;
  const selectionColumnWidth =
    typeof rowSelection?.columnWidth === 'number' ? rowSelection.columnWidth : undefined;

  const { dataGridColumns, columnKeys } = useMemo(() => {
    const keys: string[] = [];
    const mapColumn = (
      column: ColumnsType<T>[number],
      index: number
    ): ErpDataGridColumn<T> => {
      const columnKey = resolveColumnKey(column, index);
      const title = resolveColumnTitle(column, columnKey);
      if ('children' in column && Array.isArray(column.children) && column.children.length) {
        return {
          title,
          columns: column.children.map((child, childIndex) => mapColumn(child, childIndex))
        } as ErpDataGridColumn<T>;
      }
      const dataIndex = column.dataIndex;
      const field = dataIndex
        ? Array.isArray(dataIndex)
          ? dataIndex.join('.')
          : String(dataIndex)
        : columnKey;
      const align = column.align ?? 'left';
      const resolvedAlign =
        align === 'center' ? 'center' : align === 'right' ? 'right' : 'left';
      const headerAlign = column.align != null ? resolvedAlign : undefined;
      const width = typeof column.width === 'number' ? column.width : undefined;
      const headerSort = Boolean(onSortChange && column.sorter);
      const frozen = column.fixed === 'left' || column.fixed === true;
      const renderCell = column.render
        ? (value: unknown, record: T, rowIndex: number) =>
            column.render?.(value, record, rowIndex)
        : undefined;
      const invokeCellMouseHandler = (
        eventName: 'onClick' | 'onDoubleClick' | 'onContextMenu',
        nativeEvent: MouseEvent,
        cell: {
          getRow?: () => {
            getData?: () => unknown;
          };
        }
      ) => {
        if (typeof column.onCell !== 'function') {
          return;
        }
        const rowData = cell.getRow?.()?.getData?.() as
          | (T & { __erpRowIndex?: number })
          | undefined;
        if (!rowData) {
          return;
        }
        const resolvedRowIndex =
          rowData.__erpRowIndex ?? resolveRowIndex(rowData as T) ?? 0;
        const cellProps = column.onCell(rowData as T, resolvedRowIndex) as
          | HTMLAttributes<HTMLElement>
          | undefined;
        const handler = cellProps?.[eventName];
        if (typeof handler === 'function') {
          (handler as (event: MouseEvent) => void)(nativeEvent);
        }
      };
      keys.push(columnKey);
      return {
        title,
        field,
        key: columnKey,
        width,
        minWidth: 60,
        hozAlign: resolvedAlign,
        ...(headerAlign ? { headerHozAlign: headerAlign } : {}),
        headerSort,
        frozen,
        renderCell,
        cellClick: (event, cell) => {
          invokeCellMouseHandler('onClick', event, cell);
        },
        cellDblClick: (event, cell) => {
          invokeCellMouseHandler('onDoubleClick', event, cell);
        },
        cellContext: (event, cell) => {
          invokeCellMouseHandler('onContextMenu', event, cell);
        }
      };
    };
    const mapped = columns.map((column, index) => mapColumn(column, index));
    return { dataGridColumns: mapped, columnKeys: keys };
  }, [columns, onSortChange, resolveColumnKey, resolveColumnTitle, resolveRowIndex]);

  const headerFilters = useMemo(() => {
    if (!showQueryRow || !queryCells) {
      return undefined;
    }
    if (Array.isArray(queryCells)) {
      return queryCells.reduce<Record<string, ReactNode>>((acc, cell, index) => {
        const columnKey = columnKeys[index];
        if (columnKey) {
          acc[columnKey] = cell;
        }
        return acc;
      }, {});
    }
    return queryCells;
  }, [columnKeys, queryCells, showQueryRow]);

  const handleSelectionChange = useCallback(
    (keys: Key[], rows: T[]) => {
      rowSelection?.onChange?.(keys, rows);
    },
    [rowSelection]
  );

  const handleRowClick = useCallback(
    (record: T, row: { getData: () => ErpDataGridRow<T> }, event: UIEvent) => {
      const rowData = row.getData();
      const rowIndex = rowData.__erpRowIndex ?? resolveRowIndex(record) ?? 0;
      const rowProps = onRow?.(record, rowIndex);
      rowProps?.onClick?.(event);
    },
    [onRow, resolveRowIndex]
  );

  const handleRowDoubleClick = useCallback(
    (record: T, row: { getData: () => ErpDataGridRow<T> }, event: UIEvent) => {
      const rowData = row.getData();
      const rowIndex = rowData.__erpRowIndex ?? resolveRowIndex(record) ?? 0;
      const rowProps = onRow?.(record, rowIndex);
      rowProps?.onDoubleClick?.(event);
    },
    [onRow, resolveRowIndex]
  );

  const handleRowContextMenu = useCallback(
    (record: T, event: MouseEvent) => {
      const rowIndex = resolveRowIndex(record);
      const rowProps = onRow?.(record, rowIndex);
      if (contextMenuSelectsRow) {
        const key = resolveRowKeyValue(record, rowIndex);
        if (key != null && rowSelection?.onChange) {
          rowSelection.onChange([key], [record]);
        }
      }
      onContextMenuSelect?.(record, rowIndex ?? undefined);
      rowProps?.onContextMenu?.(event);
    },
    [contextMenuSelectsRow, onContextMenuSelect, onRow, resolveRowIndex, resolveRowKeyValue, rowSelection]
  );

  return (
    <WorkbenchMainTableTemplate
      className={className}
      tableClassName={resolvedTableClassName}
      tableContainerRef={handleTableContainerRef}
      toolbarLeft={toolbarLeft}
      toolbarCenter={toolbarCenter}
      toolbarRight={toolbarRight}
      header={header}
      actionBar={actionBar}
      tableContent={
        <ErpDataGrid<T>
          data={data}
          columns={dataGridColumns}
          rowKey={resolveDataGridRowKey}
          headerFilters={headerFilters}
          sortState={sortState}
          onSortChange={onSortChange}
          selectionMode={resolvedRowSelectionMode}
          showSelectionColumn={Boolean(rowSelection) && rowSelection?.type !== 'radio'}
          selectionColumnWidth={selectionColumnWidth}
          columnSettingsKey={columnSettingsKey}
          dataTestId={dataTestId}
          showColumnSettings={showColumnSettings}
          selectedRowKeys={rowSelection?.selectedRowKeys as Key[] | undefined}
          rowClickSelects={rowClickSelects}
          suppressInitialFocus={suppressInitialFocus}
          onSelectionChange={handleSelectionChange}
          showStatusBar={showStatusBar}
          summaryFields={summaryFields}
          summaryResolver={summaryResolver}
          rowClassName={rowClassName}
          onRowClick={handleRowClick}
          onRowDoubleClick={handleRowDoubleClick}
          onRowContextMenu={handleRowContextMenu}
          loading={loading}
          error={error}
          errorTitle={errorTitle}
          emptyText={emptyText}
          emptyAction={emptyAction}
          loadingTimeoutMs={loadingTimeoutMs}
          onRetry={onRetry}
        />
      }
      tableFooter={resolvedTableFooter}
      footerSummary={footerSummary}
      bottomPanel={bottomPanel}
      footerSyncKey={footerSyncKey}
      total={total}
      pagination={pagination}
      onPaginationChange={onPaginationChange}
      leftPanelWidth={leftPanelWidth}
    />
  );
};
