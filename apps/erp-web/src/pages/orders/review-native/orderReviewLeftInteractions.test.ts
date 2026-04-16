import { describe, expect, it } from 'vitest';

import type { ColumnFilters } from './types';
import {
  buildOrderReviewLeftCheckedKeys,
  clearOrderReviewMainGridColumnFilters,
  hasOrderReviewLeftManualFilter,
  reduceOrderReviewFiltersByNode,
  type OrderReviewLeftFilterNode,
} from './orderReviewLeftInteractions';

describe('orderReviewLeftInteractions', () => {
  it('detects whether manual left filter is active', () => {
    expect(hasOrderReviewLeftManualFilter({})).toBe(false);
    expect(hasOrderReviewLeftManualFilter({ platform: ['TAOBAO'] })).toBe(true);
    expect(hasOrderReviewLeftManualFilter({ tagState: ['has'] })).toBe(true);
  });

  it('builds checked keys for all supported dimensions', () => {
    const filters: ColumnFilters = {
      platform: ['TAOBAO'],
      shopId: ['SHOP-1'],
      status: ['normal'],
      timeRange: ['within7d'],
      exceptionType: ['frozen'],
      warehouseId: ['WH-01'],
      logisticsCompany: ['顺丰'],
      remarkState: ['has'],
      tagState: ['none'],
    };
    const checked = buildOrderReviewLeftCheckedKeys(filters);
    expect(checked.has('platform:TAOBAO')).toBe(true);
    expect(checked.has('shop:SHOP-1')).toBe(true);
    expect(checked.has('status:normal')).toBe(true);
    expect(checked.has('time:within7d')).toBe(true);
    expect(checked.has('exception:frozen')).toBe(true);
    expect(checked.has('warehouse:WH-01')).toBe(true);
    expect(checked.has('logistics:顺丰')).toBe(true);
    expect(checked.has('remark:has')).toBe(true);
    expect(checked.has('tag:none')).toBe(true);
  });

  it('supports single-select toggle and all-node clear', () => {
    const platformNode: OrderReviewLeftFilterNode = { key: 'platform:TAOBAO', nodeType: 'platform' };
    const next = reduceOrderReviewFiltersByNode({}, platformNode);
    expect(next.platform).toEqual(['TAOBAO']);

    const toggled = reduceOrderReviewFiltersByNode(next, platformNode);
    expect(toggled.platform).toBeUndefined();

    const clearStatusNode: OrderReviewLeftFilterNode = { key: 'status:*', nodeType: 'status' };
    const cleared = reduceOrderReviewFiltersByNode(
      { platform: ['TAOBAO'], status: ['exception'], remarkState: ['has'] },
      clearStatusNode,
    );
    expect(cleared.platform).toBeUndefined();
    expect(cleared.status).toBeUndefined();
    expect(cleared.remarkState).toBeUndefined();
  });

  it('keeps only the last selected left filter node', () => {
    const yesterdayNode: OrderReviewLeftFilterNode = { key: 'time:yesterday', nodeType: 'time' };
    const todayNode: OrderReviewLeftFilterNode = { key: 'time:today', nodeType: 'time' };

    const afterYesterday = reduceOrderReviewFiltersByNode({}, yesterdayNode);
    expect(afterYesterday.timeRange).toEqual(['yesterday']);

    const afterToday = reduceOrderReviewFiltersByNode(afterYesterday, todayNode);
    expect(afterToday.timeRange).toEqual(['today']);
    expect(afterToday.platform).toBeUndefined();
    expect(afterToday.shopId).toBeUndefined();
    expect(afterToday.status).toBeUndefined();
    expect(afterToday.exceptionType).toBeUndefined();
    expect(afterToday.warehouseId).toBeUndefined();
    expect(afterToday.logisticsCompany).toBeUndefined();
    expect(afterToday.remarkState).toBeUndefined();
    expect(afterToday.tagState).toBeUndefined();
  });

  it('clears main-grid-only column filters before applying a left filter', () => {
    const previous: ColumnFilters = {
      platform: ['TAOBAO'],
      status: ['exception'],
      orderIdKeyword: 'SO-1001',
      platformOrderIdKeyword: 'P-2002',
      receiverNameKeyword: '张三',
      receiverPhoneKeyword: '13800000000',
      createdAtFrom: '2026-04-01',
      createdAtTo: '2026-04-15',
    };

    expect(clearOrderReviewMainGridColumnFilters(previous)).toEqual({
      platform: ['TAOBAO'],
      status: ['exception'],
      orderIdKeyword: undefined,
      platformOrderIdKeyword: undefined,
      receiverNameKeyword: undefined,
      receiverPhoneKeyword: undefined,
      createdAtFrom: undefined,
      createdAtTo: undefined,
    });
  });
});
