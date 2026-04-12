import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockProcurementRequest,
  deleteMockProcurementRequest,
  getMockProcurementRequestDetail,
  getMockProcurementRequestList,
  updateMockProcurementRequest
} from '@/api/mocks';

export type ProcurementRequestStatus = 'DRAFT' | 'APPROVED' | 'REJECTED';

export type ProcurementRequest = {
  request_id: string;
  department?: string | null;
  requester?: string | null;
  status: ProcurementRequestStatus | string;
  budget_cent: number;
  expected_at?: string | null;
  remark?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export type ProcurementRequestItem = {
  sku_id: string;
  sku_code?: string | null;
  qty: number;
  price_cent: number;
};

export type ProcurementRequestDetail = {
  request: ProcurementRequest;
  items: ProcurementRequestItem[];
};

export type ProcurementRequestListQuery = {
  department?: string;
  requester?: string;
  status?: ProcurementRequestStatus | string;
  created_from?: string;
  created_to?: string;
  offset?: number;
  limit?: number;
};

export type ProcurementRequestListResponse = {
  items: ProcurementRequest[];
  total: number;
};

export type ProcurementRequestItemPayload = {
  sku_id: string;
  qty: number;
  price_cent: number;
};

export type ProcurementRequestCreatePayload = {
  department?: string | null;
  requester?: string | null;
  expected_at?: string | null;
  remark?: string | null;
  items: ProcurementRequestItemPayload[];
};

export type ProcurementRequestUpdatePayload = {
  department?: string | null;
  requester?: string | null;
  status?: ProcurementRequestStatus | string | null;
  expected_at?: string | null;
  remark?: string | null;
  items?: ProcurementRequestItemPayload[];
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

const buildQueryString = (query: ProcurementRequestListQuery): string => {
  const params = new URLSearchParams();
  if (query.department) {
    params.set('department', query.department);
  }
  if (query.requester) {
    params.set('requester', query.requester);
  }
  if (query.status) {
    params.set('status', String(query.status));
  }
  if (query.created_from) {
    params.set('created_from', query.created_from);
  }
  if (query.created_to) {
    params.set('created_to', query.created_to);
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

export const listProcurementRequests = async (
  tenantId: string,
  query: ProcurementRequestListQuery
): Promise<ProcurementRequestListResponse> => {
  if (useMocks) {
    return getMockProcurementRequestList(query);
  }
  return requestJson<ProcurementRequestListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/procurement-requests${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    'Failed to load procurement requests'
  );
};

export const getProcurementRequestDetail = async (
  tenantId: string,
  requestId: string
): Promise<ProcurementRequestDetail> => {
  if (useMocks) {
    return getMockProcurementRequestDetail(requestId);
  }
  return requestJson<ProcurementRequestDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/procurement-requests/${requestId}`,
    { headers: buildAuthHeaders() },
    'Failed to load procurement request'
  );
};

export const createProcurementRequest = async (
  tenantId: string,
  payload: ProcurementRequestCreatePayload
): Promise<ProcurementRequestDetail> => {
  if (useMocks) {
    return createMockProcurementRequest(payload);
  }
  return requestJson<ProcurementRequestDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/procurement-requests`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    'Failed to create procurement request'
  );
};

export const updateProcurementRequest = async (
  tenantId: string,
  requestId: string,
  payload: ProcurementRequestUpdatePayload
): Promise<ProcurementRequestDetail> => {
  if (useMocks) {
    return updateMockProcurementRequest(requestId, payload);
  }
  return requestJson<ProcurementRequestDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/procurement-requests/${requestId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    'Failed to update procurement request'
  );
};

export const deleteProcurementRequest = async (
  tenantId: string,
  requestId: string
): Promise<ProcurementRequest> => {
  if (useMocks) {
    return deleteMockProcurementRequest(requestId);
  }
  return requestJson<ProcurementRequest>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/procurement-requests/${requestId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    'Failed to delete procurement request'
  );
};
