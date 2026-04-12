import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders } from '@/api/client';
import { ApiError, createApiError, handleApiError } from '@/api/errors';

export type StockCountSummary = {
  count_id: string;
  warehouse_id: string;
  status: string;
  strategy: string;
  dimension: string;
  filters: StockCountFilters;
  created_at: string;
  updated_at: string;
};

export type StockCountFilters = {
  recent_days?: number | null;
  brand?: string | null;
  category?: string | null;
  scope_type?: string | null;
  scope_ids?: string[] | null;
  count_mode?: string | null;
  count_type?: string | null;
  freeze_stock?: boolean | null;
  created_by?: string | null;
};

export type StockCountItem = {
  count_item_id: string;
  sku_id: string;
  location_id?: string | null;
  system_qty: number;
  counted_qty: number;
  diff_qty: number;
  diff_reason?: string | null;
};

export type StockCountDetail = {
  count: StockCountSummary;
  items: StockCountItem[];
  audit_required: boolean;
};

export type StockCountListQuery = {
  warehouse_id?: string;
  status?: string;
  count_type?: string;
  scope_type?: string;
  keyword?: string;
  created_from?: string;
  created_to?: string;
  offset?: number;
  limit?: number;
};

export type StockCountListResponse = {
  items: StockCountSummary[];
  total: number;
};

export type StockCountItemUpdate = {
  count_item_id: string;
  counted_qty: number;
  diff_reason?: string;
};

export type StockCountCreatePayload = {
  warehouse_id: string;
  strategy?: string;
  dimension?: string;
  recent_days?: number;
  brand?: string;
  category?: string;
  scope_type?: string;
  scope_ids?: string[];
  count_mode?: string;
  count_type?: string;
  freeze_stock?: boolean;
};

export type StockCountImportItem = {
  sku_id: string;
  location_id?: string;
  counted_qty: number;
  diff_reason?: string;
};

export type StockCountImportPayload = {
  warehouse_id: string;
  dimension?: string;
  scope_type?: string;
  count_mode?: string;
  count_type?: string;
  freeze_stock?: boolean;
  items: StockCountImportItem[];
};

const STOCK_COUNT_REQUEST_TIMEOUT_MS = 15000;

const requestJson = async <T>(input: RequestInfo, init: RequestInit, message: string) => {
  const controller = new AbortController();
  const timeoutId = globalThis.setTimeout(
    () => controller.abort(),
    STOCK_COUNT_REQUEST_TIMEOUT_MS
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

const buildQueryString = (query: StockCountListQuery): string => {
  const params = new URLSearchParams();
  if (query.warehouse_id) {
    params.set('warehouse_id', query.warehouse_id);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.count_type) {
    params.set('count_type', query.count_type);
  }
  if (query.scope_type) {
    params.set('scope_type', query.scope_type);
  }
  if (query.keyword) {
    params.set('keyword', query.keyword);
  }
  if (query.created_from) {
    params.set('created_from', query.created_from);
  }
  if (query.created_to) {
    params.set('created_to', query.created_to);
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

export const listStockCounts = async (
  tenantId: string,
  query: StockCountListQuery
): Promise<StockCountListResponse> => {
  if (useMocks) {
    return { items: [], total: 0 };
  }
  return requestJson<StockCountListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stock-counts${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取盘点单失败'
  );
};

export const createStockCount = async (
  tenantId: string,
  payload: StockCountCreatePayload
): Promise<StockCountDetail> => {
  if (useMocks) {
    throw createApiError('Mock未实现', 501);
  }
  return requestJson<StockCountDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stock-counts`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建盘点单失败'
  );
};

export const getStockCount = async (
  tenantId: string,
  countId: string
): Promise<StockCountDetail> => {
  if (useMocks) {
    throw createApiError('Mock未实现', 501);
  }
  return requestJson<StockCountDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stock-counts/${countId}`,
    {
      headers: buildAuthHeaders()
    },
    '获取盘点单失败'
  );
};

export const updateStockCountItems = async (
  tenantId: string,
  countId: string,
  items: StockCountItemUpdate[]
): Promise<StockCountDetail> => {
  if (useMocks) {
    throw createApiError('Mock未实现', 501);
  }
  return requestJson<StockCountDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stock-counts/${countId}/items`,
    {
      method: 'PATCH',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify({ items })
    },
    '更新盘点明细失败'
  );
};

export const importStockCount = async (
  tenantId: string,
  payload: StockCountImportPayload
): Promise<StockCountDetail> => {
  if (useMocks) {
    throw createApiError('Mock未实现', 501);
  }
  return requestJson<StockCountDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stock-counts/import`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '导入盘点数据失败'
  );
};

export const confirmStockCountDiff = async (
  tenantId: string,
  countId: string
): Promise<StockCountDetail> => {
  if (useMocks) {
    throw createApiError('Mock未实现', 501);
  }
  return requestJson<StockCountDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stock-counts/${countId}/confirm-diff`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '确认差异失败'
  );
};

export const applyStockCount = async (
  tenantId: string,
  countId: string
): Promise<StockCountDetail> => {
  if (useMocks) {
    throw createApiError('Mock未实现', 501);
  }
  return requestJson<StockCountDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stock-counts/${countId}/apply`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '应用盘点失败'
  );
};

export const cancelStockCount = async (
  tenantId: string,
  countId: string
): Promise<StockCountDetail> => {
  if (useMocks) {
    throw createApiError('Mock未实现', 501);
  }
  return requestJson<StockCountDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stock-counts/${countId}/cancel`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '作废盘点单失败'
  );
};
