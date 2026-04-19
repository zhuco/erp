import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type InboundDocStatus =
  | 'DRAFT'
  | 'AUDITED'
  | 'RECEIVING'
  | 'PUTAWAY_DONE'
  | 'CANCELLED';
export type InboundDocType = 'PURCHASE' | 'RETURN' | 'TRANSFER' | 'OTHER';

export type InboundDocSummary = {
  doc_id: string;
  warehouse_id: string;
  doc_type: InboundDocType;
  doc_no: string;
  status: InboundDocStatus;
  ref_type?: string | null;
  ref_id?: string | null;
  supplier_id?: string | null;
  supplier_name?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export type InboundDocItem = {
  sku_id: string;
  sku_code?: string | null;
  barcode?: string | null;
  planned_qty: number;
  received_qty: number;
  putaway_qty: number;
};

export type InboundDocDetail = {
  doc: InboundDocSummary & {
    ref_type?: string | null;
    ref_id?: string | null;
  };
  items: InboundDocItem[];
};

export type InboundInspection = {
  inspection_id: string;
  doc_id: string;
  sku_id: string;
  qty: number;
  result: string;
  note?: string | null;
  operator_id?: string | null;
  operation_id: string;
  created_at: string;
};

export type InboundInspectionListResponse = {
  items: InboundInspection[];
};

export type InboundDocListResponse = {
  items: InboundDocSummary[];
  total: number;
};

export type InboundDocListQuery = {
  status?: InboundDocStatus;
  doc_type?: InboundDocType;
  warehouse_id?: string;
  doc_no?: string;
  doc_no_keyword?: string;
  ref_type?: string;
  ref_id?: string;
  supplier_id?: string;
  sku_id?: string;
  created_from?: string;
  created_to?: string;
  offset?: number;
  limit?: number;
};

export type InboundDocCreatePayload = {
  warehouse_id: string;
  doc_type: InboundDocType;
  doc_no: string;
  ref_type?: string | null;
  ref_id?: string | null;
  items: Array<{
    sku_id: string;
    planned_qty: number;
  }>;
};

export type InboundReceivePayload = {
  items: Array<{
    sku_id: string;
    qty: number;
    operation_id: string;
  }>;
};

export type InboundInspectPayload = {
  items: Array<{
    sku_id: string;
    qty: number;
    result: string;
    note?: string;
    operation_id: string;
  }>;
};

export type InboundPutawayPayload = {
  items: Array<{
    sku_id: string;
    qty: number;
    operation_id: string;
    from_area?: string;
    to_location_id?: string;
  }>;
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

const buildQueryString = (query: InboundDocListQuery): string => {
  const params = new URLSearchParams();
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.doc_type) {
    params.set('doc_type', query.doc_type);
  }
  if (query.warehouse_id) {
    params.set('warehouse_id', query.warehouse_id);
  }
  if (query.doc_no) {
    params.set('doc_no', query.doc_no);
  }
  if (query.doc_no_keyword) {
    params.set('doc_no_keyword', query.doc_no_keyword);
  }
  if (query.ref_type) {
    params.set('ref_type', query.ref_type);
  }
  if (query.ref_id) {
    params.set('ref_id', query.ref_id);
  }
  if (query.supplier_id) {
    params.set('supplier_id', query.supplier_id);
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

const mockDetail = (docId?: string): InboundDocDetail => ({
  doc: {
    doc_id: docId ?? 'mock-doc',
    warehouse_id: 'mock-warehouse',
    doc_type: 'PURCHASE',
    doc_no: 'INB-0001',
    status: 'RECEIVING',
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString()
  },
  items: [
    {
      sku_id: 'mock-sku',
      sku_code: 'SKU-001',
      planned_qty: 10,
      received_qty: 2,
      putaway_qty: 0
    }
  ]
});

export const createOperationId = (): string => {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return crypto.randomUUID();
  }
  return `inbound-${Date.now()}-${Math.random().toString(16).slice(2)}`;
};

export const listInbounds = async (
  tenantId: string,
  query: InboundDocListQuery
): Promise<InboundDocListResponse> => {
  if (useMocks) {
    return { items: [], total: 0 };
  }
  return requestJson<InboundDocListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/inbounds${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取入库单失败'
  );
};

export const getInboundDetail = async (
  tenantId: string,
  docId: string
): Promise<InboundDocDetail> => {
  if (useMocks) {
    return mockDetail(docId);
  }
  return requestJson<InboundDocDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/inbounds/${docId}`,
    {
      headers: buildAuthHeaders()
    },
    '获取入库单失败'
  );
};

export const createInboundDoc = async (
  tenantId: string,
  payload: InboundDocCreatePayload
): Promise<InboundDocDetail> => {
  if (useMocks) {
    if (!payload.items.length) {
      throw createApiError('缺少入库明细', 400);
    }
    return mockDetail();
  }
  return requestJson<InboundDocDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/inbounds`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建入库单失败'
  );
};

export const receiveInbound = async (
  tenantId: string,
  docId: string,
  payload: InboundReceivePayload
): Promise<InboundDocDetail> => {
  if (useMocks) {
    return mockDetail(docId);
  }
  return requestJson<InboundDocDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/inbounds/${docId}/receive`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '提交收货失败'
  );
};

export const listInboundInspections = async (
  tenantId: string,
  docId: string
): Promise<InboundInspectionListResponse> => {
  if (useMocks) {
    return { items: [] };
  }
  return requestJson<InboundInspectionListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/inbounds/${docId}/inspections`,
    {
      headers: buildAuthHeaders()
    },
    '获取质检记录失败'
  );
};

export const inspectInbound = async (
  tenantId: string,
  docId: string,
  payload: InboundInspectPayload
): Promise<void> => {
  if (useMocks) {
    return;
  }
  await requestJson<void>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/inbounds/${docId}/inspect`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '提交质检失败'
  );
};

export const putawayInbound = async (
  tenantId: string,
  docId: string,
  payload: InboundPutawayPayload
): Promise<InboundDocDetail> => {
  if (useMocks) {
    return mockDetail(docId);
  }
  return requestJson<InboundDocDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/inbounds/${docId}/putaway`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '提交上架失败'
  );
};

export const auditInboundDoc = async (
  tenantId: string,
  docId: string
): Promise<InboundDocDetail> => {
  if (useMocks) {
    return mockDetail(docId);
  }
  return requestJson<InboundDocDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/inbounds/${docId}/audit`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      }
    },
    '审核入库单失败'
  );
};
