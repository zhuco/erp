import { describe, expect, it } from 'vitest';

import { toOrderWorkbenchGoodsRows } from './orderWorkbenchGoodsRows';

describe('toOrderWorkbenchGoodsRows', () => {
  it('keeps shop product name and spec visible for manual matching rows', () => {
    const rows = toOrderWorkbenchGoodsRows([
      {
        id: 'ROW-1',
        title: '夏季短袖连衣裙',
        spec_text: '雾蓝 L',
        platform_sku_id: 'TB-SKU-001',
        outer_sku_id: 'MCH-001',
        quantity: 2,
        price_cent: 1299,
        mapping_missing: true,
      },
    ]);

    expect(rows).toEqual([
      expect.objectContaining({
        __localKey: 'ROW-1',
        shopProductName: '夏季短袖连衣裙',
        shopSpec: '雾蓝 L',
        mappingMissing: true,
      }),
    ]);
  });

  it('normalizes alternate shop product fields and array specs', () => {
    const rows = toOrderWorkbenchGoodsRows([
      {
        platform_sku_id: 'JD-SKU-002',
        product_title: '工装外套',
        sale_attributes: ['黑色', 'XL'],
        qty: 1,
        price_cent: 2599,
      },
    ]);

    expect(rows[0]).toMatchObject({
      __localKey: 'JD-SKU-002-0',
      shopProductName: '工装外套',
      shopSpec: '黑色 / XL',
      quantity: 1,
      price_cent: 2599,
    });
  });
});
