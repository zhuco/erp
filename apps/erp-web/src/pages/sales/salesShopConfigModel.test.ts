import { describe, expect, it } from 'vitest';

import type { SalesShopConfig } from '@/api/salesShops';
import type { Shop } from '@/api/shops';

import {
  buildSalesShopRows,
  buildSalesShopPageWarnings,
  resolveSalesShopPageError,
} from './salesShopConfigModel';

const mockShops: Shop[] = [
  {
    shop_id: 'SHOP-1',
    tenant_id: 'TENANT-1',
    platform: 'taobao',
    platform_shop_id: 'TB-1',
    name: 'Taobao Shop 1',
    status: 'AUTHORIZED',
    auth_meta_json: {},
    created_at: '2026-04-01T00:00:00.000Z',
    updated_at: '2026-04-02T00:00:00.000Z',
    token_status: null,
  },
  {
    shop_id: 'SHOP-2',
    tenant_id: 'TENANT-1',
    platform: 'jd',
    platform_shop_id: 'JD-2',
    name: 'JD Shop 2',
    status: 'AUTHORIZED',
    auth_meta_json: {},
    created_at: '2026-04-01T00:00:00.000Z',
    updated_at: '2026-04-02T00:00:00.000Z',
    token_status: null,
  },
];

const mockConfigs: SalesShopConfig[] = [
  {
    shop_id: 'SHOP-1',
    platform: 'taobao',
    platform_shop_id: 'TB-1',
    name: 'Taobao Shop 1',
    status: 'AUTHORIZED',
    auto_download_orders: true,
    auto_to_review: true,
    download_interval_minutes: 5,
    auto_merge_non_shipping_time: false,
    non_shipping_time_start: null,
    non_shipping_time_end: null,
    enable_stock_sync: true,
    enable_after_sales_download: true,
    enable_logistics_upload: true,
  },
];

describe('salesShopConfigModel', () => {
  it('falls back to shop list when config list is empty', () => {
    const rows = buildSalesShopRows([], mockShops);
    expect(rows).toHaveLength(2);
    expect(rows[0]).toMatchObject({
      shop_id: 'SHOP-1',
      platform: 'taobao',
      name: 'Taobao Shop 1',
    });
    expect(rows[1]).toMatchObject({
      shop_id: 'SHOP-2',
      platform: 'jd',
      name: 'JD Shop 2',
    });
  });

  it('merges config rows with missing shops from shop list', () => {
    const rows = buildSalesShopRows(mockConfigs, mockShops);
    expect(rows).toHaveLength(2);
    const fallback = rows.find((row) => row.shop_id === 'SHOP-2');
    expect(fallback).toMatchObject({
      shop_id: 'SHOP-2',
      auto_download_orders: false,
      download_interval_minutes: 30,
    });
  });

  it('does not block table when dependency queries fail but rows exist', () => {
    const dependencyError = new Error('shops failed');
    const error = resolveSalesShopPageError({
      hasRows: true,
      configError: null,
      shopsError: dependencyError,
      syncJobsError: dependencyError,
    });
    expect(error).toBeNull();
  });

  it('returns config error when primary query fails and no rows exist', () => {
    const configError = new Error('configs failed');
    const error = resolveSalesShopPageError({
      hasRows: false,
      configError,
      shopsError: null,
      syncJobsError: null,
    });
    expect(error).toBe(configError);
  });

  it('builds warnings for partial dependency failures', () => {
    const warnings = buildSalesShopPageWarnings({
      shopsError: new Error('shops failed'),
      syncJobsError: new Error('sync failed'),
    });
    expect(warnings).toEqual([
      '店铺基础信息加载失败，列表已使用配置数据降级展示。',
      '同步任务状态加载失败，列表中的进度与状态可能延迟。',
    ]);
  });
});
