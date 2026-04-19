import { describe, expect, it } from 'vitest';

import {
  applyShiftRangeSelection,
  resolveAgGridHeaderSelectionState,
  resolveShiftRangeSelectionAction,
  toggleAgGridHeaderSelection,
} from './agGridSelectionHelpers';

type FakeRowNode = {
  selected: boolean;
  isSelected: () => boolean;
  setSelected: (next: boolean) => void;
};

const createRows = (size: number, selectedIndexes: number[] = []) =>
  Array.from({ length: size }, (_, index) => {
    const row: FakeRowNode = {
      selected: selectedIndexes.includes(index),
      isSelected() {
        return row.selected;
      },
      setSelected(next: boolean) {
        row.selected = next;
      },
    };
    return row;
  });

const collectSelectedIndexes = (rows: FakeRowNode[]) =>
  rows.flatMap((row, index) => (row.selected ? [index] : []));

describe('resolveShiftRangeSelectionAction', () => {
  it('区间内存在未勾选行时会返回整段勾选动作', () => {
    const rows = createRows(6, [1, 3]);

    const action = resolveShiftRangeSelectionAction({
      getDisplayedRowAtIndex: (index) => rows[index],
      anchorIndex: 1,
      currentRowIndex: 4,
    });

    expect(action).toMatchObject({
      startIndex: 1,
      endIndex: 4,
      shouldSelect: true,
    });
  });

  it('区间内全部已勾选时会返回整段取消动作', () => {
    const rows = createRows(6, [2, 3, 4]);

    const action = resolveShiftRangeSelectionAction({
      getDisplayedRowAtIndex: (index) => rows[index],
      anchorIndex: 4,
      currentRowIndex: 2,
    });

    expect(action).toMatchObject({
      startIndex: 2,
      endIndex: 4,
      shouldSelect: false,
    });
  });
});

describe('applyShiftRangeSelection', () => {
  it('区间内有部分勾选时会补齐整段勾选', () => {
    const rows = createRows(5, [1]);

    applyShiftRangeSelection({
      getDisplayedRowAtIndex: (index) => rows[index],
      anchorIndex: 1,
      currentRowIndex: 3,
    });

    expect(collectSelectedIndexes(rows)).toEqual([1, 2, 3]);
  });

  it('区间内全部勾选时会整段取消勾选', () => {
    const rows = createRows(5, [1, 2, 3]);

    applyShiftRangeSelection({
      getDisplayedRowAtIndex: (index) => rows[index],
      anchorIndex: 1,
      currentRowIndex: 3,
    });

    expect(collectSelectedIndexes(rows)).toEqual([]);
  });

  it('支持从靠后的行反向 Shift 到靠前的行', () => {
    const rows = createRows(6, [4]);

    applyShiftRangeSelection({
      getDisplayedRowAtIndex: (index) => rows[index],
      anchorIndex: 4,
      currentRowIndex: 1,
    });

    expect(collectSelectedIndexes(rows)).toEqual([1, 2, 3, 4]);
  });
});

describe('toggleAgGridHeaderSelection', () => {
  it('表头未全选时会选中当前筛选结果中的全部行', () => {
    const rows = createRows(5, [1]);

    toggleAgGridHeaderSelection({
      headerChecked: false,
      deselectAll: () => {
        rows.forEach((row) => row.setSelected(false));
      },
      forEachNodeAfterFilter: (callback) => {
        rows.forEach((row) => callback(row));
      },
    });

    expect(collectSelectedIndexes(rows)).toEqual([0, 1, 2, 3, 4]);
  });

  it('表头已全选时会取消全部勾选', () => {
    const rows = createRows(4, [0, 1, 2, 3]);

    toggleAgGridHeaderSelection({
      headerChecked: true,
      deselectAll: () => {
        rows.forEach((row) => row.setSelected(false));
      },
      forEachNodeAfterFilter: (callback) => {
        rows.forEach((row) => callback(row));
      },
    });

    expect(collectSelectedIndexes(rows)).toEqual([]);
  });
});

describe('resolveAgGridHeaderSelectionState', () => {
  it('没有可见行时返回未勾选且非半选', () => {
    const state = resolveAgGridHeaderSelectionState<FakeRowNode>({
      forEachNodeAfterFilter: () => undefined,
    });

    expect(state).toEqual({
      checked: false,
      indeterminate: false,
      totalCount: 0,
      selectedCount: 0,
    });
  });

  it('可见行部分勾选时返回半选态', () => {
    const rows = createRows(4, [1, 2]);

    const state = resolveAgGridHeaderSelectionState({
      forEachNodeAfterFilter: (callback) => {
        rows.forEach((row) => callback(row));
      },
    });

    expect(state).toEqual({
      checked: false,
      indeterminate: true,
      totalCount: 4,
      selectedCount: 2,
    });
  });

  it('可见行全部勾选时返回全选态', () => {
    const rows = createRows(3, [0, 1, 2]);

    const state = resolveAgGridHeaderSelectionState({
      forEachNodeAfterFilter: (callback) => {
        rows.forEach((row) => callback(row));
      },
    });

    expect(state).toEqual({
      checked: true,
      indeterminate: false,
      totalCount: 3,
      selectedCount: 3,
    });
  });
});
