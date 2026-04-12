import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders } from '@/api/client';
import { ApiError, createApiError, handleApiError } from '@/api/errors';
import { createMockWarehouse, getMockWarehouseList, setMockDefaultWarehouse } from '@/api/mocks';

export type Warehouse = {
  warehouse_id: string;
  tenant_id: string;
  name: string;
  code: string;
  is_default: boolean;
  status?: string | null;
  created_at: string;
  updated_at: string;
};

export type WarehouseListQuery = {
  keyword?: string;
  status?: string;
  offset?: number;
  limit?: number;
};

export type WarehouseListResponse = {
  items: Warehouse[];
  total: number;
};

export type WarehouseCreatePayload = {
  name: string;
  code: string;
  status?: string;
  is_default?: boolean;
};

export type WarehouseUpdatePayload = {
  name?: string;
  code?: string;
  status?: string;
  is_default?: boolean;
};

const WAREHOUSE_REQUEST_TIMEOUT_MS = 15000;

const buildQueryString = (query: WarehouseListQuery): string => {
  const params = new URLSearchParams();
  if (query.keyword) {
    params.set('keyword', query.keyword);
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

const requestJson = async <T>(input: RequestInfo, init: RequestInit, message: string) => {
  const controller = new AbortController();
  const timeoutId = globalThis.setTimeout(
    () => controller.abort(),
    WAREHOUSE_REQUEST_TIMEOUT_MS
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
    throw createApiError(isTimeout ? '请求超时，请重试' : message, status);
  } finally {
    globalThis.clearTimeout(timeoutId);
  }
};

export const listWarehouses = async (
  tenantId: string,
  query: WarehouseListQuery
): Promise<WarehouseListResponse> => {
  if (useMocks) {
    return getMockWarehouseList(query);
  }
  const response = await requestJson<WarehouseListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/warehouses${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取仓库列表失败'
  );
  const offset = query.offset ?? 0;
  const limit = query.limit ?? response.items.length;
  return {
    items: response.items,
    total:
      response.total ??
      offset + response.items.length + (response.items.length === limit ? limit : 0)
  };
};

export const createWarehouse = async (
  tenantId: string,
  payload: WarehouseCreatePayload
): Promise<Warehouse> => {
  if (useMocks) {
    return createMockWarehouse(payload);
  }
  return requestJson<Warehouse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/warehouses`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建仓库失败'
  );
};

export const updateWarehouse = async (
  tenantId: string,
  warehouseId: string,
  payload: WarehouseUpdatePayload
): Promise<Warehouse> => {
  if (useMocks) {
    const created = createMockWarehouse({ name: payload.name ?? 'Warehouse', code: 'WH' });
    return { ...created, ...payload, warehouse_id: warehouseId };
  }
  return requestJson<Warehouse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/warehouses/${warehouseId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新仓库失败'
  );
};

export const deleteWarehouse = async (
  tenantId: string,
  warehouseId: string
): Promise<Warehouse> => {
  return requestJson<Warehouse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/warehouses/${warehouseId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除仓库失败'
  );
};

export const setDefaultWarehouse = async (
  tenantId: string,
  warehouseId: string
): Promise<Warehouse> => {
  if (useMocks) {
    const updated = setMockDefaultWarehouse(warehouseId);
    if (!updated) {
      throw createApiError('仓库不存在', 404);
    }
    return updated;
  }
  return requestJson<Warehouse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/warehouses/${warehouseId}/default`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '设置默认仓库失败'
  );
};
