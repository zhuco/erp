import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders } from '@/api/client';
import { ApiError, createApiError, handleApiError, isErrorResponse } from '@/api/errors';
import { applyMockInventoryAction, getMockInventoryList } from '@/api/mocks';

import type { components } from './generated/schema';

export type InventorySnapshot = {
  id?: string;
  platform?: components['schemas']['Platform'];
  sku_id: string;
  sku_code?: string | null;
  barcode?: string | null;
  sku_name?: string | null;
  spec?: string | null;
  brand?: string | null;
  category?: string | null;
  warehouse_id?: string | null;
  area_id?: string | null;
  area_code?: string | null;
  location_id?: string | null;
  location_code?: string | null;
  cost_cent?: number | null;
  on_hand?: number | null;
  reserved: number;
  frozen?: number | null;
  in_transit?: number | null;
  available: number;
  virtual_qty?: number | null;
  display_qty?: number | null;
  batch_enabled?: boolean | null;
  serial_enabled?: boolean | null;
  updated_at?: string | null;
};
export type InventorySummary = {
  sku_id: string;
  sku_code?: string | null;
  barcode?: string | null;
  on_hand?: number | null;
  reserved: number;
  frozen?: number | null;
  in_transit?: number | null;
  available: number;
  updated_at?: string | null;
};
export type InventoryCostRecord = InventorySummary & {
  unit_cost_cent: number;
  total_cost_cent: number;
};
export type InventoryListResponse = {
  items: InventorySnapshot[];
  total: number;
};
export type InventorySummaryListResponse = {
  items: InventorySummary[];
  total: number;
};
export type InventoryCostListResponse = {
  items: InventoryCostRecord[];
  total: number;
};
export type InventoryListQuery = {
  platform?: components['schemas']['Platform'];
  warehouse_id?: string;
  area_id?: string;
  location_id?: string;
  sku_id?: string;
  sku_code?: string;
  barcode?: string;
  tag?: string;
  keyword?: string;
  brand?: string;
  category?: string;
  inventory_status?: string;
  spec_mode?: string;
  tree_key?: string;
  on_hand_negative?: boolean;
  available_below_safety?: boolean;
  include_zero?: boolean;
  cursor?: string;
  offset?: number;
  limit?: number;
};

export type InventoryActionKind = 'ADJUST' | 'RESERVE' | 'RELEASE';

export type InventoryActionPayload = {
  warehouse_id: string;
  sku_id: string;
  qty: number;
  order_id?: string | null;
  order_item_id?: string | null;
  ref_type?: string | null;
  ref_id?: string | null;
};

export type InventoryAtpAllocation = {
  batch_id: string;
  batch_no: string;
  expire_at?: string | null;
  allocated_qty: number;
};

export type InventoryAtpResponse = {
  warehouse_id: string;
  sku_id: string;
  on_hand: number;
  reserved: number;
  frozen: number;
  available: number;
  request_qty: number;
  atp_qty: number;
  shortage_qty: number;
  allocations: InventoryAtpAllocation[];
};

export type InventoryAtpQuery = {
  warehouse_id: string;
  sku_id: string;
  request_qty?: number;
};

export type InventoryReconciliationCheckPayload = {
  warehouse_id: string;
  sku_id: string;
  expected_on_hand: number;
  source_ref?: string | null;
};

export type InventoryReconciliationResult = {
  reconciliation_id: string;
  warehouse_id: string;
  sku_id: string;
  expected_on_hand: number;
  actual_on_hand: number;
  diff_qty: number;
  status: string;
  source_ref?: string | null;
  created_at: string;
};

const INVENTORY_REQUEST_TIMEOUT_MS = 15000;
const enableInventoryDiagnostics = import.meta.env.DEV || import.meta.env.MODE === 'test';

const logInventoryRequest = ({
  url,
  method,
  status,
  durationMs,
  errorSummary
}: {
  url: string;
  method: string;
  status?: number;
  durationMs: number;
  errorSummary?: string;
}) => {
  if (!enableInventoryDiagnostics) {
    return;
  }
  const statusLabel = status ?? 'NETWORK_ERROR';
  const summary = errorSummary ? ` error="${errorSummary}"` : '';
  console.info(
    `[inventory] ${method.toUpperCase()} ${url} status=${statusLabel} duration=${durationMs}ms${summary}`
  );
};

