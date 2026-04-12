import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockProductUnitConversion,
  deleteMockProductUnitConversion,
  getMockProductUnitConversion,
  getMockProductUnitConversionList,
  updateMockProductUnitConversion
} from '@/api/mocks';

export type ProductUnitConversion = {
  conversion_id: string;
  tenant_id: string;
  sku_id: string;
  sku_code?: string | null;
  sku_name?: string | null;
  base_unit: string;
  aux_unit: string;
  ratio: number;
  rounding: string;
  scenario: string;
  status: string;
  created_at?: string | null;
  updated_at?: string | null;
};

export type ProductUnitConversionListQuery = {
  sku_id?: string;
  scenario?: string;
  status?: string;
  offset?: number;
  limit?: number;
};

export type ProductUnitConversionListResponse = {
  items: ProductUnitConversion[];
  total: number;
};

export type ProductUnitConversionCreatePayload = {
  sku_id: string;
  base_unit: string;
  aux_unit: string;
  ratio?: number;
  rounding?: string;
  scenario: string;
  status?: string;
};

export type ProductUnitConversionUpdatePayload = {
  base_unit?: string;
  aux_unit?: string;
  ratio?: number;
  rounding?: string;
  scenario?: string;
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

const buildQueryString = (query: ProductUnitConversionListQuery): string => {
  const params = new URLSearchParams();
  if (query.sku_id) {
    params.set('sku_id', query.sku_id);
  }
  if (query.scenario) {
    params.set('scenario', query.scenario);
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

export const listProductUnitConversions = async (
  tenantId: string,
  query: ProductUnitConversionListQuery
): Promise<ProductUnitConversionListResponse> => {
  if (useMocks) {
    return getMockProductUnitConversionList(query);
  }
  return requestJson<ProductUnitConversionListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-unit-conversions${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取单位换算失败'
  );
};

export const getProductUnitConversion = async (
  tenantId: string,
  conversionId: string
): Promise<ProductUnitConversion> => {
  if (useMocks) {
    return getMockProductUnitConversion(conversionId);
  }
  return requestJson<ProductUnitConversion>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-unit-conversions/${conversionId}`,
    { headers: buildAuthHeaders() },
    '获取单位换算详情失败'
  );
};

export const createProductUnitConversion = async (
  tenantId: string,
  payload: ProductUnitConversionCreatePayload
): Promise<ProductUnitConversion> => {
  if (useMocks) {
    return createMockProductUnitConversion(payload);
  }
  return requestJson<ProductUnitConversion>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-unit-conversions`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建单位换算失败'
  );
};

export const updateProductUnitConversion = async (
  tenantId: string,
  conversionId: string,
  payload: ProductUnitConversionUpdatePayload
): Promise<ProductUnitConversion> => {
  if (useMocks) {
    return updateMockProductUnitConversion(conversionId, payload);
  }
  return requestJson<ProductUnitConversion>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-unit-conversions/${conversionId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新单位换算失败'
  );
};

export const deleteProductUnitConversion = async (
  tenantId: string,
  conversionId: string
): Promise<ProductUnitConversion> => {
  if (useMocks) {
    return deleteMockProductUnitConversion(conversionId);
  }
  return requestJson<ProductUnitConversion>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-unit-conversions/${conversionId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除单位换算失败'
  );
};
