import { Button, Dropdown, Modal, Space, Table } from 'antd';
import type { MenuProps, TablePaginationConfig, TableProps } from 'antd';
import { HolderOutlined, LeftOutlined, RightOutlined } from '@ant-design/icons';
import type { ColumnType, SortOrder, SorterResult } from 'antd/es/table/interface';
import type { DragEvent, ReactNode } from 'react';
import { isValidElement, useCallback, useEffect, useMemo, useRef, useState } from 'react';

import { useWorkbenchPanelsOptional } from '@/app/layout/workbenchPanels';
import { EmptyState } from '@/components/EmptyState';
import { ErrorState, LoadingNotice } from '@/components/QueryState';

import './ServerTable.css';

export type ServerTableSortState = {
  field?: string;
  order?: SortOrder;
};

export type ServerTableColumn<T> = ColumnType<T> & {
  queryCell?: ReactNode;
};

export type ServerTableColumns<T> = ServerTableColumn<T>[];

type ServerTableProps<T extends { id?: string }> = {
  columns: TableProps<T>['columns'];
  data: T[];
  loading?: boolean;
  total: number;
  pagination: TablePaginationConfig;
  onPaginationChange: (pagination: TablePaginationConfig) => void;
  onSortChange?: (sortState: ServerTableSortState) => void;
  sortState?: ServerTableSortState;
  columnSettingsKey?: string;
  showColumnSettings?: boolean;
  rowKey?: TableProps<T>['rowKey'];
  rowSelection?: TableProps<T>['rowSelection'];
  onRow?: TableProps<T>['onRow'];
  rowClassName?: TableProps<T>['rowClassName'];
  virtual?: boolean;
  scroll?: TableProps<T>['scroll'];
  summary?: TableProps<T>['summary'];
  error?: unknown;
  onRetry?: () => void;
  errorTitle?: string;
  emptyText?: string;
  emptyAction?: ReactNode;
  loadingTimeoutMs?: number;
  queryRow?: ReactNode;
  queryCells?: ReactNode[] | Record<string, ReactNode>;
  showQueryRow?: boolean;
  queryRowPlacement?: 'top' | 'bottom';
  enablePanels?: boolean;
  stackGap?: number;
};

const resolveColumnKey = <T,>(column: ColumnType<T>, index: number): string => {
  if (column.key != null) {
    return String(column.key);
  }
  if (column.dataIndex != null) {
    return Array.isArray(column.dataIndex) ? column.dataIndex.join('.') : String(column.dataIndex);
  }
  return `column-${index}`;
};

const resolveNodeText = (node: ReactNode): string | undefined => {
  if (node == null || typeof node === 'boolean') {
    return undefined;
  }
  if (typeof node === 'string' || typeof node === 'number') {
    return String(node);
  }
  if (Array.isArray(node)) {
    for (const child of node) {
      const resolved = resolveNodeText(child);
      if (resolved) {
        return resolved;
      }
    }
    return undefined;
  }
  if (isValidElement(node)) {
    return resolveNodeText(node.props?.children);
  }
  return undefined;
};

const resolveColumnTitle = <T,>(column: ColumnType<T>, fallback: string): string => {
  if (typeof column.title === 'string') {
    return column.title;
  }
  if (typeof column.title === 'number') {
    return String(column.title);
  }
  return resolveNodeText(column.title) ?? fallback;
};

const resolveSorterField = <T,>(sorter?: SorterResult<T>): string | undefined => {
  if (!sorter) {
    return undefined;
  }
  const field = sorter.columnKey ?? sorter.field;
  if (Array.isArray(field)) {
    return field.join('.');
  }
  return field != null ? String(field) : undefined;
};