const buildQueryString = (query: InventoryListQuery): string => {
  const params = new URLSearchParams();
  if (query.platform) {
    params.set('platform', query.platform);
  }
  if (query.warehouse_id) {
    params.set('warehouse_id', query.warehouse_id);
  }
  if (query.area_id) {
    params.set('area_id', query.area_id);
  }
  if (query.location_id) {
    params.set('location_id', query.location_id);
  }
  if (query.sku_id) {
    params.set('sku_id', query.sku_id);
  }
  if (query.sku_code) {
    params.set('sku_code', query.sku_code);
  }
  if (query.barcode) {
    params.set('barcode', query.barcode);
  }
  if (query.tag) {
    params.set('tag', query.tag);
  }
  if (query.keyword) {
    params.set('keyword', query.keyword);
  }
  if (query.brand) {
    params.set('brand', query.brand);
  }
  if (query.category) {
    params.set('category', query.category);
  }
  if (query.inventory_status) {
    params.set('inventory_status', query.inventory_status);
  }
  if (query.spec_mode) {
    params.set('spec_mode', query.spec_mode);
  }
  if (query.tree_key) {
    params.set('tree_key', query.tree_key);
  }
  if (query.on_hand_negative) {
    params.set('on_hand_negative', 'true');
  }
  if (query.available_below_safety) {
    params.set('available_below_safety', 'true');
  }
  if (query.include_zero) {
    params.set('include_zero', 'true');
  }
  if (query.cursor) {
    params.set('cursor', query.cursor);
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

export type VirtualInventoryAdjustMode = 'SET' | 'DELTA';

export type VirtualInventoryAdjustPayload = {
  warehouse_id: string;
  sku_id: string;
  area_id?: string | null;
  location_id?: string | null;
  mode: VirtualInventoryAdjustMode;
  qty: number;
  reason?: string | null;
};

export type VirtualInventoryAdjustResponse = {
  warehouse_id: string;
  sku_id: string;
  area_id?: string | null;
  location_id?: string | null;
  virtual_qty: number;
  updated_at?: string | null;
};

const mockVirtualInventory = new Map<string, number>();

const buildVirtualInventoryKey = (
  warehouseId: string,
  skuId: string,
  locationId?: string | null
) => `${warehouseId}-${skuId}-${locationId ?? 'NA'}`;

const requestJson = async <T>(input: RequestInfo, init: RequestInit, message: string) => {
  const url = typeof input === 'string' ? input : input.toString();
  const method = init.method ?? 'GET';
  const start = Date.now();
  const controller = new AbortController();
  const timeoutId = globalThis.setTimeout(
    () => controller.abort(),
    INVENTORY_REQUEST_TIMEOUT_MS
  );

  try {
    const response = await fetch(input, { ...init, signal: controller.signal });
    let payload: unknown = undefined;
    try {
      payload = await response.json();
    } catch {
      payload = undefined;
    }
    const durationMs = Date.now() - start;
    if (!response.ok) {
      const errorSummary = isErrorResponse(payload)
        ? payload.message
        : response.statusText || 'request failed';
      logInventoryRequest({
        url,
        method,
        status: response.status,
        durationMs,
        errorSummary
      });
      handleApiError(response.status);
      throw createApiError(message, response.status, payload);
    }
    logInventoryRequest({ url, method, status: response.status, durationMs });
    return payload as T;
  } catch (error) {
    if (error instanceof ApiError) {
      throw error;
    }
    const durationMs = Date.now() - start;
    const isTimeout = error instanceof DOMException && error.name === 'AbortError';
    const status = isTimeout ? 408 : undefined;
    const errorSummary = error instanceof Error ? error.message : 'request failed';
    logInventoryRequest({ url, method, status, durationMs, errorSummary });
    handleApiError(status);
    throw createApiError(isTimeout ? '请求超时，请重试' : message, status);
  } finally {
    globalThis.clearTimeout(timeoutId);
  }
};

type InventoryListApiResponse = {
  items?: InventorySnapshot[];
  data?: InventorySnapshot[];
  total?: number;
  next_cursor?: string | null;
};

const normalizeInventoryListResponse = (
  payload: InventoryListApiResponse | InventoryListResponse,
  query: InventoryListQuery
): InventoryListResponse => {
  const items = payload.items ?? payload.data ?? [];
  if (payload.total != null) {
    return {
      items,
      total: payload.total
    };
  }
  const offset = query.offset ?? 0;
  const limit = query.limit ?? items.length;
  const hasMore = Boolean(payload.next_cursor) || (limit > 0 && items.length === limit);
  return {
    items,
    total: offset + items.length + (hasMore ? limit : 0)
  };
};

export const listInventory = async (
  tenantId: string,
  query: InventoryListQuery
): Promise<InventoryListResponse> => {
  if (useMocks) {
    const response = getMockInventoryList(query);
    if (!mockVirtualInventory.size) {
      return response;
    }
    return {
      ...response,
      items: response.items.map((item) => {
        const key = buildVirtualInventoryKey(
          item.warehouse_id ?? 'NA',
          item.sku_id,
          item.location_id
        );
        const virtualQty = mockVirtualInventory.get(key);
        if (virtualQty == null) {
          return item;
        }
        return { ...item, virtual_qty: virtualQty };
      })
    };
  }
  const response = await requestJson<InventoryListApiResponse | InventoryListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inv/balances${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取库存失败'
  );
  return normalizeInventoryListResponse(response, query);
};

export const listInventorySummary = async (
  tenantId: string,
  query: InventoryListQuery
): Promise<InventorySummaryListResponse> => {
  if (useMocks) {
    const mock = await getMockInventoryList(query);
    return {
      items: mock.items.map((item) => ({
        sku_id: item.sku_id,
        sku_code: item.sku_code ?? null,
        barcode: item.barcode ?? null,
        on_hand: item.on_hand ?? null,
        reserved: item.reserved,
        frozen: item.frozen ?? null,
        in_transit: item.in_transit ?? null,
        available: item.available,
        updated_at: item.updated_at ?? null
      })),
      total: mock.total
    };
  }
  return requestJson<InventorySummaryListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inv/summary${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取库存汇总失败'
  );
};

export const listInventoryCosts = async (
  tenantId: string,
  query: InventoryListQuery
): Promise<InventoryCostListResponse> => {
  if (useMocks) {
    const mock = await getMockInventoryList(query);
    return {
      items: mock.items.map((item) => ({
        sku_id: item.sku_id,
        sku_code: item.sku_code ?? null,
        barcode: item.barcode ?? null,
        on_hand: item.on_hand ?? null,
        reserved: item.reserved,
        frozen: item.frozen ?? null,
        in_transit: item.in_transit ?? null,
        available: item.available,
        updated_at: item.updated_at ?? null,
        unit_cost_cent: 0,
        total_cost_cent: 0
      })),
      total: mock.total
    };
  }
  return requestJson<InventoryCostListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inv/costs${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取库存成本失败'
  );
};

const requestInventoryAction = async (
  tenantId: string,
  kind: InventoryActionKind,
  payload: InventoryActionPayload
): Promise<InventorySnapshot> => {
  if (useMocks) {
    const updated = applyMockInventoryAction(kind, payload);
    if (!updated) {
      throw createApiError('库存记录不存在', 404);
    }
    return updated;
  }
  const baseBody = {
    warehouse_id: payload.warehouse_id,
    sku_id: payload.sku_id
  };
  if (kind === 'ADJUST') {
    return requestJson<InventorySnapshot>(
      `${apiBaseUrl}/v1/tenants/${tenantId}/inv/adjust`,
      {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          ...buildAuthHeaders()
        },
        body: JSON.stringify({
          ...baseBody,
          quantity_delta: payload.qty,
          reason: payload.ref_type ?? null,
          reference_id: payload.ref_id ?? null
        })
      },
      '提交库存操作失败'
    );
  }

  const orderItemId = payload.order_item_id ?? payload.ref_id ?? null;
  const body =
    kind === 'RESERVE'
      ? {
          ...baseBody,
          quantity: payload.qty,
          order_id: payload.order_id ?? null,
          order_item_id: orderItemId,
          reference_id: payload.ref_id ?? null
        }
      : {
          ...baseBody,
          quantity: payload.qty,
          order_item_id: orderItemId,
          reference_id: payload.ref_id ?? null
        };
  const endpoint =
    kind === 'RESERVE'
      ? `${apiBaseUrl}/v1/tenants/${tenantId}/inventory/reservations`
      : `${apiBaseUrl}/v1/tenants/${tenantId}/inventory/releases`;
  return requestJson<InventorySnapshot>(
    endpoint,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(body)
    },
    '提交库存操作失败'
  );
};

