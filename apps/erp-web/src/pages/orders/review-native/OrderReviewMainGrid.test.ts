import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it, vi } from 'vitest';

import { OrderTagBadges } from '@/components/OrderTagBadges';

import { buildOrderReviewMainGridColDefs, buildOrderReviewMainGridDefaultColDef } from './OrderReviewMainGrid';

describe('OrderReviewMainGrid column config', () => {
  it('uses narrow min-width constraints for business columns', () => {
    const colDefs = buildOrderReviewMainGridColDefs({
      logisticsOptions: [],
      onOpenColumnSettings: vi.fn(),
      onToggleAllSelection: vi.fn(),
      platformFilterOptions: [],
      shopOptions: [],
    });

    const colDefMap = new Map(colDefs.map((colDef) => [String(colDef.colId ?? ''), colDef]));

    expect(colDefMap.get('id')?.minWidth).toBeLessThanOrEqual(64);
    expect(colDefMap.get('platform_order_id')?.minWidth).toBeLessThanOrEqual(64);
    expect(colDefMap.get('platform')?.minWidth).toBeLessThanOrEqual(56);
    expect(colDefMap.get('shop_id')?.minWidth).toBeLessThanOrEqual(60);
    expect(colDefMap.get('receiver_name')?.minWidth).toBeLessThanOrEqual(56);
    expect(colDefMap.get('receiver_phone')?.minWidth).toBeLessThanOrEqual(60);
    expect(colDefMap.get('logistics_company')?.minWidth).toBeLessThanOrEqual(60);
    expect(colDefMap.get('total_amount_cent')?.minWidth).toBeLessThanOrEqual(60);
    expect(colDefMap.get('goods_qty')).toBeDefined();
    expect(colDefMap.get('ship_warehouse_name')).toBeDefined();
    expect(colDefMap.get('review_status')).toBeDefined();
    expect(colDefMap.get('tracking_no')).toBeDefined();
    expect(colDefMap.get('exception_badges')).toBeDefined();
  });

  it('keeps business columns sortable while structural columns stay unsorted', () => {
    const colDefs = buildOrderReviewMainGridColDefs({
      logisticsOptions: [],
      onOpenColumnSettings: vi.fn(),
      onToggleAllSelection: vi.fn(),
      platformFilterOptions: [],
      shopOptions: [],
    });

    const colDefMap = new Map(colDefs.map((colDef) => [String(colDef.colId ?? ''), colDef]));

    expect(colDefMap.get('__erpSelection')?.sortable).toBe(false);
    expect(colDefMap.get('__erpRowIndex')?.sortable).toBe(false);
    expect(colDefMap.get('__erpSelection')?.pinned).toBe('left');
    expect(colDefMap.get('__erpRowIndex')?.pinned).toBe('left');
    expect(colDefMap.get('__erpSelection')?.suppressMovable).toBe(true);
    expect(colDefMap.get('__erpRowIndex')?.suppressMovable).toBe(true);
    expect(colDefMap.get('id')?.sortable).toBe(true);
    expect(colDefMap.get('created_at')?.sortable).toBe(true);
    expect(colDefMap.get('receiver_name')?.sortable).toBe(true);
  });

  it('uses a resizable default column definition with a compact min-width', () => {
    const defaultColDef = buildOrderReviewMainGridDefaultColDef(true);

    expect(defaultColDef.resizable).toBe(true);
    expect(defaultColDef.sortable).toBe(true);
    expect(defaultColDef.minWidth).toBe(52);
    expect(defaultColDef.floatingFilter).toBe(true);
  });

  it('uses mark column rendering without forcing a dash placeholder', () => {
    const colDefs = buildOrderReviewMainGridColDefs({
      logisticsOptions: [],
      onOpenColumnSettings: vi.fn(),
      onToggleAllSelection: vi.fn(),
      platformFilterOptions: [],
      shopOptions: [],
    });

    const markColumn = colDefs.find((colDef) => String(colDef.colId ?? '') === 'exception_badges');
    const valueGetter = markColumn?.valueGetter as ((params: unknown) => unknown) | undefined;
    const cellRenderer =
      markColumn?.cellRenderer as
        | ((params: unknown) => { type?: unknown; props?: { emptyText?: unknown; singleLine?: boolean } })
        | undefined;

    expect(markColumn?.headerName).toBe('记号');
    expect(valueGetter?.({ data: undefined } as never)).toEqual([]);

    const element = cellRenderer?.({ value: [] } as never);
    expect(element?.type).toBe(OrderTagBadges);
    expect(element?.props?.emptyText).toBeNull();
    expect(element?.props?.singleLine).toBe(true);
  });

  it('uses unified compact row metrics for the industrial workbench grid', () => {
    const source = readFileSync(resolve(__dirname, 'OrderReviewMainGrid.tsx'), 'utf8');

    expect(source).toContain('rowHeight={ORDER_WORKBENCH_GRID_ROW_HEIGHT}');
    expect(source).toContain('headerHeight={ORDER_WORKBENCH_GRID_HEADER_HEIGHT}');
  });

  it('uses tighter shared row metrics and cell padding for the industrial workbench main grid', () => {
    const metricsSource = readFileSync(resolve(__dirname, '../shared/orderWorkbenchGridMetrics.ts'), 'utf8');
    const cssSource = readFileSync(resolve(__dirname, '../orderReviewPage.css'), 'utf8');

    expect(metricsSource).toContain('ORDER_WORKBENCH_GRID_ROW_HEIGHT = 22');
    expect(cssSource).toContain('--ag-cell-horizontal-padding: 4px;');
    expect(cssSource).toContain('--ag-grid-size: 4px;');
  });

  it('suppresses space-key scrolling at the shared grid boundary without debouncing vertical scrollbar rendering', () => {
    const source = readFileSync(resolve(__dirname, 'OrderReviewMainGrid.tsx'), 'utf8');

    expect(source).toContain('suppressKeyboardEvent={suppressKeyboardSpaceScroll}');
    expect(source).toContain('onKeyUpCapture={handleSpaceKeyUpCapture}');
    expect(source).not.toContain('debounceVerticalScrollbar');
  });

  it('persists sort state when business-column sorting changes', () => {
    const source = readFileSync(resolve(__dirname, 'OrderReviewMainGrid.tsx'), 'utf8');

    expect(source).toContain('const handleSortChanged = useCallback(');
    expect(source).toContain('schedulePersistColumnState(event.api);');
    expect(source).toContain('onSortChanged={handleSortChanged}');
  });
});
