import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import { getMockLogisticsShipmentList, getMockShipmentTracking } from '@/api/mocks';

export type LogisticsShipmentSummary = {
  shipment_id: string;
  carrier_code?: string | null;
  tracking_no?: string | null;
  status: string;
  weight_gram?: number | null;
  updated_at?: string | null;
};

export type LogisticsShipmentListQuery = {
  tracking_no?: string;
  offset?: number;
  limit?: number;
};

export type LogisticsShipmentListResponse = {
  items: LogisticsShipmentSummary[];
  total: number;
};

export type ShipmentTrackingResponse = {
  shipment_id: string;
  carrier_code?: string | null;
  tracking_no?: string | null;
  latest_status: string;
  latest_description?: string | null;
  latest_at?: string | null;
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

const buildQueryString = (query: LogisticsShipmentListQuery): string => {
  const params = new URLSearchParams();
  if (query.tracking_no) {
    params.set('tracking_no', query.tracking_no);
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

export const listLogisticsShipments = async (
  tenantId: string,
  query: LogisticsShipmentListQuery
): Promise<LogisticsShipmentListResponse> => {
  if (useMocks) {
    return getMockLogisticsShipmentList(query);
  }
  return requestJson<LogisticsShipmentListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/shipments${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取物流信息失败'
  );
};

export const getShipmentTracking = async (
  tenantId: string,
  shipmentId: string
): Promise<ShipmentTrackingResponse> => {
  if (useMocks) {
    return getMockShipmentTracking(shipmentId);
  }
  return requestJson<ShipmentTrackingResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/shipments/${shipmentId}/tracking`,
    {
      headers: buildAuthHeaders()
    },
    '获取物流轨迹失败'
  );
};
