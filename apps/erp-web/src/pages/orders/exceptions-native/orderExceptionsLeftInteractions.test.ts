import { describe, expect, it } from 'vitest';

import type { OrderExceptionSummary } from '@/api/orders';

import {
  buildOrderExceptionsCheckedKeys,
  filterOrderExceptionsRows,
  hasOrderExceptionsLeftManualFilter,
  reduceOrderExceptionsFiltersByNode,
  type OrderExceptionsLeftFilterNode,
  type OrderExceptionsLeftFilters,
} from './orderExceptionsLeftInteractions';

const rowFixture: OrderExceptionSummary = {
  order_id: 'o-1',
  order_no: 'SO-001',
  platform: 'TAOBAO',
  shop_id: 'shop-a',
  receiver_name: 'user-a',
  total_amount_cent: 10000,
  audit_status: 'PENDING',
  exception_status: 'OPEN',
  fulfillment_status: 'PENDING',
  reserve_status: null,
  target_warehouse_id: 'wh-hz',
  blocking_reason: 'stock shortage',
  assignee_id: 'u-1',
  assignee_name: 'reviewer-a',
  latest_action: 'pending',
  latest_action_at: '2026-04-01T08:00:00Z',
};

describe('orderExceptionsLeftInteractions', () => {
  it('detects whether manual filter is active', () => {
    expect(hasOrderExceptionsLeftManualFilter({})).toBe(false);
    expect(hasOrderExceptionsLeftManualFilter({ status: ['OPEN'] })).toBe(true);
    expect(hasOrderExceptionsLeftManualFilter({ sla: ['mine'] })).toBe(true);
  });

  it('supports single-select toggle and clears other left dimensions', () => {
    const statusNode: OrderExceptionsLeftFilterNode = { key: 'status:OPEN', nodeType: 'status' };
    const next = reduceOrderExceptionsFiltersByNode({}, statusNode);
    expect(next.status).toEqual(['OPEN']);

    const toggled = reduceOrderExceptionsFiltersByNode(next, statusNode);
    expect(toggled.status).toBeUndefined();

    const clearNode: OrderExceptionsLeftFilterNode = { key: 'status:*', nodeType: 'status' };
    const cleared = reduceOrderExceptionsFiltersByNode(
      { status: ['OPEN'], assignee: ['reviewer-a'] },
      clearNode,
    );
    expect(cleared.status).toBeUndefined();
    expect(cleared.assignee).toBeUndefined();

    const assigneeNode: OrderExceptionsLeftFilterNode = {
      key: 'assignee:reviewer-a',
      nodeType: 'assignee',
    };
    const switched = reduceOrderExceptionsFiltersByNode(
      { status: ['OPEN'], warehouseId: ['wh-hz'] },
      assigneeNode,
    );
    expect(switched.status).toBeUndefined();
    expect(switched.warehouseId).toBeUndefined();
    expect(switched.assignee).toEqual(['reviewer-a']);
  });

  it('builds checked keys for all dimensions', () => {
    const filters: OrderExceptionsLeftFilters = {
      status: ['OPEN'],
      assignee: ['reviewer-a'],
      sla: ['mine'],
      warehouseId: ['wh-hz'],
    };
    const checked = buildOrderExceptionsCheckedKeys(filters);
    expect(checked.has('status:OPEN')).toBe(true);
    expect(checked.has('assignee:reviewer-a')).toBe(true);
    expect(checked.has('sla:mine')).toBe(true);
    expect(checked.has('warehouse:wh-hz')).toBe(true);
  });

  it('filters rows by status/assignee/sla/warehouse', () => {
    const rows: OrderExceptionSummary[] = [
      rowFixture,
      {
        ...rowFixture,
        order_id: 'o-2',
        order_no: 'SO-002',
        exception_status: 'PROCESSING',
        assignee_name: 'reviewer-b',
        target_warehouse_id: 'wh-gz',
        latest_action_at: new Date().toISOString(),
      },
    ];

    const result = filterOrderExceptionsRows(
      rows,
      {
        status: ['OPEN'],
        assignee: ['reviewer-a'],
        sla: ['mine'],
        warehouseId: ['wh-hz'],
      },
      {
        currentUserId: 'u-1',
        nowTs: Date.parse('2026-04-01T09:00:00Z'),
        overdueThresholdMinutes: 15,
      },
    );

    expect(result).toHaveLength(1);
    expect(result[0]?.order_id).toBe('o-1');
  });
});
