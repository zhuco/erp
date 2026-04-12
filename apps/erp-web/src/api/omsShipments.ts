import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import type { PlatformCode } from '@/features/platform';

export type OmsShipmentStatus =
  | 'CREATED'
  | 'SHIPPED'
  | 'PARTIALLY_SHIPPED'
  | 'CANCELLED'
  | 'ON_HOLD';

export type OmsShipmentItem = {
  order_item_id: string;
  sku_id: string;
  sku_code?: string | null;
  qty: number;
};

export type OmsShipmentSummary = {
  shipment_id: string;
  shipment_no?: string | null;
  shipment_type?: string;
  origin_shipment_id?: string | null;
  tenant_id: string;
  order_id: string;
  shop_id: string;
  platform: PlatformCode | string;
  platform_order_id: string;
  warehouse_id: string;
  carrier_code?: string | null;
  tracking_no?: string | null;
  status: OmsShipmentStatus | string;
  weight_gram?: number | null;
  shipped_at?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
  items?: OmsShipmentItem[] | null;
};

export type OmsShipmentListResponse = {
  items: OmsShipmentSummary[];
  total: number;
};

export type OmsShipmentListQuery = {
  order_id?: string;
  warehouse_id?: string;
  status?: string;
  offset?: number;
  limit?: number;
};

export type OmsAllocationShortage = {
  order_item_id: string;
  sku_id: string;
  requested: number;
  available: number;
};

export type OmsAllocationRequest = {
  carrier_code?: string;
  idempotency_key?: string;
};

export type OmsResendRequest = {
  origin_shipment_id?: string;
  carrier_code?: string;
  idempotency_key?: string;
};

export type OmsReleaseResponse = {
  released: number;
};

export type OmsShipmentCancelRequest = {
  reason?: string;
  idempotency_key?: string;
};

export type OmsShipmentCancelResponse = {
  cancel_request_id: string;
  status: string;
};

export type OmsAllocationResponse = {
  status: string;
  shipments: OmsShipmentSummary[];
  shortages: OmsAllocationShortage[];
};

export type OmsShipRequest = {
  carrier_code?: string;
  tracking_no?: string;
  shipped_at?: string;
  weight_gram?: number;
  idempotency_key?: string;
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

const buildQueryString = (query: OmsShipmentListQuery): string => {
  const params = new URLSearchParams();
  if (query.order_id) {
    params.set('order_id', query.order_id);
  }
  if (query.warehouse_id) {
    params.set('warehouse_id', query.warehouse_id);
  }
  if (query.status) {
    params.set('status', query.status);
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

export const createOmsIdempotencyKey = (): string => {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return crypto.randomUUID();
  }
  return `oms-${Date.now()}-${Math.random().toString(16).slice(2)}`;
};

export const listOmsShipments = async (
  tenantId: string,
  query: OmsShipmentListQuery
): Promise<OmsShipmentListResponse> => {
  if (useMocks) {
    return { items: [], total: 0 };
  }
  return requestJson<OmsShipmentListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/oms/shipments${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取包裹列表失败'
  );
};

export const listOmsOrderShipments = async (
  tenantId: string,
  orderId: string
): Promise<OmsShipmentListResponse> => {
  if (useMocks) {
    return { items: [], total: 0 };
  }
  return requestJson<OmsShipmentListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/oms/orders/${orderId}/shipments`,
    {
      headers: buildAuthHeaders()
    },
    '获取订单包裹失败'
  );
};

export const getOmsShipment = async (
  tenantId: string,
  shipmentId: string
): Promise<OmsShipmentSummary> => {
  if (useMocks) {
    return {
      shipment_id: shipmentId,
      tenant_id: tenantId,
      order_id: '',
      shop_id: '',
      platform: 'taobao',
      platform_order_id: '',
      warehouse_id: '',
      status: 'CREATED'
    };
  }
  return requestJson<OmsShipmentSummary>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/oms/shipments/${shipmentId}`,
    {
      headers: buildAuthHeaders()
    },
    '获取包裹详情失败'
  );
};

export const allocateOmsOrder = async (
  tenantId: string,
  orderId: string,
  payload: OmsAllocationRequest
): Promise<OmsAllocationResponse> => {
  if (useMocks) {
    return { status: 'ALLOCATED', shipments: [], shortages: [] };
  }
  return requestJson<OmsAllocationResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/oms/orders/${orderId}/allocate`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...(payload.idempotency_key ? { 'Idempotency-Key': payload.idempotency_key } : {}),
        ...buildAuthHeaders()
      },
      body: JSON.stringify({ carrier_code: payload.carrier_code ?? null })
    },
    '分配库存失败'
  );
};

export const createOmsShipments = async (
  tenantId: string,
  orderId: string,
  payload: OmsAllocationRequest
): Promise<OmsAllocationResponse> => {
  if (useMocks) {
    return { status: 'ALLOCATED', shipments: [], shortages: [] };
  }
  return requestJson<OmsAllocationResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/oms/orders/${orderId}/shipments`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...(payload.idempotency_key ? { 'Idempotency-Key': payload.idempotency_key } : {}),
        ...buildAuthHeaders()
      },
      body: JSON.stringify({ carrier_code: payload.carrier_code ?? null })
    },
    '创建包裹失败'
  );
};

