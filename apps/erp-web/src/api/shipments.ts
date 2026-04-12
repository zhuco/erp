import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import { getMockShipmentList, requestMockShipment, retryMockShipment } from '@/api/mocks';
import type { PlatformCode } from '@/features/platform';

export type ShipmentStatus = 'REQUESTED' | 'SUCCESS' | 'FAILED';
export type ShipmentPushStatus = 'SUCCESS' | 'FAILED' | 'RETRYING' | 'DEAD';

export type ShipmentSummary = {
  shipment_id: string;
  order_id: string;
  platform: PlatformCode;
  platform_order_id?: string | null;
  shop_id: string;
  warehouse_id: string;
  carrier_code: string;
  tracking_no?: string | null;
  status: ShipmentStatus;
  weight_gram?: number | null;
  sender_address_id?: string | null;
  requested_at?: string | null;
  shipped_at?: string | null;
  updated_at?: string | null;
  last_error?: string | null;
  push_status?: ShipmentPushStatus | null;
  push_error_message?: string | null;
  push_attempted_at?: string | null;
};

export type ShipmentPackage = {
  package_id: string;
  shipment_id: string;
  package_no: string;
  carrier_code?: string | null;
  tracking_no?: string | null;
  weight_gram?: number | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export type ShipmentConfirmResponse = {
  status: string;
};

export type ShipmentPushAttempt = {
  attempt_id: string;
  outbox_id: string;
  shipment_id: string;
  shop_id: string;
  platform: PlatformCode | string;
  platform_order_id: string;
  carrier_code?: string | null;
  tracking_no?: string | null;
  shipped_at?: string | null;
  status: string;
  attempt: number;
  duration_ms: number;
  error_code: string;
  error_message: string;
  request_json: Record<string, unknown>;
  response_json: Record<string, unknown>;
  next_retry_at?: string | null;
  dead_letter_id?: string | null;
  trigger: string;
  created_at: string;
};

export type ShipmentPushAttemptListQuery = {
  status?: string;
  platform_order_id?: string;
  cursor?: string;
  limit?: number;
};

export type ShipmentPushAttemptListResponse = {
  data: ShipmentPushAttempt[];
  next_cursor?: string | null;
};

export type ShipmentPackageListResponse = {
  items: ShipmentPackage[];
};

export type ShipmentListResponse = {
  items: ShipmentSummary[];
  total: number;
};

export type ShipmentListQuery = {
  platform?: PlatformCode;
  shop_id?: string;
  status?: string;
  updated_from?: string;
  updated_to?: string;
  platform_order_id?: string;
  offset?: number;
  limit?: number;
};

export type ShipOrderPayload = {
  warehouse_id: string;
  carrier_code: string;
  tracking_no?: string;
  weight_gram?: number | null;
  idempotency_key: string;
};

export type ShipmentPackageUpsertPayload = {
  packages: Array<{
    package_no: string;
    carrier_code?: string;
    tracking_no?: string;
    weight_gram?: number | null;
  }>;
};

type ShipmentApi = {
  shipment_id: string;
  order_id: string;
  shop_id: string;
  platform: PlatformCode;
  warehouse_id: string;
  carrier_code: string;
  tracking_no?: string | null;
  status: ShipmentStatus;
  weight_gram?: number | null;
  sender_address_id?: string | null;
  created_at?: string;
  updated_at?: string;
  shipped_at?: string | null;
  platform_order_id?: string | null;
  last_error?: string | null;
  push_status?: ShipmentPushStatus | null;
  push_error_message?: string | null;
  push_attempted_at?: string | null;
};

type ShipmentListApiResponse = {
  data: ShipmentApi[];
};

const buildQueryString = (query: ShipmentListQuery): string => {
  const params = new URLSearchParams();
  if (query.platform) {
    params.set('platform', query.platform);
  }
  if (query.shop_id) {
    params.set('shop_id', query.shop_id);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.updated_from) {
    params.set('updated_from', query.updated_from);
  }
  if (query.updated_to) {
    params.set('updated_to', query.updated_to);
  }
  if (query.platform_order_id) {
    params.set('platform_order_id', query.platform_order_id);
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  if (query.offset != null) {
    params.set('offset', String(query.offset));
  }
  const serialized = params.toString();
  return serialized ? `?${serialized}` : '';
};

const buildPushAttemptQueryString = (query: ShipmentPushAttemptListQuery): string => {
  const params = new URLSearchParams();
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.platform_order_id) {
    params.set('platform_order_id', query.platform_order_id);
  }
  if (query.cursor) {
    params.set('cursor', query.cursor);
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  const serialized = params.toString();
  return serialized ? `?${serialized}` : '';
};

const normalizeShipment = (item: ShipmentApi): ShipmentSummary => ({
  shipment_id: item.shipment_id,
  order_id: item.order_id,
  platform: item.platform,
  platform_order_id: item.platform_order_id ?? null,
  shop_id: item.shop_id,
  warehouse_id: item.warehouse_id,
  carrier_code: item.carrier_code,
  tracking_no: item.tracking_no ?? null,
  status: item.status,
  weight_gram: item.weight_gram ?? null,
  sender_address_id: item.sender_address_id ?? null,
  requested_at: item.created_at ?? null,
  shipped_at: item.shipped_at ?? null,
  updated_at: item.updated_at ?? null,
  last_error: item.last_error ?? null,
  push_status: item.push_status ?? null,
  push_error_message: item.push_error_message ?? null,
  push_attempted_at: item.push_attempted_at ?? null
});

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

export const createIdempotencyKey = (): string => {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return crypto.randomUUID();
  }
  return `ship-${Date.now()}-${Math.random().toString(16).slice(2)}`;
};

export const listShipments = async (
  tenantId: string,
  query: ShipmentListQuery
): Promise<ShipmentListResponse> => {
  if (useMocks) {
    return getMockShipmentList(query);
  }
  const response = await requestJson<ShipmentListApiResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/shipments${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取发货列表失败'
  );
  const items = response.data.map(normalizeShipment);
  const offset = query.offset ?? 0;
  const limit = query.limit ?? items.length;
  const total = offset + items.length + (items.length === limit ? limit : 0);
  return { items, total };
};

export const shipOrder = async (
  tenantId: string,
  orderId: string,
  payload: ShipOrderPayload
): Promise<ShipmentSummary> => {
  if (useMocks) {
    return requestMockShipment(orderId, payload);
  }
  const response = await requestJson<ShipmentApi>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/orders/${orderId}/ship`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Idempotency-Key': payload.idempotency_key,
        ...buildAuthHeaders()
      },
      body: JSON.stringify({
        warehouse_id: payload.warehouse_id,
        carrier_code: payload.carrier_code,
        tracking_no: payload.tracking_no ?? null,
        weight_gram: payload.weight_gram ?? null,
        print_label: false
      })
    },
    '提交发货失败'
  );
  return normalizeShipment(response);
};

export const confirmShipment = async (
  tenantId: string,
  shipmentId: string
): Promise<ShipmentConfirmResponse> => {
  if (useMocks) {
    return { status: 'queued' };
  }
  return requestJson<ShipmentConfirmResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/shipments/${shipmentId}/confirm`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      }
    },
    '发货回写失败'
  );
};

