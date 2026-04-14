import type { SalesShopConfig } from '@/api/salesShops';
import type { Shop } from '@/api/shops';

type ResolveSalesShopPageErrorOptions = {
  hasRows: boolean;
  configError: unknown;
  shopsError: unknown;
  syncJobsError: unknown;
};

type BuildSalesShopPageWarningsOptions = {
  shopsError: unknown;
  syncJobsError: unknown;
};

const fallbackConfigFromShop = (shop: Shop): SalesShopConfig => ({
  shop_id: shop.shop_id,
  platform: shop.platform,
  platform_shop_id: shop.platform_shop_id,
  name: shop.name,
  status: shop.status,
  auto_download_orders: false,
  auto_to_review: false,
  download_interval_minutes: 30,
  auto_merge_non_shipping_time: false,
  non_shipping_time_start: null,
  non_shipping_time_end: null,
  enable_stock_sync: false,
  enable_after_sales_download: false,
  enable_logistics_upload: false,
});

export const buildSalesShopRows = (
  configItems: SalesShopConfig[],
  shops: Shop[],
): SalesShopConfig[] => {
  if (!configItems.length) {
    return shops.map((shop) => fallbackConfigFromShop(shop));
  }

  const configMap = new Map(configItems.map((item) => [item.shop_id, item]));
  shops.forEach((shop) => {
    if (!configMap.has(shop.shop_id)) {
      configMap.set(shop.shop_id, fallbackConfigFromShop(shop));
    }
  });

  return Array.from(configMap.values());
};

export const resolveSalesShopPageError = ({
  hasRows,
  configError,
  shopsError,
  syncJobsError,
}: ResolveSalesShopPageErrorOptions) => {
  if (configError) {
    return configError;
  }

  if (!hasRows && shopsError) {
    return shopsError;
  }

  if (!hasRows && syncJobsError) {
    return syncJobsError;
  }

  return null;
};

export const buildSalesShopPageWarnings = ({
  shopsError,
  syncJobsError,
}: BuildSalesShopPageWarningsOptions) => {
  const warnings: string[] = [];

  if (shopsError) {
    warnings.push('店铺基础信息加载失败，列表已使用配置数据降级展示。');
  }

  if (syncJobsError) {
    warnings.push('同步任务状态加载失败，列表中的进度与状态可能延迟。');
  }

  return warnings;
};
