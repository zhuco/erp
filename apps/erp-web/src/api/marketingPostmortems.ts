import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockMarketingPostmortem,
  deleteMockMarketingPostmortem,
  listMockMarketingPostmortems,
  updateMockMarketingPostmortem
} from '@/api/mocks';

export type MarketingPostmortem = {
  postmortem_id: string;
  tenant_id: string;
  name: string;
  promotion_id?: string | null;
  channel_id?: string | null;
  summary?: string | null;
  metrics_json: unknown;
  created_by?: string | null;
  created_at: string;
  updated_at: string;
};

export type MarketingPostmortemListQuery = {
  keyword?: string;
  promotion_id?: string;
  channel_id?: string;
  offset?: number;
  limit?: number;
};

export type MarketingPostmortemListResponse = {
  items: MarketingPostmortem[];
  total: number;
};

export type MarketingPostmortemCreatePayload = {
  name: string;
  promotion_id?: string;
  channel_id?: string;
  summary?: string;
  metrics_json?: unknown;
};

export type MarketingPostmortemUpdatePayload = {
  name?: string;
  promotion_id?: string;
  channel_id?: string;
  summary?: string;
  metrics_json?: unknown;
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

const buildQueryString = (query: MarketingPostmortemListQuery): string => {
  const params = new URLSearchParams();
  if (query.keyword) {
    params.set('keyword', query.keyword);
  }
  if (query.promotion_id) {
    params.set('promotion_id', query.promotion_id);
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

export const listMarketingPostmortems = async (
  tenantId: string,
  query: MarketingPostmortemListQuery
): Promise<MarketingPostmortemListResponse> => {
  if (useMocks) {
    return listMockMarketingPostmortems(query);
  }
  return requestJson<MarketingPostmortemListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/postmortems${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取活动复盘失败'
  );
};

export const createMarketingPostmortem = async (
  tenantId: string,
  payload: MarketingPostmortemCreatePayload
): Promise<MarketingPostmortem> => {
  if (useMocks) {
    return createMockMarketingPostmortem(payload);
  }
  return requestJson<MarketingPostmortem>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/postmortems`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建活动复盘失败'
  );
};

export const updateMarketingPostmortem = async (
  tenantId: string,
  postmortemId: string,
  payload: MarketingPostmortemUpdatePayload
): Promise<MarketingPostmortem> => {
  if (useMocks) {
    return updateMockMarketingPostmortem(postmortemId, payload);
  }
  return requestJson<MarketingPostmortem>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/postmortems/${postmortemId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新活动复盘失败'
  );
};

export const deleteMarketingPostmortem = async (
  tenantId: string,
  postmortemId: string
): Promise<MarketingPostmortem> => {
  if (useMocks) {
    return deleteMockMarketingPostmortem(postmortemId);
  }
  return requestJson<MarketingPostmortem>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/postmortems/${postmortemId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除活动复盘失败'
  );
};
