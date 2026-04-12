import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders } from '@/api/client';
import { ApiError, createApiError, handleApiError, isErrorResponse } from '@/api/errors';
import {
  createMockInventoryAlertRule,
  deleteMockInventoryAlertRule,
  getMockInventoryAlertSummary,
  getMockInventoryAmountAnalysis,
  getMockInventoryTurnoverAnalysis,
  listMockInventoryAlertRules,
  listMockInventoryAlerts,
  updateMockInventoryAlertRule
} from '@/api/mocks';

export type InventoryAlertType = 'OUT_OF_STOCK' | 'LOW_STOCK' | 'HIGH_STOCK' | 'NO_SALES';
export type InventoryAlertLevel = 'HIGH' | 'MEDIUM' | 'LOW';

export type InventoryAlertSummary = {
  out_of_stock: number;
  low_stock: number;
  high_stock: number;
  no_sales: number;
  updated_at?: string | null;
};

export type InventoryAlertRecord = {
  alert_id: string;
  sku_id: string;
  sku_code?: string | null;
  sku_name?: string | null;
  spec?: string | null;
  warehouse_id?: string | null;
  warehouse_name?: string | null;
  brand?: string | null;
  category?: string | null;
  available_qty: number;
  available_days?: number | null;
  sales_30d?: number | null;
  alert_type: InventoryAlertType;
  alert_level: InventoryAlertLevel;
  suggestion?: string | null;
  inventory_amount?: number | null;
  turnover_days?: number | null;
  stock_age_days?: number | null;
};

export type InventoryInsightQuery = {
  date_from?: string;
  date_to?: string;
  warehouse_id?: string;
  brand?: string;
  category?: string;
  keyword?: string;
  alert_level?: InventoryAlertLevel[];
  alert_type?: InventoryAlertType;
};

export type InventoryAlertListQuery = InventoryInsightQuery & {
  offset?: number;
  limit?: number;
  sort_field?: string;
  sort_order?: 'asc' | 'desc';
};

export type InventoryAlertListResponse = {
  items: InventoryAlertRecord[];
  total: number;
};

export type InventoryAlertRule = {
  rule_id: string;
  name: string;
  warehouse_id?: string | null;
  category?: string | null;
  brand?: string | null;
  sku_keyword?: string | null;
  available_qty_threshold: number;
  available_days_threshold: number;
  alert_level: InventoryAlertLevel;
  enabled: boolean;
  updated_at?: string | null;
  created_at?: string | null;
};

export type InventoryAlertRulePayload = {
  name: string;
  warehouse_id?: string | null;
  category?: string | null;
  brand?: string | null;
  sku_keyword?: string | null;
  available_qty_threshold: number;
  available_days_threshold: number;
  alert_level: InventoryAlertLevel;
  enabled: boolean;
};

export type InventoryAlertRuleListQuery = {
  offset?: number;
  limit?: number;
};

export type InventoryAlertRuleListResponse = {
  items: InventoryAlertRule[];
  total: number;
};

export type InventoryAmountRanking = {
  dimension_id: string;
  dimension_name: string;
  inventory_qty: number;
  inventory_amount: number;
  share: number;
};

export type InventoryAmountTrendPoint = {
  period: string;
  inventory_amount: number;
};

export type InventoryAmountAnalysis = {
  warehouse_ranking: InventoryAmountRanking[];
  brand_share: InventoryAmountRanking[];
  category_share: InventoryAmountRanking[];
  sku_share: InventoryAmountRanking[];
  trend: InventoryAmountTrendPoint[];
};

export type InventoryTurnoverBucket = {
  label: string;
  sku_count: number;
  inventory_amount: number;
  share: number;
};

export type InventorySlowMovingItem = {
  dimension_name: string;
  avg_turnover_days: number;
  sku_count: number;
};

export type InventoryTrendPoint = {
  date: string;
  inventory_qty: number;
  inventory_amount: number;
};

export type InventoryAgeBucket = {
  label: string;
  sku_count: number;
  inventory_amount: number;
  share: number;
};

export type InventoryDeadStockItem = {
  sku_id: string;
  sku_code: string;
  sku_name?: string | null;
  warehouse_name?: string | null;
  dead_days: number;
  inventory_qty: number;
  inventory_amount: number;
};

export type InventoryAbcItem = {
  level: 'A' | 'B' | 'C';
  sku_count: number;
  inventory_amount: number;
  share: number;
  metric: string;
};

export type InventoryTurnoverAnalysis = {
  turnover_buckets: InventoryTurnoverBucket[];
  slow_moving: InventorySlowMovingItem[];
  history: InventoryTrendPoint[];
  age_buckets: InventoryAgeBucket[];
  dead_stock: InventoryDeadStockItem[];
  abc_classes: InventoryAbcItem[];
};

const INSIGHTS_REQUEST_TIMEOUT_MS = 15000;
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

