import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockProcurementReconciliation,
  deleteMockProcurementReconciliation,
  getMockProcurementReconciliationList,
  updateMockProcurementReconciliation
} from '@/api/mocks';

export type ProcurementReconciliationStatus = 'OPEN' | 'CONFIRMED';

export type ProcurementReconciliation = {
  reconciliation_id: string;
  supplier_id: string;
  supplier_name?: string | null;
  period_start: string;
  period_end: string;
  amount_cent: number;
  status: ProcurementReconciliationStatus | string;
  remark?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export type ProcurementReconciliationListQuery = {
  supplier_id?: string;
  status?: ProcurementReconciliationStatus | string;
  period_from?: string;
  period_to?: string;
  offset?: number;
  limit?: number;
};

export type ProcurementReconciliationListResponse = {
  items: ProcurementReconciliation[];
  total: number;
};

export type ProcurementReconciliationCreatePayload = {
  supplier_id: string;
  period_start: string;
  period_end: string;
  amount_cent: number;
  status?: ProcurementReconciliationStatus | string | null;
  remark?: string | null;
};

export type ProcurementReconciliationUpdatePayload = {
  supplier_id?: string | null;
  period_start?: string | null;
  period_end?: string | null;
  amount_cent?: number | null;
  status?: ProcurementReconciliationStatus | string | null;
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

const buildQueryString = (query: ProcurementReconciliationListQuery): string => {
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

export const listProcurementReconciliations = async (
  tenantId: string,
  query: ProcurementReconciliationListQuery
): Promise<ProcurementReconciliationListResponse> => {
  if (useMocks) {
    return getMockProcurementReconciliationList(query);
  }
  return requestJson<ProcurementReconciliationListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/procurement-reconciliations${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    'Failed to load procurement reconciliations'
  );
};

export const createProcurementReconciliation = async (
  tenantId: string,
  payload: ProcurementReconciliationCreatePayload
): Promise<ProcurementReconciliation> => {
  if (useMocks) {
    return createMockProcurementReconciliation(payload);
  }
  return requestJson<ProcurementReconciliation>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/procurement-reconciliations`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    'Failed to create procurement reconciliation'
  );
};

export const updateProcurementReconciliation = async (
  tenantId: string,
  reconciliationId: string,
  payload: ProcurementReconciliationUpdatePayload
): Promise<ProcurementReconciliation> => {
  if (useMocks) {
    return updateMockProcurementReconciliation(reconciliationId, payload);
  }
  return requestJson<ProcurementReconciliation>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/procurement-reconciliations/${reconciliationId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    'Failed to update procurement reconciliation'
  );
};

export const deleteProcurementReconciliation = async (
  tenantId: string,
  reconciliationId: string
): Promise<ProcurementReconciliation> => {
  if (useMocks) {
    return deleteMockProcurementReconciliation(reconciliationId);
  }
  return requestJson<ProcurementReconciliation>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/procurement-reconciliations/${reconciliationId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    'Failed to delete procurement reconciliation'
  );
};
