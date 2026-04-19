import type { ColDef, ICellRendererParams } from 'ag-grid-community';

import { AgGridSelectionHeaderCheckbox } from './AgGridSelectionHeaderCheckbox';
import './agGridWorkbenchStructuralColumns.css';

export const ORDER_WORKBENCH_STRUCTURAL_COLUMN_IDS = new Set(['__erpSelection', '__erpRowIndex']);

type BuildAgGridWorkbenchStructuralColDefsOptions = {
  onToggleAllSelection: () => void;
  rowIndexHeaderName?: string;
  pinLeft?: boolean;
};

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

const resolveSourceRowIndex = (rowNode: { rowIndex?: number | null; sourceRowIndex?: number | null } | null | undefined) => {
  const sourceRowIndex =
    typeof rowNode?.sourceRowIndex === 'number' && Number.isFinite(rowNode.sourceRowIndex)
      ? rowNode.sourceRowIndex
      : null;
  if (sourceRowIndex != null && sourceRowIndex >= 0) {
    return sourceRowIndex;
  }
  const displayedRowIndex =
    typeof rowNode?.rowIndex === 'number' && Number.isFinite(rowNode.rowIndex) ? rowNode.rowIndex : 0;
  return Math.max(0, displayedRowIndex);
};

export const buildAgGridWorkbenchStructuralColDefs = <TRow,>({
  onToggleAllSelection,
  rowIndexHeaderName = '',
  pinLeft = false,
}: BuildAgGridWorkbenchStructuralColDefsOptions): ColDef<TRow>[] => [
  {
    colId: '__erpSelection',
    headerName: '',
    width: 36,
    minWidth: 28,
    maxWidth: 44,
    resizable: true,
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
    width: 48,
    minWidth: 38,
    maxWidth: 58,
    sortable: false,
    filter: 'agNumberColumnFilter',
    resizable: true,
    ...(pinLeft
      ? {
          pinned: 'left' as const,
          lockPinned: true,
          lockPosition: 'left' as const,
          suppressMovable: true,
        }
      : {}),
    valueGetter: (params) => resolveSourceRowIndex(params.node as { rowIndex?: number | null; sourceRowIndex?: number | null }) + 1,
    comparator: (_left, _right, nodeA, nodeB) => {
      const leftIndex = resolveSourceRowIndex(nodeA as { rowIndex?: number | null; sourceRowIndex?: number | null });
      const rightIndex = resolveSourceRowIndex(nodeB as { rowIndex?: number | null; sourceRowIndex?: number | null });
      return leftIndex - rightIndex;
    },
    headerClass: 'order-review-main-header-center order-review-main-row-index-header',
    cellClass: 'order-review-main-row-index-cell',
  },
];
