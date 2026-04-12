import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type LogisticsMatchRule = {
  rule_id: string;
  tenant_id: string;
  name: string;
  enabled: boolean;
  priority: number;
  product_id?: string | null;
  conditions_json?: unknown;
  created_at?: string;
  updated_at?: string;
};

export type LogisticsMatchRuleListQuery = {
  keyword?: string;
  enabled?: boolean;
  offset?: number;
  limit?: number;
};

export type LogisticsMatchRuleListResponse = {
  items: LogisticsMatchRule[];
  total: number;
};

export type LogisticsMatchRuleCreatePayload = {
  name: string;
  enabled?: boolean;
  priority?: number;
  product_id?: string | null;
  conditions_json?: unknown;
};

export type LogisticsMatchRuleUpdatePayload = {
  name?: string;
  enabled?: boolean;
  priority?: number;
  product_id?: string | null;
  conditions_json?: unknown;
};

type LogisticsMatchRuleListApiResponse = {
  items: LogisticsMatchRule[];
  total: number;
};

const buildQueryString = (query: LogisticsMatchRuleListQuery): string => {
  const params = new URLSearchParams();
  if (query.keyword) {
    params.set('keyword', query.keyword);
  }
  if (query.enabled !== undefined) {
    params.set('enabled', String(query.enabled));
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

export const listLogisticsMatchRules = async (
  tenantId: string,
  query: LogisticsMatchRuleListQuery
): Promise<LogisticsMatchRuleListResponse> => {
  if (useMocks) {
    return { items: [], total: 0 };
  }
  const response = await requestJson<LogisticsMatchRuleListApiResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/match-rules${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取物流匹配规则失败'
  );
  return { items: response.items, total: response.total };
};

export const createLogisticsMatchRule = async (
  tenantId: string,
  payload: LogisticsMatchRuleCreatePayload
): Promise<LogisticsMatchRule> => {
  if (useMocks) {
    return {
      rule_id: `${Date.now()}`,
      tenant_id: tenantId,
      name: payload.name,
      enabled: payload.enabled ?? true,
      priority: payload.priority ?? 0,
      product_id: payload.product_id ?? null,
      conditions_json: payload.conditions_json ?? [],
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<LogisticsMatchRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/match-rules`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建物流匹配规则失败'
  );
};

export const updateLogisticsMatchRule = async (
  tenantId: string,
  ruleId: string,
  payload: LogisticsMatchRuleUpdatePayload
): Promise<LogisticsMatchRule> => {
  if (useMocks) {
    return {
      rule_id: ruleId,
      tenant_id: tenantId,
      name: payload.name ?? 'mock',
      enabled: payload.enabled ?? true,
      priority: payload.priority ?? 0,
      product_id: payload.product_id ?? null,
      conditions_json: payload.conditions_json ?? [],
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<LogisticsMatchRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/match-rules/${ruleId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新物流匹配规则失败'
  );
};

export const deleteLogisticsMatchRule = async (
  tenantId: string,
  ruleId: string
): Promise<LogisticsMatchRule> => {
  if (useMocks) {
    return {
      rule_id: ruleId,
      tenant_id: tenantId,
      name: 'mock',
      enabled: true,
      priority: 0,
      product_id: null,
      conditions_json: [],
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<LogisticsMatchRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/match-rules/${ruleId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除物流匹配规则失败'
  );
};
