import type { KeyboardEvent as ReactKeyboardEvent, MouseEvent as ReactMouseEvent, MutableRefObject } from 'react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { flushSync } from 'react-dom';
import type {
  CellClickedEvent,
  CellFocusedEvent,
  CellKeyDownEvent,
  GridApi,
  RowClickedEvent,
} from 'ag-grid-community';

import { applyShiftRangeSelection } from './agGridSelectionHelpers';
import { resolveSelectionDiagnosticsEnabled } from './selectionDiagnostics';

type UseAgGridWorkbenchSelectionOptions<TRow> = {
  apiRef: MutableRefObject<GridApi<TRow> | null>;
  debugKey?: string;
  syncingSelectionRef: MutableRefObject<boolean>;
  focusedRowKey: string | null;
  resolveRowKey: (record: TRow) => string | null;
  emitSelectionKeys: (api: GridApi<TRow>) => void;
  onRowActive: (record: TRow) => void;
  onFocus: (record: TRow) => void;
};

const isValidRowIndex = (rowIndex: number | null | undefined): rowIndex is number => rowIndex != null && rowIndex >= 0;
const enableSelectionDiagnostics = resolveSelectionDiagnosticsEnabled();
const selectionCheckboxTargetSelector = '.ag-selection-checkbox-hitbox, .ag-selection-checkbox';

export type AgGridSelectionDiagnostic = {
  action: 'activate-row' | 'ignore' | 'range-selection' | 'skip' | 'toggle-selection';
  colId: string;
  gridKey: string;
  hitZone:
    | 'keyboard-space'
    | 'ordinary-cell'
    | 'row'
    | 'selection-checkbox'
    | 'selection-column'
    | 'selection-column-bubble'
    | 'skip-drag-click';
  rowIndex: number | null;
  rowKey: string | null;
  selectedCount: number;
  selectedRowKeys: string[];
  sequence: number;
  summary: string;
};

export const isEditableAgGridKeyboardTarget = (target: EventTarget | null) => {
  if (!(target instanceof HTMLElement)) return false;
  const tagName = target.tagName;
  if (tagName === 'INPUT' || tagName === 'TEXTAREA' || tagName === 'SELECT') return true;
  if (target.isContentEditable) return true;
  if (target.closest('.ag-floating-filter') || target.closest('.ag-cell-inline-editing')) return true;
  return false;
};

export const isSelectionCheckboxTarget = (target: EventTarget | null) => {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  return Boolean(target.closest(selectionCheckboxTargetSelector));
};

