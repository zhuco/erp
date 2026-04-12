import { apiBaseUrl } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type SkuSerialRule = {
  rule_id: string;
  tenant_id: string;
  sku_id?: string | null;
  name: string;
  prefix: string;
  seq_length: number;
  increment_step: number;
  enabled: boolean;
  created_at?: string | null;
  updated_at?: string | null;
};

export type SkuSerialRuleListQuery = {
  sku_id?: string;
  enabled?: boolean;
  offset?: number;
  limit?: number;
};

export type SkuSerialRuleListResponse = {
  items: SkuSerialRule[];
  total: number;
};

export type SkuSerialRulePayload = {
  sku_id?: string | null;
  name: string;
  prefix: string;
  seq_length?: number;
  increment_step?: number;
  enabled?: boolean;
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

const buildQueryString = (query: SkuSerialRuleListQuery): string => {
  const params = new URLSearchParams();
  if (query.sku_id) {
    params.set('sku_id', query.sku_id);
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

export const listSkuSerialRules = async (
  tenantId: string,
  query: SkuSerialRuleListQuery
): Promise<SkuSerialRuleListResponse> =>
  requestJson<SkuSerialRuleListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/sku-serial-rules${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取序列号规则失败'
  );

export const createSkuSerialRule = async (
  tenantId: string,
  payload: SkuSerialRulePayload
): Promise<SkuSerialRule> =>
  requestJson<SkuSerialRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/sku-serial-rules`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建序列号规则失败'
  );

export const updateSkuSerialRule = async (
  tenantId: string,
  ruleId: string,
  payload: SkuSerialRulePayload
): Promise<SkuSerialRule> =>
  requestJson<SkuSerialRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/sku-serial-rules/${ruleId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新序列号规则失败'
  );

export const deleteSkuSerialRule = async (
  tenantId: string,
  ruleId: string
): Promise<SkuSerialRule> =>
  requestJson<SkuSerialRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/sku-serial-rules/${ruleId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除序列号规则失败'
  );
