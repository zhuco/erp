import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockCourier,
  deleteMockCourier,
  getMockCourierList,
  updateMockCourier
} from '@/api/mocks';

export type Courier = {
  courier_id: string;
  tenant_id: string;
  name: string;
  code: string;
  status: string;
  created_at?: string | null;
  updated_at?: string | null;
};

export type CourierListQuery = {
  keyword?: string;
  status?: string;
  offset?: number;
  limit?: number;
};

export type CourierListResponse = {
  items: Courier[];
  total: number;
};

export type CourierCreatePayload = {
  name: string;
  code: string;
  status?: string;
};

export type CourierUpdatePayload = {
  name?: string;
  code?: string;
  status?: string;
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

const buildQueryString = (query: CourierListQuery): string => {
  const params = new URLSearchParams();
  if (query.keyword) {
    params.set('keyword', query.keyword);
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

export const listCouriers = async (
  tenantId: string,
  query: CourierListQuery
): Promise<CourierListResponse> => {
  if (useMocks) {
    return getMockCourierList(query);
  }
  return requestJson<CourierListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/couriers${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取快递列表失败'
  );
};

export const createCourier = async (
  tenantId: string,
  payload: CourierCreatePayload
): Promise<Courier> => {
  if (useMocks) {
    return createMockCourier(payload);
  }
  return requestJson<Courier>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/couriers`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建快递失败'
  );
};

export const updateCourier = async (
  tenantId: string,
  courierId: string,
  payload: CourierUpdatePayload
): Promise<Courier> => {
  if (useMocks) {
    return updateMockCourier(courierId, payload);
  }
  return requestJson<Courier>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/couriers/${courierId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新快递失败'
  );
};

export const deleteCourier = async (tenantId: string, courierId: string): Promise<Courier> => {
  if (useMocks) {
    return deleteMockCourier(courierId);
  }
  return requestJson<Courier>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/couriers/${courierId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除快递失败'
  );
};
