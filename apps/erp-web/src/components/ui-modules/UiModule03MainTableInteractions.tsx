import { Checkbox } from 'antd';
import type { TableProps } from 'antd';
import type { Key, MouseEvent as ReactMouseEvent, RefObject } from 'react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

type TableInteractionContextMenu<T> = {
  record: T;
  index?: number;
  event: MouseEvent;
};

export type UiModule03MainTableInteractionsOptions<T extends { id?: string }> = {
  data: T[];
  rowKey: (record: T) => Key;
  tableContainerRef: RefObject<HTMLDivElement>;
  activeRow?: T | null;
  activeRowKey?: Key | null;
  onActiveRowChange?: (record: T) => void;
  onRowDoubleClick?: (record: T) => void;
  onRowContextMenu?: (payload: TableInteractionContextMenu<T>) => void;
  contextMenuSelectsRow?: boolean;
  enableRowClickSelection?: boolean;
  enableRowClickMultiSelection?: boolean;
  enableSpaceSelection?: boolean;
};

export type UiModule03MainTableInteractionsResult<T extends { id?: string }> = {
  selectedRowKeys: Key[];
  selectedRows: T[];
  rowSelection: TableProps<T>['rowSelection'];
  rowClassName: TableProps<T>['rowClassName'];
  onRow: TableProps<T>['onRow'];
  columnWidths: Record<string, number>;
  handleResizeStart: (
    columnKey: string,
    width: number,
    minWidth: number
  ) => (event: ReactMouseEvent<HTMLSpanElement>) => void;
  clearSelection: () => void;
};

const interactiveSelector =
  'button, a, input, textarea, .ant-select, .ant-select-selector, .ant-input, .ant-input-affix-wrapper, .ant-input-number, .ant-picker, .ant-checkbox, .ant-checkbox-wrapper, .workbench-table-query';

