import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockProductChannelAvailability,
  deleteMockProductChannelAvailability,
  getMockProductChannelAvailability,
  getMockProductChannelAvailabilityList,
  updateMockProductChannelAvailability
} from '@/api/mocks';

export type ProductChannelAvailability = {
  availability_id: string;
  tenant_id: string;
  sku_id: string;
  sku_code?: string | null;
  sku_name?: string | null;
  channel_id?: string | null;
  channel_platform?: string | null;
  warehouse_id?: string | null;
  warehouse_name?: string | null;
  status: string;
  available_qty: number;
  rule?: string | null;
  on_shelf_at?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export type ProductChannelAvailabilityListQuery = {
  sku_id?: string;
  channel_id?: string;
  warehouse_id?: string;
  status?: string;
  offset?: number;
  limit?: number;
};

export type ProductChannelAvailabilityListResponse = {
  items: ProductChannelAvailability[];
  total: number;
};

export type ProductChannelAvailabilityCreatePayload = {
  sku_id: string;
  channel_id?: string | null;
  warehouse_id?: string | null;
  status?: string;
  available_qty?: number;
  rule?: string | null;
  on_shelf_at?: string | null;
};

export type ProductChannelAvailabilityUpdatePayload = {
  channel_id?: string | null;
  warehouse_id?: string | null;
  status?: string;
  available_qty?: number;
  rule?: string | null;
  on_shelf_at?: string | null;
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

const buildQueryString = (query: ProductChannelAvailabilityListQuery): string => {
  const params = new URLSearchParams();
  if (query.sku_id) {
    params.set('sku_id', query.sku_id);
  }
  if (query.channel_id) {
    params.set('channel_id', query.channel_id);
  }
  if (query.warehouse_id) {
    params.set('warehouse_id', query.warehouse_id);
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

export const listProductChannelAvailability = async (
  tenantId: string,
  query: ProductChannelAvailabilityListQuery
): Promise<ProductChannelAvailabilityListResponse> => {
  if (useMocks) {
    return getMockProductChannelAvailabilityList(query);
  }
  return requestJson<ProductChannelAvailabilityListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-channel-availability${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取可售渠道失败'
  );
};

export const getProductChannelAvailability = async (
  tenantId: string,
  availabilityId: string
): Promise<ProductChannelAvailability> => {
  if (useMocks) {
    return getMockProductChannelAvailability(availabilityId);
  }
  return requestJson<ProductChannelAvailability>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-channel-availability/${availabilityId}`,
    { headers: buildAuthHeaders() },
    '获取可售渠道详情失败'
  );
};

export const createProductChannelAvailability = async (
  tenantId: string,
  payload: ProductChannelAvailabilityCreatePayload
): Promise<ProductChannelAvailability> => {
  if (useMocks) {
    return createMockProductChannelAvailability(payload);
  }
  return requestJson<ProductChannelAvailability>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-channel-availability`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建可售渠道失败'
  );
};

export const updateProductChannelAvailability = async (
  tenantId: string,
  availabilityId: string,
  payload: ProductChannelAvailabilityUpdatePayload
): Promise<ProductChannelAvailability> => {
  if (useMocks) {
    return updateMockProductChannelAvailability(availabilityId, payload);
  }
  return requestJson<ProductChannelAvailability>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-channel-availability/${availabilityId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新可售渠道失败'
  );
};

export const deleteProductChannelAvailability = async (
  tenantId: string,
  availabilityId: string
): Promise<ProductChannelAvailability> => {
  if (useMocks) {
    return deleteMockProductChannelAvailability(availabilityId);
  }
  return requestJson<ProductChannelAvailability>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-channel-availability/${availabilityId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除可售渠道失败'
  );
};
