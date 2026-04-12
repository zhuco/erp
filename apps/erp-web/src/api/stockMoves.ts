import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders } from '@/api/client';
import { ApiError, createApiError, handleApiError } from '@/api/errors';

export type StockMoveSummary = {
  move_id: string;
  warehouse_id: string;
  from_location_id?: string | null;
  to_location_id?: string | null;
  status: string;
  created_at: string;
  updated_at: string;
};

export type StockMoveItem = {
  move_item_id: string;
  sku_id: string;
  qty: number;
};

export type StockMoveDetail = {
  doc: StockMoveSummary;
  items: StockMoveItem[];
};

export type StockMoveListQuery = {
  warehouse_id?: string;
  status?: string;
  offset?: number;
  limit?: number;
};

export type StockMoveListResponse = {
  items: StockMoveSummary[];
  total: number;
};

export type StockMoveItemCreate = {
  sku_id: string;
  qty: number;
};

export type StockMoveCreatePayload = {
  warehouse_id: string;
  from_location_id?: string;
  to_location_id?: string;
  items: StockMoveItemCreate[];
};

const STOCK_MOVE_REQUEST_TIMEOUT_MS = 15000;

const requestJson = async <T>(input: RequestInfo, init: RequestInit, message: string) => {
  const controller = new AbortController();
  const timeoutId = globalThis.setTimeout(
    () => controller.abort(),
    STOCK_MOVE_REQUEST_TIMEOUT_MS
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

const buildQueryString = (query: StockMoveListQuery): string => {
  const params = new URLSearchParams();
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

export const listStockMoves = async (
  tenantId: string,
  query: StockMoveListQuery
): Promise<StockMoveListResponse> => {
  if (useMocks) {
    return { items: [], total: 0 };
  }
  return requestJson<StockMoveListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stock-moves${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取移库单失败'
  );
};

export const createStockMove = async (
  tenantId: string,
  payload: StockMoveCreatePayload
): Promise<StockMoveDetail> => {
  if (useMocks) {
    throw createApiError('Mock未实现', 501);
  }
  return requestJson<StockMoveDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stock-moves`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建移库单失败'
  );
};

export const getStockMove = async (
  tenantId: string,
  moveId: string
): Promise<StockMoveDetail> => {
  if (useMocks) {
    throw createApiError('Mock未实现', 501);
  }
  return requestJson<StockMoveDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stock-moves/${moveId}`,
    {
      headers: buildAuthHeaders()
    },
    '获取移库单失败'
  );
};

export const applyStockMove = async (
  tenantId: string,
  moveId: string
): Promise<StockMoveDetail> => {
  if (useMocks) {
    throw createApiError('Mock未实现', 501);
  }
  return requestJson<StockMoveDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stock-moves/${moveId}/apply`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '应用移库失败'
  );
};
