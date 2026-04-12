import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type ShipmentPushAttemptStatus = 'SUCCESS' | 'FAILED' | 'RETRYING' | 'DEAD';

export type ShipmentPushAttempt = {
  attempt_id: string;
  outbox_id: string;
  shipment_id: string;
  shop_id: string;
  platform: string;
  platform_order_id: string;
  carrier_code?: string | null;
  tracking_no?: string | null;
  shipped_at?: string | null;
  status: ShipmentPushAttemptStatus;
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
  status?: ShipmentPushAttemptStatus;
  platform_order_id?: string;
  cursor?: string;
  limit?: number;
};

export type CursorListResponse<T> = {
  data: T[];
  next_cursor?: string | null;
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

const buildQueryString = (query: ShipmentPushAttemptListQuery): string => {
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

export const listShipmentPushAttempts = async (
  tenantId: string,
  query: ShipmentPushAttemptListQuery
): Promise<CursorListResponse<ShipmentPushAttempt>> => {
  if (useMocks) {
    return { data: [], next_cursor: null };
  }
  return requestJson<CursorListResponse<ShipmentPushAttempt>>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/shipment-push-attempts${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取发货回传失败记录失败'
  );
};
