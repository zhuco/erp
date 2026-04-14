import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  getMockAfterSalesOpsDetail,
  getMockAfterSalesOpsList,
  getMockAfterSalesOpsCreate,
  getMockRefundDoc,
  getMockRefundDocs,
  getMockAfterSalesImportJobs,
  getMockAfterSalesImportJob,
  putawayMockAfterSales
} from '@/api/mocks';
import type { OmsAllocationResponse } from '@/api/omsShipments';

export type AfterSalesType = 'REFUND_ONLY' | 'RETURN_REFUND' | 'EXCHANGE' | 'RESEND';
export type AfterSalesStatus =
  | 'OPEN'
  | 'APPROVED'
  | 'RECEIVED'
  | 'REFUNDED'
  | 'CLOSED'
  | 'REJECTED';
export type RefundStatus = 'PENDING' | 'SENT' | 'FAILED' | 'DONE';
export type RefundChannel = 'PLATFORM' | 'MANUAL';

export type AfterSalesSummary = {
  id: string;
  tenant_id: string;
  order_id: string;
  type: AfterSalesType;
  status: AfterSalesStatus;
  reason_code?: string | null;
  amount_cent: number;
  sla_deadline_at?: string | null;
  sla_status?: 'ON_TRACK' | 'DUE_SOON' | 'OVERDUE' | 'CLOSED' | string | null;
  sla_remaining_minutes?: number | null;
  escalation_role?: string | null;
  created_at: string;
  updated_at: string;
};

export type AfterSalesItem = {
  sku_id: string;
  qty: number;
  refund_cent: number;
};

export type ReturnReceipt = {
  id: string;
  after_sales_id: string;
  warehouse_id: string;
  status: 'RECEIVING' | 'PUTAWAY_DONE' | 'CANCELLED';
  received_at?: string | null;
  putaway_at?: string | null;
  created_at: string;
  updated_at: string;
};

export type RefundDoc = {
  id: string;
  after_sales_id: string;
  amount_cent: number;
  status: RefundStatus;
  channel: RefundChannel;
  created_at: string;
  updated_at: string;
};

export type AfterSalesShipment = {
  shipment_id: string;
  shipment_type: string;
  origin_shipment_id?: string | null;
  created_at: string;
};

export type AfterSalesDetail = {
  after_sales: AfterSalesSummary;
  items: AfterSalesItem[];
  return_receipt?: ReturnReceipt | null;
  refund_doc?: RefundDoc | null;
  shipments?: AfterSalesShipment[];
  raw_json?: Record<string, unknown> | null;
  audit_logs?: AfterSalesAuditLog[];
};

export type AfterSalesAuditLog = {
  audit_id: string;
  action: string;
  user_id?: string | null;
  before_json?: Record<string, unknown> | null;
  after_json?: Record<string, unknown> | null;
  created_at: string;
};

export type AfterSalesImportJob = {
  job_id: string;
  file_name?: string | null;
  file_type: string;
  status: string;
  progress: number;
  total_rows: number;
  success_rows: number;
  failed_rows: number;
  attempts: number;
  last_error: string;
  created_at: string;
  updated_at: string;
};

export type AfterSalesListQuery = {
  status?: AfterSalesStatus;
  after_sales_type?: AfterSalesType;
  order_id?: string;
  created_from?: string;
  created_to?: string;
  offset?: number;
  limit?: number;
};

export type RefundDocListQuery = {
  status?: RefundStatus;
  after_sales_id?: string;
  created_from?: string;
  created_to?: string;
  offset?: number;
  limit?: number;
};

export type AfterSalesListResponse = {
  items: AfterSalesSummary[];
  total: number;
  limit: number;
  offset: number;
};

export type RefundDocListResponse = {
  items: RefundDoc[];
  total: number;
  limit: number;
  offset: number;
};

export type AfterSalesImportJobListQuery = {
  status?: string;
  offset?: number;
  limit?: number;
};

export type AfterSalesImportJobListResponse = {
  items: AfterSalesImportJob[];
  total: number;
  limit: number;
  offset: number;
};

export type AfterSalesCreatePayload = {
  order_id: string;
  type: AfterSalesType;
  reason_code?: string;
  amount_cent?: number;
  items: AfterSalesItem[];
};

export type AfterSalesReceivePayload = {
  warehouse_id: string;
  putaway_done?: boolean;
};