export const useAgGridWorkbenchSelection = <TRow>({
  apiRef,
  debugKey,
  syncingSelectionRef,
  focusedRowKey,
  resolveRowKey,
  emitSelectionKeys,
  onRowActive,
  onFocus,
}: UseAgGridWorkbenchSelectionOptions<TRow>) => {
  const rangeAnchorRowIndexRef = useRef<number | null>(null);
  const lastFocusedRowKeyRef = useRef<string | null>(null);
  const diagnosticSequenceRef = useRef(0);
  const pointerDownRef = useRef<{ x: number; y: number } | null>(null);
  const pointerDraggedRef = useRef(false);
  const mouseInteractingRef = useRef(false);
  const skipPointerClickRef = useRef(false);
  const suppressRowActivationRef = useRef(false);
  const clearSuppressRowActivationTimerRef = useRef<number | null>(null);
  const clearSkipPointerClickTimerRef = useRef<number | null>(null);
  const clearMouseInteractingTimerRef = useRef<number | null>(null);
  const pointerTrackingHandlersRef = useRef<{
    move: (event: MouseEvent) => void;
    up: () => void;
  } | null>(null);
  const [selectionDiagnostic, setSelectionDiagnostic] = useState<AgGridSelectionDiagnostic | null>(null);

  useEffect(() => {
    lastFocusedRowKeyRef.current = focusedRowKey;
  }, [focusedRowKey]);

  const stopPointerTracking = useCallback(() => {
    const handlers = pointerTrackingHandlersRef.current;
    if (handlers) {
      window.removeEventListener('mousemove', handlers.move, true);
      window.removeEventListener('mouseup', handlers.up, true);
      pointerTrackingHandlersRef.current = null;
    }
    pointerDownRef.current = null;
    pointerDraggedRef.current = false;
    mouseInteractingRef.current = false;
  }, []);

  useEffect(
    () => () => {
      stopPointerTracking();
      if (clearSkipPointerClickTimerRef.current != null && typeof window !== 'undefined') {
        window.clearTimeout(clearSkipPointerClickTimerRef.current);
        clearSkipPointerClickTimerRef.current = null;
      }
      if (clearSuppressRowActivationTimerRef.current != null && typeof window !== 'undefined') {
        window.clearTimeout(clearSuppressRowActivationTimerRef.current);
        clearSuppressRowActivationTimerRef.current = null;
      }
      if (clearMouseInteractingTimerRef.current != null && typeof window !== 'undefined') {
        window.clearTimeout(clearMouseInteractingTimerRef.current);
        clearMouseInteractingTimerRef.current = null;
      }
      skipPointerClickRef.current = false;
      suppressRowActivationRef.current = false;
      mouseInteractingRef.current = false;
    },
    [stopPointerTracking],
  );

  const activateRecord = useCallback(
    (record: TRow) => {
      onRowActive(record);
      onFocus(record);
    },
    [onFocus, onRowActive],
  );

  const commitSelectionKeys = useCallback(
    (api: GridApi<TRow>) => {
      flushSync(() => {
        emitSelectionKeys(api);
      });
    },
    [emitSelectionKeys],
  );

  const collectSelectedRowKeys = useCallback(
    (api: GridApi<TRow>) =>
      api
        .getSelectedNodes()
        .map((node) => (node.data ? resolveRowKey(node.data) : null))
        .filter((key): key is string => Boolean(key)),
    [resolveRowKey],
  );

  const publishSelectionDiagnostic = useCallback(
    ({
      action,
      api,
      colId,
      hitZone,
      rowIndex,
      rowKey,
    }: {
      action: AgGridSelectionDiagnostic['action'];
      api: GridApi<TRow> | null;
      colId: string;
      hitZone: AgGridSelectionDiagnostic['hitZone'];
      rowIndex: number | null;
      rowKey: string | null;
    }) => {
      if (!enableSelectionDiagnostics || !debugKey) {
        return;
      }

      const selectedRowKeys = api ? collectSelectedRowKeys(api) : [];
      const diagnostic: AgGridSelectionDiagnostic = {
        action,
        colId,
        gridKey: debugKey,
        hitZone,
        rowIndex,
        rowKey,
        selectedCount: selectedRowKeys.length,
        selectedRowKeys,
        sequence: diagnosticSequenceRef.current + 1,
        summary: `点击诊断：${hitZone === 'selection-checkbox' || hitZone === 'selection-column' ? '选择列' : hitZone === 'keyboard-space' ? '空格键' : '普通列'} -> ${
          action === 'toggle-selection'
            ? '勾选'
            : action === 'range-selection'
              ? '连续勾选'
              : action === 'activate-row'
                ? '激活行'
                : action === 'skip'
                  ? '跳过'
                  : '忽略'
        }${rowKey ? ` (${rowKey})` : ''}`,
      };
      diagnosticSequenceRef.current = diagnostic.sequence;
      setSelectionDiagnostic(diagnostic);

      if (typeof window !== 'undefined') {
        const debugWindow = window as Window & {
          __ERP_SELECTION_DIAGNOSTICS__?: Record<string, AgGridSelectionDiagnostic>;
        };
        if (!debugWindow.__ERP_SELECTION_DIAGNOSTICS__) {
          debugWindow.__ERP_SELECTION_DIAGNOSTICS__ = {};
        }
        debugWindow.__ERP_SELECTION_DIAGNOSTICS__[debugKey] = diagnostic;
      }
    },
    [collectSelectedRowKeys, debugKey],
  );

  const armSuppressRowActivation = useCallback(() => {
    suppressRowActivationRef.current = true;
    if (typeof window === 'undefined') {
      return;
    }
    if (clearSuppressRowActivationTimerRef.current != null) {
      window.clearTimeout(clearSuppressRowActivationTimerRef.current);
    }
    clearSuppressRowActivationTimerRef.current = window.setTimeout(() => {
      suppressRowActivationRef.current = false;
      clearSuppressRowActivationTimerRef.current = null;
    }, 0);
  }, []);

  const toggleRowNodeSelection = useCallback(
    (api: GridApi<TRow>, rowNode: NonNullable<ReturnType<GridApi<TRow>['getDisplayedRowAtIndex']>>) => {
      if (!rowNode.data) return;

      syncingSelectionRef.current = true;
      rowNode.setSelected(!rowNode.isSelected());
      syncingSelectionRef.current = false;
      api.refreshCells({
        force: true,
        columns: ['__erpSelection'],
      });
      commitSelectionKeys(api);

      if (isValidRowIndex(rowNode.rowIndex)) {
        rangeAnchorRowIndexRef.current = rowNode.rowIndex;
      }
    },
    [commitSelectionKeys, syncingSelectionRef],
  );

  const applyPointerSelection = useCallback(
    (
      api: GridApi<TRow>,
      rowNode: NonNullable<ReturnType<GridApi<TRow>['getDisplayedRowAtIndex']>>,
      modifierState: { shiftPressed: boolean; ctrlOrMetaPressed: boolean },
    ) => {
      const currentRowIndex = rowNode.rowIndex ?? null;
      if (!isValidRowIndex(currentRowIndex)) return;

      if (modifierState.shiftPressed || modifierState.ctrlOrMetaPressed) {
        syncingSelectionRef.current = true;
        try {
          if (modifierState.shiftPressed) {
            applyShiftRangeSelection({
              getDisplayedRowAtIndex: (index) => api.getDisplayedRowAtIndex(index),
              anchorIndex: rangeAnchorRowIndexRef.current ?? currentRowIndex,
              currentRowIndex,
            });
          } else {
            rowNode.setSelected(!rowNode.isSelected());
          }
        } finally {
          syncingSelectionRef.current = false;
        }
        rangeAnchorRowIndexRef.current = currentRowIndex;
        api.refreshCells({
          force: true,
          columns: ['__erpSelection'],
        });
        commitSelectionKeys(api);
        return;
      }

      toggleRowNodeSelection(api, rowNode);
    },
    [commitSelectionKeys, syncingSelectionRef, toggleRowNodeSelection],
  );

  const handleRowClicked = useCallback(
    (event: RowClickedEvent<TRow>) => {
      if (suppressRowActivationRef.current) return;
      if (skipPointerClickRef.current) return;
      if (!event.data) return;
      const eventTarget = event.event?.target as HTMLElement | null;
      if (eventTarget?.closest('.ag-cell')) {
        return;
      }
      activateRecord(event.data);
      publishSelectionDiagnostic({
        action: 'activate-row',
        api: event.api,
        colId: '',
        hitZone: 'row',
        rowIndex: event.node?.rowIndex ?? null,
        rowKey: resolveRowKey(event.data),
      });
    },
    [activateRecord, publishSelectionDiagnostic, resolveRowKey],
  );

  const handleCellClicked = useCallback(
    (event: CellClickedEvent<TRow>) => {
      const record = event.data;
      if (!record) return;

      if (skipPointerClickRef.current) {
        publishSelectionDiagnostic({
          action: 'skip',
          api: event.api,
          colId: event.colDef?.colId ?? event.column?.getColId?.() ?? '',
          hitZone: 'skip-drag-click',
          rowIndex: event.node?.rowIndex ?? null,
          rowKey: resolveRowKey(record),
        });
        return;
      }

      const nativeEvent = event.event as MouseEvent | undefined;
      const shiftPressed = Boolean(nativeEvent?.shiftKey);
      const ctrlOrMetaPressed = Boolean(nativeEvent?.ctrlKey || nativeEvent?.metaKey);
      const rowNode = event.node;
      const api = event.api;
      const currentRowIndex = rowNode?.rowIndex ?? null;
      const colId = event.colDef?.colId ?? event.column?.getColId?.() ?? '';
      const eventTarget = nativeEvent?.target as HTMLElement | null;
      const clickedSelectionCheckbox = isSelectionCheckboxTarget(eventTarget);

      if (colId === '__erpSelection') {
        if (!rowNode) {
          return;
        }
        if (!clickedSelectionCheckbox) {
          publishSelectionDiagnostic({
            action: 'ignore',
            api,
            colId,
            hitZone: 'selection-column',
            rowIndex: currentRowIndex,
            rowKey: resolveRowKey(record),
          });
          return;
        }
        armSuppressRowActivation();
        applyPointerSelection(api, rowNode, {
          shiftPressed,
          ctrlOrMetaPressed,
        });
        publishSelectionDiagnostic({
          action: shiftPressed ? 'range-selection' : 'toggle-selection',
          api,
          colId,
          hitZone: 'selection-checkbox',
          rowIndex: currentRowIndex,
          rowKey: resolveRowKey(record),
        });
        return;
      }

      if (rowNode && (shiftPressed || ctrlOrMetaPressed)) {
        applyPointerSelection(api, rowNode, {
          shiftPressed,
          ctrlOrMetaPressed,
        });
        publishSelectionDiagnostic({
          action: shiftPressed ? 'range-selection' : 'toggle-selection',
          api,
          colId,
          hitZone: 'ordinary-cell',
          rowIndex: currentRowIndex,
          rowKey: resolveRowKey(record),
        });
        return;
      }

      if (isValidRowIndex(currentRowIndex)) {
        rangeAnchorRowIndexRef.current = currentRowIndex;
      }

      activateRecord(record);
      publishSelectionDiagnostic({
        action: 'activate-row',
        api,
        colId,
        hitZone: 'ordinary-cell',
        rowIndex: currentRowIndex,
        rowKey: resolveRowKey(record),
      });
    },
    [activateRecord, applyPointerSelection, armSuppressRowActivation, publishSelectionDiagnostic, resolveRowKey],
  );

  const handleCellFocused = useCallback(
    (event: CellFocusedEvent<TRow>) => {
      if (mouseInteractingRef.current) return;
      const rowIndex = event.rowIndex;
      if (!isValidRowIndex(rowIndex)) return;

      const rowNode = event.api.getDisplayedRowAtIndex(rowIndex);
      const record = rowNode?.data;
      if (!record) return;

      const rowKey = resolveRowKey(record) || null;
      if (!rowKey || rowKey === lastFocusedRowKeyRef.current) {
        return;
      }

      lastFocusedRowKeyRef.current = rowKey;
      activateRecord(record);
    },
    [activateRecord, resolveRowKey],
  );

  const handleGridMouseDownCapture = useCallback(
    (event: ReactMouseEvent<HTMLDivElement>) => {
      if (event.button !== 0) return;

      const target = event.target as HTMLElement | null;
      if (isEditableAgGridKeyboardTarget(target)) return;

      const cell = target?.closest('.ag-cell') as HTMLElement | null;
      const colId = cell?.getAttribute('col-id') ?? '';
      if (!cell || !colId) return;

      if (colId === '__erpSelection') {
        mouseInteractingRef.current = true;
        if (typeof window !== 'undefined') {
          if (clearMouseInteractingTimerRef.current != null) {
            window.clearTimeout(clearMouseInteractingTimerRef.current);
          }
          clearMouseInteractingTimerRef.current = window.setTimeout(() => {
            mouseInteractingRef.current = false;
            clearMouseInteractingTimerRef.current = null;
          }, 0);
        }
        return;
      }

      stopPointerTracking();
      pointerDownRef.current = { x: event.clientX, y: event.clientY };
      pointerDraggedRef.current = false;
      mouseInteractingRef.current = true;

      const handleMove = (moveEvent: MouseEvent) => {
        if (!pointerDownRef.current || moveEvent.buttons !== 1) return;
        const dx = Math.abs(moveEvent.clientX - pointerDownRef.current.x);
        const dy = Math.abs(moveEvent.clientY - pointerDownRef.current.y);
        if (dx >= 8 || dy >= 8) {
          pointerDraggedRef.current = true;
        }
      };

      const handleUp = () => {
        const cellRanges = apiRef.current?.getCellRanges?.() ?? [];
        if (pointerDraggedRef.current && cellRanges.length > 0) {
          skipPointerClickRef.current = true;
          if (clearSkipPointerClickTimerRef.current != null && typeof window !== 'undefined') {
            window.clearTimeout(clearSkipPointerClickTimerRef.current);
          }
          if (typeof window !== 'undefined') {
            clearSkipPointerClickTimerRef.current = window.setTimeout(() => {
              skipPointerClickRef.current = false;
              clearSkipPointerClickTimerRef.current = null;
            }, 0);
          }
        }
        stopPointerTracking();
      };

      pointerTrackingHandlersRef.current = { move: handleMove, up: handleUp };
      window.addEventListener('mousemove', handleMove, true);
      window.addEventListener('mouseup', handleUp, true);
    },
    [apiRef, stopPointerTracking],
  );

  const handleCellKeyDown = useCallback(
    (event: CellKeyDownEvent<TRow>) => {
      const nativeEvent = event.event as KeyboardEvent | undefined;
      if (!nativeEvent) return;
      if (nativeEvent.code !== 'Space' && nativeEvent.key !== ' ') return;
      if (isEditableAgGridKeyboardTarget(nativeEvent.target)) return;

      nativeEvent.preventDefault();
      nativeEvent.stopPropagation();

      const rowNode = event.node;
      if (!rowNode?.data) return;

      toggleRowNodeSelection(event.api, rowNode);
      publishSelectionDiagnostic({
        action: 'toggle-selection',
        api: event.api,
        colId: event.colDef?.colId ?? event.column?.getColId?.() ?? '',
        hitZone: 'keyboard-space',
        rowIndex: rowNode.rowIndex ?? null,
        rowKey: resolveRowKey(rowNode.data),
      });
    },
    [publishSelectionDiagnostic, resolveRowKey, toggleRowNodeSelection],
  );

  const handleSpaceKeyDownCapture = useCallback(
    (event: ReactKeyboardEvent<HTMLDivElement>) => {
      if (event.key !== ' ' && event.key !== 'Spacebar') return;
      if (isEditableAgGridKeyboardTarget(event.target)) return;

      const api = apiRef.current;
      if (!api) return;

      event.preventDefault();
      event.stopPropagation();

      let rowNode = null as ReturnType<GridApi<TRow>['getDisplayedRowAtIndex']> | null;
      const focusedCell = api.getFocusedCell();
      if (focusedCell) {
        rowNode = api.getDisplayedRowAtIndex(focusedCell.rowIndex);
      }

      if (!rowNode && focusedRowKey) {
        api.forEachNode((node) => {
          if (rowNode || !node.data) return;
          const rowKey = resolveRowKey(node.data);
          if (rowKey && rowKey === focusedRowKey) {
            rowNode = node;
          }
        });
      }

      if (!rowNode) {
        rowNode = api.getDisplayedRowAtIndex(0);
      }
      if (!rowNode?.data) return;

      toggleRowNodeSelection(api, rowNode);
      publishSelectionDiagnostic({
        action: 'toggle-selection',
        api,
        colId: '__erpSelection',
        hitZone: 'keyboard-space',
        rowIndex: rowNode.rowIndex ?? null,
        rowKey: resolveRowKey(rowNode.data),
      });
    },
    [apiRef, focusedRowKey, publishSelectionDiagnostic, resolveRowKey, toggleRowNodeSelection],
  );

  return {
    handleCellClicked,
    handleCellFocused,
    handleCellKeyDown,
    handleGridMouseDownCapture,
    handleRowClicked,
    handleSpaceKeyDownCapture,
    selectionDiagnostic,
    selectionDiagnosticsEnabled: enableSelectionDiagnostics && Boolean(debugKey),
  };
};
