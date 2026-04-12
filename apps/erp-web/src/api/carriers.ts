import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type Carrier = {
  tenant_id: string;
  carrier_code: string;
  name: string;
  created_at: string;
  updated_at: string;
};

export type CarrierListQuery = {
  keyword?: string;
  offset?: number;
  limit?: number;
};

export type CarrierListResponse = {
  items: Carrier[];
  total: number;
};

export type CarrierCreatePayload = {
  carrier_code: string;
  name: string;
};

export type CarrierUpdatePayload = {
  name?: string;
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

const buildQueryString = (query: CarrierListQuery): string => {
  const params = new URLSearchParams();
  if (query.keyword) {
    params.set('keyword', query.keyword);
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

export const listCarriers = async (
  tenantId: string,
  query: CarrierListQuery
): Promise<CarrierListResponse> => {
  if (useMocks) {
    return { items: [], total: 0 };
  }
  return requestJson<CarrierListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/carriers${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取承运商列表失败'
  );
};

export const createCarrier = async (
  tenantId: string,
  payload: CarrierCreatePayload
): Promise<Carrier> => {
  if (useMocks) {
    return {
      tenant_id: tenantId,
      carrier_code: payload.carrier_code,
      name: payload.name,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<Carrier>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/carriers`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建承运商失败'
  );
};

export const updateCarrier = async (
  tenantId: string,
  carrierCode: string,
  payload: CarrierUpdatePayload
): Promise<Carrier> => {
  if (useMocks) {
    return {
      tenant_id: tenantId,
      carrier_code: carrierCode,
      name: payload.name ?? carrierCode,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<Carrier>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/carriers/${carrierCode}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新承运商失败'
  );
};

export const deleteCarrier = async (
  tenantId: string,
  carrierCode: string
): Promise<Carrier> => {
  if (useMocks) {
    return {
      tenant_id: tenantId,
      carrier_code: carrierCode,
      name: carrierCode,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<Carrier>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/carriers/${carrierCode}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除承运商失败'
  );
};
