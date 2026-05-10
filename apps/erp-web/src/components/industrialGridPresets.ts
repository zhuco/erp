export type ErpGridPresetName = 'industrial-review-compact';

export type ErpGridPresetConfig = {
  className?: string;
  rowHeight: number;
  headerHeight: number;
  floatingFilter?: boolean;
  showRowIndex: boolean;
  rowIndexWidth: number;
  selectionColumnWidth: number;
  selectionCellMode: 'ag' | 'custom';
  selectionHeaderMode: 'default' | 'allInvert';
  structuralColumnStyle?: 'default' | 'order-review';
  animateRows: boolean;
};

export const INDUSTRIAL_REVIEW_GRID_COMPACT_PRESET: ErpGridPresetConfig = {
  className: 'erp-data-grid--industrial-compact order-review-main-grid-shell',
  rowHeight: 22,
  headerHeight: 30,
  floatingFilter: true,
  showRowIndex: true,
  rowIndexWidth: 40,
  selectionColumnWidth: 24,
  selectionCellMode: 'custom',
  selectionHeaderMode: 'allInvert',
  structuralColumnStyle: 'order-review',
  animateRows: false,
};

export const resolveErpGridPreset = (
  preset?: ErpGridPresetName | null,
): ErpGridPresetConfig | null => {
  if (preset === 'industrial-review-compact') {
    return INDUSTRIAL_REVIEW_GRID_COMPACT_PRESET;
  }
  return null;
};