export const retryShipment = async (
  tenantId: string,
  shipment: ShipmentSummary
): Promise<ShipmentSummary> => {
  if (useMocks) {
    return retryMockShipment(shipment);
  }
  return shipOrder(tenantId, shipment.order_id, {
    warehouse_id: shipment.warehouse_id,
    carrier_code: shipment.carrier_code,
    tracking_no: shipment.tracking_no ?? undefined,
    weight_gram: shipment.weight_gram ?? undefined,
    idempotency_key: createIdempotencyKey()
  });
};

export const listShipmentPackages = async (
  tenantId: string,
  shipmentId: string
): Promise<ShipmentPackageListResponse> => {
  if (useMocks) {
    return { items: [] };
  }
  return requestJson<ShipmentPackageListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/shipments/${shipmentId}/packages`,
    {
      headers: buildAuthHeaders()
    },
    '获取包裹列表失败'
  );
};

export const upsertShipmentPackages = async (
  tenantId: string,
  shipmentId: string,
  payload: ShipmentPackageUpsertPayload
): Promise<ShipmentPackageListResponse> => {
  if (useMocks) {
    return { items: [] };
  }
  return requestJson<ShipmentPackageListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/shipments/${shipmentId}/packages`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新包裹信息失败'
  );
};

export const listShipmentPushAttempts = async (
  tenantId: string,
  query: ShipmentPushAttemptListQuery
): Promise<ShipmentPushAttemptListResponse> => {
  if (useMocks) {
    return { data: [], next_cursor: null };
  }
  return requestJson<ShipmentPushAttemptListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/shipment-push-attempts${buildPushAttemptQueryString(
      query
    )}`,
    {
      headers: buildAuthHeaders()
    },
    '获取回写记录失败'
  );
};