export type AfterSalesRefundPayload = {
  amount_cent?: number;
  channel?: RefundChannel;
};

export type AfterSalesShipmentPayload = {
  origin_shipment_id?: string;
  carrier_code?: string;
};

const buildQueryString = (query: Record<string, unknown>): string => {
  const params = new URLSearchParams();
  Object.entries(query).forEach(([key, value]) => {
    if (value === undefined || value === null || value === '') {
      return;
    }
    params.set(key, String(value));
  });
  const serialized = params.toString();
  return serialized ? `?${serialized}` : '';
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

export const createIdempotencyKey = (prefix = 'as'): string => {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return `${prefix}-${crypto.randomUUID()}`;
  }
  return `${prefix}-${Date.now()}-${Math.random().toString(16).slice(2)}`;
};

export const listAfterSales = async (
  tenantId: string,
  query: AfterSalesListQuery
): Promise<AfterSalesListResponse> => {
  if (useMocks) {
    return getMockAfterSalesOpsList(query);
  }
  return requestJson<AfterSalesListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取售后列表失败'
  );
};

export const getAfterSalesDetail = async (
  tenantId: string,
  afterSalesId: string
): Promise<AfterSalesDetail> => {
  if (useMocks) {
    return getMockAfterSalesOpsDetail(afterSalesId);
  }
  return requestJson<AfterSalesDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/${afterSalesId}`,
    {
      headers: buildAuthHeaders()
    },
    '获取售后详情失败'
  );
};

export const getAfterSalesRaw = async (
  tenantId: string,
  afterSalesId: string
): Promise<Record<string, unknown>> => {
  if (useMocks) {
    return (await getMockAfterSalesOpsDetail(afterSalesId)).raw_json ?? {};
  }
  return requestJson<Record<string, unknown>>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/${afterSalesId}/raw`,
    {
      headers: buildAuthHeaders()
    },
    '获取售后原始数据失败'
  );
};

export const listAfterSalesLogs = async (
  tenantId: string,
  afterSalesId: string,
  limit = 200
): Promise<AfterSalesAuditLog[]> => {
  if (useMocks) {
    return (await getMockAfterSalesOpsDetail(afterSalesId)).audit_logs ?? [];
  }
  return requestJson<AfterSalesAuditLog[]>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/${afterSalesId}/logs${buildQueryString({
      limit
    })}`,
    {
      headers: buildAuthHeaders()
    },
    '获取售后处理记录失败'
  );
};

export const createAfterSales = async (
  tenantId: string,
  payload: AfterSalesCreatePayload
): Promise<AfterSalesDetail> => {
  if (useMocks) {
    return getMockAfterSalesOpsCreate(payload);
  }
  return requestJson<AfterSalesDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Idempotency-Key': createIdempotencyKey('as-create'),
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建售后失败'
  );
};

export const approveAfterSales = async (
  tenantId: string,
  afterSalesId: string
): Promise<AfterSalesSummary> => {
  if (useMocks) {
    return (await getMockAfterSalesOpsDetail(afterSalesId)).after_sales;
  }
  return requestJson<AfterSalesSummary>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/${afterSalesId}/approve`,
    {
      method: 'POST',
      headers: {
        'Idempotency-Key': createIdempotencyKey('as-approve'),
        ...buildAuthHeaders()
      }
    },
    '审核售后失败'
  );
};

export const rejectAfterSales = async (
  tenantId: string,
  afterSalesId: string
): Promise<AfterSalesSummary> => {
  if (useMocks) {
    return (await getMockAfterSalesOpsDetail(afterSalesId)).after_sales;
  }
  return requestJson<AfterSalesSummary>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/${afterSalesId}/reject`,
    {
      method: 'POST',
      headers: {
        'Idempotency-Key': createIdempotencyKey('as-reject'),
        ...buildAuthHeaders()
      }
    },
    '驳回售后失败'
  );
};

export const receiveAfterSales = async (
  tenantId: string,
  afterSalesId: string,
  payload: AfterSalesReceivePayload
): Promise<AfterSalesDetail> => {
  if (useMocks) {
    return getMockAfterSalesOpsDetail(afterSalesId);
  }
  return requestJson<AfterSalesDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/${afterSalesId}/receive`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Idempotency-Key': createIdempotencyKey('as-receive'),
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '售后收货失败'
  );
};

