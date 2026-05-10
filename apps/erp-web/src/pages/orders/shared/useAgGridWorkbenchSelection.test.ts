import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest';
import type { GridApi } from 'ag-grid-community';

import {
  resolveSpaceSelectionRowNode,
  suppressAgGridSpaceKeyboardEvent,
  suppressAgGridSpaceKeyboardEventOnKeyUp,
} from './useAgGridWorkbenchSelection';

describe('useAgGridWorkbenchSelection keyboard guards', () => {
  const originalHTMLElement = globalThis.HTMLElement;

  class FakeHTMLElement {
    tagName: string;
    isContentEditable = false;

    constructor(tagName = 'DIV') {
      this.tagName = tagName;
    }

    closest() {
      return null;
    }
  }

  beforeAll(() => {
    globalThis.HTMLElement = FakeHTMLElement as unknown as typeof HTMLElement;
  });

  afterAll(() => {
    globalThis.HTMLElement = originalHTMLElement;
  });

  it('fully consumes non-editable space key events to prevent grid and page scrolling', () => {
    const target = new FakeHTMLElement('DIV');
    const event = {
      code: 'Space',
      key: ' ',
      preventDefault: vi.fn(),
      stopImmediatePropagation: vi.fn(),
      stopPropagation: vi.fn(),
      target,
    };

    expect(suppressAgGridSpaceKeyboardEvent(event as never)).toBe(true);
    expect(event.preventDefault).toHaveBeenCalledTimes(1);
    expect(event.stopPropagation).toHaveBeenCalledTimes(1);
    expect(event.stopImmediatePropagation).toHaveBeenCalledTimes(1);
  });

  it('does not consume editable input space key events', () => {
    const target = new FakeHTMLElement('INPUT');
    const event = {
      code: 'Space',
      key: ' ',
      preventDefault: vi.fn(),
      stopImmediatePropagation: vi.fn(),
      stopPropagation: vi.fn(),
      target,
    };

    expect(suppressAgGridSpaceKeyboardEvent(event as never)).toBe(false);
    expect(event.preventDefault).not.toHaveBeenCalled();
    expect(event.stopPropagation).not.toHaveBeenCalled();
    expect(event.stopImmediatePropagation).not.toHaveBeenCalled();
  });

  it('also consumes non-editable space keyup events to block trailing viewport scroll', () => {
    const target = new FakeHTMLElement('DIV');
    const event = {
      code: 'Space',
      key: ' ',
      preventDefault: vi.fn(),
      stopImmediatePropagation: vi.fn(),
      stopPropagation: vi.fn(),
      target,
    };

    expect(suppressAgGridSpaceKeyboardEventOnKeyUp(event as never)).toBe(true);
    expect(event.preventDefault).toHaveBeenCalledTimes(1);
    expect(event.stopPropagation).toHaveBeenCalledTimes(1);
    expect(event.stopImmediatePropagation).toHaveBeenCalledTimes(1);
  });
});

type DemoRow = {
  key: string;
};

const buildGridApiStub = ({
  focusedRowIndex,
  rows,
}: {
  focusedRowIndex: number | null;
  rows: DemoRow[];
}) => {
  const nodes = rows.map((row, rowIndex) => ({
    data: row,
    rowIndex,
  }));

  return {
    forEachNode: (callback: (node: { data: DemoRow; rowIndex: number }) => void) => {
      nodes.forEach((node) => callback(node));
    },
    getDisplayedRowAtIndex: (rowIndex: number) => nodes[rowIndex] ?? null,
    getFocusedCell: () => (focusedRowIndex == null ? null : { rowIndex: focusedRowIndex }),
  } as Pick<GridApi<DemoRow>, 'forEachNode' | 'getDisplayedRowAtIndex' | 'getFocusedCell'>;
};

describe('resolveSpaceSelectionRowNode', () => {
  it('prefers the latest clicked row when the grid focused cell is still unavailable', () => {
    const api = buildGridApiStub({
      focusedRowIndex: null,
      rows: [{ key: 'row-1' }, { key: 'row-2' }],
    });

    const rowNode = resolveSpaceSelectionRowNode({
      api,
      focusedRowKey: 'row-1',
      recentRowKey: 'row-2',
      resolveRowKey: (row) => row?.key ?? null,
    });

    expect(rowNode?.data?.key).toBe('row-2');
  });

  it('falls back to the external focused row when there is no immediate click context', () => {
    const api = buildGridApiStub({
      focusedRowIndex: null,
      rows: [{ key: 'row-1' }, { key: 'row-2' }],
    });

    const rowNode = resolveSpaceSelectionRowNode({
      api,
      focusedRowKey: 'row-2',
      recentRowKey: null,
      resolveRowKey: (row) => row?.key ?? null,
    });

    expect(rowNode?.data?.key).toBe('row-2');
  });
});
