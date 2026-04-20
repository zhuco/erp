import type { OrderExceptionSummary } from '@/api/orders';

import {
  buildOrderExceptionsScreenFilterOptions,
  createOrderExceptionsScreenFilterState,
  filterOrderExceptionsByScreenFilters,
  hasOrderExceptionsScreenFilters,
  normalizeOrderExceptionsScreenFilterState,
} from './orderExceptionsScreenFilters';

import { describe, expect, it } from 'vitest';

const createExceptionRow = (
  overrides: Partial<OrderExceptionSummary> & {
    order_id: string;
    order_no: string;
  },
): OrderExceptionSummary =>
  ({
    order_id: overrides.order_id,
    order_no: overrides.order_no,
    platform: 'TAOBAO',
    shop_id: 'SHOP-01',
    receiver_name: '张三',
    total_amount_cent: 1000,
    audit_status: 'UNDER_REVIEW',
    exception_status: 'OPEN',
    fulfillment_status: 'PENDING',
    reserve_status: null,
    target_warehouse_id: 'WH-01',
    blocking_reason: '地址缺失',
    assignee_id: 'u-1',
    assignee_name: '客服甲',
    latest_action: '待处理',
    latest_action_at: '2026-04-19T09:00:00+08:00',
    ...overrides,
  }) satisfies OrderExceptionSummary;

describe('orderExceptionsScreenFilters', () => {
  it('creates an empty default screen filter state', () => {
    expect(createOrderExceptionsScreenFilterState()).toEqual({
      orderQuery: '',
      platform: '',
      shopId: '',
      auditStatus: '',
      exceptionStatus: '',
      slaStatus: '',
      warehouseId: '',
      assigneeName: '',
      blockingReasonKeyword: '',
    });
  });

  it('detects whether screen filters are active after normalization', () => {
    expect(hasOrderExceptionsScreenFilters(createOrderExceptionsScreenFilterState())).toBe(false);
    expect(
      hasOrderExceptionsScreenFilters(
        normalizeOrderExceptionsScreenFilterState({
          orderQuery: ' SO-1001 ',
        }),
      ),
    ).toBe(true);
    expect(
      normalizeOrderExceptionsScreenFilterState({
        slaStatus: 'due_soon',
      }).slaStatus,
    ).toBe('DUE_SOON');
  });

  it('filters rows by formal screen filters including computed SLA status', () => {
    const rows = [
      createExceptionRow({
        order_id: 'ex-1',
        order_no: 'SO-1001',
        platform: 'TAOBAO',
        shop_id: 'SHOP-A',
        receiver_name: '张三',
        audit_status: 'UNDER_REVIEW',
        exception_status: 'OPEN',
        target_warehouse_id: 'WH-A',
        blocking_reason: '地址缺失',
        assignee_name: '客服甲',
        latest_action_at: '2026-04-19T09:50:00+08:00',
        sla_remaining_minutes: 10,
      } as Partial<OrderExceptionSummary>),
      createExceptionRow({
        order_id: 'ex-2',
        order_no: 'SO-1002',
        platform: 'JD',
        shop_id: 'SHOP-B',
        receiver_name: '李四',
        audit_status: 'REVIEW_REQUIRED',
        exception_status: 'ESCALATED',
        target_warehouse_id: 'WH-B',
        blocking_reason: '库存锁定',
        assignee_name: '客服乙',
        latest_action_at: '2026-04-19T08:00:00+08:00',
        sla_remaining_minutes: -5,
      } as Partial<OrderExceptionSummary>),
    ];

    const result = filterOrderExceptionsByScreenFilters(rows, {
      orderQuery: 'SO-1002',
      platform: 'JD',
      shopId: 'SHOP-B',
      auditStatus: 'REVIEW_REQUIRED',
      exceptionStatus: 'ESCALATED',
      slaStatus: 'OVERDUE',
      warehouseId: 'WH-B',
      assigneeName: '客服乙',
      blockingReasonKeyword: '库存',
    });

    expect(result).toHaveLength(1);
    expect(result[0]?.order_id).toBe('ex-2');
  });

  it('builds sorted screen filter options with deduplication and labels', () => {
    const options = buildOrderExceptionsScreenFilterOptions(
      ['JD', 'TAOBAO', 'JD', '', null, 'PDD'],
      (value) => `平台-${value}`,
    );

    expect(options).toEqual([
      { value: 'JD', label: '平台-JD' },
      { value: 'PDD', label: '平台-PDD' },
      { value: 'TAOBAO', label: '平台-TAOBAO' },
    ]);
  });
});