export const cancelAfterSalesReceive = async (
  tenantId: string,
  afterSalesId: string
): Promise<AfterSalesDetail> => {
  if (useMocks) {
    return getMockAfterSalesOpsDetail(afterSalesId);
  }
  return requestJson<AfterSalesDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/${afterSalesId}/receive/cancel`,
    {
      method: 'POST',
      headers: {
        'Idempotency-Key': createIdempotencyKey('as-receive-cancel'),
        ...buildAuthHeaders()
      }
    },
    '撤销售后收货失败'
  );
};

export const redoAfterSalesReceive = async (
  tenantId: string,
  afterSalesId: string,
  payload: AfterSalesReceivePayload
): Promise<AfterSalesDetail> => {
  if (useMocks) {
    return getMockAfterSalesOpsDetail(afterSalesId);
  }
  return requestJson<AfterSalesDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/${afterSalesId}/receive/redo`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Idempotency-Key': createIdempotencyKey('as-receive-redo'),
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '重做售后收货失败'
  );
};

export const putawayAfterSales = async (
  tenantId: string,
  afterSalesId: string
): Promise<AfterSalesDetail> => {
  if (useMocks) {
    return putawayMockAfterSales(afterSalesId);
  }
  return requestJson<AfterSalesDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/${afterSalesId}/putaway`,
    {
      method: 'POST',
      headers: {
        'Idempotency-Key': createIdempotencyKey('as-putaway'),
        ...buildAuthHeaders()
      }
    },
    '库存回补失败'
  );
};

export const createAfterSalesExchangeShipment = async (
  tenantId: string,
  afterSalesId: string,
  payload: AfterSalesShipmentPayload
): Promise<OmsAllocationResponse> => {
  if (useMocks) {
    return { status: 'ALLOCATED', shipments: [], shortages: [] };
  }
  return requestJson<OmsAllocationResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/${afterSalesId}/exchange`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Idempotency-Key': createIdempotencyKey('as-exchange'),
        ...buildAuthHeaders()
      },
      body: JSON.stringify({
        origin_shipment_id: payload.origin_shipment_id ?? null,
        carrier_code: payload.carrier_code ?? null
      })
    },
    '生成换货包裹失败'
  );
};

export const createAfterSalesResendShipment = async (
  tenantId: string,
  afterSalesId: string,
  payload: AfterSalesShipmentPayload
): Promise<OmsAllocationResponse> => {
  if (useMocks) {
    return { status: 'ALLOCATED', shipments: [], shortages: [] };
  }
  return requestJson<OmsAllocationResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/${afterSalesId}/resend`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Idempotency-Key': createIdempotencyKey('as-resend'),
        ...buildAuthHeaders()
      },
      body: JSON.stringify({
        origin_shipment_id: payload.origin_shipment_id ?? null,
        carrier_code: payload.carrier_code ?? null
      })
    },
    '生成补发包裹失败'
  );
};

export const requestAfterSalesRefund = async (
  tenantId: string,
  afterSalesId: string,
  payload: AfterSalesRefundPayload
): Promise<RefundDoc> => {
  if (useMocks) {
    return getMockRefundDoc(afterSalesId, payload);
  }
  return requestJson<RefundDoc>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/${afterSalesId}/refund`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Idempotency-Key': createIdempotencyKey('as-refund'),
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '发起退款失败'
  );
};

export const listRefundDocs = async (
  tenantId: string,
  query: RefundDocListQuery
): Promise<RefundDocListResponse> => {
  if (useMocks) {
    return getMockRefundDocs(query);
  }
  return requestJson<RefundDocListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/refund-docs${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取退款列表失败'
  );
};

export const createAfterSalesImportJob = async (
  tenantId: string,
  file: File
): Promise<AfterSalesImportJob> => {
  if (useMocks) {
    return getMockAfterSalesImportJob(file.name);
  }
  const formData = new FormData();
  formData.append('file', file);
  return requestJson<AfterSalesImportJob>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/import`,
    {
      method: 'POST',
      headers: {
        'Idempotency-Key': createIdempotencyKey('as-import'),
        ...buildAuthHeaders()
      },
      body: formData
    },
    '售后导入失败'
  );
};

export const listAfterSalesImportJobs = async (
  tenantId: string,
  query: AfterSalesImportJobListQuery
): Promise<AfterSalesImportJobListResponse> => {
  if (useMocks) {
    return getMockAfterSalesImportJobs(query);
  }
  return requestJson<AfterSalesImportJobListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/import-jobs${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取导入任务失败'
  );
};