const isKeyboardInputElement = (target: HTMLElement | null) => {
  if (!target) {
    return false;
  }
  if (target.isContentEditable) {
    return true;
  }
  if (['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName)) {
    return true;
  }
  return Boolean(
    target.closest(
      'input, textarea, select, .ant-select, .ant-input, .ant-input-affix-wrapper, .ant-input-number, .ant-picker, .ant-checkbox, .ant-radio, .ant-switch'
    )
  );
};

export const useUiModule03MainTableInteractions = <T extends { id?: string }>({
  data,
  rowKey,
  tableContainerRef,
  activeRow,
  activeRowKey,
  onActiveRowChange,
  onRowDoubleClick,
  onRowContextMenu,
  contextMenuSelectsRow = true,
  enableRowClickSelection = false,
  enableRowClickMultiSelection = false,
  enableSpaceSelection = true
}: UiModule03MainTableInteractionsOptions<T>): UiModule03MainTableInteractionsResult<T> => {
  const [selectedRowKeys, setSelectedRowKeys] = useState<Key[]>([]);
  const [selectedRows, setSelectedRows] = useState<T[]>([]);
  const [columnWidths, setColumnWidths] = useState<Record<string, number>>({});
  const lastSelectedIndexRef = useRef<number | null>(null);
  const pendingSelectionRef = useRef<Key[] | null>(null);
  const suppressRowClickRef = useRef(false);
  const dragSelectionStartRef = useRef<{ x: number; y: number } | null>(null);
  const isDragSelectingRef = useRef(false);
  const dragSelectionHandlersRef = useRef<{
    move: (event: MouseEvent) => void;
    up: (event: MouseEvent) => void;
  } | null>(null);
  const resizeStateRef = useRef<{
    key: string;
    startX: number;
    startWidth: number;
    minWidth: number;
  } | null>(null);
  const resizeFrameRef = useRef<number | null>(null);
  const resizePendingRef = useRef<{ key: string; width: number } | null>(null);
  const bodyUserSelectRef = useRef<string | null>(null);
  const cellSelectionFrameRef = useRef<number | null>(null);
  const cellSelectionPendingRef = useRef<{
    startRow: number;
    endRow: number;
    startCol: number;
    endCol: number;
  } | null>(null);
  const cellSelectionStartRef = useRef<{ row: number; col: number } | null>(null);
  const cellSelectionAnchorRef = useRef<{ x: number; y: number } | null>(null);
  const cellSelectionActiveRef = useRef(false);
  const cellSelectionUserSelectRef = useRef<string | null>(null);
  const cellSelectionHandlersRef = useRef<{
    move: (event: MouseEvent) => void;
    up: (event: MouseEvent) => void;
  } | null>(null);
  const [cellSelection, setCellSelection] = useState<{
    startRow: number;
    endRow: number;
    startCol: number;
    endCol: number;
  } | null>(null);

  const resolveActiveRowKey = useMemo(() => {
    if (activeRowKey !== undefined) {
      return activeRowKey ?? null;
    }
    if (activeRow) {
      return rowKey(activeRow);
    }
    return null;
  }, [activeRow, activeRowKey, rowKey]);

  const isInteractiveTarget = useCallback((target: HTMLElement) => {
    return Boolean(target.closest(interactiveSelector));
  }, []);

  const resolveRecordIndex = useCallback(
    (record: T) =>
      data.findIndex((item) => {
        return rowKey(item) === rowKey(record);
      }),
    [data, rowKey]
  );

  const syncSelection = useCallback(
    (keys: Key[], rows?: T[]) => {
      const uniqueKeys = Array.from(new Set(keys));
      const keySet = new Set(uniqueKeys);
      setSelectedRowKeys(uniqueKeys);
      if (rows) {
        setSelectedRows(rows.filter((row) => keySet.has(rowKey(row))));
      } else {
        setSelectedRows(data.filter((row) => keySet.has(rowKey(row))));
      }
      if (uniqueKeys.length === 0) {
        lastSelectedIndexRef.current = null;
        return;
      }
      if (uniqueKeys.length === data.length && data.length > 0) {
        lastSelectedIndexRef.current = data.length - 1;
      }
    },
    [data, rowKey]
  );

  const clearSelection = useCallback(() => {
    syncSelection([]);
  }, [syncSelection]);

  const selectionAllChecked =
    data.length > 0 && selectedRowKeys.length === data.length;
  const selectionIndeterminate =
    selectedRowKeys.length > 0 && selectedRowKeys.length < data.length;

  const handleSelectAllToggle = useCallback(
    (checked: boolean) => {
      if (checked) {
        const keys = data.map((record) => rowKey(record));
        syncSelection(keys, data);
        return;
      }
      syncSelection([]);
    },
    [data, rowKey, syncSelection]
  );

  const handleSelectionHeaderMouseDown = useCallback(
    (event: ReactMouseEvent<HTMLDivElement>) => {
      event.preventDefault();
      handleSelectAllToggle(!selectionAllChecked);
    },
    [handleSelectAllToggle, selectionAllChecked]
  );

  const selectionHeader = useMemo(
    () => (
      <div
        className="order-review-selection-header"
        onMouseDownCapture={handleSelectionHeaderMouseDown}
      >
        <Checkbox checked={selectionAllChecked} indeterminate={selectionIndeterminate} />
      </div>
    ),
    [handleSelectionHeaderMouseDown, selectionAllChecked, selectionIndeterminate]
  );

  const rowSelection = useMemo<TableProps<T>['rowSelection']>(
    () => ({
      selectedRowKeys,
      columnTitle: selectionHeader,
      onChange: (keys, rows, info) => {
        if (info?.type === 'multiple' && pendingSelectionRef.current) {
          const pending = pendingSelectionRef.current;
          pendingSelectionRef.current = null;
          syncSelection(pending);
          return;
        }
        pendingSelectionRef.current = null;
        syncSelection(keys, rows as T[]);
      },
      onSelect: (record, _selected, _rows, event) => {
        const index = resolveRecordIndex(record);
        if (index < 0) {
          return;
        }
        const mouseEvent = event as MouseEvent | undefined;
        if (mouseEvent?.shiftKey && lastSelectedIndexRef.current != null) {
          const start = Math.min(lastSelectedIndexRef.current, index);
          const end = Math.max(lastSelectedIndexRef.current, index);
          const rangeKeys = data
            .slice(start, end + 1)
            .map((row) => rowKey(row));
          const merged =
            mouseEvent.ctrlKey || mouseEvent.metaKey
              ? Array.from(new Set([...selectedRowKeys, ...rangeKeys]))
              : rangeKeys;
          pendingSelectionRef.current = merged;
          syncSelection(merged);
        }
        lastSelectedIndexRef.current = index;
      }
    }),
    [data, resolveRecordIndex, rowKey, selectedRowKeys, selectionHeader, syncSelection]
  );

  const rowClassName = useCallback(
    (record: T) => {
      if (!resolveActiveRowKey) {
        return '';
      }
      return rowKey(record) === resolveActiveRowKey ? 'order-review-row-active' : '';
    },
    [resolveActiveRowKey, rowKey]
  );

  const stopDragSelectionTracking = useCallback(() => {
    const handlers = dragSelectionHandlersRef.current;
    if (!handlers) {
      return;
    }
    window.removeEventListener('mousemove', handlers.move);
    window.removeEventListener('mouseup', handlers.up);
    dragSelectionHandlersRef.current = null;
  }, []);

  const startDragSelectionTracking = useCallback(
    (event: ReactMouseEvent<HTMLElement>) => {
      if (event.button !== 0) {
        return;
      }
      const target = event.target as HTMLElement;
      if (isInteractiveTarget(target)) {
        return;
      }
      stopDragSelectionTracking();
      dragSelectionStartRef.current = { x: event.clientX, y: event.clientY };
      isDragSelectingRef.current = false;
      suppressRowClickRef.current = false;

      const handleMove = (moveEvent: MouseEvent) => {
        if (!dragSelectionStartRef.current || moveEvent.buttons !== 1) {
          return;
        }
        const dx = Math.abs(moveEvent.clientX - dragSelectionStartRef.current.x);
        const dy = Math.abs(moveEvent.clientY - dragSelectionStartRef.current.y);
        if (dx > 4 || dy > 4) {
          isDragSelectingRef.current = true;
          suppressRowClickRef.current = true;
        }
      };

      const handleUp = () => {
        dragSelectionStartRef.current = null;
        const selection = window.getSelection?.();
        const selectionActive = Boolean(selection && !selection.isCollapsed);
        if (selectionActive || isDragSelectingRef.current) {
          suppressRowClickRef.current = true;
          setTimeout(() => {
            isDragSelectingRef.current = false;
          }, 0);
          setTimeout(() => {
            suppressRowClickRef.current = false;
          }, 200);
        }
        stopDragSelectionTracking();
      };

      dragSelectionHandlersRef.current = { move: handleMove, up: handleUp };
      window.addEventListener('mousemove', handleMove);
      window.addEventListener('mouseup', handleUp);
    },
    [isInteractiveTarget, stopDragSelectionTracking]
  );

  useEffect(() => () => stopDragSelectionTracking(), [stopDragSelectionTracking]);

  const stopCellSelectionTracking = useCallback(() => {
    const handlers = cellSelectionHandlersRef.current;
    if (handlers) {
      window.removeEventListener('mousemove', handlers.move);
      window.removeEventListener('mouseup', handlers.up);
      cellSelectionHandlersRef.current = null;
    }
    if (cellSelectionFrameRef.current !== null) {
      window.cancelAnimationFrame(cellSelectionFrameRef.current);
      cellSelectionFrameRef.current = null;
    }
    cellSelectionPendingRef.current = null;
    cellSelectionActiveRef.current = false;
    cellSelectionStartRef.current = null;
    cellSelectionAnchorRef.current = null;
    if (cellSelectionUserSelectRef.current !== null) {
      document.body.style.userSelect = cellSelectionUserSelectRef.current;
      cellSelectionUserSelectRef.current = null;
    }
  }, []);

  const updateCellSelection = useCallback((rowIndex: number, colIndex: number) => {
    const start = cellSelectionStartRef.current;
    if (!start) {
      return;
    }
    const next = {
      startRow: Math.min(start.row, rowIndex),
      endRow: Math.max(start.row, rowIndex),
      startCol: Math.min(start.col, colIndex),
      endCol: Math.max(start.col, colIndex)
    };
    cellSelectionPendingRef.current = next;
    if (cellSelectionFrameRef.current !== null) {
      return;
    }
    cellSelectionFrameRef.current = window.requestAnimationFrame(() => {
      cellSelectionFrameRef.current = null;
      const pending = cellSelectionPendingRef.current;
      cellSelectionPendingRef.current = null;
      if (!pending) {
        return;
      }
      setCellSelection((prev) => {
        if (
          prev &&
          prev.startRow === pending.startRow &&
          prev.endRow === pending.endRow &&
          prev.startCol === pending.startCol &&
          prev.endCol === pending.endCol
        ) {
          return prev;
        }
        return pending;
      });
    });
  }, []);

  const handleCellSelectionMouseDown = useCallback(
    (event: MouseEvent | ReactMouseEvent<HTMLElement>) => {
      if (event.button !== 0) {
        return;
      }
      const target = event.target as HTMLElement;
      if (isInteractiveTarget(target)) {
        return;
      }
      if (!target.closest('.ant-table')) {
        return;
      }
      if (!target.closest('.ant-table-tbody')) {
        return;
      }
      if (target.closest('.workbench-table-query')) {
        return;
      }
      if (target.closest('thead')) {
        return;
      }
      const cell = target.closest('td') as HTMLTableCellElement | null;
      if (!cell) {
        return;
      }
      const row = cell.parentElement as HTMLTableRowElement | null;
      const body = tableContainerRef.current?.querySelector('.ant-table-tbody');
      if (!row || !body || !body.contains(row)) {
        return;
      }
      if (cell.classList.contains('ant-table-selection-column') || cell.cellIndex === 0) {
        return;
      }
      event.preventDefault();
      stopCellSelectionTracking();
      const rowIndex = row.sectionRowIndex;
      const colIndex = cell.cellIndex;
      cellSelectionStartRef.current = { row: rowIndex, col: colIndex };
      cellSelectionAnchorRef.current = { x: event.clientX, y: event.clientY };
      cellSelectionActiveRef.current = false;

      const handleMove = (moveEvent: MouseEvent) => {
        if (moveEvent.buttons !== 1) {
          return;
        }
        const start = cellSelectionStartRef.current;
        const anchor = cellSelectionAnchorRef.current;
        if (!start || !anchor) {
          return;
        }
        if (!cellSelectionActiveRef.current) {
          const dx = Math.abs(moveEvent.clientX - anchor.x);
          const dy = Math.abs(moveEvent.clientY - anchor.y);
          if (dx < 4 && dy < 4) {
            return;
          }
          cellSelectionActiveRef.current = true;
          suppressRowClickRef.current = true;
          cellSelectionUserSelectRef.current = document.body.style.userSelect;
          document.body.style.userSelect = 'none';
          updateCellSelection(start.row, start.col);
        }
        if (!cellSelectionActiveRef.current) {
          return;
        }
        const element = document.elementFromPoint(
          moveEvent.clientX,
          moveEvent.clientY
        ) as HTMLElement | null;
        const moveCell = element?.closest('td') as HTMLTableCellElement | null;
        if (!moveCell) {
          return;
        }
        const moveRow = moveCell.parentElement as HTMLTableRowElement | null;
        if (!moveRow || !body.contains(moveRow)) {
          return;
        }
        if (moveCell.classList.contains('ant-table-selection-column') || moveCell.cellIndex === 0) {
          return;
        }
        updateCellSelection(moveRow.sectionRowIndex, moveCell.cellIndex);
      };

      const handleUp = () => {
        const wasActive = cellSelectionActiveRef.current;
        stopCellSelectionTracking();
        if (wasActive) {
          suppressRowClickRef.current = true;
          setTimeout(() => {
            suppressRowClickRef.current = false;
          }, 200);
        }
      };

      cellSelectionHandlersRef.current = { move: handleMove, up: handleUp };
      window.addEventListener('mousemove', handleMove);
      window.addEventListener('mouseup', handleUp);
    },
    [isInteractiveTarget, stopCellSelectionTracking, tableContainerRef, updateCellSelection]
  );

  useEffect(() => () => stopCellSelectionTracking(), [stopCellSelectionTracking]);

  useEffect(() => {
    const body = tableContainerRef.current?.querySelector('.ant-table-tbody');
    if (!body) {
      return;
    }
    const handleMouseDown = (event: MouseEvent) => {
      handleCellSelectionMouseDown(event);
    };
    body.addEventListener('mousedown', handleMouseDown);
    return () => body.removeEventListener('mousedown', handleMouseDown);
  }, [handleCellSelectionMouseDown, tableContainerRef, data]);

  useEffect(() => {
    setCellSelection(null);
  }, [data]);

  useEffect(() => {
    const container = tableContainerRef.current;
    if (!container) {
      return;
    }
    const rows = Array.from(container.querySelectorAll('tbody tr'));
    rows.forEach((row) =>
      Array.from(row.querySelectorAll('td')).forEach((cell) =>
        cell.classList.remove('order-review-cell-selected')
      )
    );
    if (!cellSelection) {
      return;
    }
    for (let rowIndex = cellSelection.startRow; rowIndex <= cellSelection.endRow; rowIndex += 1) {
      const row = rows[rowIndex] as HTMLTableRowElement | undefined;
      if (!row) {
        continue;
      }
      const cells = Array.from(row.querySelectorAll('td'));
      for (
        let colIndex = cellSelection.startCol;
        colIndex <= cellSelection.endCol;
        colIndex += 1
      ) {
        const cell = cells[colIndex];
        if (cell && !cell.classList.contains('ant-table-selection-column')) {
          cell.classList.add('order-review-cell-selected');
        }
      }
    }
  }, [cellSelection, tableContainerRef]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (!cellSelection) {
        return;
      }
      if (!(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== 'c') {
        return;
      }
      const active = document.activeElement as HTMLElement | null;
      if (active && (active.tagName === 'INPUT' || active.tagName === 'TEXTAREA')) {
        return;
      }
      const container = tableContainerRef.current;
      if (!container) {
        return;
      }
      const rows = Array.from(container.querySelectorAll('tbody tr'));
      const lines: string[] = [];
      for (let rowIndex = cellSelection.startRow; rowIndex <= cellSelection.endRow; rowIndex += 1) {
        const row = rows[rowIndex] as HTMLTableRowElement | undefined;
        if (!row) {
          continue;
        }
        const cells = Array.from(row.querySelectorAll('td'));
        const rowValues: string[] = [];
        for (
          let colIndex = cellSelection.startCol;
          colIndex <= cellSelection.endCol;
          colIndex += 1
        ) {
          const cell = cells[colIndex];
          if (!cell || cell.classList.contains('ant-table-selection-column')) {
            continue;
          }
          rowValues.push(cell.innerText.trim());
        }
        if (rowValues.length) {
          lines.push(rowValues.join('\t'));
        }
      }
      if (!lines.length) {
        return;
      }
      const text = lines.join('\n');
      event.preventDefault();
      if (navigator.clipboard?.writeText) {
        navigator.clipboard.writeText(text).catch(() => {
          const textarea = document.createElement('textarea');
          textarea.value = text;
          textarea.style.position = 'fixed';
          textarea.style.opacity = '0';
          document.body.appendChild(textarea);
          textarea.select();
          document.execCommand('copy');
          document.body.removeChild(textarea);
        });
      } else {
        const textarea = document.createElement('textarea');
        textarea.value = text;
        textarea.style.position = 'fixed';
        textarea.style.opacity = '0';
        document.body.appendChild(textarea);
        textarea.select();
        document.execCommand('copy');
        document.body.removeChild(textarea);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [cellSelection, tableContainerRef]);

  const toggleSelectionByKey = useCallback(
    (key: Key, index: number, event?: KeyboardEvent) => {
      if (event?.shiftKey && lastSelectedIndexRef.current != null) {
        const start = Math.min(lastSelectedIndexRef.current, index);
        const end = Math.max(lastSelectedIndexRef.current, index);
        const rangeKeys = data.slice(start, end + 1).map((item) => rowKey(item));
        const merged =
          event.ctrlKey || event.metaKey
            ? Array.from(new Set([...selectedRowKeys, ...rangeKeys]))
            : rangeKeys;
        syncSelection(merged);
        lastSelectedIndexRef.current = index;
        return;
      }
      const keySet = new Set(selectedRowKeys);
      if (keySet.has(key)) {
        keySet.delete(key);
      } else {
        keySet.add(key);
      }
      const nextKeys = Array.from(keySet);
      syncSelection(nextKeys);
      lastSelectedIndexRef.current = nextKeys.length === 0 ? null : index;
    },
    [data, rowKey, selectedRowKeys, syncSelection]
  );

  useEffect(() => {
    if (!enableSpaceSelection) {
      return;
    }
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== ' ' && event.code !== 'Space' && event.key !== 'Spacebar') {
        return;
      }
      const target = event.target as HTMLElement | null;
      const container = tableContainerRef.current;
      if (isKeyboardInputElement(target)) {
        return;
      }
      if (container && target && target !== document.body && !container.contains(target)) {
        return;
      }
      if (!resolveActiveRowKey) {
        return;
      }
      const activeIndex = data.findIndex((record) => rowKey(record) === resolveActiveRowKey);
      if (activeIndex < 0) {
        return;
      }
      const key = rowKey(data[activeIndex]);
      event.preventDefault();
      toggleSelectionByKey(key, activeIndex, event);
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [data, enableSpaceSelection, resolveActiveRowKey, rowKey, tableContainerRef, toggleSelectionByKey]);

  useEffect(() => {
    if (!selectedRowKeys.length) {
      setSelectedRows([]);
      return;
    }
    const keySet = new Set(selectedRowKeys);
    setSelectedRows(data.filter((row) => keySet.has(rowKey(row))));
  }, [data, rowKey, selectedRowKeys]);

  const onRow = useCallback<TableProps<T>['onRow']>(
    (record, index) => ({
      onMouseDown: startDragSelectionTracking,
      onContextMenu: (event) => {
        event.preventDefault();
        if (contextMenuSelectsRow) {
          onActiveRowChange?.(record);
        }
        onRowContextMenu?.({ record, index, event });
      },
      onDoubleClick: (event) => {
        const target = event.target as HTMLElement;
        if (isInteractiveTarget(target)) {
          event.preventDefault();
          return;
        }
        onRowDoubleClick?.(record);
      },
      onClick: (event) => {
        const target = event.target as HTMLElement;
        if (suppressRowClickRef.current || isDragSelectingRef.current) {
          suppressRowClickRef.current = false;
          isDragSelectingRef.current = false;
          dragSelectionStartRef.current = null;
          return;
        }
        if (isInteractiveTarget(target)) {
          onActiveRowChange?.(record);
          return;
        }
        const selection = window.getSelection?.();
        if (selection && !selection.isCollapsed) {
          suppressRowClickRef.current = false;
          isDragSelectingRef.current = false;
          dragSelectionStartRef.current = null;
          return;
        }
        onActiveRowChange?.(record);
        const allowRowClickSelection =
          enableRowClickSelection ||
          (enableRowClickMultiSelection &&
            (selectedRowKeys.length > 0 || event.shiftKey || event.ctrlKey || event.metaKey));
        if (!allowRowClickSelection) {
          return;
        }
        const currentIndex = index ?? resolveRecordIndex(record);
        if (currentIndex < 0) {
          return;
        }
        const key = rowKey(record);
        if (
          enableRowClickMultiSelection &&
          selectedRowKeys.length > 0 &&
          !event.shiftKey &&
          !event.ctrlKey &&
          !event.metaKey
        ) {
          toggleSelectionByKey(key, currentIndex);
          return;
        }
        if (event.shiftKey && lastSelectedIndexRef.current != null) {
          const start = Math.min(lastSelectedIndexRef.current, currentIndex);
          const end = Math.max(lastSelectedIndexRef.current, currentIndex);
          const rangeKeys = data
            .slice(start, end + 1)
            .map((row) => rowKey(row));
          const merged =
            event.ctrlKey || event.metaKey
              ? Array.from(new Set([...selectedRowKeys, ...rangeKeys]))
              : rangeKeys;
          syncSelection(merged);
          lastSelectedIndexRef.current = currentIndex;
        } else if (event.ctrlKey || event.metaKey) {
          toggleSelectionByKey(key, currentIndex);
        } else {
          syncSelection([key]);
          lastSelectedIndexRef.current = currentIndex;
        }
      }
    }),
    [
      contextMenuSelectsRow,
      data,
      enableRowClickSelection,
      enableRowClickMultiSelection,
      isInteractiveTarget,
      onActiveRowChange,
      onRowContextMenu,
      onRowDoubleClick,
      resolveRecordIndex,
      rowKey,
      selectedRowKeys,
      startDragSelectionTracking,
      syncSelection,
      toggleSelectionByKey
    ]
  );

  const handleResizeStart = useCallback(
    (columnKey: string, width: number, minWidth: number) =>
      (event: ReactMouseEvent<HTMLSpanElement>) => {
        event.preventDefault();
        event.stopPropagation();
        resizeStateRef.current = {
          key: columnKey,
          startX: event.clientX,
          startWidth: width,
          minWidth
        };
        bodyUserSelectRef.current = document.body.style.userSelect;
        document.body.style.userSelect = 'none';
        document.body.style.cursor = 'col-resize';
      },
    []
  );

  useEffect(() => {
    const handleMouseMove = (event: MouseEvent) => {
      const state = resizeStateRef.current;
      if (!state) {
        return;
      }
      const delta = event.clientX - state.startX;
      const nextWidth = Math.max(state.minWidth, state.startWidth + delta);
      resizePendingRef.current = { key: state.key, width: nextWidth };
      if (resizeFrameRef.current !== null) {
        return;
      }
      resizeFrameRef.current = window.requestAnimationFrame(() => {
        resizeFrameRef.current = null;
        const pending = resizePendingRef.current;
        resizePendingRef.current = null;
        if (!pending) {
          return;
        }
        setColumnWidths((prev) => {
          if (prev[pending.key] === pending.width) {
            return prev;
          }
          return { ...prev, [pending.key]: pending.width };
        });
      });
    };
    const handleMouseUp = () => {
      if (!resizeStateRef.current) {
        return;
      }
      if (resizeFrameRef.current !== null) {
        window.cancelAnimationFrame(resizeFrameRef.current);
        resizeFrameRef.current = null;
      }
      const pending = resizePendingRef.current;
      resizePendingRef.current = null;
      if (pending) {
        setColumnWidths((prev) => {
          if (prev[pending.key] === pending.width) {
            return prev;
          }
          return { ...prev, [pending.key]: pending.width };
        });
      }
      resizeStateRef.current = null;
      document.body.style.cursor = '';
      if (bodyUserSelectRef.current !== null) {
        document.body.style.userSelect = bodyUserSelectRef.current;
        bodyUserSelectRef.current = null;
      }
    };
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
      if (resizeFrameRef.current !== null) {
        window.cancelAnimationFrame(resizeFrameRef.current);
        resizeFrameRef.current = null;
      }
      resizePendingRef.current = null;
    };
  }, []);

  return {
    selectedRowKeys,
    selectedRows,
    rowSelection,
    rowClassName,
    onRow,
    columnWidths,
    handleResizeStart,
    clearSelection
  };
};
