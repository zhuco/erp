import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type OrderReviewRule = {
  rule_id: string;
  tenant_id: string;
  name: string;
  enabled: boolean;
  priority: number;
  conditions_json: Record<string, unknown> | unknown[];
  action: string;
  reason: string;
  created_at: string;
  updated_at: string;
};

export type OrderReviewRuleListQuery = {
  enabled?: boolean;
  offset?: number;
  limit?: number;
};

export type OrderReviewRuleListResponse = {
  items: OrderReviewRule[];
  total: number;
};

export type OrderReviewRuleCreatePayload = {
  name: string;
  enabled?: boolean;
  priority?: number;
  conditions_json?: Record<string, unknown> | unknown[];
  action: string;
  reason?: string;
};

export type OrderReviewRuleUpdatePayload = {
  name?: string;
  enabled?: boolean;
  priority?: number;
  conditions_json?: Record<string, unknown> | unknown[];
  action?: string;
  reason?: string;
};

const requestJson = async <T>(input: RequestInfo, init: RequestInit, message: string) => {
  const response = await fetch(input, init);
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

const buildQueryString = (query: OrderReviewRuleListQuery): string => {
  const params = new URLSearchParams();
  if (query.enabled != null) {
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

export const listOrderReviewRules = async (
  tenantId: string,
  query: OrderReviewRuleListQuery
): Promise<OrderReviewRuleListResponse> => {
  if (useMocks) {
    return { items: [], total: 0 };
  }
  return requestJson<OrderReviewRuleListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/oms/review-rules${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取审单规则失败'
  );
};

export const createOrderReviewRule = async (
  tenantId: string,
  payload: OrderReviewRuleCreatePayload
): Promise<OrderReviewRule> => {
  if (useMocks) {
    return {
      rule_id: `${Date.now()}`,
      tenant_id: tenantId,
      name: payload.name,
      enabled: payload.enabled ?? true,
      priority: payload.priority ?? 0,
      conditions_json: payload.conditions_json ?? [],
      action: payload.action,
      reason: payload.reason ?? '',
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<OrderReviewRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/oms/review-rules`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建审单规则失败'
  );
};

export const updateOrderReviewRule = async (
  tenantId: string,
  ruleId: string,
  payload: OrderReviewRuleUpdatePayload
): Promise<OrderReviewRule> => {
  if (useMocks) {
    return {
      rule_id: ruleId,
      tenant_id: tenantId,
      name: payload.name ?? 'mock',
      enabled: payload.enabled ?? true,
      priority: payload.priority ?? 0,
      conditions_json: payload.conditions_json ?? [],
      action: payload.action ?? 'PASS',
      reason: payload.reason ?? '',
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<OrderReviewRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/oms/review-rules/${ruleId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新审单规则失败'
  );
};

export const deleteOrderReviewRule = async (
  tenantId: string,
  ruleId: string
): Promise<OrderReviewRule> => {
  if (useMocks) {
    return {
      rule_id: ruleId,
      tenant_id: tenantId,
      name: 'mock',
      enabled: false,
      priority: 0,
      conditions_json: [],
      action: 'PASS',
      reason: '',
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<OrderReviewRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/oms/review-rules/${ruleId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除审单规则失败'
  );
};
