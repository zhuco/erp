import { describe, expect, it } from 'vitest';

import type { OrderExceptionSummary } from '@/api/orders';

import {
  filterOrderExceptionSelectionKeys,
  findOrderExceptionRowIndex,
  hasOrderExceptionRowKey,
  normalizeOrderExceptionKey,
  resolveNextOrderExceptionActiveKey,
  resolveOrderExceptionRowKey,
} from './orderExceptionsRowKey';

const rows = [
  { order_id: '1001' },
  { order_id: '1002' },
] as OrderExceptionSummary[];

describe('orderExceptionsRowKey', () => {
  it('会统一规范化订单异常主键', () => {
    expect(normalizeOrderExceptionKey(1001)).toBe('1001');
    expect(normalizeOrderExceptionKey(' 1002 ')).toBe('1002');
    expect(resolveOrderExceptionRowKey({ order_id: ' 1003 ' } as OrderExceptionSummary)).toBe('1003');
  });

  it('支持字符串和数字主键的等值比较', () => {
    expect(hasOrderExceptionRowKey({ order_id: '1001' } as OrderExceptionSummary, 1001)).toBe(true);
    expect(hasOrderExceptionRowKey({ order_id: '1001' } as OrderExceptionSummary, '1002')).toBe(false);
  });

  it('会过滤不存在的勾选键并去重', () => {
    expect(filterOrderExceptionSelectionKeys(rows, [1001, '1002', '1002', '9999'])).toEqual(['1001', '1002']);
  });

  it('会优先保留当前仍然存在的激活键', () => {
    expect(resolveNextOrderExceptionActiveKey(rows, 1002)).toBe('1002');
  });

  it('当前激活键不存在时会回退到首行', () => {
    expect(resolveNextOrderExceptionActiveKey(rows, '9999')).toBe('1001');
    expect(resolveNextOrderExceptionActiveKey([], '9999')).toBeNull();
  });

  it('能够用规范化主键找到行索引', () => {
    expect(findOrderExceptionRowIndex(rows, 1002)).toBe(1);
    expect(findOrderExceptionRowIndex(rows, 'not-found')).toBe(-1);
  });
});
