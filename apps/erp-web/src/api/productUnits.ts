import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockProductUnit,
  getMockProductUnitList,
  updateMockProductUnit
} from '@/api/mocks';

export type ProductUnit = {
  unit_id: string;
  tenant_id: string;
  name: string;
  created_at?: string | null;
  updated_at?: string | null;
};

export type ProductUnitListQuery = {
  keyword?: string;
  offset?: number;
  limit?: number;
};

export type ProductUnitListResponse = {
  items: ProductUnit[];
  total: number;
};

export type ProductUnitCreatePayload = {
  name: string;
};

export type ProductUnitUpdatePayload = {
  name?: string;
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

const buildQueryString = (query: ProductUnitListQuery): string => {
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

export const listProductUnits = async (
  tenantId: string,
  query: ProductUnitListQuery
): Promise<ProductUnitListResponse> => {
  if (useMocks) {
    return getMockProductUnitList(query);
  }
  return requestJson<ProductUnitListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-units${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取基础单位失败'
  );
};

export const createProductUnit = async (
  tenantId: string,
  payload: ProductUnitCreatePayload
): Promise<ProductUnit> => {
  if (useMocks) {
    return createMockProductUnit(payload);
  }
  return requestJson<ProductUnit>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-units`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建基础单位失败'
  );
};

export const updateProductUnit = async (
  tenantId: string,
  unitId: string,
  payload: ProductUnitUpdatePayload
): Promise<ProductUnit> => {
  if (useMocks) {
    return updateMockProductUnit(unitId, payload);
  }
  return requestJson<ProductUnit>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-units/${unitId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新基础单位失败'
  );
};
