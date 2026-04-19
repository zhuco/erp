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
    expect(String(colDefs[0]?.cellClass ?? '')).toContain('order-workbench-selection-cell-centered');
    expect(String(colDefs[0]?.headerClass ?? '')).toContain('order-workbench-selection-header-centered');

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
    expect(String(colDefs[0]?.cellClass ?? '')).toContain('order-workbench-selection-cell-centered');

    expect(colDefs[1]?.colId).toBe('__erpRowIndex');
    expect(colDefs[1]?.pinned).toBe('left');
    expect(colDefs[1]?.lockPinned).toBe(true);
    expect(colDefs[1]?.lockPosition).toBe('left');
    expect(colDefs[1]?.suppressMovable).toBe(true);
    expect(colDefs[1]?.sortable).toBe(false);
  });

  it('renders row selection checkbox inside a dedicated hitbox container', () => {
    const colDefs = buildAgGridWorkbenchStructuralColDefs<{ id: string }>({
      onToggleAllSelection: vi.fn(),
    });

    const renderer = colDefs[0]?.cellRenderer;
    expect(typeof renderer).toBe('function');

    const rendered = renderer?.({
      node: {
        data: { id: 'row-1' },
        isSelected: () => false,
      },
    } as never) as { props?: { className?: string; children?: { props?: { className?: string } } } };

    expect(rendered?.props?.className).toContain('ag-selection-checkbox-hitbox');
    expect(rendered?.props?.children?.props?.className).toContain('ag-selection-checkbox');
  });

  it('uses source row index for the row-index value getter and comparator', () => {
    const colDefs = buildAgGridWorkbenchStructuralColDefs<{ id: string }>({
      onToggleAllSelection: vi.fn(),
    });
    const rowIndexColDef = colDefs[1];

    expect(rowIndexColDef?.colId).toBe('__erpRowIndex');
    expect(typeof rowIndexColDef?.valueGetter).toBe('function');
    expect(typeof rowIndexColDef?.comparator).toBe('function');

    const value = rowIndexColDef?.valueGetter?.({
      node: {
        rowIndex: 9,
        sourceRowIndex: 2,
      },
    } as never);
    expect(value).toBe(3);

    const compareResult = rowIndexColDef?.comparator?.(
      0,
      0,
      { rowIndex: 20, sourceRowIndex: 4 } as never,
      { rowIndex: 1, sourceRowIndex: 7 } as never,
      false,
    );
    expect(compareResult).toBeLessThan(0);
  });
});