export const adjustInventory = async (
  tenantId: string,
  payload: InventoryActionPayload
): Promise<InventorySnapshot> => requestInventoryAction(tenantId, 'ADJUST', payload);

export const reserveInventory = async (
  tenantId: string,
  payload: InventoryActionPayload
): Promise<InventorySnapshot> => requestInventoryAction(tenantId, 'RESERVE', payload);

export const releaseInventory = async (
  tenantId: string,
  payload: InventoryActionPayload
): Promise<InventorySnapshot> => requestInventoryAction(tenantId, 'RELEASE', payload);

export const adjustVirtualInventory = async (
  tenantId: string,
  payload: VirtualInventoryAdjustPayload
): Promise<VirtualInventoryAdjustResponse> => {
  if (useMocks) {
    const key = buildVirtualInventoryKey(
      payload.warehouse_id,
      payload.sku_id,
      payload.location_id
    );
    const current = mockVirtualInventory.get(key) ?? 0;
    const next =
      payload.mode === 'SET' ? payload.qty : Math.max(0, current + payload.qty);
    mockVirtualInventory.set(key, next);
    return {
      warehouse_id: payload.warehouse_id,
      sku_id: payload.sku_id,
      area_id: payload.area_id ?? null,
      location_id: payload.location_id ?? null,
      virtual_qty: next,
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<VirtualInventoryAdjustResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inv/virtual-adjust`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify({
        warehouse_id: payload.warehouse_id,
        sku_id: payload.sku_id,
        area_id: payload.area_id ?? null,
        location_id: payload.location_id ?? null,
        mode: payload.mode,
        qty: payload.qty,
        reason: payload.reason ?? null
      })
    },
    '调整虚拟库存失败'
  );
};
