import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockMarketingGiftPolicy,
  deleteMockMarketingGiftPolicy,
  listMockMarketingGiftPolicies,
  updateMockMarketingGiftPolicy
} from '@/api/mocks';

export type MarketingGiftPolicy = {
  policy_id: string;
  tenant_id: string;
  name: string;
  status: string;
  strategy: string;
  probability?: number | null;
  scope_json: unknown;
  gift_pool: unknown;
  start_at?: string | null;
  end_at?: string | null;
  created_by?: string | null;
  created_at: string;
  updated_at: string;
};

export type MarketingGiftPolicyListQuery = {
  keyword?: string;
  status?: string;
  offset?: number;
  limit?: number;
};

export type MarketingGiftPolicyListResponse = {
  items: MarketingGiftPolicy[];
  total: number;
};

export type MarketingGiftPolicyCreatePayload = {
  name: string;
  status?: string;
  strategy?: string;
  probability?: number;
  scope_json?: unknown;
  gift_pool?: unknown;
  start_at?: string;
  end_at?: string;
};

export type MarketingGiftPolicyUpdatePayload = {
  name?: string;
  status?: string;
  strategy?: string;
  probability?: number;
  scope_json?: unknown;
  gift_pool?: unknown;
  start_at?: string;
  end_at?: string;
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

const buildQueryString = (query: MarketingGiftPolicyListQuery): string => {
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

export const listMarketingGiftPolicies = async (
  tenantId: string,
  query: MarketingGiftPolicyListQuery
): Promise<MarketingGiftPolicyListResponse> => {
  if (useMocks) {
    return listMockMarketingGiftPolicies(query);
  }
  return requestJson<MarketingGiftPolicyListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/gift-policies${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取赠品策略失败'
  );
};

export const createMarketingGiftPolicy = async (
  tenantId: string,
  payload: MarketingGiftPolicyCreatePayload
): Promise<MarketingGiftPolicy> => {
  if (useMocks) {
    return createMockMarketingGiftPolicy(payload);
  }
  return requestJson<MarketingGiftPolicy>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/gift-policies`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建赠品策略失败'
  );
};

export const updateMarketingGiftPolicy = async (
  tenantId: string,
  policyId: string,
  payload: MarketingGiftPolicyUpdatePayload
): Promise<MarketingGiftPolicy> => {
  if (useMocks) {
    return updateMockMarketingGiftPolicy(policyId, payload);
  }
  return requestJson<MarketingGiftPolicy>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/gift-policies/${policyId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新赠品策略失败'
  );
};

export const deleteMarketingGiftPolicy = async (
  tenantId: string,
  policyId: string
): Promise<MarketingGiftPolicy> => {
  if (useMocks) {
    return deleteMockMarketingGiftPolicy(policyId);
  }
  return requestJson<MarketingGiftPolicy>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/gift-policies/${policyId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除赠品策略失败'
  );
};
