import dayjs from 'dayjs';
import { describe, expect, it } from 'vitest';

import {
  buildMockOrderNo,
  calculateMockOrderGoodsAmountCent,
  calculateMockOrderGoodsQty,
  createMockOrderNoRegistry,
  ensureMockOrderCanReview,
} from './orderMockGovernance';

describe('orderMockGovernance', () => {
  it('builds HC-prefixed order numbers with expandable daily global sequences', () => {
    const currentDay = dayjs('2026-04-19');

    expect(buildMockOrderNo(currentDay, 1)).toBe('HC202604190001');
    expect(buildMockOrderNo(currentDay, 9999)).toBe('HC202604199999');
    expect(buildMockOrderNo(currentDay, 10000)).toBe('HC2026041910000');
  });

  it('continues the daily global sequence from existing order numbers', () => {
    const registry = createMockOrderNoRegistry([
      'HC202604190009',
      'HC2026041910000',
      'INVALID-ORDER-NO',
    ]);

    expect(registry.reserve('2026-04-19')).toBe('HC2026041910001');
    expect(registry.reserve('2026-04-20')).toBe('HC202604200001');
  });

  it('derives goods quantity and amount from line items', () => {
    const items = [
      { qty: 2, price_cent: 300 },
      { quantity: 3, price_cent: 500 },
      { qty: 1, price_cent: 1200 },
    ];

    expect(calculateMockOrderGoodsQty(items)).toBe(6);
    expect(calculateMockOrderGoodsAmountCent(items)).toBe(3300);
  });

  it('blocks order review when there are no goods', () => {
    expect(() => ensureMockOrderCanReview([], 'HC202604190001')).toThrow('订单 HC202604190001 无货品，不允许审核');
    expect(() => ensureMockOrderCanReview([{ qty: 1, price_cent: 100 }], 'HC202604190002')).not.toThrow();
  });
});
