import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockProductCategory,
  deleteMockProductCategory,
  getMockProductCategoryList,
  updateMockProductCategory
} from '@/api/mocks';

export type ProductCategory = {
  category_id: string;
  tenant_id: string;
  name: string;
  created_at?: string | null;
  updated_at?: string | null;
};

export type ProductCategoryListQuery = {
  keyword?: string;
  offset?: number;
  limit?: number;
};

export type ProductCategoryListResponse = {
  items: ProductCategory[];
  total: number;
};

export type ProductCategoryCreatePayload = {
  name: string;
};

export type ProductCategoryUpdatePayload = {
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

const buildQueryString = (query: ProductCategoryListQuery): string => {
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

export const listProductCategories = async (
  tenantId: string,
  query: ProductCategoryListQuery
): Promise<ProductCategoryListResponse> => {
  if (useMocks) {
    return getMockProductCategoryList(query);
  }
  return requestJson<ProductCategoryListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-categories${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取商品分类失败'
  );
};

export const createProductCategory = async (
  tenantId: string,
  payload: ProductCategoryCreatePayload
): Promise<ProductCategory> => {
  if (useMocks) {
    return createMockProductCategory(payload);
  }
  return requestJson<ProductCategory>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-categories`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建商品分类失败'
  );
};

export const updateProductCategory = async (
  tenantId: string,
  categoryId: string,
  payload: ProductCategoryUpdatePayload
): Promise<ProductCategory> => {
  if (useMocks) {
    return updateMockProductCategory(categoryId, payload);
  }
  return requestJson<ProductCategory>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-categories/${categoryId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新商品分类失败'
  );
};

export const deleteProductCategory = async (
  tenantId: string,
  categoryId: string
): Promise<ProductCategory> => {
  if (useMocks) {
    return deleteMockProductCategory(categoryId);
  }
  return requestJson<ProductCategory>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-categories/${categoryId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除商品分类失败'
  );
};
