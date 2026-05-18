import type { ColDef, ICellRendererParams } from 'ag-grid-community';
import { useLayoutEffect, useRef } from 'react';

import { WorkbenchColumnSettingsFloatingControl } from '@/components/page-templates';

import { AgGridSelectionHeaderCheckbox } from './AgGridSelectionHeaderCheckbox';
import './agGridWorkbenchStructuralColumns.css';

export const ORDER_WORKBENCH_STRUCTURAL_COLUMN_IDS = new Set(['__erpSelection', '__erpRowIndex']);

export type WorkbenchSourceRowIndexed = {
  __erpSourceRowIndex: number;
};

type BuildAgGridWorkbenchStructuralColDefsOptions = {
  onOpenColumnSettings?: () => void;
  onToggleAllSelection: () => void;
  rowIndexHeaderName?: string;
  pinLeft?: boolean;
};

const WORKBENCH_SELECTION_COLUMN_WIDTH = 24;
const WORKBENCH_ROW_INDEX_COLUMN_WIDTH = 40;

const renderSelectionCheckbox = <TRow,>(params: ICellRendererParams<TRow>) => {
  const rowNode = params.node;
  if (!rowNode?.data) {
    return null;
  }

  return (
    <span className="ag-selection-checkbox-hitbox" aria-hidden="true">
      <span
        className={`ag-selection-checkbox erp-selection-checkbox-visual${rowNode.isSelected() ? ' is-checked' : ''}`}
        title="Toggle selection"
      />
    </span>
  );
};

const readWorkbenchSourceRowIndex = (rowData: unknown) => {
  const value = (rowData as { __erpSourceRowIndex?: unknown } | null | undefined)?.__erpSourceRowIndex;
  return typeof value === 'number' && Number.isFinite(value) && value >= 0 ? value : null;
};

export const attachWorkbenchSourceRowIndex = <TRow extends object>(
  rows: TRow[],
): Array<TRow & WorkbenchSourceRowIndexed> =>
  rows.map((row, index) => {
    const existingIndex = readWorkbenchSourceRowIndex(row);
    return {
      ...(row as TRow),
      __erpSourceRowIndex: existingIndex ?? index,
    };
  });

const resolveDisplayedRowIndex = (rowNode: { rowIndex?: number | null } | null | undefined) => {
  const displayedRowIndex =
    typeof rowNode?.rowIndex === 'number' && Number.isFinite(rowNode.rowIndex) ? rowNode.rowIndex : 0;
  return Math.max(0, displayedRowIndex);
};

const resolveDisplayedRowIndexText = (
  rowElement: Element | null,
  rowNode: { rowIndex?: number | null } | null | undefined,
) => {
  const rawRowIndex = rowElement?.getAttribute('row-index');
  const parsedRowIndex = Number(rawRowIndex);
  if (Number.isFinite(parsedRowIndex) && parsedRowIndex >= 0) {
    return String(parsedRowIndex + 1);
  }
  return String(resolveDisplayedRowIndex(rowNode) + 1);
};

const WorkbenchRowIndexCell = <TRow,>(params: ICellRendererParams<TRow>) => {
  const valueRef = useRef<HTMLSpanElement | null>(null);

  useLayoutEffect(() => {
    const target = valueRef.current;
    if (!target) {
      return;
    }

    const rowElement = target.closest('.ag-row');
    const updateText = () => {
      if (!valueRef.current) {
        return;
      }
      valueRef.current.textContent = resolveDisplayedRowIndexText(rowElement, params.node);
    };

    updateText();

    if (!(rowElement instanceof HTMLElement)) {
      return;
    }

    const observer = new MutationObserver((mutations) => {
      if (mutations.some((mutation) => mutation.type === 'attributes' && mutation.attributeName === 'row-index')) {
        updateText();
      }
    });

    observer.observe(rowElement, {
      attributes: true,
      attributeFilter: ['row-index'],
    });

    return () => {
      observer.disconnect();
    };
  }, [params.node]);

  return (
    <span ref={valueRef} className="order-workbench-row-index-value">
      {resolveDisplayedRowIndex(params.node) + 1}
    </span>
  );
};

export const buildAgGridWorkbenchStructuralColDefs = <TRow,>({
  onOpenColumnSettings,
  onToggleAllSelection,
  rowIndexHeaderName = '序',
  pinLeft = false,
}: BuildAgGridWorkbenchStructuralColDefsOptions): ColDef<TRow>[] => [
  {
    colId: '__erpSelection',
    headerName: '',
    width: WORKBENCH_SELECTION_COLUMN_WIDTH,
    minWidth: WORKBENCH_SELECTION_COLUMN_WIDTH,
    maxWidth: WORKBENCH_SELECTION_COLUMN_WIDTH,
    resizable: false,
    suppressSizeToFit: true,
    sortable: false,
    filter: false,
    ...(pinLeft
      ? {
          pinned: 'left' as const,
          lockPinned: true,
          lockPosition: 'left' as const,
          suppressMovable: true,
        }
      : {}),
    headerClass:
      'order-review-main-header-center order-review-main-selection-header-cell order-workbench-selection-header-centered',
    cellClass: 'order-review-main-selection-cell order-workbench-selection-cell-centered',
    cellRenderer: renderSelectionCheckbox,
    headerComponent: AgGridSelectionHeaderCheckbox,
    headerComponentParams: {
      onToggleAllSelection,
      testId: 'erp-selection-header-checkbox',
    },
  },
  {
    colId: '__erpRowIndex',
    headerName: rowIndexHeaderName,
    width: WORKBENCH_ROW_INDEX_COLUMN_WIDTH,
    minWidth: WORKBENCH_ROW_INDEX_COLUMN_WIDTH,
    maxWidth: WORKBENCH_ROW_INDEX_COLUMN_WIDTH,
    sortable: false,
    suppressSizeToFit: true,
    enableCellChangeFlash: false,
    filter: 'agTextColumnFilter',
    resizable: false,
    floatingFilterComponent: WorkbenchColumnSettingsFloatingControl,
    floatingFilterComponentParams: {
      ariaLabel: '列设置',
      buttonLabel: '',
      onOpenColumnSettings,
      testId: 'workbench-column-settings-trigger',
      controlClassName: 'workbench-column-settings-floating-control--row-index',
      buttonClassName: 'workbench-column-settings-floating-button--row-index',
    },
    ...(pinLeft
      ? {
          pinned: 'left' as const,
          lockPinned: true,
          lockPosition: 'left' as const,
          suppressMovable: true,
        }
      : {}),
    cellRenderer: WorkbenchRowIndexCell,
    valueGetter: (params) => resolveDisplayedRowIndex(params.node as { rowIndex?: number | null }) + 1,
    comparator: (_left, _right, nodeA, nodeB) => {
      const leftIndex = resolveDisplayedRowIndex(nodeA as { rowIndex?: number | null });
      const rightIndex = resolveDisplayedRowIndex(nodeB as { rowIndex?: number | null });
      return leftIndex - rightIndex;
    },
    headerClass:
      'order-review-main-header-center order-review-main-row-index-header order-workbench-row-index-header-centered',
    cellClass: 'order-review-main-row-index-cell order-workbench-row-index-cell-centered',
  },
];
