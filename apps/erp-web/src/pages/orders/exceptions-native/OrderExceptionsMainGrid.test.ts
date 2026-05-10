import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it, vi } from 'vitest';

import { OrderTagBadges } from '@/components/OrderTagBadges';

import { buildOrderExceptionsMainGridColDefs } from './OrderExceptionsMainGrid';

describe('OrderExceptionsMainGrid column config', () => {
  it('adds a standardized mark column for exception workbench rows', () => {
    const colDefs = buildOrderExceptionsMainGridColDefs({
      platformOptions: [],
      shopOptions: [],
      warehouseOptions: [],
      auditStatusOptions: [],
      exceptionStatusOptions: [],
      slaStatusOptions: [],
      assigneeOptions: [],
      onOpenColumnSettings: vi.fn(),
      onToggleAllSelection: vi.fn(),
      resolveAuditLabel: (value) => value ?? '-',
      resolveExceptionLabel: (value) => value ?? '-',
    });

    const colDefMap = new Map(colDefs.map((colDef) => [String(colDef.colId ?? ''), colDef]));

    expect(colDefMap.get('builtin_marks')).toBeDefined();
    expect(colDefMap.get('builtin_marks')?.headerName).toBe('记号');
  });

  it('renders the mark column in single-line truncation mode', () => {
    const colDefs = buildOrderExceptionsMainGridColDefs({
      platformOptions: [],
      shopOptions: [],
      warehouseOptions: [],
      auditStatusOptions: [],
      exceptionStatusOptions: [],
      slaStatusOptions: [],
      assigneeOptions: [],
      onOpenColumnSettings: vi.fn(),
      onToggleAllSelection: vi.fn(),
      resolveAuditLabel: (value) => value ?? '-',
      resolveExceptionLabel: (value) => value ?? '-',
    });

    const markColumn = colDefs.find((colDef) => String(colDef.colId ?? '') === 'builtin_marks');
    const cellRenderer =
      markColumn?.cellRenderer as
        | ((params: unknown) => { type?: unknown; props?: { singleLine?: boolean; emptyText?: unknown } })
        | undefined;

    const element = cellRenderer?.({
      data: {
        order_id: 'EX-1',
        order_no: 'SO-1',
        platform: 'TAOBAO',
        shop_id: 'SHOP-1',
        total_amount_cent: 1000,
        audit_status: 'UNDER_REVIEW',
        exception_status: 'OPEN',
        fulfillment_status: 'PENDING',
        reserve_status: null,
        target_warehouse_id: 'WH-1',
        blocking_reason: '地址缺失',
      },
    } as never);

    expect(element?.type).toBe(OrderTagBadges);
    expect(element?.props?.emptyText).toBeNull();
    expect(element?.props?.singleLine).toBe(true);
  });

  it('uses readable Chinese headers and filter labels for the recheck main grid', () => {
    const colDefs = buildOrderExceptionsMainGridColDefs({
      platformOptions: [{ label: '淘宝', value: 'TAOBAO' }],
      shopOptions: [{ label: '旗舰店', value: 'SHOP-1' }],
      warehouseOptions: [{ label: '华东仓', value: 'WH-1' }],
      auditStatusOptions: [{ label: '待复核', value: 'UNDER_REVIEW' }],
      exceptionStatusOptions: [{ label: '处理中', value: 'OPEN' }],
      slaStatusOptions: [{ label: '正常', value: 'NORMAL' }],
      assigneeOptions: [{ label: '张三', value: '张三' }],
      onOpenColumnSettings: vi.fn(),
      onToggleAllSelection: vi.fn(),
      resolveAuditLabel: (value) => value ?? '-',
      resolveExceptionLabel: (value) => value ?? '-',
    });

    const colDefMap = new Map(colDefs.map((colDef) => [String(colDef.colId ?? ''), colDef]));

    expect(colDefMap.get('platform')?.headerName).toBe('平台');
    expect(colDefMap.get('shop_id')?.headerName).toBe('店铺');
    expect(colDefMap.get('total_amount_cent')?.headerName).toBe('金额');
    expect(colDefMap.get('reserve_status')?.headerName).toBe('库存占用');
    expect(colDefMap.get('sla_remaining_minutes')?.headerName).toBe('SLA 剩余');
    expect(colDefMap.get('sla_deadline_at')?.headerName).toBe('SLA 截止');
    expect(colDefMap.get('blocking_reason')?.headerName).toBe('阻断原因');

    expect(colDefMap.get('platform')?.floatingFilterComponentParams?.emptyLabel).toBe('全部平台');
    expect(colDefMap.get('shop_id')?.floatingFilterComponentParams?.emptyLabel).toBe('全部店铺');
    expect(colDefMap.get('receiver_name')?.floatingFilterComponentParams?.placeholder).toBe('筛收件人');
    expect(colDefMap.get('target_warehouse_id')?.floatingFilterComponentParams?.emptyLabel).toBe('全部仓库');
  });

  it('uses unified compact row metrics for the industrial workbench grid', () => {
    const source = readFileSync(resolve(__dirname, 'OrderExceptionsMainGrid.tsx'), 'utf8');

    expect(source).toContain('rowHeight={ORDER_WORKBENCH_GRID_ROW_HEIGHT}');
    expect(source).toContain('headerHeight={ORDER_WORKBENCH_GRID_HEADER_HEIGHT}');
  });

  it('suppresses space-key scrolling at the shared grid boundary without debouncing vertical scrollbar rendering', () => {
    const source = readFileSync(resolve(__dirname, 'OrderExceptionsMainGrid.tsx'), 'utf8');

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
    const source = readFileSync(resolve(__dirname, 'OrderExceptionsMainGrid.tsx'), 'utf8');

    expect(source).toContain('if (targetApi.getDisplayedRowCount() > 0) {');
    expect(source).toContain('<Spin size="large" />');
    expect(source).not.toContain('<Spin size="large" tip=');
  });

  it('persists sort state when business-column sorting changes', () => {
    const source = readFileSync(resolve(__dirname, 'OrderExceptionsMainGrid.tsx'), 'utf8');

    expect(source).toContain('const handleSortChanged = useCallback(');
    expect(source).toContain('schedulePersistColumnState(event.api);');
    expect(source).toContain('onSortChanged={handleSortChanged}');
  });
});
