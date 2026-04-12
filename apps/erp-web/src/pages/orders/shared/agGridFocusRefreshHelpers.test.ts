import { describe, expect, it, vi } from 'vitest';

import { redrawFocusedRows } from './agGridFocusRefreshHelpers';

describe('redrawFocusedRows', () => {
  it('只重绘旧焦点行和新焦点行', () => {
    const rowA = { id: 'A' };
    const rowB = { id: 'B' };
    const redrawRows = vi.fn();
    const getRowNode = vi.fn((key: string) => {
      if (key === 'A') return rowA;
      if (key === 'B') return rowB;
      return null;
    });

    redrawFocusedRows({
      api: {
        getRowNode,
        redrawRows,
      },
      previousFocusedRowKey: 'A',
      nextFocusedRowKey: 'B',
    });

    expect(getRowNode).toHaveBeenCalledWith('A');
    expect(getRowNode).toHaveBeenCalledWith('B');
    expect(redrawRows).toHaveBeenCalledWith({ rowNodes: [rowA, rowB] });
  });

  it('相同焦点只重绘一次', () => {
    const rowA = { id: 'A' };
    const redrawRows = vi.fn();
    const getRowNode = vi.fn(() => rowA);

    redrawFocusedRows({
      api: {
        getRowNode,
        redrawRows,
      },
      previousFocusedRowKey: 'A',
      nextFocusedRowKey: 'A',
    });

    expect(getRowNode).toHaveBeenCalledTimes(1);
    expect(redrawRows).toHaveBeenCalledWith({ rowNodes: [rowA] });
  });

  it('只重绘当前页面上仍然存在的焦点行', () => {
    const rowB = { id: 'B' };
    const redrawRows = vi.fn();
    const getRowNode = vi.fn((key: string) => {
      if (key === 'B') return rowB;
      return null;
    });

    redrawFocusedRows({
      api: {
        getRowNode,
        redrawRows,
      },
      previousFocusedRowKey: 'A',
      nextFocusedRowKey: 'B',
    });

    expect(redrawRows).toHaveBeenCalledWith({ rowNodes: [rowB] });
  });

  it('页面上没有对应行时不触发整表重绘', () => {
    const redrawRows = vi.fn();
    const getRowNode = vi.fn(() => null);

    redrawFocusedRows({
      api: {
        getRowNode,
        redrawRows,
      },
      previousFocusedRowKey: 'A',
      nextFocusedRowKey: 'B',
    });

    expect(redrawRows).not.toHaveBeenCalled();
  });
});
