import { describe, expect, it } from 'vitest';

import { getMockAfterSalesList } from './afterSales';

describe('售后列表 mock 契约', () => {
  it('返回列表摘要字段而不是详情 case 结构', () => {
    const response = getMockAfterSalesList({ page_size: 1 });
    const first = response.data[0];

    expect(first).toBeTruthy();
    expect(first?.case_id).toBeTruthy();
    expect(first?.tenant_id).toBeTruthy();
    expect(first?.shop_id).toBeTruthy();
    expect(first?.platform).toBeTruthy();
    expect(first?.case_status).toBeTruthy();
    expect(Object.prototype.hasOwnProperty.call(first ?? {}, 'platform_order_id')).toBe(true);
    expect(first).not.toHaveProperty('items');
  });
});
