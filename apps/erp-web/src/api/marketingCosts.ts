import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockMarketingCost,
  deleteMockMarketingCost,
  listMockMarketingCosts,
  updateMockMarketingCost
} from '@/api/mocks';

export type MarketingCost = {
  cost_id: string;
  tenant_id: string;
  name: string;
  channel_id?: string | null;
  amount_cent: number;
  currency: string;
  allocation_rule: unknown;
  period_start?: string | null;
  period_end?: string | null;
  status: string;
  created_by?: string | null;
  created_at: string;
  updated_at: string;
};

export type MarketingCostListQuery = {
  keyword?: string;
  status?: string;
  channel_id?: string;
  offset?: number;
  limit?: number;
};

export type MarketingCostListResponse = {
  items: MarketingCost[];
  total: number;
};

export type MarketingCostCreatePayload = {
  name: string;
  channel_id?: string;
  amount_cent: number;
  currency?: string;
  allocation_rule?: unknown;
  period_start?: string;
  period_end?: string;
  status?: string;
};

export type MarketingCostUpdatePayload = {
  name?: string;
  channel_id?: string;
  amount_cent?: number;
  currency?: string;
  allocation_rule?: unknown;
  period_start?: string;
  period_end?: string;
  status?: string;
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

const buildQueryString = (query: MarketingCostListQuery): string => {
  const params = new URLSearchParams();
  if (query.keyword) {
    params.set('keyword', query.keyword);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.channel_id) {
    params.set('channel_id', query.channel_id);
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

export const listMarketingCosts = async (
  tenantId: string,
  query: MarketingCostListQuery
): Promise<MarketingCostListResponse> => {
  if (useMocks) {
    return listMockMarketingCosts(query);
  }
  return requestJson<MarketingCostListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/costs${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取促销费用失败'
  );
};

export const createMarketingCost = async (
  tenantId: string,
  payload: MarketingCostCreatePayload
): Promise<MarketingCost> => {
  if (useMocks) {
    return createMockMarketingCost(payload);
  }
  return requestJson<MarketingCost>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/costs`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建促销费用失败'
  );
};

export const updateMarketingCost = async (
  tenantId: string,
  costId: string,
  payload: MarketingCostUpdatePayload
): Promise<MarketingCost> => {
  if (useMocks) {
    return updateMockMarketingCost(costId, payload);
  }
  return requestJson<MarketingCost>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/costs/${costId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新促销费用失败'
  );
};

export const deleteMarketingCost = async (
  tenantId: string,
  costId: string
): Promise<MarketingCost> => {
  if (useMocks) {
    return deleteMockMarketingCost(costId);
  }
  return requestJson<MarketingCost>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/costs/${costId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除促销费用失败'
  );
};
