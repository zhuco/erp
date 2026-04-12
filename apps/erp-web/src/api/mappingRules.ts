import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type MappingRule = {
  rule_id: string;
  tenant_id: string;
  platform: string;
  entity_type: string;
  version: number;
  enabled: boolean;
  rule_json: Record<string, unknown>;
  created_at?: string | null;
  updated_at?: string | null;
};

export type MappingRuleListQuery = {
  platform?: string;
  entity_type?: string;
  enabled?: boolean;
  offset?: number;
  limit?: number;
};

export type MappingRuleListResponse = {
  items: MappingRule[];
  total: number;
};

export type MappingRuleCreatePayload = {
  platform: string;
  entity_type: string;
  version: number;
  enabled?: boolean;
  rule_json?: Record<string, unknown>;
};

export type MappingRuleUpdatePayload = {
  enabled?: boolean;
  rule_json?: Record<string, unknown>;
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

const buildQueryString = (query: MappingRuleListQuery): string => {
  const params = new URLSearchParams();
  if (query.platform) {
    params.set('platform', query.platform);
  }
  if (query.entity_type) {
    params.set('entity_type', query.entity_type);
  }
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

export const listMappingRules = async (
  tenantId: string,
  query: MappingRuleListQuery
): Promise<MappingRuleListResponse> => {
  if (useMocks) {
    return { items: [], total: 0 };
  }
  return requestJson<MappingRuleListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/mapping-rules${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取映射规则失败'
  );
};

export const createMappingRule = async (
  tenantId: string,
  payload: MappingRuleCreatePayload
): Promise<MappingRule> => {
  if (useMocks) {
    throw createApiError('Mock not supported', 500);
  }
  return requestJson<MappingRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/mapping-rules`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建映射规则失败'
  );
};

export const updateMappingRule = async (
  tenantId: string,
  ruleId: string,
  payload: MappingRuleUpdatePayload
): Promise<MappingRule> => {
  if (useMocks) {
    throw createApiError('Mock not supported', 500);
  }
  return requestJson<MappingRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/mapping-rules/${ruleId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新映射规则失败'
  );
};
