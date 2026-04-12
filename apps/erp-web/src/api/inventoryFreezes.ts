import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders } from '@/api/client';
import { ApiError, createApiError, handleApiError } from '@/api/errors';

export type InventoryFreeze = {
  freeze_id: string;
  warehouse_id: string;
  sku_id: string;
  qty: number;
  reason?: string | null;
  status: string;
  created_at: string;
  updated_at: string;
};

export type InventoryFreezeListQuery = {
  warehouse_id?: string;
  sku_id?: string;
  status?: string;
  offset?: number;
  limit?: number;
};

export type InventoryFreezeListResponse = {
  items: InventoryFreeze[];
  total: number;
};

export type InventoryFreezeCreatePayload = {
  warehouse_id: string;
  sku_id: string;
  qty: number;
  reason?: string;
};

const FREEZE_REQUEST_TIMEOUT_MS = 15000;

const requestJson = async <T>(input: RequestInfo, init: RequestInit, message: string) => {
  const controller = new AbortController();
  const timeoutId = globalThis.setTimeout(
    () => controller.abort(),
    FREEZE_REQUEST_TIMEOUT_MS
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

const buildQueryString = (query: InventoryFreezeListQuery): string => {
  const params = new URLSearchParams();
  if (query.warehouse_id) {
    params.set('warehouse_id', query.warehouse_id);
  }
  if (query.sku_id) {
    params.set('sku_id', query.sku_id);
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

export const listInventoryFreezes = async (
  tenantId: string,
  query: InventoryFreezeListQuery
): Promise<InventoryFreezeListResponse> => {
  if (useMocks) {
    return { items: [], total: 0 };
  }
  return requestJson<InventoryFreezeListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inventory/freezes${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取库存冻结失败'
  );
};

export const createInventoryFreeze = async (
  tenantId: string,
  payload: InventoryFreezeCreatePayload
): Promise<InventoryFreeze> => {
  if (useMocks) {
    throw createApiError('Mock未实现', 501);
  }
  return requestJson<InventoryFreeze>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inventory/freezes`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建冻结失败'
  );
};

export const cancelInventoryFreeze = async (
  tenantId: string,
  freezeId: string
): Promise<InventoryFreeze> => {
  if (useMocks) {
    throw createApiError('Mock未实现', 501);
  }
  return requestJson<InventoryFreeze>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inventory/freezes/${freezeId}/cancel`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '取消冻结失败'
  );
};
