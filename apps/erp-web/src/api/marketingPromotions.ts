import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockMarketingPromotion,
  deleteMockMarketingPromotion,
  listMockMarketingPromotions,
  updateMockMarketingPromotion
} from '@/api/mocks';

export type MarketingPromotion = {
  promotion_id: string;
  tenant_id: string;
  name: string;
  promo_type: string;
  status: string;
  priority: number;
  exclusive: boolean;
  scope_type: string;
  scope_json: unknown;
  rule_json: unknown;
  start_at?: string | null;
  end_at?: string | null;
  created_by?: string | null;
  created_at: string;
  updated_at: string;
};

export type MarketingPromotionListQuery = {
  keyword?: string;
  status?: string;
  promo_type?: string;
  offset?: number;
  limit?: number;
};

export type MarketingPromotionListResponse = {
  items: MarketingPromotion[];
  total: number;
};

export type MarketingPromotionCreatePayload = {
  name: string;
  promo_type: string;
  status?: string;
  priority?: number;
  exclusive?: boolean;
  scope_type?: string;
  scope_json?: unknown;
  rule_json?: unknown;
  start_at?: string;
  end_at?: string;
};

export type MarketingPromotionUpdatePayload = {
  name?: string;
  promo_type?: string;
  status?: string;
  priority?: number;
  exclusive?: boolean;
  scope_type?: string;
  scope_json?: unknown;
  rule_json?: unknown;
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

const buildQueryString = (query: MarketingPromotionListQuery): string => {
  const params = new URLSearchParams();
  if (query.keyword) {
    params.set('keyword', query.keyword);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.promo_type) {
    params.set('promo_type', query.promo_type);
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

export const listMarketingPromotions = async (
  tenantId: string,
  query: MarketingPromotionListQuery
): Promise<MarketingPromotionListResponse> => {
  if (useMocks) {
    return listMockMarketingPromotions(query);
  }
  return requestJson<MarketingPromotionListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/promotions${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取促销活动失败'
  );
};

export const createMarketingPromotion = async (
  tenantId: string,
  payload: MarketingPromotionCreatePayload
): Promise<MarketingPromotion> => {
  if (useMocks) {
    return createMockMarketingPromotion(payload);
  }
  return requestJson<MarketingPromotion>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/promotions`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建促销活动失败'
  );
};

export const updateMarketingPromotion = async (
  tenantId: string,
  promotionId: string,
  payload: MarketingPromotionUpdatePayload
): Promise<MarketingPromotion> => {
  if (useMocks) {
    return updateMockMarketingPromotion(promotionId, payload);
  }
  return requestJson<MarketingPromotion>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/promotions/${promotionId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新促销活动失败'
  );
};

export const deleteMarketingPromotion = async (
  tenantId: string,
  promotionId: string
): Promise<MarketingPromotion> => {
  if (useMocks) {
    return deleteMockMarketingPromotion(promotionId);
  }
  return requestJson<MarketingPromotion>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/promotions/${promotionId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除促销活动失败'
  );
};
