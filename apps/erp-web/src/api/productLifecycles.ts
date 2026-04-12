import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockProductLifecycle,
  deleteMockProductLifecycle,
  getMockProductLifecycle,
  getMockProductLifecycleList,
  updateMockProductLifecycle
} from '@/api/mocks';

export type ProductLifecycle = {
  lifecycle_id: string;
  tenant_id: string;
  sku_id: string;
  sku_code?: string | null;
  sku_name?: string | null;
  status: string;
  inventory_qty: number;
  remark?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export type ProductLifecycleListQuery = {
  sku_id?: string;
  status?: string;
  offset?: number;
  limit?: number;
};

export type ProductLifecycleListResponse = {
  items: ProductLifecycle[];
  total: number;
};

export type ProductLifecycleCreatePayload = {
  sku_id: string;
  status?: string;
  inventory_qty?: number;
  remark?: string | null;
};

export type ProductLifecycleUpdatePayload = {
  status?: string;
  inventory_qty?: number;
  remark?: string | null;
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

const buildQueryString = (query: ProductLifecycleListQuery): string => {
  const params = new URLSearchParams();
  if (query.sku_id) {
    params.set('sku_id', query.sku_id);
  }
  if (query.status) {
    params.set('status', query.status);
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

export const listProductLifecycles = async (
  tenantId: string,
  query: ProductLifecycleListQuery
): Promise<ProductLifecycleListResponse> => {
  if (useMocks) {
    return getMockProductLifecycleList(query);
  }
  return requestJson<ProductLifecycleListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-lifecycles${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取生命周期失败'
  );
};

export const getProductLifecycle = async (
  tenantId: string,
  lifecycleId: string
): Promise<ProductLifecycle> => {
  if (useMocks) {
    return getMockProductLifecycle(lifecycleId);
  }
  return requestJson<ProductLifecycle>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-lifecycles/${lifecycleId}`,
    { headers: buildAuthHeaders() },
    '获取生命周期详情失败'
  );
};

export const createProductLifecycle = async (
  tenantId: string,
  payload: ProductLifecycleCreatePayload
): Promise<ProductLifecycle> => {
  if (useMocks) {
    return createMockProductLifecycle(payload);
  }
  return requestJson<ProductLifecycle>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-lifecycles`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建生命周期失败'
  );
};

export const updateProductLifecycle = async (
  tenantId: string,
  lifecycleId: string,
  payload: ProductLifecycleUpdatePayload
): Promise<ProductLifecycle> => {
  if (useMocks) {
    return updateMockProductLifecycle(lifecycleId, payload);
  }
  return requestJson<ProductLifecycle>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-lifecycles/${lifecycleId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新生命周期失败'
  );
};

export const deleteProductLifecycle = async (
  tenantId: string,
  lifecycleId: string
): Promise<ProductLifecycle> => {
  if (useMocks) {
    return deleteMockProductLifecycle(lifecycleId);
  }
  return requestJson<ProductLifecycle>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-lifecycles/${lifecycleId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除生命周期失败'
  );
};
