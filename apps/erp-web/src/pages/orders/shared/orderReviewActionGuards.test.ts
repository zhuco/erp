import { describe, expect, it } from 'vitest';

import type { OrderSummary } from '@/api/orders';

import { hasOrdersWithEmptyLogistics } from './orderReviewActionGuards';

const toOrder = (input: Partial<OrderSummary> & Record<string, unknown>): OrderSummary => input as OrderSummary;

describe('order review action guards', () => {
  it('returns true when selected orders include empty logistics company', () => {
    const rows: OrderSummary[] = [
      toOrder({ id: 'A', logistics_company: '' }),
      toOrder({ id: 'B', logistics_company: 'SF Express' }),
    ];

    const result = hasOrdersWithEmptyLogistics(['A'], rows, (row) => String(row.id ?? ''));
    expect(result).toBe(true);
  });

  it('returns false when selected orders all have logistics company', () => {
    const rows: OrderSummary[] = [
      toOrder({ id: 'A', logistics_company: 'YTO Express' }),
      toOrder({ id: 'B', carrier_name: 'ZTO Express' }),
    ];

    const result = hasOrdersWithEmptyLogistics(['A', 'B'], rows, (row) => String(row.id ?? ''));
    expect(result).toBe(false);
  });
});
