import { describe, expect, it } from 'vitest';

import { getMockOrderDetail, getMockOrderList, updateMockOrder } from './orders';

describe('mock orders logistics', () => {
  it('does not inject logistics default when order has no logistics', () => {
    const detail = getMockOrderDetail('ORDER-NOT-EXISTS');
    expect(String(detail.order.logistics_company ?? '').trim()).toBe('');
  });

  it('persists logistics company through updateMockOrder', () => {
    const firstOrderId = getMockOrderList({ offset: 0, limit: 1 }).items[0]?.id;
    expect(firstOrderId).toBeTruthy();
    const orderId = String(firstOrderId);

    updateMockOrder(orderId, { logistics_company: 'YTO Express' });
    const detail = getMockOrderDetail(orderId);

    expect(detail.order.logistics_company).toBe('YTO Express');
  });

  it('returns updated logistics company in order list rows', () => {
    const firstOrder = getMockOrderList({ offset: 0, limit: 1 }).items[0];
    expect(firstOrder).toBeTruthy();

    const orderId = String(firstOrder?.id ?? '');
    updateMockOrder(orderId, { logistics_company: '中通快递' });

    const list = getMockOrderList({ offset: 0, limit: 20 });
    const updatedRow = list.items.find((item) => item.id === orderId);

    expect(updatedRow?.logistics_company).toBe('中通快递');
  });
});
