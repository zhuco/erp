import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  getMockSalesShopConfig,
  getMockSalesShopConfigList,
  updateMockSalesShopConfig
} from '@/api/mocks';

export type SalesShopConfig = {
  shop_id: string;
  platform: string;
  platform_shop_id: string;
  name: string;
  status: string;
  auto_download_orders: boolean;
  auto_to_review: boolean;
  download_interval_minutes: number;
  auto_merge_non_shipping_time: boolean;
  non_shipping_time_start?: string | null;
  non_shipping_time_end?: string | null;
  enable_stock_sync: boolean;
  enable_after_sales_download: boolean;
  enable_logistics_upload: boolean;
};

export type SalesShopConfigListQuery = {
  platform?: string;
  status?: string;
  shop_id?: string;
  offset?: number;
  limit?: number;
};

export type SalesShopConfigRequest = {
  auto_download_orders: boolean;
  auto_to_review: boolean;
  download_interval_minutes: number;
  auto_merge_non_shipping_time: boolean;
  non_shipping_time_start?: string | null;
  non_shipping_time_end?: string | null;
  enable_stock_sync: boolean;
  enable_after_sales_download: boolean;
  enable_logistics_upload: boolean;
};

export type SalesShopConfigListResponse = {
  items: SalesShopConfig[];
  total: number;
};

const requestJson = async <T>(input: RequestInfo, init: RequestInit, message: string) => {
  const response = await fetchWithAuth(input, init);
  let payload: unknown = undefined;
  try {
    payload = await response.json();
  } catch {
    payload = undefined;
  }
  if (!response.ok) {
    handleApiError(response.status);
    throw createApiError(message, response.status, payload);
  }
  return payload as T;
};

const buildQueryString = (query: SalesShopConfigListQuery): string => {
  const params = new URLSearchParams();
  if (query.platform) {
    params.set('platform', query.platform);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.shop_id) {
    params.set('shop_id', query.shop_id);
  }
  if (query.offset != null) {
    params.set('offset', String(query.offset));
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  const serialized = params.toString();
  return serialized ? `?${serialized}` : '';
};

export const listSalesShopConfigs = async (
  tenantId: string,
  query: SalesShopConfigListQuery
): Promise<SalesShopConfigListResponse> => {
  if (useMocks) {
    return getMockSalesShopConfigList(query);
  }
  return requestJson<SalesShopConfigListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/sales/shops${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取网店配置失败'
  );
};

export const getSalesShopConfig = async (
  tenantId: string,
  shopId: string
): Promise<SalesShopConfig> => {
  if (useMocks) {
    return getMockSalesShopConfig(shopId);
  }
  return requestJson<SalesShopConfig>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/sales/shops/${shopId}`,
    { headers: buildAuthHeaders() },
    '获取网店配置失败'
  );
};

export const updateSalesShopConfig = async (
  tenantId: string,
  shopId: string,
  payload: SalesShopConfigRequest
): Promise<SalesShopConfig> => {
  if (useMocks) {
    return updateMockSalesShopConfig(shopId, payload);
  }
  return requestJson<SalesShopConfig>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/sales/shops/${shopId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新网店配置失败'
  );
};
