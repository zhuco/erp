import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type LogisticsProduct = {
  product_id: string;
  tenant_id: string;
  carrier_code: string;
  code: string;
  name: string;
  status: string;
  created_at: string;
  updated_at: string;
};

export type LogisticsProductListQuery = {
  keyword?: string;
  carrier_code?: string;
  status?: string;
  offset?: number;
  limit?: number;
};

export type LogisticsProductListResponse = {
  items: LogisticsProduct[];
  total: number;
};

export type LogisticsProductCreatePayload = {
  carrier_code: string;
  code: string;
  name: string;
  status?: string;
};

export type LogisticsProductUpdatePayload = {
  carrier_code?: string;
  code?: string;
  name?: string;
  status?: string;
};

const requestJson = async <T>(input: RequestInfo, init: RequestInit, message: string) => {
  const response = await fetch(input, init);
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

const buildQueryString = (query: LogisticsProductListQuery): string => {
  const params = new URLSearchParams();
  if (query.keyword) {
    params.set('keyword', query.keyword);
  }
  if (query.carrier_code) {
    params.set('carrier_code', query.carrier_code);
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

export const listLogisticsProducts = async (
  tenantId: string,
  query: LogisticsProductListQuery
): Promise<LogisticsProductListResponse> => {
  if (useMocks) {
    return { items: [], total: 0 };
  }
  return requestJson<LogisticsProductListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics-products${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取物流产品失败'
  );
};

export const createLogisticsProduct = async (
  tenantId: string,
  payload: LogisticsProductCreatePayload
): Promise<LogisticsProduct> => {
  if (useMocks) {
    return {
      product_id: `${Date.now()}`,
      tenant_id: tenantId,
      carrier_code: payload.carrier_code,
      code: payload.code,
      name: payload.name,
      status: payload.status ?? 'ACTIVE',
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<LogisticsProduct>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics-products`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建物流产品失败'
  );
};

export const updateLogisticsProduct = async (
  tenantId: string,
  productId: string,
  payload: LogisticsProductUpdatePayload
): Promise<LogisticsProduct> => {
  if (useMocks) {
    return {
      product_id: productId,
      tenant_id: tenantId,
      carrier_code: payload.carrier_code ?? 'mock',
      code: payload.code ?? 'mock',
      name: payload.name ?? 'mock',
      status: payload.status ?? 'ACTIVE',
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<LogisticsProduct>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics-products/${productId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新物流产品失败'
  );
};

export const deleteLogisticsProduct = async (
  tenantId: string,
  productId: string
): Promise<LogisticsProduct> => {
  if (useMocks) {
    return {
      product_id: productId,
      tenant_id: tenantId,
      carrier_code: 'mock',
      code: 'mock',
      name: 'mock',
      status: 'ACTIVE',
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<LogisticsProduct>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics-products/${productId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除物流产品失败'
  );
};
