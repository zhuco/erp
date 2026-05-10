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
    const defaultColDef = buildOrderReviewMainGridDefaultColDef(true, vi.fn(() => false));

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

  it('uses readable Chinese headers and filter labels for the review main grid', () => {
    const colDefs = buildOrderReviewMainGridColDefs({
      logisticsOptions: [{ label: '顺丰', value: 'SF' }],
      onOpenColumnSettings: vi.fn(),
      onToggleAllSelection: vi.fn(),
      platformFilterOptions: [{ label: '淘宝', value: 'TAOBAO' }],
      shopOptions: [{ label: '旗舰店', value: 'SHOP-1' }],
    });

    const colDefMap = new Map(colDefs.map((colDef) => [String(colDef.colId ?? ''), colDef]));

    expect(colDefMap.get('platform_order_id')?.headerName).toBe('平台单号');
    expect(colDefMap.get('platform')?.headerName).toBe('平台');
    expect(colDefMap.get('created_at')?.headerName).toBe('下单时间');
    expect(colDefMap.get('shop_id')?.headerName).toBe('店铺');
    expect(colDefMap.get('logistics_company')?.headerName).toBe('物流公司');
    expect(colDefMap.get('tracking_no')?.headerName).toBe('物流单号');
    expect(colDefMap.get('total_amount_cent')?.headerName).toBe('应收合计');

    expect(colDefMap.get('platform')?.floatingFilterComponentParams?.emptyLabel).toBe('全部平台');
    expect(colDefMap.get('shop_id')?.floatingFilterComponentParams?.emptyLabel).toBe('全部店铺');
    expect(colDefMap.get('receiver_name')?.floatingFilterComponentParams?.placeholder).toBe('筛收件人');
    expect(colDefMap.get('receiver_phone')?.floatingFilterComponentParams?.placeholder).toBe('筛手机号');
    expect(colDefMap.get('logistics_company')?.floatingFilterComponentParams?.emptyLabel).toBe('全部物流');
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

    expect(source).not.toContain('suppressKeyboardEvent={suppressKeyboardSpaceScroll}');
    expect(source).toContain('suppressKeyboardEvent: suppressKeyboardSpaceScroll');
    expect(source).toContain('onKeyUpCapture={handleSpaceKeyUpCapture}');
    expect(source).not.toContain('suppressRowClickSelection');
    expect(source).toContain('enableClickSelection: false');
    expect(source).not.toContain('debounceVerticalScrollbar');
    expect(source).not.toContain('suppressCellFlash');
    expect(source).not.toContain('volatile');
  });

  it('guards viewport reset on empty datasets and avoids standalone Spin tip usage', () => {
    const source = readFileSync(resolve(__dirname, 'OrderReviewMainGrid.tsx'), 'utf8');

    expect(source).toContain('if (targetApi.getDisplayedRowCount() > 0) {');
    expect(source).toContain('<Spin size="large" />');
    expect(source).not.toContain('<Spin size="large" tip=');
  });

  it('persists sort state when business-column sorting changes', () => {
    const source = readFileSync(resolve(__dirname, 'OrderReviewMainGrid.tsx'), 'utf8');

    expect(source).toContain('const handleSortChanged = useCallback(');
    expect(source).toContain('schedulePersistColumnState(event.api);');
    expect(source).toContain('onSortChanged={handleSortChanged}');
  });
});
