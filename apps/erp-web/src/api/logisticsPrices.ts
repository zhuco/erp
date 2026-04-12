import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type LogisticsPrice = {
  price_id: string;
  tenant_id: string;
  carrier_code: string;
  product_id?: string | null;
  region_code: string;
  weight_min_kg: number;
  weight_max_kg: number;
  price_cent: number;
  created_at: string;
  updated_at: string;
};

export type LogisticsPriceListQuery = {
  carrier_code?: string;
  product_id?: string;
  region_code?: string;
  weight_kg?: number;
  offset?: number;
  limit?: number;
};

export type LogisticsPriceListResponse = {
  items: LogisticsPrice[];
  total: number;
};

export type LogisticsPriceCreatePayload = {
  carrier_code: string;
  product_id?: string | null;
  region_code: string;
  weight_min_kg: number;
  weight_max_kg: number;
  price_cent: number;
};

export type LogisticsPriceUpdatePayload = {
  carrier_code?: string;
  product_id?: string | null;
  region_code?: string;
  weight_min_kg?: number;
  weight_max_kg?: number;
  price_cent?: number;
};

export type LogisticsPriceEstimateRequest = {
  shipment_id?: string;
  order_id?: string;
  carrier_code?: string;
  product_id?: string | null;
  region_code?: string;
  weight_kg?: number;
};

export type LogisticsPriceEstimateResponse = {
  matched: boolean;
  carrier_code: string;
  product_id?: string | null;
  region_code: string;
  weight_kg: number;
  price?: LogisticsPrice | null;
};

const requestJson = async <T>(input: RequestInfo, init: RequestInit, message: string) => {
  const response = await fetch(input, init);
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

const buildQueryString = (query: LogisticsPriceListQuery): string => {
  const params = new URLSearchParams();
  if (query.carrier_code) {
    params.set('carrier_code', query.carrier_code);
  }
  if (query.product_id) {
    params.set('product_id', query.product_id);
  }
  if (query.region_code) {
    params.set('region_code', query.region_code);
  }
  if (query.weight_kg != null) {
    params.set('weight_kg', String(query.weight_kg));
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

export const listLogisticsPrices = async (
  tenantId: string,
  query: LogisticsPriceListQuery
): Promise<LogisticsPriceListResponse> => {
  if (useMocks) {
    return { items: [], total: 0 };
  }
  return requestJson<LogisticsPriceListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics-prices${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取物流价格失败'
  );
};

export const createLogisticsPrice = async (
  tenantId: string,
  payload: LogisticsPriceCreatePayload
): Promise<LogisticsPrice> => {
  if (useMocks) {
    return {
      price_id: `${Date.now()}`,
      tenant_id: tenantId,
      carrier_code: payload.carrier_code,
      product_id: payload.product_id ?? null,
      region_code: payload.region_code,
      weight_min_kg: payload.weight_min_kg,
      weight_max_kg: payload.weight_max_kg,
      price_cent: payload.price_cent,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<LogisticsPrice>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics-prices`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建物流价格失败'
  );
};

export const updateLogisticsPrice = async (
  tenantId: string,
  priceId: string,
  payload: LogisticsPriceUpdatePayload
): Promise<LogisticsPrice> => {
  if (useMocks) {
    return {
      price_id: priceId,
      tenant_id: tenantId,
      carrier_code: payload.carrier_code ?? 'mock',
      product_id: payload.product_id ?? null,
      region_code: payload.region_code ?? 'mock',
      weight_min_kg: payload.weight_min_kg ?? 0,
      weight_max_kg: payload.weight_max_kg ?? 0,
      price_cent: payload.price_cent ?? 0,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<LogisticsPrice>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics-prices/${priceId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新物流价格失败'
  );
};

export const deleteLogisticsPrice = async (
  tenantId: string,
  priceId: string
): Promise<LogisticsPrice> => {
  if (useMocks) {
    return {
      price_id: priceId,
      tenant_id: tenantId,
      carrier_code: 'mock',
      product_id: null,
      region_code: 'mock',
      weight_min_kg: 0,
      weight_max_kg: 0,
      price_cent: 0,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<LogisticsPrice>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics-prices/${priceId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除物流价格失败'
  );
};

export const estimateLogisticsPrice = async (
  tenantId: string,
  payload: LogisticsPriceEstimateRequest
): Promise<LogisticsPriceEstimateResponse> => {
  if (useMocks) {
    return {
      matched: false,
      carrier_code: payload.carrier_code ?? 'mock',
      product_id: payload.product_id ?? null,
      region_code: payload.region_code ?? 'mock',
      weight_kg: payload.weight_kg ?? 0,
      price: null
    };
  }
  return requestJson<LogisticsPriceEstimateResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics-prices/estimate`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '运费测算失败'
  );
};
