import { describe, expect, it } from 'vitest';

import {
  clearOrderExceptionsMainGridColumnFilters,
  mergeOrderExceptionsFilterOptionsWithCurrentValue,
  resolveSingleOrderExceptionsColumnFilters,
} from './OrderExceptionsGridFilters';

describe('OrderExceptionsGridFilters', () => {
  it('在切换到第二个列筛选时只保留最新的一列', () => {
    const nextFilters = resolveSingleOrderExceptionsColumnFilters(
      {
        platform: ['PDD'],
      },
      {
        platform: ['PDD'],
        shopId: ['SHOP-2'],
      },
    );

    expect(nextFilters).toEqual({
      shopId: ['SHOP-2'],
    });
  });

  it('会把当前已选值补回筛选选项，避免下拉框留空', () => {
    const options = mergeOrderExceptionsFilterOptionsWithCurrentValue(
      [{ label: '淘宝', value: 'TAOBAO' }],
      'PDD',
      (value) => `平台:${value}`,
    );

    expect(options).toEqual([
      { label: '淘宝', value: 'TAOBAO' },
      { label: '平台:PDD', value: 'PDD' },
    ]);
  });
  it('clears all recheck main-grid column filters before applying a left filter', () => {
    expect(
      clearOrderExceptionsMainGridColumnFilters({
        platform: ['PDD'],
        shopId: ['SHOP-2'],
        blockingReasonKeyword: '库存不足',
      }),
    ).toEqual({});
  });
});
