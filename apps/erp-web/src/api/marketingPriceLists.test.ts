import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@/utils/env', () => ({
  apiBaseUrl: 'http://localhost:8080',
  useMocks: true
}));

import {
  createMarketingPriceItem,
  createMarketingPriceList,
  getMarketingPriceList,
  listMarketingPriceLists,
  updateMarketingPriceItem,
  updateMarketingPriceList
} from './marketingPriceLists';

const tenantId = '11111111-1111-1111-1111-111111111111';

describe('marketingPriceLists mock API', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('creates price list and queryable by keyword', async () => {
    const created = await createMarketingPriceList(tenantId, {
      name: 'UT-价格表-001',
      currency: 'CNY',
      status: 'DRAFT'
    });
    expect(created.name).toBe('UT-价格表-001');

    const list = await listMarketingPriceLists(tenantId, { keyword: 'UT-价格表-001' });
    expect(list.total).toBeGreaterThanOrEqual(1);
    expect(list.items.some((item) => item.price_list_id === created.price_list_id)).toBe(true);
  });

  it('updates price list and item detail', async () => {
    const created = await createMarketingPriceList(tenantId, {
      name: 'UT-价格表-002',
      currency: 'CNY'
    });
    const updatedList = await updateMarketingPriceList(tenantId, created.price_list_id, {
      name: 'UT-价格表-002-已更新',
      status: 'ACTIVE'
    });
    expect(updatedList.name).toBe('UT-价格表-002-已更新');
    expect(updatedList.status).toBe('ACTIVE');

    const createdItem = await createMarketingPriceItem(tenantId, created.price_list_id, {
      sku_id: 'SKU-UT-PRICE-1',
      base_price_cent: 1000,
      channel_price_cent: 980
    });
    expect(createdItem.sku_id).toBe('SKU-UT-PRICE-1');

    const updatedItem = await updateMarketingPriceItem(tenantId, createdItem.item_id, {
      base_price_cent: 1200,
      member_price_cent: 900
    });
    expect(updatedItem.base_price_cent).toBe(1200);
    expect(updatedItem.member_price_cent).toBe(900);

    const detail = await getMarketingPriceList(tenantId, created.price_list_id);
    expect(detail.items.some((item) => item.item_id === createdItem.item_id)).toBe(true);
  });
});