const buildQueryString = (query: InventoryAlertListQuery | InventoryInsightQuery): string => {
  const params = new URLSearchParams();
  if (query.date_from) {
    params.set('date_from', query.date_from);
  }
  if (query.date_to) {
    params.set('date_to', query.date_to);
  }
  if (query.warehouse_id) {
    params.set('warehouse_id', query.warehouse_id);
  }
  if (query.brand) {
    params.set('brand', query.brand);
  }
  if (query.category) {
    params.set('category', query.category);
  }
  if (query.keyword) {
    params.set('keyword', query.keyword);
  }
  if (query.alert_level && query.alert_level.length > 0) {
    params.set('alert_level', query.alert_level.join(','));
  }
  if (query.alert_type) {
    params.set('alert_type', query.alert_type);
  }
  if ('offset' in query && query.offset != null) {
    params.set('offset', String(query.offset));
  }
  if ('limit' in query && query.limit != null) {
    params.set('limit', String(query.limit));
  }
  if ('sort_field' in query && query.sort_field) {
    params.set('sort_field', query.sort_field);
  }
  if ('sort_order' in query && query.sort_order) {
    params.set('sort_order', query.sort_order);
  }
  const serialized = params.toString();
  return serialized ? `?${serialized}` : '';
};

const requestJson = async <T>(input: RequestInfo, init: RequestInit, message: string) => {
  const url = typeof input === 'string' ? input : input.toString();
  const method = init.method ?? 'GET';
  const start = Date.now();
  const controller = new AbortController();
  const timeoutId = globalThis.setTimeout(
    () => controller.abort(),
    INSIGHTS_REQUEST_TIMEOUT_MS
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

export const getInventoryAlertSummary = async (
  tenantId: string,
  query: InventoryInsightQuery
): Promise<InventoryAlertSummary> => {
  if (useMocks) {
    return getMockInventoryAlertSummary(query);
  }
  return requestJson<InventoryAlertSummary>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inventory/insights/alerts/summary${buildQueryString(
      query
    )}`,
    {
      headers: buildAuthHeaders()
    },
    '获取库存预警概览失败'
  );
};

export const listInventoryAlerts = async (
  tenantId: string,
  query: InventoryAlertListQuery
): Promise<InventoryAlertListResponse> => {
  if (useMocks) {
    return listMockInventoryAlerts(query);
  }
  return requestJson<InventoryAlertListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inventory/insights/alerts${buildQueryString(
      query
    )}`,
    {
      headers: buildAuthHeaders()
    },
    '获取库存预警明细失败'
  );
};

export const getInventoryAmountAnalysis = async (
  tenantId: string,
  query: InventoryInsightQuery
): Promise<InventoryAmountAnalysis> => {
  if (useMocks) {
    return getMockInventoryAmountAnalysis(query);
  }
  return requestJson<InventoryAmountAnalysis>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inventory/insights/amount-analysis${buildQueryString(
      query
    )}`,
    {
      headers: buildAuthHeaders()
    },
    '获取库存金额分析失败'
  );
};

export const getInventoryTurnoverAnalysis = async (
  tenantId: string,
  query: InventoryInsightQuery
): Promise<InventoryTurnoverAnalysis> => {
  if (useMocks) {
    return getMockInventoryTurnoverAnalysis(query);
  }
  return requestJson<InventoryTurnoverAnalysis>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inventory/insights/turnover-analysis${buildQueryString(
      query
    )}`,
    {
      headers: buildAuthHeaders()
    },
    '获取库存周转分析失败'
  );
};

export const listInventoryAlertRules = async (
  tenantId: string,
  query: InventoryAlertRuleListQuery = {}
): Promise<InventoryAlertRuleListResponse> => {
  if (useMocks) {
    return listMockInventoryAlertRules(query);
  }
  return requestJson<InventoryAlertRuleListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inventory/insights/alert-rules${buildQueryString(
      query
    )}`,
    {
      headers: buildAuthHeaders()
    },
    '获取预警规则失败'
  );
};

export const createInventoryAlertRule = async (
  tenantId: string,
  payload: InventoryAlertRulePayload
): Promise<InventoryAlertRule> => {
  if (useMocks) {
    return createMockInventoryAlertRule(payload);
  }
  return requestJson<InventoryAlertRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inventory/insights/alert-rules`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建预警规则失败'
  );
};

export const updateInventoryAlertRule = async (
  tenantId: string,
  ruleId: string,
  payload: InventoryAlertRulePayload
): Promise<InventoryAlertRule> => {
  if (useMocks) {
    return updateMockInventoryAlertRule(ruleId, payload);
  }
  return requestJson<InventoryAlertRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inventory/insights/alert-rules/${ruleId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新预警规则失败'
  );
};

export const deleteInventoryAlertRule = async (
  tenantId: string,
  ruleId: string
): Promise<void> => {
  if (useMocks) {
    deleteMockInventoryAlertRule(ruleId);
    return;
  }
  await requestJson(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inventory/insights/alert-rules/${ruleId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除预警规则失败'
  );
};
