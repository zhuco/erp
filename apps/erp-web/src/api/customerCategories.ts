import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockCustomerCategory,
  deleteMockCustomerCategory,
  getMockCustomerCategoryList,
  updateMockCustomerCategory
} from '@/api/mocks';

export type CustomerCategory = {
  category_id: string;
  tenant_id: string;
  name: string;
  description?: string | null;
  status?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export type CustomerCategoryListQuery = {
  keyword?: string;
  status?: string;
  offset?: number;
  limit?: number;
};

export type CustomerCategoryListResponse = {
  items: CustomerCategory[];
  total: number;
};

export type CustomerCategoryCreatePayload = {
  name: string;
  description?: string;
  status?: string;
};

export type CustomerCategoryUpdatePayload = {
  name?: string;
  description?: string;
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

const buildQueryString = (query: CustomerCategoryListQuery): string => {
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

export const listCustomerCategories = async (
  tenantId: string,
  query: CustomerCategoryListQuery
): Promise<CustomerCategoryListResponse> => {
  if (useMocks) {
    return getMockCustomerCategoryList(query);
  }
  return requestJson<CustomerCategoryListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/customer-categories${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取客户分类失败'
  );
};

export const createCustomerCategory = async (
  tenantId: string,
  payload: CustomerCategoryCreatePayload
): Promise<CustomerCategory> => {
  if (useMocks) {
    return createMockCustomerCategory(payload);
  }
  return requestJson<CustomerCategory>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/customer-categories`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建客户分类失败'
  );
};

export const updateCustomerCategory = async (
  tenantId: string,
  categoryId: string,
  payload: CustomerCategoryUpdatePayload
): Promise<CustomerCategory> => {
  if (useMocks) {
    return updateMockCustomerCategory(categoryId, payload);
  }
  return requestJson<CustomerCategory>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/customer-categories/${categoryId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新客户分类失败'
  );
};

export const deleteCustomerCategory = async (
  tenantId: string,
  categoryId: string
): Promise<CustomerCategory> => {
  if (useMocks) {
    return deleteMockCustomerCategory(categoryId);
  }
  return requestJson<CustomerCategory>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/customer-categories/${categoryId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除客户分类失败'
  );
};
