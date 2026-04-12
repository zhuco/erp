import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockMarketingCoupon,
  deleteMockMarketingCoupon,
  listMockMarketingCoupons,
  updateMockMarketingCoupon
} from '@/api/mocks';

export type MarketingCoupon = {
  coupon_id: string;
  tenant_id: string;
  name: string;
  code_prefix?: string | null;
  discount_type: string;
  discount_value: number;
  max_usage: number;
  per_user_limit: number;
  status: string;
  rules_json: unknown;
  start_at?: string | null;
  end_at?: string | null;
  created_by?: string | null;
  created_at: string;
  updated_at: string;
};

export type MarketingCouponListQuery = {
  keyword?: string;
  status?: string;
  discount_type?: string;
  offset?: number;
  limit?: number;
};

export type MarketingCouponListResponse = {
  items: MarketingCoupon[];
  total: number;
};

export type MarketingCouponCreatePayload = {
  name: string;
  code_prefix?: string;
  discount_type: string;
  discount_value: number;
  max_usage?: number;
  per_user_limit?: number;
  status?: string;
  rules_json?: unknown;
  start_at?: string;
  end_at?: string;
};

export type MarketingCouponUpdatePayload = {
  name?: string;
  code_prefix?: string;
  discount_type?: string;
  discount_value?: number;
  max_usage?: number;
  per_user_limit?: number;
  status?: string;
  rules_json?: unknown;
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

const buildQueryString = (query: MarketingCouponListQuery): string => {
  const params = new URLSearchParams();
  if (query.keyword) {
    params.set('keyword', query.keyword);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.discount_type) {
    params.set('discount_type', query.discount_type);
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

export const listMarketingCoupons = async (
  tenantId: string,
  query: MarketingCouponListQuery
): Promise<MarketingCouponListResponse> => {
  if (useMocks) {
    return listMockMarketingCoupons(query);
  }
  return requestJson<MarketingCouponListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/coupons${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取优惠券失败'
  );
};

export const createMarketingCoupon = async (
  tenantId: string,
  payload: MarketingCouponCreatePayload
): Promise<MarketingCoupon> => {
  if (useMocks) {
    return createMockMarketingCoupon(payload);
  }
  return requestJson<MarketingCoupon>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/coupons`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建优惠券失败'
  );
};

export const updateMarketingCoupon = async (
  tenantId: string,
  couponId: string,
  payload: MarketingCouponUpdatePayload
): Promise<MarketingCoupon> => {
  if (useMocks) {
    return updateMockMarketingCoupon(couponId, payload);
  }
  return requestJson<MarketingCoupon>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/coupons/${couponId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新优惠券失败'
  );
};

export const deleteMarketingCoupon = async (
  tenantId: string,
  couponId: string
): Promise<MarketingCoupon> => {
  if (useMocks) {
    return deleteMockMarketingCoupon(couponId);
  }
  return requestJson<MarketingCoupon>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/coupons/${couponId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除优惠券失败'
  );
};
