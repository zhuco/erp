import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockProcurementSourcing,
  deleteMockProcurementSourcing,
  getMockProcurementSourcingList,
  updateMockProcurementSourcing
} from '@/api/mocks';

export type ProcurementSourcingStatus = 'OPEN' | 'REVIEWED';

export type ProcurementSourcing = {
  sourcing_id: string;
  title: string;
  category?: string | null;
  supplier_id?: string | null;
  supplier_name?: string | null;
  status: ProcurementSourcingStatus | string;
  target_price_cent: number;
  remark?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export type ProcurementSourcingListQuery = {
  keyword?: string;
  supplier_id?: string;
  status?: ProcurementSourcingStatus | string;
  created_from?: string;
  created_to?: string;
  offset?: number;
  limit?: number;
};

export type ProcurementSourcingListResponse = {
  items: ProcurementSourcing[];
  total: number;
};

export type ProcurementSourcingCreatePayload = {
  title: string;
  category?: string | null;
  supplier_id?: string | null;
  status?: ProcurementSourcingStatus | string | null;
  target_price_cent: number;
  remark?: string | null;
};

export type ProcurementSourcingUpdatePayload = {
  title?: string | null;
  category?: string | null;
  supplier_id?: string | null;
  status?: ProcurementSourcingStatus | string | null;
  target_price_cent?: number | null;
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

const buildQueryString = (query: ProcurementSourcingListQuery): string => {
  const params = new URLSearchParams();
  if (query.keyword) {
    params.set('keyword', query.keyword);
  }
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

export const listProcurementSourcing = async (
  tenantId: string,
  query: ProcurementSourcingListQuery
): Promise<ProcurementSourcingListResponse> => {
  if (useMocks) {
    return getMockProcurementSourcingList(query);
  }
  return requestJson<ProcurementSourcingListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/procurement-sourcing${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    'Failed to load procurement sourcing'
  );
};

export const createProcurementSourcing = async (
  tenantId: string,
  payload: ProcurementSourcingCreatePayload
): Promise<ProcurementSourcing> => {
  if (useMocks) {
    return createMockProcurementSourcing(payload);
  }
  return requestJson<ProcurementSourcing>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/procurement-sourcing`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    'Failed to create procurement sourcing'
  );
};

export const updateProcurementSourcing = async (
  tenantId: string,
  sourcingId: string,
  payload: ProcurementSourcingUpdatePayload
): Promise<ProcurementSourcing> => {
  if (useMocks) {
    return updateMockProcurementSourcing(sourcingId, payload);
  }
  return requestJson<ProcurementSourcing>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/procurement-sourcing/${sourcingId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    'Failed to update procurement sourcing'
  );
};

export const deleteProcurementSourcing = async (
  tenantId: string,
  sourcingId: string
): Promise<ProcurementSourcing> => {
  if (useMocks) {
    return deleteMockProcurementSourcing(sourcingId);
  }
  return requestJson<ProcurementSourcing>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/procurement-sourcing/${sourcingId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    'Failed to delete procurement sourcing'
  );
};
