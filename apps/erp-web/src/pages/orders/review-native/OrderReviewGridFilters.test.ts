import { describe, expect, it } from 'vitest';

import type { ColumnFilters } from './types';
import {
  buildMainGridFilterModel,
  extractMainGridColumnFilters,
  hasSameMainGridColumnFilters,
} from './OrderReviewGridFilters';

describe('OrderReviewGridFilters', () => {
  it('builds set-filter models with all selected values', () => {
    const filters: ColumnFilters = {
      platform: ['taobao', 'jd'],
      shopId: ['SHOP-taobao-1', 'SHOP-jd-2'],
      logisticsCompany: ['顺丰', '圆通'],
    };

    expect(buildMainGridFilterModel(filters)).toMatchObject({
      platform: { filterType: 'set', values: ['taobao', 'jd'] },
      shop_id: { filterType: 'set', values: ['SHOP-taobao-1', 'SHOP-jd-2'] },
      logistics_company: { filterType: 'set', values: ['顺丰', '圆通'] },
    });
  });

  it('extracts all selected values from the grid filter model', () => {
    expect(
      extractMainGridColumnFilters({
        platform: { filterType: 'set', values: ['taobao', 'jd'] },
        shop_id: { filterType: 'set', values: ['SHOP-taobao-1', 'SHOP-jd-2'] },
        logistics_company: { filterType: 'set', values: ['顺丰', '圆通'] },
      }),
    ).toMatchObject({
      platform: ['taobao', 'jd'],
      shopId: ['SHOP-taobao-1', 'SHOP-jd-2'],
      logisticsCompany: ['顺丰', '圆通'],
    });
  });

  it('treats multi-select values with the same members as equal regardless of order', () => {
    expect(
      hasSameMainGridColumnFilters(
        {
          platform: ['jd', 'taobao'],
          shopId: ['SHOP-jd-2', 'SHOP-taobao-1'],
          logisticsCompany: ['圆通', '顺丰'],
        },
        {
          platform: ['taobao', 'jd'],
          shopId: ['SHOP-taobao-1', 'SHOP-jd-2'],
          logisticsCompany: ['顺丰', '圆通'],
        },
      ),
    ).toBe(true);
  });
});
