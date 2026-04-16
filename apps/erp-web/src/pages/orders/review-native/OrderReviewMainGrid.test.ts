import { describe, expect, it, vi } from 'vitest';

import { buildOrderReviewMainGridColDefs, buildOrderReviewMainGridDefaultColDef } from './OrderReviewMainGrid';

describe('OrderReviewMainGrid column config', () => {
  it('uses narrow min-width constraints for business columns', () => {
    const colDefs = buildOrderReviewMainGridColDefs({
      headerChecked: false,
      headerIndeterminate: false,
      logisticsOptions: [],
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
      headerChecked: false,
      headerIndeterminate: false,
      logisticsOptions: [],
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
});
