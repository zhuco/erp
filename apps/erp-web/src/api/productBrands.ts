import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockProductBrand,
  deleteMockProductBrand,
  getMockProductBrandList,
  updateMockProductBrand
} from '@/api/mocks';

export type ProductBrand = {
  brand_id: string;
  tenant_id: string;
  name: string;
  created_at?: string | null;
  updated_at?: string | null;
};

export type ProductBrandListQuery = {
  keyword?: string;
  offset?: number;
  limit?: number;
};

export type ProductBrandListResponse = {
  items: ProductBrand[];
  total: number;
};

export type ProductBrandCreatePayload = {
  name: string;
};

export type ProductBrandUpdatePayload = {
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

const buildQueryString = (query: ProductBrandListQuery): string => {
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

export const listProductBrands = async (
  tenantId: string,
  query: ProductBrandListQuery
): Promise<ProductBrandListResponse> => {
  if (useMocks) {
    return getMockProductBrandList(query);
  }
  return requestJson<ProductBrandListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-brands${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取商品品牌失败'
  );
};

export const createProductBrand = async (
  tenantId: string,
  payload: ProductBrandCreatePayload
): Promise<ProductBrand> => {
  if (useMocks) {
    return createMockProductBrand(payload);
  }
  return requestJson<ProductBrand>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-brands`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建商品品牌失败'
  );
};

export const updateProductBrand = async (
  tenantId: string,
  brandId: string,
  payload: ProductBrandUpdatePayload
): Promise<ProductBrand> => {
  if (useMocks) {
    return updateMockProductBrand(brandId, payload);
  }
  return requestJson<ProductBrand>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-brands/${brandId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新商品品牌失败'
  );
};

export const deleteProductBrand = async (
  tenantId: string,
  brandId: string
): Promise<ProductBrand> => {
  if (useMocks) {
    return deleteMockProductBrand(brandId);
  }
  return requestJson<ProductBrand>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-brands/${brandId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除商品品牌失败'
  );
};
