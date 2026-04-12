import { describe, expect, it } from 'vitest';

import type { OrderExceptionSummary } from '@/api/orders';

import { toOrderExceptionsGridRows } from './orderExceptionsGridRows';

const buildRow = (overrides: Partial<OrderExceptionSummary> = {}): OrderExceptionSummary =>
  ({
    order_id: 'ORDER-1',
    order_no: 'NO-1',
    platform: 'TAOBAO',
    shop_id: 'SHOP-1',
    receiver_name: '张三',
    total_amount_cent: 1000,
    audit_status: 'REVIEW_REQUIRED',
    exception_status: 'OPEN',
    fulfillment_status: 'NOT_SUBMITTED',
    reserve_status: null,
    target_warehouse_id: 'WH-1',
    blocking_reason: '库存不足',
    assignee_id: null,
    assignee_name: null,
    latest_action: null,
    latest_action_at: null,
    ...overrides,
  }) satisfies OrderExceptionSummary;

describe('orderExceptionsGridRows', () => {
  it('会为重复 order_id 生成唯一的行 id', () => {
    const rows = toOrderExceptionsGridRows([
      buildRow({ order_id: 'ORDER-1', order_no: 'NO-1-A' }),
      buildRow({ order_id: 'ORDER-1', order_no: 'NO-1-B' }),
      buildRow({ order_id: 'ORDER-1', order_no: 'NO-1-C' }),
    ]);

    const rowIds = rows.map((item) => item.__erpRowId);
    expect(new Set(rowIds).size).toBe(3);
  });

  it('会为缺失 order_id 的行生成非空 id', () => {
    const rows = toOrderExceptionsGridRows([
      buildRow({ order_id: '  ', order_no: 'EMPTY-1' }),
      buildRow({ order_id: '', order_no: 'EMPTY-2' }),
    ]);

    rows.forEach((item) => {
      expect(item.__erpRowId.trim()).not.toBe('');
    });
    expect(new Set(rows.map((item) => item.__erpRowId)).size).toBe(2);
  });
});
