import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders } from '@/api/client';
import { ApiError, createApiError, handleApiError } from '@/api/errors';

export type StockTransferSummary = {
  transfer_id: string;
  from_warehouse_id: string;
  to_warehouse_id: string;
  status: string;
  created_at: string;
  updated_at: string;
};

export type StockTransferItem = {
  transfer_item_id: string;
  sku_id: string;
  qty: number;
};

export type StockTransferDetail = {
  transfer: StockTransferSummary;
  items: StockTransferItem[];
};

export type StockTransferListQuery = {
  warehouse_id?: string;
  status?: string;
  offset?: number;
  limit?: number;
};

export type StockTransferListResponse = {
  items: StockTransferSummary[];
  total: number;
};

export type StockTransferItemCreate = {
  sku_id: string;
  qty: number;
};

export type StockTransferCreatePayload = {
  from_warehouse_id: string;
  to_warehouse_id: string;
  items: StockTransferItemCreate[];
};

const STOCK_TRANSFER_REQUEST_TIMEOUT_MS = 15000;

const requestJson = async <T>(input: RequestInfo, init: RequestInit, message: string) => {
  const controller = new AbortController();
  const timeoutId = globalThis.setTimeout(
    () => controller.abort(),
    STOCK_TRANSFER_REQUEST_TIMEOUT_MS
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

const buildQueryString = (query: StockTransferListQuery): string => {
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

export const listStockTransfers = async (
  tenantId: string,
  query: StockTransferListQuery
): Promise<StockTransferListResponse> => {
  if (useMocks) {
    return { items: [], total: 0 };
  }
  return requestJson<StockTransferListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stock-transfers${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取调拨单失败'
  );
};

export const createStockTransfer = async (
  tenantId: string,
  payload: StockTransferCreatePayload
): Promise<StockTransferDetail> => {
  if (useMocks) {
    throw createApiError('Mock未实现', 501);
  }
  return requestJson<StockTransferDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stock-transfers`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建调拨单失败'
  );
};

export const getStockTransfer = async (
  tenantId: string,
  transferId: string
): Promise<StockTransferDetail> => {
  if (useMocks) {
    throw createApiError('Mock未实现', 501);
  }
  return requestJson<StockTransferDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stock-transfers/${transferId}`,
    {
      headers: buildAuthHeaders()
    },
    '获取调拨单失败'
  );
};

export const confirmStockTransferOutbound = async (
  tenantId: string,
  transferId: string
): Promise<StockTransferDetail> => {
  if (useMocks) {
    throw createApiError('Mock未实现', 501);
  }
  return requestJson<StockTransferDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stock-transfers/${transferId}/confirm-outbound`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '调拨出库确认失败'
  );
};

export const confirmStockTransferInbound = async (
  tenantId: string,
  transferId: string
): Promise<StockTransferDetail> => {
  if (useMocks) {
    throw createApiError('Mock未实现', 501);
  }
  return requestJson<StockTransferDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stock-transfers/${transferId}/confirm-inbound`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '调拨入库确认失败'
  );
};
