import { describe, expect, it, vi } from 'vitest';

import {
  buildAgGridWorkbenchStructuralColDefs,
  ORDER_WORKBENCH_STRUCTURAL_COLUMN_IDS,
} from './agGridWorkbenchStructuralColumns';

describe('agGridWorkbenchStructuralColumns', () => {
  it('defines structural columns without fixed left pinning by default', () => {
    const colDefs = buildAgGridWorkbenchStructuralColDefs<{ id: string }>({
      onToggleAllSelection: vi.fn(),
    });

    expect(colDefs).toHaveLength(2);
    expect(Array.from(ORDER_WORKBENCH_STRUCTURAL_COLUMN_IDS)).toEqual(['__erpSelection', '__erpRowIndex']);

    expect(colDefs[0]?.colId).toBe('__erpSelection');
    expect(colDefs[0]?.pinned).toBeUndefined();
    expect(colDefs[0]?.lockPinned).toBeUndefined();
    expect(colDefs[0]?.lockPosition).toBeUndefined();
    expect(colDefs[0]?.suppressMovable).toBeUndefined();

    expect(colDefs[1]?.colId).toBe('__erpRowIndex');
    expect(colDefs[1]?.pinned).toBeUndefined();
    expect(colDefs[1]?.lockPinned).toBeUndefined();
    expect(colDefs[1]?.lockPosition).toBeUndefined();
    expect(colDefs[1]?.suppressMovable).toBeUndefined();
  });

  it('supports keeping structural columns fixed at the far left when explicitly requested', () => {
    const colDefs = buildAgGridWorkbenchStructuralColDefs<{ id: string }>({
      onToggleAllSelection: vi.fn(),
      pinLeft: true,
    });

    expect(colDefs[0]?.colId).toBe('__erpSelection');
    expect(colDefs[0]?.pinned).toBe('left');
    expect(colDefs[0]?.lockPinned).toBe(true);
    expect(colDefs[0]?.lockPosition).toBe('left');
    expect(colDefs[0]?.suppressMovable).toBe(true);

    expect(colDefs[1]?.colId).toBe('__erpRowIndex');
    expect(colDefs[1]?.pinned).toBe('left');
    expect(colDefs[1]?.lockPinned).toBe(true);
    expect(colDefs[1]?.lockPosition).toBe('left');
    expect(colDefs[1]?.suppressMovable).toBe(true);
  });
});
