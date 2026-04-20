import { describe, expect, it } from 'vitest';

import { resolveOrderBuiltinTags } from '@/features/orderTags';

import { getMockOrderDetail, getMockOrderList, listMockOrderExceptions } from './orders';

describe('mock orders dataset scale', () => {
  it('provides enough records for order review and exception workbenches', () => {
    const allOrders = getMockOrderList({ offset: 0, limit: 1 });
    const reviewOrders = getMockOrderList({ review_status: 'PENDING', offset: 0, limit: 1 });
    const exceptionOrders = listMockOrderExceptions({ offset: 0, limit: 1 });

    expect(allOrders.total).toBeGreaterThan(5000);
    expect(reviewOrders.total).toBe(1500);
    expect(exceptionOrders.total).toBe(1100);
  });

  it('spreads standardized marks and custom tags across review orders', () => {
    const reviewOrders = getMockOrderList({ review_status: 'PENDING', offset: 0, limit: 1500 }).items;

    const builtinMarkKeys = new Set(
      reviewOrders.flatMap((order) =>
        resolveOrderBuiltinTags(order as Record<string, unknown>).map((tag) => tag.key),
      ),
    );
    const customTaggedOrders = reviewOrders.filter((order) => {
      const tags = (order as { tags?: unknown }).tags;
      return Array.isArray(tags) && tags.some((item) => String(item ?? '').trim());
    });

    expect(Array.from(builtinMarkKeys)).toEqual(
      expect.arrayContaining([
        'unmatched',
        'shortage',
        'frozen',
        'presale',
        'timeout',
        'refund',
        'merged',
        'split',
      ]),
    );
    expect(customTaggedOrders.length).toBeGreaterThan(0);
  });

  it('returns distinct goods information for different orders', () => {
    const pendingOrders = getMockOrderList({ review_status: 'PENDING', offset: 0, limit: 2 }).items;
    expect(pendingOrders).toHaveLength(2);

    const firstOrderId = String(pendingOrders[0]?.id ?? '');
    const secondOrderId = String(pendingOrders[1]?.id ?? '');

    const firstDetail = getMockOrderDetail(firstOrderId);
    const secondDetail = getMockOrderDetail(secondOrderId);

    expect(firstDetail.items[0]?.title).not.toBe(secondDetail.items[0]?.title);
    expect(firstDetail.items[0]?.spec_text).not.toBe(secondDetail.items[0]?.spec_text);
  });
});
