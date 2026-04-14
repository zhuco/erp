import type { ColDef, ICellRendererParams } from 'ag-grid-community';

import { AgGridSelectionHeaderCheckbox } from './AgGridSelectionHeaderCheckbox';

export const ORDER_WORKBENCH_STRUCTURAL_COLUMN_IDS = new Set(['__erpSelection', '__erpRowIndex']);

type BuildAgGridWorkbenchStructuralColDefsOptions = {
  onToggleAllSelection: () => void;
  rowIndexHeaderName?: string;
  pinLeft?: boolean;
};

const renderSelectionCheckbox = <TRow,>(params: ICellRendererParams<TRow>) => {
  const rowNode = params.node;
  if (!rowNode?.data) return null;

  return (
    <span
      className={`ag-selection-checkbox erp-selection-checkbox-visual${rowNode.isSelected() ? ' is-checked' : ''}`}
      title="勾选/取消勾选"
      aria-hidden="true"
    />
  );
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
    headerClass: 'order-review-main-header-center order-review-main-selection-header-cell',
    cellClass: 'order-review-main-selection-cell',
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
    filter: false,
    resizable: true,
    ...(pinLeft
      ? {
          pinned: 'left' as const,
          lockPinned: true,
          lockPosition: 'left' as const,
          suppressMovable: true,
        }
      : {}),
    valueGetter: (params) => Number(params.node?.rowIndex ?? 0) + 1,
    headerClass: 'order-review-main-header-center order-review-main-row-index-header',
    cellClass: 'order-review-main-row-index-cell',
  },
];
