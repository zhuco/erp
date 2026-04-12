import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockProcurementPlan,
  deleteMockProcurementPlan,
  getMockProcurementPlanDetail,
  getMockProcurementPlanList,
  updateMockProcurementPlan
} from '@/api/mocks';

export type ProcurementPlanStatus = 'DRAFT' | 'APPROVED' | 'RELEASED';

export type ProcurementPlan = {
  plan_id: string;
  request_id?: string | null;
  supplier_id?: string | null;
  supplier_name?: string | null;
  status: ProcurementPlanStatus | string;
  total_cent: number;
  planned_at?: string | null;
  expected_at?: string | null;
  remark?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export type ProcurementPlanItem = {
  sku_id: string;
  sku_code?: string | null;
  qty: number;
  price_cent: number;
};

export type ProcurementPlanDetail = {
  plan: ProcurementPlan;
  items: ProcurementPlanItem[];
};

export type ProcurementPlanListQuery = {
  supplier_id?: string;
  status?: ProcurementPlanStatus | string;
  created_from?: string;
  created_to?: string;
  offset?: number;
  limit?: number;
};

export type ProcurementPlanListResponse = {
  items: ProcurementPlan[];
  total: number;
};

export type ProcurementPlanItemPayload = {
  sku_id: string;
  qty: number;
  price_cent: number;
};

export type ProcurementPlanCreatePayload = {
  request_id?: string | null;
  supplier_id?: string | null;
  planned_at?: string | null;
  expected_at?: string | null;
  remark?: string | null;
  items: ProcurementPlanItemPayload[];
};

export type ProcurementPlanUpdatePayload = {
  supplier_id?: string | null;
  status?: ProcurementPlanStatus | string | null;
  planned_at?: string | null;
  expected_at?: string | null;
  remark?: string | null;
  items?: ProcurementPlanItemPayload[];
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

const buildQueryString = (query: ProcurementPlanListQuery): string => {
  const params = new URLSearchParams();
  if (query.supplier_id) {
    params.set('supplier_id', query.supplier_id);
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

export const listProcurementPlans = async (
  tenantId: string,
  query: ProcurementPlanListQuery
): Promise<ProcurementPlanListResponse> => {
  if (useMocks) {
    return getMockProcurementPlanList(query);
  }
  return requestJson<ProcurementPlanListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/procurement-plans${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    'Failed to load procurement plans'
  );
};

export const getProcurementPlanDetail = async (
  tenantId: string,
  planId: string
): Promise<ProcurementPlanDetail> => {
  if (useMocks) {
    return getMockProcurementPlanDetail(planId);
  }
  return requestJson<ProcurementPlanDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/procurement-plans/${planId}`,
    { headers: buildAuthHeaders() },
    'Failed to load procurement plan'
  );
};

export const createProcurementPlan = async (
  tenantId: string,
  payload: ProcurementPlanCreatePayload
): Promise<ProcurementPlanDetail> => {
  if (useMocks) {
    return createMockProcurementPlan(payload);
  }
  return requestJson<ProcurementPlanDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/procurement-plans`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    'Failed to create procurement plan'
  );
};

export const updateProcurementPlan = async (
  tenantId: string,
  planId: string,
  payload: ProcurementPlanUpdatePayload
): Promise<ProcurementPlanDetail> => {
  if (useMocks) {
    return updateMockProcurementPlan(planId, payload);
  }
  return requestJson<ProcurementPlanDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/procurement-plans/${planId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    'Failed to update procurement plan'
  );
};

export const deleteProcurementPlan = async (
  tenantId: string,
  planId: string
): Promise<ProcurementPlan> => {
  if (useMocks) {
    return deleteMockProcurementPlan(planId);
  }
  return requestJson<ProcurementPlan>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/procurement-plans/${planId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    'Failed to delete procurement plan'
  );
};
