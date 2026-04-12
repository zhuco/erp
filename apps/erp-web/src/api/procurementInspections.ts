import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockProcurementInspection,
  deleteMockProcurementInspection,
  getMockProcurementInspectionDetail,
  getMockProcurementInspectionList,
  updateMockProcurementInspection
} from '@/api/mocks';

export type ProcurementInspectionStatus = 'PENDING' | 'PASSED' | 'FAILED';

export type ProcurementInspection = {
  inspection_id: string;
  receipt_id?: string | null;
  supplier_id?: string | null;
  supplier_name?: string | null;
  warehouse_id?: string | null;
  status: ProcurementInspectionStatus | string;
  inspected_at?: string | null;
  remark?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export type ProcurementInspectionItem = {
  sku_id: string;
  sku_code?: string | null;
  qty: number;
  passed_qty: number;
  failed_qty: number;
  result?: string | null;
};

export type ProcurementInspectionDetail = {
  inspection: ProcurementInspection;
  items: ProcurementInspectionItem[];
};

export type ProcurementInspectionListQuery = {
  supplier_id?: string;
  warehouse_id?: string;
  status?: ProcurementInspectionStatus | string;
  created_from?: string;
  created_to?: string;
  offset?: number;
  limit?: number;
};

export type ProcurementInspectionListResponse = {
  items: ProcurementInspection[];
  total: number;
};

export type ProcurementInspectionItemPayload = {
  sku_id: string;
  qty: number;
  passed_qty: number;
  failed_qty: number;
  result?: string | null;
};

export type ProcurementInspectionCreatePayload = {
  receipt_id?: string | null;
  supplier_id?: string | null;
  warehouse_id?: string | null;
  inspected_at?: string | null;
  remark?: string | null;
  items: ProcurementInspectionItemPayload[];
};

export type ProcurementInspectionUpdatePayload = {
  status?: ProcurementInspectionStatus | string | null;
  inspected_at?: string | null;
  remark?: string | null;
  items?: ProcurementInspectionItemPayload[];
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

const buildQueryString = (query: ProcurementInspectionListQuery): string => {
  const params = new URLSearchParams();
  if (query.supplier_id) {
    params.set('supplier_id', query.supplier_id);
  }
  if (query.warehouse_id) {
    params.set('warehouse_id', query.warehouse_id);
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

export const listProcurementInspections = async (
  tenantId: string,
  query: ProcurementInspectionListQuery
): Promise<ProcurementInspectionListResponse> => {
  if (useMocks) {
    return getMockProcurementInspectionList(query);
  }
  return requestJson<ProcurementInspectionListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/procurement-inspections${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    'Failed to load procurement inspections'
  );
};

export const getProcurementInspectionDetail = async (
  tenantId: string,
  inspectionId: string
): Promise<ProcurementInspectionDetail> => {
  if (useMocks) {
    return getMockProcurementInspectionDetail(inspectionId);
  }
  return requestJson<ProcurementInspectionDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/procurement-inspections/${inspectionId}`,
    { headers: buildAuthHeaders() },
    'Failed to load procurement inspection'
  );
};

export const createProcurementInspection = async (
  tenantId: string,
  payload: ProcurementInspectionCreatePayload
): Promise<ProcurementInspectionDetail> => {
  if (useMocks) {
    return createMockProcurementInspection(payload);
  }
  return requestJson<ProcurementInspectionDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/procurement-inspections`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    'Failed to create procurement inspection'
  );
};

export const updateProcurementInspection = async (
  tenantId: string,
  inspectionId: string,
  payload: ProcurementInspectionUpdatePayload
): Promise<ProcurementInspectionDetail> => {
  if (useMocks) {
    return updateMockProcurementInspection(inspectionId, payload);
  }
  return requestJson<ProcurementInspectionDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/procurement-inspections/${inspectionId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    'Failed to update procurement inspection'
  );
};

export const deleteProcurementInspection = async (
  tenantId: string,
  inspectionId: string
): Promise<ProcurementInspection> => {
  if (useMocks) {
    return deleteMockProcurementInspection(inspectionId);
  }
  return requestJson<ProcurementInspection>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/procurement-inspections/${inspectionId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    'Failed to delete procurement inspection'
  );
};
