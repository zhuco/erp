import * as agGridWorkbenchStructuralColumns from './agGridWorkbenchStructuralColumns';
import { describe, expect, it, vi } from 'vitest';

import {
  buildAgGridWorkbenchStructuralColDefs,
  ORDER_WORKBENCH_STRUCTURAL_COLUMN_IDS,
} from './agGridWorkbenchStructuralColumns';

describe('agGridWorkbenchStructuralColumns', () => {
  it('defines structural columns without fixed left pinning by default', () => {
    const colDefs = buildAgGridWorkbenchStructuralColDefs<{ id: string }>({
      onToggleAllSelection: vi.fn(),
      onOpenColumnSettings: vi.fn(),
    });

    expect(colDefs).toHaveLength(2);
    expect(Array.from(ORDER_WORKBENCH_STRUCTURAL_COLUMN_IDS)).toEqual(['__erpSelection', '__erpRowIndex']);

    expect(colDefs[0]?.colId).toBe('__erpSelection');
    expect(colDefs[0]?.pinned).toBeUndefined();
    expect(colDefs[0]?.lockPinned).toBeUndefined();
    expect(colDefs[0]?.lockPosition).toBeUndefined();
    expect(colDefs[0]?.suppressMovable).toBeUndefined();
    expect(colDefs[0]?.width).toBe(24);
    expect(colDefs[0]?.minWidth).toBe(24);
    expect(colDefs[0]?.maxWidth).toBe(24);
    expect(colDefs[0]?.resizable).toBe(false);
    expect(String(colDefs[0]?.cellClass ?? '')).toContain('order-workbench-selection-cell-centered');
    expect(String(colDefs[0]?.headerClass ?? '')).toContain('order-workbench-selection-header-centered');

    expect(colDefs[1]?.colId).toBe('__erpRowIndex');
    expect(colDefs[1]?.headerName).toBe('\u5e8f');
    expect(colDefs[1]?.pinned).toBeUndefined();
    expect(colDefs[1]?.lockPinned).toBeUndefined();
    expect(colDefs[1]?.lockPosition).toBeUndefined();
    expect(colDefs[1]?.suppressMovable).toBeUndefined();
    expect(colDefs[1]?.width).toBe(40);
    expect(colDefs[1]?.minWidth).toBe(40);
    expect(colDefs[1]?.maxWidth).toBe(40);
    expect(colDefs[1]?.resizable).toBe(false);
    expect(colDefs[1]?.sortable).toBe(false);
    expect(colDefs[1]?.filter).toBe('agTextColumnFilter');
    expect(colDefs[1]?.suppressCellFlash).toBe(true);
    expect(colDefs[1]?.volatile).toBe(true);
    expect(typeof colDefs[1]?.cellRenderer).toBe('function');
    expect(typeof colDefs[1]?.floatingFilterComponent).toBe('function');
    expect((colDefs[1]?.floatingFilterComponentParams as { buttonLabel?: string } | undefined)?.buttonLabel).toBe('');
    expect((colDefs[1]?.floatingFilterComponentParams as { ariaLabel?: string } | undefined)?.ariaLabel).toBe(
      '\u5217\u8bbe\u7f6e',
    );
    expect(String(colDefs[1]?.cellClass ?? '')).toContain('order-workbench-row-index-cell-centered');
    expect(String(colDefs[1]?.headerClass ?? '')).toContain('order-workbench-row-index-header-centered');
  });

  it('supports keeping structural columns fixed at the far left when explicitly requested', () => {
    const colDefs = buildAgGridWorkbenchStructuralColDefs<{ id: string }>({
      onToggleAllSelection: vi.fn(),
      onOpenColumnSettings: vi.fn(),
      pinLeft: true,
    });

    expect(colDefs[0]?.colId).toBe('__erpSelection');
    expect(colDefs[0]?.pinned).toBe('left');
    expect(colDefs[0]?.lockPinned).toBe(true);
    expect(colDefs[0]?.lockPosition).toBe('left');
    expect(colDefs[0]?.suppressMovable).toBe(true);
    expect(colDefs[0]?.resizable).toBe(false);
    expect(String(colDefs[0]?.cellClass ?? '')).toContain('order-workbench-selection-cell-centered');

    expect(colDefs[1]?.colId).toBe('__erpRowIndex');
    expect(colDefs[1]?.pinned).toBe('left');
    expect(colDefs[1]?.lockPinned).toBe(true);
    expect(colDefs[1]?.lockPosition).toBe('left');
    expect(colDefs[1]?.suppressMovable).toBe(true);
    expect(colDefs[1]?.resizable).toBe(false);
    expect(colDefs[1]?.sortable).toBe(false);
    expect(colDefs[1]?.suppressCellFlash).toBe(true);
    expect(colDefs[1]?.volatile).toBe(true);
    expect(typeof colDefs[1]?.cellRenderer).toBe('function');
    expect(
      typeof (colDefs[1]?.floatingFilterComponentParams as { onOpenColumnSettings?: unknown } | undefined)
        ?.onOpenColumnSettings,
    ).toBe('function');
  });

  it('renders row selection checkbox inside a dedicated hitbox container', () => {
    const colDefs = buildAgGridWorkbenchStructuralColDefs<{ id: string }>({
      onToggleAllSelection: vi.fn(),
      onOpenColumnSettings: vi.fn(),
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

  it('uses the current displayed row index for the row-index value getter and comparator', () => {
    const colDefs = buildAgGridWorkbenchStructuralColDefs<{ id: string }>({
      onToggleAllSelection: vi.fn(),
      onOpenColumnSettings: vi.fn(),
    });
    const rowIndexColDef = colDefs[1];

    expect(rowIndexColDef?.colId).toBe('__erpRowIndex');
    expect(typeof rowIndexColDef?.valueGetter).toBe('function');
    expect(typeof rowIndexColDef?.comparator).toBe('function');

    const valueGetter = rowIndexColDef?.valueGetter as ((params: unknown) => unknown) | undefined;
    const value = valueGetter?.({
      node: {
        rowIndex: 9,
        sourceRowIndex: 2,
      },
    } as never);
    expect(value).toBe(10);

    const comparator = rowIndexColDef?.comparator as
      | ((left: unknown, right: unknown, nodeA: unknown, nodeB: unknown, isDescending: boolean) => number)
      | undefined;
    const compareResult = comparator?.(
      0,
      0,
      { rowIndex: 20, sourceRowIndex: 4 } as never,
      { rowIndex: 1, sourceRowIndex: 7 } as never,
      false,
    );
    expect(compareResult).toBeGreaterThan(0);
  });

  it('ignores source row metadata and always follows the current displayed row index', () => {
    const colDefs = buildAgGridWorkbenchStructuralColDefs<{ id: string }>({
      onToggleAllSelection: vi.fn(),
      onOpenColumnSettings: vi.fn(),
    });
    const rowIndexColDef = colDefs[1];

    const valueGetter = rowIndexColDef?.valueGetter as ((params: unknown) => unknown) | undefined;
    const value = valueGetter?.({
      data: {
        id: 'row-1',
        __erpSourceRowIndex: 4,
      },
      node: {
        rowIndex: 99,
      },
    } as never);

    expect(value).toBe(100);

    const comparator = rowIndexColDef?.comparator as
      | ((left: unknown, right: unknown, nodeA: unknown, nodeB: unknown, isDescending: boolean) => number)
      | undefined;
    const compareResult = comparator?.(
      0,
      0,
      {
        rowIndex: 40,
        data: { id: 'row-a', __erpSourceRowIndex: 2 },
      } as never,
      {
        rowIndex: 1,
        data: { id: 'row-b', __erpSourceRowIndex: 8 },
      } as never,
      false,
    );

    expect(compareResult).toBeGreaterThan(0);
  });

  it('adds stable source row metadata for the current loaded dataset', () => {
    expect(typeof agGridWorkbenchStructuralColumns.attachWorkbenchSourceRowIndex).toBe('function');

    const rows = agGridWorkbenchStructuralColumns.attachWorkbenchSourceRowIndex?.([
      { id: 'row-1', name: 'Alpha' },
      { id: 'row-2', name: 'Beta' },
    ]);

    expect(rows).toEqual([
      { id: 'row-1', name: 'Alpha', __erpSourceRowIndex: 0 },
      { id: 'row-2', name: 'Beta', __erpSourceRowIndex: 1 },
    ]);
  });

  it('keeps pre-indexed metadata intact for callers that still reuse it elsewhere', () => {
    const sourceRows = agGridWorkbenchStructuralColumns.attachWorkbenchSourceRowIndex?.([
      { id: 'row-1', name: 'Alpha' },
      { id: 'row-2', name: 'Beta' },
      { id: 'row-3', name: 'Gamma' },
    ]);

    const sortedSubset = [sourceRows?.[2], sourceRows?.[0]].filter(Boolean);
    const rows = agGridWorkbenchStructuralColumns.attachWorkbenchSourceRowIndex?.(sortedSubset);

    expect(rows).toEqual([
      { id: 'row-3', name: 'Gamma', __erpSourceRowIndex: 2 },
      { id: 'row-1', name: 'Alpha', __erpSourceRowIndex: 0 },
    ]);
  });
});
