import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type InboundReject = {
  reject_id: string;
  doc_id: string;
  doc_no: string;
  doc_type: string;
  warehouse_id: string;
  sku_id: string;
  sku_code?: string | null;
  barcode?: string | null;
  qty: number;
  reason?: string | null;
  operator_id?: string | null;
  created_at: string;
};

export type InboundRejectListQuery = {
  doc_no_keyword?: string;
  sku_id?: string;
  created_from?: string;
  created_to?: string;
  offset?: number;
  limit?: number;
};

export type InboundRejectListResponse = {
  items: InboundReject[];
  total: number;
};

export type InboundRejectCreatePayload = {
  items: Array<{
    sku_id: string;
    qty: number;
    reason?: string;
  }>;
};

export type InboundRejectCreateResponse = {
  inserted: number;
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

const buildQueryString = (query: InboundRejectListQuery): string => {
  const params = new URLSearchParams();
  if (query.doc_no_keyword) {
    params.set('doc_no_keyword', query.doc_no_keyword);
  }
  if (query.sku_id) {
    params.set('sku_id', query.sku_id);
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

export const listInboundRejects = async (
  tenantId: string,
  query: InboundRejectListQuery
): Promise<InboundRejectListResponse> => {
  if (useMocks) {
    return { items: [], total: 0 };
  }
  return requestJson<InboundRejectListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/inbound-rejects${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取拒收记录失败'
  );
};

export const createInboundRejects = async (
  tenantId: string,
  docId: string,
  payload: InboundRejectCreatePayload
): Promise<InboundRejectCreateResponse> => {
  if (useMocks) {
    return { inserted: payload.items.length };
  }
  return requestJson<InboundRejectCreateResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/inbounds/${docId}/rejects`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '提交拒收失败'
  );
};
