import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockShopProductDownload,
  createMockShopProductMatchTask,
  createMockShopProductSkuTask,
  deleteMockShopProductMatches,
  getMockShopProductSkuList,
  getMockShopProductTaskList,
  manualMockShopProductMatch
} from '@/api/mocks';
import type { PlatformCode } from '@/features/platform';

export type ShopProductSku = {
  sku_record_id: string;
  tenant_id: string;
  shop_id: string;
  platform: PlatformCode;
  platform_product_id: string;
  platform_sku_id: string;
  merchant_code?: string | null;
  merchant_sku_code?: string | null;
  product_title: string;
  sku_title?: string | null;
  spec_text?: string | null;
  price_cent?: number | null;
  stock_qty?: number | null;
  status: string;
  barcode?: string | null;
  image_url?: string | null;
  platform_updated_at?: string | null;
  mapping_id?: string | null;
  mapped_sku_id?: string | null;
  mapped_sku_code?: string | null;
  mapped_sku_name?: string | null;
  match_source?: string | null;
  mapping_status?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export type ShopProductSkuListQuery = {
  platform?: PlatformCode;
  shop_ids?: string[];
  product_type?: string;
  status?: string;
  keyword?: string;
  match_status?: string;
  updated_from?: string;
  updated_to?: string;
  offset?: number;
  limit?: number;
};

export type ShopProductSkuListResponse = {
  items: ShopProductSku[];
  total: number;
};

export type ShopProductTask = {
  task_id: string;
  tenant_id: string;
  task_type: string;
  status: string;
  shop_id?: string | null;
  platform?: PlatformCode | null;
  total_count: number;
  success_count: number;
  failure_count: number;
  error: string;
  payload_json: Record<string, unknown>;
  result_json: Record<string, unknown>;
  started_at: string;
  finished_at?: string | null;
  created_at: string;
  updated_at: string;
};

export type ShopProductTaskListQuery = {
  task_type?: string;
  status?: string;
  offset?: number;
  limit?: number;
};

export type ShopProductTaskListResponse = {
  items: ShopProductTask[];
  total: number;
};

export type ShopProductMatchFailure = {
  sku_record_id: string;
  platform_sku_id: string;
  merchant_sku_code?: string | null;
  reason: string;
};

export type ShopProductDownloadPayload = {
  platform?: PlatformCode;
  shop_ids?: string[];
  product_type?: string;
  status?: string;
  updated_from?: string;
  updated_to?: string;
  keyword?: string;
  auto_match?: boolean;
};

export type ShopProductDownloadResponse = {
  task: ShopProductTask;
  auto_match_task?: ShopProductTask | null;
  auto_match_failures: ShopProductMatchFailure[];
};

export type ShopProductCreateSkuPayload = {
  sku_record_ids?: string[];
  filters?: ShopProductSkuListQuery;
  import_price?: boolean;
  import_image?: boolean;
  import_stock?: boolean;
  warehouse_id?: string;
};

export type ShopProductAutoMatchPayload = {
  sku_record_ids?: string[];
  filters?: ShopProductSkuListQuery;
};

export type ShopProductManualMatchPayload = {
  sku_record_ids: string[];
  sku_id: string;
};

export type ShopProductDeleteMatchPayload = {
  sku_record_ids: string[];
};

export type ShopProductMatchResponse = {
  task: ShopProductTask;
  failures: ShopProductMatchFailure[];
};

export type ShopProductTaskResponse = {
  task: ShopProductTask;
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

const buildSkuQueryString = (query: ShopProductSkuListQuery): string => {
  const params = new URLSearchParams();
  if (query.platform) {
    params.set('platform', query.platform);
  }
  if (query.shop_ids?.length) {
    params.set('shop_ids', query.shop_ids.join(','));
  }
  if (query.product_type) {
    params.set('product_type', query.product_type);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.keyword) {
    params.set('keyword', query.keyword);
  }
  if (query.match_status) {
    params.set('match_status', query.match_status);
  }
  if (query.updated_from) {
    params.set('updated_from', query.updated_from);
  }
  if (query.updated_to) {
    params.set('updated_to', query.updated_to);
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  if (query.offset != null) {
    params.set('offset', String(query.offset));
  }
  const serialized = params.toString();
  return serialized ? `?${serialized}` : '';
};

const buildTaskQueryString = (query: ShopProductTaskListQuery): string => {
  const params = new URLSearchParams();
  if (query.task_type) {
    params.set('task_type', query.task_type);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  if (query.offset != null) {
    params.set('offset', String(query.offset));
  }
  const serialized = params.toString();
  return serialized ? `?${serialized}` : '';
};

export const listShopProductSkus = async (
  tenantId: string,
  query: ShopProductSkuListQuery
): Promise<ShopProductSkuListResponse> => {
  if (useMocks) {
    return getMockShopProductSkuList(query);
  }
  return requestJson<ShopProductSkuListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/shop-products/skus${buildSkuQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取网店商品失败'
  );
};

export const listShopProductTasks = async (
  tenantId: string,
  query: ShopProductTaskListQuery
): Promise<ShopProductTaskListResponse> => {
  if (useMocks) {
    return getMockShopProductTaskList(query);
  }
  return requestJson<ShopProductTaskListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/shop-products/tasks${buildTaskQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取下载任务失败'
  );
};

export const downloadShopProducts = async (
  tenantId: string,
  payload: ShopProductDownloadPayload
): Promise<ShopProductDownloadResponse> => {
  if (useMocks) {
    return createMockShopProductDownload(payload);
  }
  return requestJson<ShopProductDownloadResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/shop-products/download`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '下载网店商品失败'
  );
};

export const createShopProductSkus = async (
  tenantId: string,
  payload: ShopProductCreateSkuPayload
): Promise<ShopProductTaskResponse> => {
  if (useMocks) {
    return createMockShopProductSkuTask(payload);
  }
  return requestJson<ShopProductTaskResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/shop-products/create-skus`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建货品档案失败'
  );
};

export const autoMatchShopProducts = async (
  tenantId: string,
  payload: ShopProductAutoMatchPayload
): Promise<ShopProductMatchResponse> => {
  if (useMocks) {
    return createMockShopProductMatchTask(payload);
  }
  return requestJson<ShopProductMatchResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/shop-products/auto-match`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '自动匹配失败'
  );
};

export const manualMatchShopProducts = async (
  tenantId: string,
  payload: ShopProductManualMatchPayload
): Promise<ShopProductTaskResponse> => {
  if (useMocks) {
    return manualMockShopProductMatch(payload);
  }
  return requestJson<ShopProductTaskResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/shop-products/manual-match`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '手动匹配失败'
  );
};

export const deleteShopProductMatches = async (
  tenantId: string,
  payload: ShopProductDeleteMatchPayload
): Promise<ShopProductTaskResponse> => {
  if (useMocks) {
    return deleteMockShopProductMatches(payload);
  }
  return requestJson<ShopProductTaskResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/shop-products/delete-match`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '删除匹配失败'
  );
};
