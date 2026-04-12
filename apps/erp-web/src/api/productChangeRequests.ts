import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockProductChangeRequest,
  deleteMockProductChangeRequest,
  getMockProductChangeRequest,
  getMockProductChangeRequestList,
  updateMockProductChangeRequest
} from '@/api/mocks';

export type ProductChangeHistoryItem = {
  time: string;
  action: string;
  actor: string;
  note?: string | null;
};

export type ProductChangeRequest = {
  request_id: string;
  tenant_id: string;
  sku_id: string;
  sku_code?: string | null;
  sku_name?: string | null;
  title: string;
  change_type: string;
  status: string;
  applicant: string;
  approver?: string | null;
  reason?: string | null;
  history: ProductChangeHistoryItem[];
  created_at?: string | null;
  updated_at?: string | null;
};

export type ProductChangeRequestListQuery = {
  sku_id?: string;
  status?: string;
  change_type?: string;
  offset?: number;
  limit?: number;
};

export type ProductChangeRequestListResponse = {
  items: ProductChangeRequest[];
  total: number;
};

export type ProductChangeRequestCreatePayload = {
  sku_id: string;
  title: string;
  change_type: string;
  status?: string;
  applicant: string;
  approver?: string | null;
  reason?: string | null;
  history?: ProductChangeHistoryItem[];
};

export type ProductChangeRequestUpdatePayload = {
  title?: string;
  change_type?: string;
  status?: string;
  approver?: string | null;
  reason?: string | null;
  history?: ProductChangeHistoryItem[];
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

const buildQueryString = (query: ProductChangeRequestListQuery): string => {
  const params = new URLSearchParams();
  if (query.sku_id) {
    params.set('sku_id', query.sku_id);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.change_type) {
    params.set('change_type', query.change_type);
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

export const listProductChangeRequests = async (
  tenantId: string,
  query: ProductChangeRequestListQuery
): Promise<ProductChangeRequestListResponse> => {
  if (useMocks) {
    return getMockProductChangeRequestList(query);
  }
  return requestJson<ProductChangeRequestListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-change-requests${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取变更申请失败'
  );
};

export const getProductChangeRequest = async (
  tenantId: string,
  requestId: string
): Promise<ProductChangeRequest> => {
  if (useMocks) {
    return getMockProductChangeRequest(requestId);
  }
  return requestJson<ProductChangeRequest>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-change-requests/${requestId}`,
    { headers: buildAuthHeaders() },
    '获取变更申请详情失败'
  );
};

export const createProductChangeRequest = async (
  tenantId: string,
  payload: ProductChangeRequestCreatePayload
): Promise<ProductChangeRequest> => {
  if (useMocks) {
    return createMockProductChangeRequest(payload);
  }
  return requestJson<ProductChangeRequest>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-change-requests`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建变更申请失败'
  );
};

export const updateProductChangeRequest = async (
  tenantId: string,
  requestId: string,
  payload: ProductChangeRequestUpdatePayload
): Promise<ProductChangeRequest> => {
  if (useMocks) {
    return updateMockProductChangeRequest(requestId, payload);
  }
  return requestJson<ProductChangeRequest>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-change-requests/${requestId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新变更申请失败'
  );
};

export const deleteProductChangeRequest = async (
  tenantId: string,
  requestId: string
): Promise<ProductChangeRequest> => {
  if (useMocks) {
    return deleteMockProductChangeRequest(requestId);
  }
  return requestJson<ProductChangeRequest>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/product-change-requests/${requestId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除变更申请失败'
  );
};
