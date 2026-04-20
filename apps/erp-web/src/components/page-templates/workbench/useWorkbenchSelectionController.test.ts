import { describe, expect, it } from 'vitest';

import { resolveWorkbenchSelectionState } from './useWorkbenchSelectionController';

type DemoRow = {
  key: string;
};

const resolveRowKey = (row: DemoRow | null | undefined) => row?.key ?? null;

describe('useWorkbenchSelectionController helpers', () => {
  it('trims selected rows when the filtered scope no longer contains them', () => {
    const nextState = resolveWorkbenchSelectionState({
      rows: [{ key: 'row-1' }, { key: 'row-2' }],
      selectionRows: [{ key: 'row-1' }],
      state: {
        selectedRowKeys: ['row-1', 'row-9'],
        focusedRowKey: 'row-9',
        activeRowKey: 'row-9',
        selectionAnchorKey: 'row-9',
      },
      resolveRowKey,
    });

    expect(nextState.selectedRowKeys).toEqual(['row-1']);
    expect(nextState.selectionAnchorKey).toBe('row-1');
  });

  it('falls back to the first visible row for active and focused state', () => {
    const nextState = resolveWorkbenchSelectionState({
      rows: [{ key: 'row-1' }, { key: 'row-2' }],
      state: {
        selectedRowKeys: [],
        focusedRowKey: 'missing-focus',
        activeRowKey: 'missing-active',
        selectionAnchorKey: null,
      },
      resolveRowKey,
    });

    expect(nextState.activeRowKey).toBe('row-1');
    expect(nextState.focusedRowKey).toBe('row-1');
  });

  it('clears persisted selection state once loading has completed with no rows', () => {
    const nextState = resolveWorkbenchSelectionState({
      rows: [],
      loading: false,
      state: {
        selectedRowKeys: ['row-1'],
        focusedRowKey: 'row-1',
        activeRowKey: 'row-1',
        selectionAnchorKey: 'row-1',
      },
      resolveRowKey,
    });

    expect(nextState).toEqual({
      selectedRowKeys: [],
      focusedRowKey: null,
      activeRowKey: null,
      selectionAnchorKey: null,
    });
  });
});