export const ServerTable = <T extends { id?: string }>({
  columns,
  data,
  loading,
  total,
  pagination,
  onPaginationChange,
  onSortChange,
  sortState,
  columnSettingsKey,
  showColumnSettings = true,
  rowKey,
  rowSelection,
  onRow,
  rowClassName,
  virtual,
  summary,
  scroll,
  error,
  onRetry,
  errorTitle,
  emptyText,
  emptyAction,
  loadingTimeoutMs,
  queryRow,
  queryCells,
  showQueryRow = true,
  queryRowPlacement = 'bottom',
  enablePanels = true,
  stackGap = 12
}: ServerTableProps<T>) => {
  const workbenchPanels = useWorkbenchPanelsOptional();
  const normalizedColumns = useMemo(() => (columns ?? []) as ServerTableColumn<T>[], [columns]);
  const [columnMenu, setColumnMenu] = useState<{ x: number; y: number } | null>(null);
  const [columnSettingsOpen, setColumnSettingsOpen] = useState(false);
  const [activeAvailableKey, setActiveAvailableKey] = useState<string | null>(null);
  const [activeVisibleKey, setActiveVisibleKey] = useState<string | null>(null);
  const [draggingColumnKey, setDraggingColumnKey] = useState<string | null>(null);
  const columnEntries = useMemo(
    () =>
      normalizedColumns.map((column, index) => {
        const columnKey = resolveColumnKey(column, index);
        const resolvedColumn =
          column.key != null || column.dataIndex != null
            ? column
            : { ...column, key: columnKey };
        return { key: columnKey, column: resolvedColumn };
      }),
    [normalizedColumns]
  );
  const allColumnKeys = useMemo(() => columnEntries.map((entry) => entry.key), [columnEntries]);
  const columnTitleMap = useMemo(() => {
    const map = new Map<string, string>();
    columnEntries.forEach(({ key, column }) => {
      map.set(key, resolveColumnTitle(column, key));
    });
    return map;
  }, [columnEntries]);

  const [visibleColumnKeys, setVisibleColumnKeys] = useState<string[]>(() => {
    if (!columnSettingsKey) {
      return allColumnKeys;
    }
    const stored = localStorage.getItem(columnSettingsKey);
    if (stored) {
      try {
        const parsed = JSON.parse(stored) as string[];
        if (Array.isArray(parsed) && parsed.length > 0) {
          return parsed;
        }
      } catch {
        return allColumnKeys;
      }
    }
    return allColumnKeys;
  });

  useEffect(() => {
    setVisibleColumnKeys((prev) => {
      const next = prev.filter((key) => allColumnKeys.includes(key));
      const missing = allColumnKeys.filter((key) => !next.includes(key));
      const merged = [...next, ...missing];
      if (merged.length === prev.length && merged.every((value, index) => value === prev[index])) {
        return prev;
      }
      return merged;
    });
  }, [allColumnKeys]);

  useEffect(() => {
    if (!columnSettingsKey) {
      return;
    }
    localStorage.setItem(columnSettingsKey, JSON.stringify(visibleColumnKeys));
  }, [columnSettingsKey, visibleColumnKeys]);

  useEffect(() => {
    if (!columnMenu) {
      return;
    }
    const handleClose = (event: MouseEvent) => {
      const target = event.target as HTMLElement | null;
      if (target?.closest('.server-table-column-settings')) {
        return;
      }
      setColumnMenu(null);
    };
    window.addEventListener('mousedown', handleClose);
    return () => window.removeEventListener('mousedown', handleClose);
  }, [columnMenu]);

  useEffect(() => {
    if (!enablePanels || !workbenchPanels) {
      return;
    }
    workbenchPanels.setLeftPanelVisible(true);
    workbenchPanels.setRightPanelVisible(true);
    workbenchPanels.setBottomPanelVisible(true);
  }, [enablePanels, workbenchPanels]);

  const allowColumnSettings = showColumnSettings && normalizedColumns.length > 0;

  useEffect(() => {
    if (!allowColumnSettings && columnMenu) {
      setColumnMenu(null);
    }
  }, [allowColumnSettings, columnMenu]);

  useEffect(() => {
    if (!columnSettingsOpen) {
      return;
    }
    setActiveAvailableKey(null);
    setActiveVisibleKey(null);
  }, [columnSettingsOpen]);

  const availableColumnKeys = useMemo(
    () => allColumnKeys.filter((key) => !visibleColumnKeys.includes(key)),
    [allColumnKeys, visibleColumnKeys]
  );

  const resolveColumnLabel = useCallback(
    (columnKey: string) => columnTitleMap.get(columnKey) ?? columnKey,
    [columnTitleMap]
  );

  const handleAvailableClick = useCallback((columnKey: string) => {
    setActiveAvailableKey(columnKey);
    setActiveVisibleKey(null);
  }, []);

  const handleVisibleClick = useCallback((columnKey: string) => {
    setActiveVisibleKey(columnKey);
    setActiveAvailableKey(null);
  }, []);

  const handleAddColumn = useCallback(() => {
    if (!activeAvailableKey) {
      return;
    }
    const nextKey = activeAvailableKey;
    setVisibleColumnKeys((prev) => {
      if (prev.includes(nextKey)) {
        return prev;
      }
      return [...prev, nextKey];
    });
    setActiveAvailableKey(null);
    setActiveVisibleKey(nextKey);
  }, [activeAvailableKey]);

  const handleRemoveColumn = useCallback(() => {
    if (!activeVisibleKey || visibleColumnKeys.length <= 1) {
      return;
    }
    const removeKey = activeVisibleKey;
    setVisibleColumnKeys((prev) => prev.filter((key) => key !== removeKey));
    setActiveVisibleKey(null);
    setActiveAvailableKey(removeKey);
  }, [activeVisibleKey, visibleColumnKeys.length]);

  const reorderColumnKeys = useCallback((sourceKey: string, targetKey: string) => {
    if (sourceKey === targetKey) {
      return;
    }
    setVisibleColumnKeys((prev) => {
      const sourceIndex = prev.indexOf(sourceKey);
      const targetIndex = prev.indexOf(targetKey);
      if (sourceIndex < 0 || targetIndex < 0) {
        return prev;
      }
      const next = [...prev];
      next.splice(sourceIndex, 1);
      next.splice(targetIndex, 0, sourceKey);
      return next;
    });
  }, []);

  const handleDragStart = useCallback(
    (columnKey: string) => (event: DragEvent<HTMLDivElement>) => {
      setDraggingColumnKey(columnKey);
      event.dataTransfer.setData('text/plain', columnKey);
      event.dataTransfer.effectAllowed = 'move';
    },
    []
  );

  const handleDragOver = useCallback((event: DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);

  const handleDragDrop = useCallback(
    (columnKey: string) => (event: DragEvent<HTMLDivElement>) => {
      event.preventDefault();
      const sourceKey = draggingColumnKey || event.dataTransfer.getData('text/plain');
      if (!sourceKey || sourceKey === columnKey) {
        return;
      }
      reorderColumnKeys(sourceKey, columnKey);
      setDraggingColumnKey(null);
    },
    [draggingColumnKey, reorderColumnKeys]
  );

  const handleDragEnd = useCallback(() => {
    setDraggingColumnKey(null);
  }, []);

  const columnMenuItems = useMemo<MenuProps['items']>(
    () => [{ key: 'column-settings', label: 'Column Settings' }],
    []
  );

  const handleColumnMenuClick = useCallback<NonNullable<MenuProps['onClick']>>(
    ({ key }) => {
      if (key === 'column-settings') {
        setColumnSettingsOpen(true);
      }
      setColumnMenu(null);
    },
    []
  );

  const visibleColumns = useMemo(
    () => {
      const columnMap = new Map(columnEntries.map((entry) => [entry.key, entry.column]));
      return visibleColumnKeys
        .map((columnKey) => ({
          key: columnKey,
          column: columnMap.get(columnKey)
        }))
        .filter(
          (entry): entry is { key: string; column: ServerTableColumn<T> } =>
            Boolean(entry.column)
        )
        .map(({ key, column }) => {
          if (!sortState?.field || key !== sortState.field) {
            return column;
          }
          return {
            ...column,
            sortOrder: sortState.order
          };
        });
    },
    [columnEntries, sortState?.field, sortState?.order, visibleColumnKeys]
  );

  const columnQueryCells = useMemo(() => {
    const cells: Record<string, ReactNode> = {};
    normalizedColumns.forEach((column, index) => {
      if (column.queryCell != null) {
        cells[resolveColumnKey(column, index)] = column.queryCell;
      }
    });
    return cells;
  }, [normalizedColumns]);

  const queryPlaceholder = <span className="workbench-table-query-placeholder">&nbsp;</span>;
  const queryRowContent = queryRow ?? queryPlaceholder;
  const queryColSpan = Math.max(visibleColumns.length + (rowSelection ? 1 : 0), 1);
  const shouldUseQueryCells = Boolean(queryCells) || !queryRow;
  const resolvedQueryCells = shouldUseQueryCells
    ? queryCells ?? columnQueryCells
    : undefined;
  const queryRowElement = useMemo(
    () =>
      resolvedQueryCells ? (
        <tr className="workbench-table-query-row" key="query-row">
          {visibleColumns.length ? (
            <>
              {rowSelection ? (
                <th className="workbench-table-query-cell">
                  <div className="workbench-table-query">{queryPlaceholder}</div>
                </th>
              ) : null}
              {visibleColumns.map((column, index) => {
                const columnKey = resolveColumnKey(column, index);
                const cell = Array.isArray(resolvedQueryCells)
                  ? resolvedQueryCells[index]
                  : resolvedQueryCells?.[columnKey];
                return (
                  <th key={columnKey} className="workbench-table-query-cell">
                    <div className="workbench-table-query">{cell ?? queryPlaceholder}</div>
                  </th>
                );
              })}
            </>
          ) : (
            <th colSpan={queryColSpan} className="workbench-table-query-cell">
              <div className="workbench-table-query">{queryRowContent}</div>
            </th>
          )}
        </tr>
      ) : (
        <tr className="workbench-table-query-row" key="query-row">
          <th colSpan={queryColSpan} className="workbench-table-query-cell">
            <div className="workbench-table-query">{queryRowContent}</div>
          </th>
        </tr>
      ),
    [
      queryColSpan,
      queryPlaceholder,
      queryRowContent,
      resolvedQueryCells,
      rowSelection,
      visibleColumns
    ]
  );
  const queryRowRef = useRef<ReactNode | null>(null);
  queryRowRef.current = showQueryRow ? queryRowElement : null;

  const renderHeaderWrapper = useCallback(
    (props: React.HTMLAttributes<HTMLTableSectionElement>) => (
      <thead
        {...props}
        onContextMenu={(event) => {
          props.onContextMenu?.(event);
          if (!allowColumnSettings) {
            return;
          }
          if ((event.target as HTMLElement).closest('.workbench-table-query-row')) {
            return;
          }
          event.preventDefault();
          setColumnMenu({ x: event.clientX, y: event.clientY });
        }}
      >
        {queryRowPlacement === 'top' ? queryRowRef.current : null}
        {props.children}
        {queryRowPlacement === 'bottom' ? queryRowRef.current : null}
      </thead>
    ),
    [allowColumnSettings, queryRowPlacement]
  );

  const useHeaderWrapper = showQueryRow || allowColumnSettings;
  const components = useMemo(
    () =>
      useHeaderWrapper
        ? {
            header: {
              wrapper: renderHeaderWrapper
            }
          }
        : undefined,
    [renderHeaderWrapper, useHeaderWrapper]
  );

  const handleChange: TableProps<T>['onChange'] = (nextPagination, filters, sorter) => {
    onPaginationChange(nextPagination);
    if (onSortChange) {
      const resolvedSorter = Array.isArray(sorter) ? sorter[0] : sorter;
      onSortChange({
        field: resolveSorterField(resolvedSorter),
        order: resolvedSorter?.order ?? null
      });
    }
  };

  const showInlineError = Boolean(error) && data.length > 0;
  const emptyNode = error ? (
    <ErrorState title={errorTitle ?? '加载失败'} error={error} onRetry={onRetry} />
  ) : (
    <EmptyState description={emptyText} action={emptyAction} onRetry={onRetry} />
  );

  const resolvedScroll = useMemo(() => {
    if (virtual) {
      return { y: 520, ...(scroll ?? {}) };
    }
    return scroll;
  }, [scroll, virtual]);

  const loadingNotice = loading ? (
    <LoadingNotice loading={true} onRetry={onRetry} timeoutMs={loadingTimeoutMs} />
  ) : null;

  return (
    <>
      <Space
        direction="vertical"
        size={stackGap}
        style={{ width: '100%', height: '100%' }}
        className="server-table-stack"
      >
        {loadingNotice}
        {showInlineError ? (
          <ErrorState title={errorTitle ?? '加载失败'} error={error} onRetry={onRetry} />
        ) : null}
        <Table<T>
          rowKey={rowKey ?? 'id'}
          columns={visibleColumns}
          dataSource={data}
          loading={loading}
          rowSelection={rowSelection}
          onRow={onRow}
          rowClassName={rowClassName}
          pagination={{
            ...pagination,
            total,
            showSizeChanger: true
          }}
          onChange={handleChange}
          summary={summary}
          components={components}
          scroll={resolvedScroll}
          virtual={virtual}
          locale={{ emptyText: emptyNode }}
        />
      </Space>
      {allowColumnSettings ? (
        <>
          <Dropdown
            open={Boolean(columnMenu)}
            menu={{ items: columnMenuItems, onClick: handleColumnMenuClick }}
            onOpenChange={(open) => {
              if (!open) {
                setColumnMenu(null);
              }
            }}
            trigger={[]}
            placement="bottomLeft"
            getPopupContainer={() => document.body}
            styles={{
              root: columnMenu
                ? {
                    position: 'fixed',
                    left: columnMenu.x,
                    top: columnMenu.y,
                    zIndex: 2000
                  }
                : undefined
            }}
            popupRender={(menu) => (
              <div className="server-table-column-settings">{menu}</div>
            )}
          >
            <span
              style={{
                position: 'fixed',
                left: columnMenu?.x ?? -9999,
                top: columnMenu?.y ?? -9999,
                width: 0,
                height: 0
              }}
            />
          </Dropdown>
          <Modal
            title="Column Settings"
            open={columnSettingsOpen}
            onCancel={() => setColumnSettingsOpen(false)}
            footer={null}
            width={900}
            className="server-table-column-settings-modal"
          >
            <div className="server-table-column-settings-layout">
              <div className="server-table-column-settings-panel">
                <div className="server-table-column-settings-title">待选列</div>
                <div className="server-table-column-settings-list">
                  {availableColumnKeys.length ? (
                    availableColumnKeys.map((columnKey) => (
                      <div
                        key={columnKey}
                        className={`server-table-column-settings-item${
                          activeAvailableKey === columnKey ? ' active' : ''
                        }`}
                        onClick={() => handleAvailableClick(columnKey)}
                      >
                        <span className="server-table-column-settings-label">
                          {resolveColumnLabel(columnKey)}
                        </span>
                      </div>
                    ))
                  ) : (
                    <div className="server-table-column-settings-empty">暂无可选列</div>
                  )}
                </div>
              </div>
              <div className="server-table-column-settings-actions">
                <Button
                  type="primary"
                  icon={<RightOutlined />}
                  aria-label="Add column"
                  disabled={!activeAvailableKey}
                  onClick={handleAddColumn}
                />
                <Button
                  icon={<LeftOutlined />}
                  aria-label="Remove column"
                  disabled={!activeVisibleKey || visibleColumnKeys.length <= 1}
                  onClick={handleRemoveColumn}
                />
              </div>
              <div className="server-table-column-settings-panel">
                <div className="server-table-column-settings-title">已选列</div>
                <div className="server-table-column-settings-list">
                  {visibleColumnKeys.map((columnKey) => (
                    <div
                      key={columnKey}
                      className={`server-table-column-settings-item${
                        activeVisibleKey === columnKey ? ' active' : ''
                      }${draggingColumnKey === columnKey ? ' dragging' : ''}`}
                      onClick={() => handleVisibleClick(columnKey)}
                      draggable
                      onDragStart={handleDragStart(columnKey)}
                      onDragOver={handleDragOver}
                      onDrop={handleDragDrop(columnKey)}
                      onDragEnd={handleDragEnd}
                    >
                      <HolderOutlined className="server-table-column-settings-drag" />
                      <span className="server-table-column-settings-label">
                        {resolveColumnLabel(columnKey)}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          </Modal>
        </>
      ) : null}
    </>
  );
};
