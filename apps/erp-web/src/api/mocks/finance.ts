import dayjs from 'dayjs';

import type { ProfitSnapshot, ProfitSnapshotListResponse } from '../finance';

type ProfitSnapshotListQuery = {
  shop_id?: string;
  sku_id?: string;
  created_from?: string;
  created_to?: string;
  offset?: number;
  limit?: number;
};

const now = dayjs();

const mockProfitSnapshots: ProfitSnapshot[] = [
  {
    order_id: 'b6de0f47-b713-4876-94d2-d560f4b7b035',
    shop_id: 'SHOP-taobao-1',
    platform_order_id: 'TB-PLAT-10001',
    order_created_at: now.subtract(2, 'day').toISOString(),
    revenue_cent: 12800,
    cost_cent: 7600,
    platform_fee_cent: 420,
    shipping_fee_cent: 800,
    gross_profit_cent: 3980,
    computed_at: now.subtract(3, 'hour').toISOString()
  },
  {
    order_id: '5fb16ad6-9bd6-45c6-a2cf-16bf8a95a625',
    shop_id: 'SHOP-jd-1',
    platform_order_id: 'JD-PLAT-20018',
    order_created_at: now.subtract(1, 'day').toISOString(),
    revenue_cent: 9600,
    cost_cent: 6100,
    platform_fee_cent: 360,
    shipping_fee_cent: 600,
    gross_profit_cent: 2540,
    computed_at: now.subtract(2, 'hour').toISOString()
  },
  {
    order_id: '06d5f48d-55f6-46ef-8fd4-ccef21d64b89',
    shop_id: 'SHOP-pdd-1',
    platform_order_id: 'PDD-PLAT-30025',
    order_created_at: now.subtract(5, 'hour').toISOString(),
    revenue_cent: 5200,
    cost_cent: 3300,
    platform_fee_cent: 180,
    shipping_fee_cent: 360,
    gross_profit_cent: 1360,
    computed_at: now.subtract(1, 'hour').toISOString()
  }
];

export const listMockProfitSnapshots = async (
  query: ProfitSnapshotListQuery
): Promise<ProfitSnapshotListResponse> => {
  const filtered = mockProfitSnapshots.filter((item) => {
    if (query.shop_id && item.shop_id !== query.shop_id) {
      return false;
    }
    if (query.created_from && dayjs(item.order_created_at).isBefore(dayjs(query.created_from))) {
      return false;
    }
    if (query.created_to && dayjs(item.order_created_at).isAfter(dayjs(query.created_to))) {
      return false;
    }
    return true;
  });

  const offset = Math.max(query.offset ?? 0, 0);
  const limit = Math.max(query.limit ?? 50, 1);
  const items = filtered.slice(offset, offset + limit);

  return {
    items,
    total: filtered.length
  };
};
