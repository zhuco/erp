import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type LocationBalance = {
  warehouse_id: string;
  location_id: string;
  location_code?: string | null;
  sku_id: string;
  sku_code?: string | null;
  on_hand: number;
  updated_at?: string | null;
};

export type LocationBalanceListQuery = {
  warehouse_id?: string;
  location_id?: string;
  sku_id?: string;
  offset?: number;
  limit?: number;
};

export type LocationBalanceListResponse = {
  items: LocationBalance[];
  total: number;
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

const buildQueryString = (query: LocationBalanceListQuery): string => {
  const params = new URLSearchParams();
  if (query.warehouse_id) {
    params.set('warehouse_id', query.warehouse_id);
  }
  if (query.location_id) {
    params.set('location_id', query.location_id);
  }
  if (query.sku_id) {
    params.set('sku_id', query.sku_id);
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

export const listLocationBalances = async (
  tenantId: string,
  query: LocationBalanceListQuery
): Promise<LocationBalanceListResponse> => {
  if (useMocks) {
    return { items: [], total: 0 };
  }
  return requestJson<LocationBalanceListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inventory/location-balances${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取货位库存失败'
  );
};
