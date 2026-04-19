import { describe, expect, it } from 'vitest';

import { resolveOrderActionId, resolveOrderRowKey } from './helpers';
import type { OrderSummary } from '@/api/orders';

const asOrderSummary = (value: Partial<OrderSummary>): OrderSummary => value as OrderSummary;

describe('order review helpers null-safety', () => {
  it('resolveOrderRowKey returns empty string when record is null', () => {
    expect(() => resolveOrderRowKey(null as unknown as OrderSummary)).not.toThrow();
    expect(resolveOrderRowKey(null as unknown as OrderSummary)).toBe('');
  });

  it('resolveOrderActionId returns empty string when record is null', () => {
    expect(() => resolveOrderActionId(null as unknown as OrderSummary)).not.toThrow();
    expect(resolveOrderActionId(null as unknown as OrderSummary)).toBe('');
  });

  it('resolveOrderRowKey prefers id over fallback fields', () => {
    const record = asOrderSummary({
      id: 'ORDER-001',
      platform_order_id: 'PLAT-001',
      shop_id: 'SHOP-001',
      receiver_phone: '13800000000',
      created_at: '2026-04-16T00:00:00.000Z',
    });
    expect(resolveOrderRowKey(record)).toBe('ORDER-001');
  });
});
