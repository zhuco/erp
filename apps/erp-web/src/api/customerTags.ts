import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockCustomerTag,
  deleteMockCustomerTag,
  getMockCustomerTagList,
  updateMockCustomerTag
} from '@/api/mocks';

export type CustomerTag = {
  tag_id: string;
  tenant_id: string;
  name: string;
  color?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export type CustomerTagListQuery = {
  keyword?: string;
  offset?: number;
  limit?: number;
};

export type CustomerTagListResponse = {
  items: CustomerTag[];
  total: number;
};

export type CustomerTagCreatePayload = {
  name: string;
  color?: string;
};

export type CustomerTagUpdatePayload = {
  name?: string;
  color?: string;
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

const buildQueryString = (query: CustomerTagListQuery): string => {
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

export const listCustomerTags = async (
  tenantId: string,
  query: CustomerTagListQuery
): Promise<CustomerTagListResponse> => {
  if (useMocks) {
    return getMockCustomerTagList(query);
  }
  return requestJson<CustomerTagListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/customer-tags${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取客户标签失败'
  );
};

export const createCustomerTag = async (
  tenantId: string,
  payload: CustomerTagCreatePayload
): Promise<CustomerTag> => {
  if (useMocks) {
    return createMockCustomerTag(payload);
  }
  return requestJson<CustomerTag>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/customer-tags`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建客户标签失败'
  );
};

export const updateCustomerTag = async (
  tenantId: string,
  tagId: string,
  payload: CustomerTagUpdatePayload
): Promise<CustomerTag> => {
  if (useMocks) {
    return updateMockCustomerTag(tagId, payload);
  }
  return requestJson<CustomerTag>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/customer-tags/${tagId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新客户标签失败'
  );
};

export const deleteCustomerTag = async (
  tenantId: string,
  tagId: string
): Promise<CustomerTag> => {
  if (useMocks) {
    return deleteMockCustomerTag(tagId);
  }
  return requestJson<CustomerTag>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/customer-tags/${tagId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除客户标签失败'
  );
};
