import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders } from '@/api/client';
import { ApiError, createApiError, handleApiError } from '@/api/errors';

export type StockAdjustSummary = {
  adjust_id: string;
  warehouse_id: string;
  reason: string;
  status: string;
  created_at: string;
  updated_at: string;
};

export type StockAdjustItem = {
  adjust_item_id: string;
  sku_id: string;
  qty: number;
};

export type StockAdjustDetail = {
  doc: StockAdjustSummary;
  items: StockAdjustItem[];
};

export type StockAdjustListQuery = {
  warehouse_id?: string;
  status?: string;
  reason?: string;
  offset?: number;
  limit?: number;
};

export type StockAdjustListResponse = {
  items: StockAdjustSummary[];
  total: number;
};

export type StockAdjustItemCreate = {
  sku_id: string;
  qty: number;
};

export type StockAdjustCreatePayload = {
  warehouse_id: string;
  reason: string;
  items: StockAdjustItemCreate[];
};

const STOCK_ADJUST_REQUEST_TIMEOUT_MS = 15000;

const requestJson = async <T>(input: RequestInfo, init: RequestInit, message: string) => {
  const controller = new AbortController();
  const timeoutId = globalThis.setTimeout(
    () => controller.abort(),
    STOCK_ADJUST_REQUEST_TIMEOUT_MS
  );
  try {
    const response = await fetch(input, { ...init, signal: controller.signal });
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
  } catch (error) {
    if (error instanceof ApiError) {
      throw error;
    }
    const isTimeout = error instanceof DOMException && error.name === 'AbortError';
    const status = isTimeout ? 408 : undefined;
    handleApiError(status);
    throw createApiError(isTimeout ? '请求超时，请稍后重试' : message, status);
  } finally {
    globalThis.clearTimeout(timeoutId);
  }
};

const buildQueryString = (query: StockAdjustListQuery): string => {
  const params = new URLSearchParams();
  if (query.warehouse_id) {
    params.set('warehouse_id', query.warehouse_id);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.reason) {
    params.set('reason', query.reason);
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

export const listStockAdjustments = async (
  tenantId: string,
  query: StockAdjustListQuery
): Promise<StockAdjustListResponse> => {
  if (useMocks) {
    return { items: [], total: 0 };
  }
  return requestJson<StockAdjustListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stock-adjustments${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取调整单失败'
  );
};

export const createStockAdjustment = async (
  tenantId: string,
  payload: StockAdjustCreatePayload
): Promise<StockAdjustDetail> => {
  if (useMocks) {
    throw createApiError('Mock未实现', 501);
  }
  return requestJson<StockAdjustDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stock-adjustments`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建调整单失败'
  );
};

export const getStockAdjustment = async (
  tenantId: string,
  adjustId: string
): Promise<StockAdjustDetail> => {
  if (useMocks) {
    throw createApiError('Mock未实现', 501);
  }
  return requestJson<StockAdjustDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stock-adjustments/${adjustId}`,
    {
      headers: buildAuthHeaders()
    },
    '获取调整单失败'
  );
};

export const applyStockAdjustment = async (
  tenantId: string,
  adjustId: string
): Promise<StockAdjustDetail> => {
  if (useMocks) {
    throw createApiError('Mock未实现', 501);
  }
  return requestJson<StockAdjustDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stock-adjustments/${adjustId}/apply`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '应用调整单失败'
  );
};
