import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  confirmMockPurchaseReturn,
  createMockPurchaseReturn,
  getMockPurchaseReturnDetail,
  getMockPurchaseReturnList
} from '@/api/mocks';

export type PurchaseReturnStatus = 'DRAFT' | 'DONE';

export type PurchaseReturnSummary = {
  return_id: string;
  po_id: string;
  supplier_id?: string | null;
  supplier_name?: string | null;
  warehouse_id: string;
  status: PurchaseReturnStatus | string;
  confirmed_at?: string | null;
  created_at?: string | null;
};

export type PurchaseReturnItem = {
  sku_id: string;
  qty: number;
};

export type PurchaseReturnDetail = {
  purchase_return: PurchaseReturnSummary;
  items: PurchaseReturnItem[];
};

export type PurchaseReturnListQuery = {
  supplier_id?: string;
  warehouse_id?: string;
  status?: string;
  created_from?: string;
  created_to?: string;
  offset?: number;
  limit?: number;
};

export type PurchaseReturnListResponse = {
  items: PurchaseReturnSummary[];
  total: number;
};

export type PurchaseReturnCreatePayload = {
  po_id: string;
  warehouse_id: string;
  items: PurchaseReturnItem[];
};

export type PurchaseReturnConfirmResponse = {
  purchase_return: PurchaseReturnSummary;
  outbound_doc_id: string;
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

const buildQueryString = (query: PurchaseReturnListQuery): string => {
  const params = new URLSearchParams();
  if (query.supplier_id) {
    params.set('supplier_id', query.supplier_id);
  }
  if (query.warehouse_id) {
    params.set('warehouse_id', query.warehouse_id);
  }
  if (query.status) {
    params.set('status', query.status);
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

export const listPurchaseReturns = async (
  tenantId: string,
  query: PurchaseReturnListQuery
): Promise<PurchaseReturnListResponse> => {
  if (useMocks) {
    return getMockPurchaseReturnList(query);
  }
  return requestJson<PurchaseReturnListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/purchase-returns${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    'Failed to load purchase returns'
  );
};

export const getPurchaseReturnDetail = async (
  tenantId: string,
  returnId: string
): Promise<PurchaseReturnDetail> => {
  if (useMocks) {
    return getMockPurchaseReturnDetail(returnId);
  }
  return requestJson<PurchaseReturnDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/purchase-returns/${returnId}`,
    { headers: buildAuthHeaders() },
    'Failed to load purchase return'
  );
};

export const createPurchaseReturn = async (
  tenantId: string,
  payload: PurchaseReturnCreatePayload
): Promise<PurchaseReturnDetail> => {
  if (useMocks) {
    return createMockPurchaseReturn(payload);
  }
  return requestJson<PurchaseReturnDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/purchase-returns`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    'Failed to create purchase return'
  );
};

export const confirmPurchaseReturn = async (
  tenantId: string,
  returnId: string
): Promise<PurchaseReturnConfirmResponse> => {
  if (useMocks) {
    return confirmMockPurchaseReturn(returnId);
  }
  return requestJson<PurchaseReturnConfirmResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/purchase-returns/${returnId}/confirm`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    'Failed to confirm purchase return'
  );
};