export const shipOmsShipment = async (
  tenantId: string,
  shipmentId: string,
  payload: OmsShipRequest
): Promise<OmsShipmentSummary> => {
  if (useMocks) {
    return {
      shipment_id: shipmentId,
      tenant_id: tenantId,
      order_id: '',
      shop_id: '',
      platform: 'taobao',
      platform_order_id: '',
      warehouse_id: '',
      status: 'SHIPPED',
      shipped_at: new Date().toISOString()
    };
  }
  const idempotencyKey = payload.idempotency_key ?? createOmsIdempotencyKey();
  return requestJson<OmsShipmentSummary>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/oms/shipments/${shipmentId}/ship`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Idempotency-Key': idempotencyKey,
        ...buildAuthHeaders()
      },
      body: JSON.stringify({
        carrier_code: payload.carrier_code ?? null,
        tracking_no: payload.tracking_no ?? null,
        shipped_at: payload.shipped_at ?? null,
        weight_gram: payload.weight_gram ?? null
      })
    },
    '包裹发货失败'
  );
};

export const resendOmsOrder = async (
  tenantId: string,
  orderId: string,
  payload: OmsResendRequest
): Promise<OmsAllocationResponse> => {
  if (useMocks) {
    return { status: 'ALLOCATED', shipments: [], shortages: [] };
  }
  const idempotencyKey = payload.idempotency_key ?? createOmsIdempotencyKey();
  return requestJson<OmsAllocationResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/oms/orders/${orderId}/resend`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Idempotency-Key': idempotencyKey,
        ...buildAuthHeaders()
      },
      body: JSON.stringify({
        origin_shipment_id: payload.origin_shipment_id ?? null,
        carrier_code: payload.carrier_code ?? null
      })
    },
    '补发包裹失败'
  );
};

export const releaseOmsOrder = async (
  tenantId: string,
  orderId: string,
  idempotencyKey?: string
): Promise<OmsReleaseResponse> => {
  if (useMocks) {
    return { released: 0 };
  }
  const resolvedKey = idempotencyKey ?? createOmsIdempotencyKey();
  return requestJson<OmsReleaseResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/oms/orders/${orderId}/release`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Idempotency-Key': resolvedKey,
        ...buildAuthHeaders()
      }
    },
    '释放库存占用失败'
  );
};

export const requestOmsShipmentCancel = async (
  tenantId: string,
  shipmentId: string,
  payload: OmsShipmentCancelRequest
): Promise<OmsShipmentCancelResponse> => {
  if (useMocks) {
    return { cancel_request_id: shipmentId, status: 'PENDING' };
  }
  const idempotencyKey = payload.idempotency_key ?? createOmsIdempotencyKey();
  return requestJson<OmsShipmentCancelResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/oms/shipments/${shipmentId}/cancel-request`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Idempotency-Key': idempotencyKey,
        ...buildAuthHeaders()
      },
      body: JSON.stringify({ reason: payload.reason ?? null })
    },
    '提交撤销申请失败'
  );
};
