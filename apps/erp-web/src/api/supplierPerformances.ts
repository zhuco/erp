import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockSupplierPerformance,
  deleteMockSupplierPerformance,
  getMockSupplierPerformanceList,
  updateMockSupplierPerformance
} from '@/api/mocks';

export type SupplierPerformanceStatus = 'ACTIVE';

export type SupplierPerformance = {
  performance_id: string;
  supplier_id: string;
  supplier_name?: string | null;
  period_start: string;
  period_end: string;
  delivery_score: number;
  quality_score: number;
  price_score: number;
  total_score: number;
  status: SupplierPerformanceStatus | string;
  remark?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export type SupplierPerformanceListQuery = {
  supplier_id?: string;
  status?: SupplierPerformanceStatus | string;
  period_from?: string;
  period_to?: string;
  offset?: number;
  limit?: number;
};

export type SupplierPerformanceListResponse = {
  items: SupplierPerformance[];
  total: number;
};

export type SupplierPerformanceCreatePayload = {
  supplier_id: string;
  period_start: string;
  period_end: string;
  delivery_score: number;
  quality_score: number;
  price_score: number;
  status?: SupplierPerformanceStatus | string | null;
  remark?: string | null;
};

export type SupplierPerformanceUpdatePayload = {
  supplier_id?: string | null;
  period_start?: string | null;
  period_end?: string | null;
  delivery_score?: number | null;
  quality_score?: number | null;
  price_score?: number | null;
  status?: SupplierPerformanceStatus | string | null;
  remark?: string | null;
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

const buildQueryString = (query: SupplierPerformanceListQuery): string => {
  const params = new URLSearchParams();
  if (query.supplier_id) {
    params.set('supplier_id', query.supplier_id);
  }
  if (query.status) {
    params.set('status', String(query.status));
  }
  if (query.period_from) {
    params.set('period_from', query.period_from);
  }
  if (query.period_to) {
    params.set('period_to', query.period_to);
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

export const listSupplierPerformances = async (
  tenantId: string,
  query: SupplierPerformanceListQuery
): Promise<SupplierPerformanceListResponse> => {
  if (useMocks) {
    return getMockSupplierPerformanceList(query);
  }
  return requestJson<SupplierPerformanceListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/supplier-performances${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    'Failed to load supplier performances'
  );
};

export const createSupplierPerformance = async (
  tenantId: string,
  payload: SupplierPerformanceCreatePayload
): Promise<SupplierPerformance> => {
  if (useMocks) {
    return createMockSupplierPerformance(payload);
  }
  return requestJson<SupplierPerformance>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/supplier-performances`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    'Failed to create supplier performance'
  );
};

export const updateSupplierPerformance = async (
  tenantId: string,
  performanceId: string,
  payload: SupplierPerformanceUpdatePayload
): Promise<SupplierPerformance> => {
  if (useMocks) {
    return updateMockSupplierPerformance(performanceId, payload);
  }
  return requestJson<SupplierPerformance>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/supplier-performances/${performanceId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    'Failed to update supplier performance'
  );
};

export const deleteSupplierPerformance = async (
  tenantId: string,
  performanceId: string
): Promise<SupplierPerformance> => {
  if (useMocks) {
    return deleteMockSupplierPerformance(performanceId);
  }
  return requestJson<SupplierPerformance>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/supplier-performances/${performanceId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    'Failed to delete supplier performance'
  );
};
