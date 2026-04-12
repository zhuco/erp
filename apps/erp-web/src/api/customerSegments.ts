import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockCustomerSegment,
  deleteMockCustomerSegment,
  getMockCustomerSegmentList,
  updateMockCustomerSegment
} from '@/api/mocks';

export type CustomerSegmentType = 'DYNAMIC' | 'STATIC';

export type CustomerSegment = {
  segment_id: string;
  name: string;
  segment_type: CustomerSegmentType;
  rule_summary?: string | null;
  customer_count: number;
  status?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export type CustomerSegmentListQuery = {
  keyword?: string;
  segment_type?: CustomerSegmentType;
  status?: string;
  offset?: number;
  limit?: number;
};

export type CustomerSegmentListResponse = {
  items: CustomerSegment[];
  total: number;
};

export type CustomerSegmentCreatePayload = {
  name: string;
  segment_type: CustomerSegmentType;
  rule_summary?: string;
  status?: string;
};

export type CustomerSegmentUpdatePayload = {
  name?: string;
  segment_type?: CustomerSegmentType;
  rule_summary?: string;
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

const buildQueryString = (query: CustomerSegmentListQuery): string => {
  const params = new URLSearchParams();
  if (query.keyword) {
    params.set('keyword', query.keyword);
  }
  if (query.segment_type) {
    params.set('segment_type', query.segment_type);
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

export const listCustomerSegments = async (
  tenantId: string,
  query: CustomerSegmentListQuery
): Promise<CustomerSegmentListResponse> => {
  if (useMocks) {
    return getMockCustomerSegmentList(query);
  }
  return requestJson<CustomerSegmentListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/crm/customer-segments${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取客户分群失败'
  );
};

export const createCustomerSegment = async (
  tenantId: string,
  payload: CustomerSegmentCreatePayload
): Promise<CustomerSegment> => {
  if (useMocks) {
    return createMockCustomerSegment(payload);
  }
  return requestJson<CustomerSegment>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/crm/customer-segments`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建客户分群失败'
  );
};

export const updateCustomerSegment = async (
  tenantId: string,
  segmentId: string,
  payload: CustomerSegmentUpdatePayload
): Promise<CustomerSegment> => {
  if (useMocks) {
    return updateMockCustomerSegment(segmentId, payload);
  }
  return requestJson<CustomerSegment>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/crm/customer-segments/${segmentId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新客户分群失败'
  );
};

export const deleteCustomerSegment = async (
  tenantId: string,
  segmentId: string
): Promise<CustomerSegment> => {
  if (useMocks) {
    return deleteMockCustomerSegment(segmentId);
  }
  return requestJson<CustomerSegment>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/crm/customer-segments/${segmentId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除客户分群失败'
  );
};
