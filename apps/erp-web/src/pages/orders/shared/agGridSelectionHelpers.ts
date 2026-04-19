type SelectableRowNode = {
  isSelected: () => boolean | undefined;
  setSelected: (next: boolean) => void;
};

type ToggleAgGridHeaderSelectionOptions<TRowNode extends SelectableRowNode> = {
  headerChecked: boolean;
  deselectAll: () => void;
  forEachNodeAfterFilter: (callback: (rowNode: TRowNode) => void) => void;
};

type ResolveAgGridHeaderSelectionStateOptions<TRowNode extends SelectableRowNode> = {
  forEachNodeAfterFilter: (callback: (rowNode: TRowNode) => void) => void;
};

export type AgGridHeaderSelectionState = {
  checked: boolean;
  indeterminate: boolean;
  totalCount: number;
  selectedCount: number;
};

type ApplyShiftRangeSelectionOptions<TRowNode extends SelectableRowNode | null | undefined> = {
  getDisplayedRowAtIndex: (index: number) => TRowNode;
  anchorIndex: number;
  currentRowIndex: number;
};

const getNormalizedRangeIndexes = (anchorIndex: number, currentRowIndex: number) => {
  const startIndex = Math.max(0, Math.min(anchorIndex, currentRowIndex));
  const endIndex = Math.max(startIndex, Math.max(anchorIndex, currentRowIndex));
  return { startIndex, endIndex };
};

export const resolveShiftRangeSelectionAction = <TRowNode extends SelectableRowNode | null | undefined>({
  getDisplayedRowAtIndex,
  anchorIndex,
  currentRowIndex,
}: ApplyShiftRangeSelectionOptions<TRowNode>) => {
  if (!Number.isInteger(anchorIndex) || !Number.isInteger(currentRowIndex)) {
    return null;
  }

  const { startIndex, endIndex } = getNormalizedRangeIndexes(anchorIndex, currentRowIndex);
  let hasRow = false;
  let allSelected = true;

  for (let rowIndex = startIndex; rowIndex <= endIndex; rowIndex += 1) {
    const rowNode = getDisplayedRowAtIndex(rowIndex);
    if (!rowNode) continue;
    hasRow = true;
    if (!rowNode.isSelected()) {
      allSelected = false;
    }
  }

  if (!hasRow) {
    return null;
  }

  return {
    startIndex,
    endIndex,
    shouldSelect: !allSelected,
  };
};

export const applyShiftRangeSelection = <TRowNode extends SelectableRowNode | null | undefined>({
  getDisplayedRowAtIndex,
  anchorIndex,
  currentRowIndex,
}: ApplyShiftRangeSelectionOptions<TRowNode>) => {
  const action = resolveShiftRangeSelectionAction({
    getDisplayedRowAtIndex,
    anchorIndex,
    currentRowIndex,
  });
  if (!action) {
    return;
  }

  for (let rowIndex = action.startIndex; rowIndex <= action.endIndex; rowIndex += 1) {
    const rowNode = getDisplayedRowAtIndex(rowIndex);
    if (!rowNode) continue;
    if ((rowNode.isSelected() ?? false) === action.shouldSelect) continue;
    rowNode.setSelected(action.shouldSelect);
  }
};

export const toggleAgGridHeaderSelection = <TRowNode extends SelectableRowNode>({
  headerChecked,
  deselectAll,
  forEachNodeAfterFilter,
}: ToggleAgGridHeaderSelectionOptions<TRowNode>) => {
  if (headerChecked) {
    deselectAll();
    return;
  }

  forEachNodeAfterFilter((rowNode) => {
    if (!rowNode.isSelected()) {
      rowNode.setSelected(true);
    }
  });
};

export const resolveAgGridHeaderSelectionState = <TRowNode extends SelectableRowNode>({
  forEachNodeAfterFilter,
}: ResolveAgGridHeaderSelectionStateOptions<TRowNode>): AgGridHeaderSelectionState => {
  let totalCount = 0;
  let selectedCount = 0;

  forEachNodeAfterFilter((rowNode) => {
    totalCount += 1;
    if (rowNode.isSelected()) {
      selectedCount += 1;
    }
  });

  return {
    checked: totalCount > 0 && selectedCount === totalCount,
    indeterminate: selectedCount > 0 && selectedCount < totalCount,
    totalCount,
    selectedCount,
  };
};
