import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import type { CustomerTag } from '@/api/customerTags';
import {
  createMockCustomer,
  deleteMockCustomer,
  getMockCustomerDetail,
  getMockCustomerList,
  replaceMockCustomerTags,
  updateMockCustomer
} from '@/api/mocks';

export type Customer = {
  customer_id: string;
  tenant_id: string;
  name: string;
  phone?: string | null;
  address?: string | null;
  note?: string | null;
  status?: string | null;
  category_id?: string | null;
  category_name?: string | null;
  member_level_id?: string | null;
  member_level_name?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export type CustomerDetail = {
  customer: Customer;
  tags: CustomerTag[];
};

export type CustomerListQuery = {
  keyword?: string;
  status?: string;
  category_id?: string;
  member_level_id?: string;
  tag_id?: string;
  offset?: number;
  limit?: number;
};

export type CustomerListResponse = {
  items: Customer[];
  total: number;
};

export type CustomerCreatePayload = {
  name: string;
  phone?: string;
  address?: string;
  note?: string;
  status?: string;
  category_id?: string;
  member_level_id?: string;
};

export type CustomerUpdatePayload = {
  name?: string;
  phone?: string;
  address?: string;
  note?: string;
  status?: string;
  category_id?: string;
  member_level_id?: string;
};

export type CustomerTagReplacePayload = {
  tag_ids: string[];
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

const buildQueryString = (query: CustomerListQuery): string => {
  const params = new URLSearchParams();
  if (query.keyword) {
    params.set('keyword', query.keyword);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.category_id) {
    params.set('category_id', query.category_id);
  }
  if (query.member_level_id) {
    params.set('member_level_id', query.member_level_id);
  }
  if (query.tag_id) {
    params.set('tag_id', query.tag_id);
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

export const listCustomers = async (
  tenantId: string,
  query: CustomerListQuery
): Promise<CustomerListResponse> => {
  if (useMocks) {
    return getMockCustomerList(query);
  }
  return requestJson<CustomerListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/customers${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取客户失败'
  );
};

export const getCustomerDetail = async (
  tenantId: string,
  customerId: string
): Promise<CustomerDetail> => {
  if (useMocks) {
    return getMockCustomerDetail(customerId);
  }
  return requestJson<CustomerDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/customers/${customerId}`,
    { headers: buildAuthHeaders() },
    '获取客户详情失败'
  );
};

export const createCustomer = async (
  tenantId: string,
  payload: CustomerCreatePayload
): Promise<Customer> => {
  if (useMocks) {
    return createMockCustomer(payload);
  }
  return requestJson<Customer>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/customers`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建客户失败'
  );
};

export const updateCustomer = async (
  tenantId: string,
  customerId: string,
  payload: CustomerUpdatePayload
): Promise<Customer> => {
  if (useMocks) {
    return updateMockCustomer(customerId, payload);
  }
  return requestJson<Customer>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/customers/${customerId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新客户失败'
  );
};

export const deleteCustomer = async (
  tenantId: string,
  customerId: string
): Promise<Customer> => {
  if (useMocks) {
    return deleteMockCustomer(customerId);
  }
  return requestJson<Customer>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/customers/${customerId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除客户失败'
  );
};

export const replaceCustomerTags = async (
  tenantId: string,
  customerId: string,
  payload: CustomerTagReplacePayload
): Promise<CustomerTag[]> => {
  if (useMocks) {
    return replaceMockCustomerTags(customerId, payload);
  }
  return requestJson<CustomerTag[]>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/customers/${customerId}/tags`,
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
