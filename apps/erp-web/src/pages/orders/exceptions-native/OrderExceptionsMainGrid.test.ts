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

  it('uses unified compact row metrics for the industrial workbench grid', () => {
    const source = readFileSync(resolve(__dirname, 'OrderExceptionsMainGrid.tsx'), 'utf8');

    expect(source).toContain('rowHeight={ORDER_WORKBENCH_GRID_ROW_HEIGHT}');
    expect(source).toContain('headerHeight={ORDER_WORKBENCH_GRID_HEADER_HEIGHT}');
  });

  it('suppresses space-key scrolling at the shared grid boundary without debouncing vertical scrollbar rendering', () => {
    const source = readFileSync(resolve(__dirname, 'OrderExceptionsMainGrid.tsx'), 'utf8');

    expect(source).toContain('suppressKeyboardEvent={suppressKeyboardSpaceScroll}');
    expect(source).toContain('onKeyUpCapture={handleSpaceKeyUpCapture}');
    expect(source).not.toContain('debounceVerticalScrollbar');
  });
});
