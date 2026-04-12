import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  applyMockLogisticsWarehouseSuggestion,
  createMockLogisticsWarehouseStrategy,
  listMockLogisticsWarehouseStrategies,
  listMockLogisticsWarehouseStrategyHistory,
  listMockLogisticsWarehouseSuggestions,
  updateMockLogisticsWarehouseStrategy
} from '@/api/mocks';

export type LogisticsWarehouseStrategyPriority = 'SPEED' | 'COST';

export type LogisticsWarehouseStrategy = {
  strategy_id: string;
  tenant_id: string;
  name: string;
  priority: LogisticsWarehouseStrategyPriority | string;
  enabled: boolean;
  updated_by?: string | null;
  created_at: string;
  updated_at: string;
};

export type LogisticsWarehouseStrategyHistory = {
  record_id: string;
  tenant_id: string;
  strategy_id: string;
  summary: string;
  updated_by?: string | null;
  updated_at: string;
};

export type LogisticsWarehouseSuggestion = {
  suggestion_id: string;
  tenant_id: string;
  order_id: string;
  destination: string;
  recommended_warehouse: string;
  eta_hours: number;
  cost_cent: number;
  savings_cent: number;
  strategy_id?: string | null;
  applied: boolean;
  applied_at?: string | null;
  created_at: string;
};

export type LogisticsWarehouseStrategyListQuery = {
  enabled?: boolean;
  offset?: number;
  limit?: number;
};

export type LogisticsWarehouseStrategyListResponse = {
  items: LogisticsWarehouseStrategy[];
  total: number;
};

export type LogisticsWarehouseStrategyCreatePayload = {
  name: string;
  priority?: LogisticsWarehouseStrategyPriority;
  enabled?: boolean;
  updated_by?: string | null;
  history_summary?: string | null;
};

export type LogisticsWarehouseStrategyUpdatePayload = {
  name?: string;
  priority?: LogisticsWarehouseStrategyPriority;
  enabled?: boolean;
  updated_by?: string | null;
  history_summary?: string | null;
};

export type LogisticsWarehouseSuggestionListQuery = {
  applied?: boolean;
  offset?: number;
  limit?: number;
};

export type LogisticsWarehouseSuggestionListResponse = {
  items: LogisticsWarehouseSuggestion[];
  total: number;
};

export type LogisticsWarehouseStrategyHistoryListResponse = {
  items: LogisticsWarehouseStrategyHistory[];
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

const buildQueryString = (query: LogisticsWarehouseStrategyListQuery): string => {
  const params = new URLSearchParams();
  if (query.enabled != null) {
    params.set('enabled', query.enabled ? 'true' : 'false');
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

const buildSuggestionQuery = (query: LogisticsWarehouseSuggestionListQuery): string => {
  const params = new URLSearchParams();
  if (query.applied != null) {
    params.set('applied', query.applied ? 'true' : 'false');
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

export const listLogisticsWarehouseStrategies = async (
  tenantId: string,
  query: LogisticsWarehouseStrategyListQuery
): Promise<LogisticsWarehouseStrategyListResponse> => {
  if (useMocks) {
    return listMockLogisticsWarehouseStrategies(query);
  }
  return requestJson<LogisticsWarehouseStrategyListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/warehouse-strategies${buildQueryString(
      query
    )}`,
    { headers: buildAuthHeaders() },
    '获取仓配策略失败'
  );
};

export const createLogisticsWarehouseStrategy = async (
  tenantId: string,
  payload: LogisticsWarehouseStrategyCreatePayload
): Promise<LogisticsWarehouseStrategy> => {
  if (useMocks) {
    return createMockLogisticsWarehouseStrategy(payload);
  }
  return requestJson<LogisticsWarehouseStrategy>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/warehouse-strategies`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建仓配策略失败'
  );
};

export const updateLogisticsWarehouseStrategy = async (
  tenantId: string,
  strategyId: string,
  payload: LogisticsWarehouseStrategyUpdatePayload
): Promise<LogisticsWarehouseStrategy> => {
  if (useMocks) {
    return updateMockLogisticsWarehouseStrategy(strategyId, payload);
  }
  return requestJson<LogisticsWarehouseStrategy>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/warehouse-strategies/${strategyId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新仓配策略失败'
  );
};

export const listLogisticsWarehouseStrategyHistory = async (
  tenantId: string,
  strategyId: string
): Promise<LogisticsWarehouseStrategyHistoryListResponse> => {
  if (useMocks) {
    return listMockLogisticsWarehouseStrategyHistory(strategyId);
  }
  return requestJson<LogisticsWarehouseStrategyHistoryListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/warehouse-strategies/${strategyId}/history`,
    { headers: buildAuthHeaders() },
    '获取策略变更记录失败'
  );
};

export const listLogisticsWarehouseSuggestions = async (
  tenantId: string,
  query: LogisticsWarehouseSuggestionListQuery
): Promise<LogisticsWarehouseSuggestionListResponse> => {
  if (useMocks) {
    return listMockLogisticsWarehouseSuggestions(query);
  }
  return requestJson<LogisticsWarehouseSuggestionListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/warehouse-suggestions${buildSuggestionQuery(
      query
    )}`,
    { headers: buildAuthHeaders() },
    '获取仓配优化建议失败'
  );
};

export const applyLogisticsWarehouseSuggestion = async (
  tenantId: string,
  suggestionId: string
): Promise<LogisticsWarehouseSuggestion> => {
  if (useMocks) {
    return applyMockLogisticsWarehouseSuggestion(suggestionId);
  }
  return requestJson<LogisticsWarehouseSuggestion>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/warehouse-suggestions/${suggestionId}/apply`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '回写仓配建议失败'
  );
};
