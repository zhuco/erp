import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockShop,
  deleteMockShop,
  getMockShop,
  getMockShopAuthStatus,
  getMockShopList,
  requestMockTokenRefresh,
  startMockShopAuthorize,
  submitMockShopAuth,
  updateMockShop
} from '@/api/mocks';
import type { PlatformCode } from '@/features/platform';

export type ShopTokenStatus = {
  has_access_token: boolean;
  has_refresh_token: boolean;
  expires_at?: string | null;
  expires_in_minutes?: number | null;
  is_expired: boolean;
  is_expiring_soon: boolean;
  scopes?: string | null;
  last_refresh_at?: string | null;
  last_error: string;
  updated_at: string;
};

export type Shop = {
  shop_id: string;
  tenant_id: string;
  platform: PlatformCode;
  platform_shop_id: string;
  name: string;
  status: string;
  auth_meta_json: Record<string, unknown>;
  created_at: string;
  updated_at: string;
  token_status?: ShopTokenStatus | null;
};

export type ShopListQuery = {
  platform?: PlatformCode;
  status?: string;
  offset?: number;
  limit?: number;
};

export type ShopListResponse = {
  items: Shop[];
  total: number;
};

export type ShopCreatePayload = {
  platform: PlatformCode;
  platform_shop_id: string;
  name: string;
  status?: string;
  credentials?: Record<string, unknown>;
  auth_meta_json?: Record<string, unknown>;
};

export type ShopUpdatePayload = {
  name?: string;
  status?: string;
  credentials?: Record<string, unknown>;
  auth_meta_json?: Record<string, unknown>;
};

export type ShopAuthorizePayload = {
  redirect_uri?: string;
};

export type ShopAuthorizeResponse = {
  authorize_url: string;
  state: string;
};

export type ShopAuthCallbackPayload = {
  code?: string;
  state?: string;
  error?: string;
  error_description?: string;
  redirect_uri?: string;
  access_token?: string;
  refresh_token?: string;
  expires_at?: string;
  scopes?: string;
  auth_meta_json?: Record<string, unknown>;
};

export type TokenRefreshRequestResponse = {
  outbox_id: string;
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

const buildQueryString = (query: ShopListQuery): string => {
  const params = new URLSearchParams();
  if (query.platform) {
    params.set('platform', query.platform);
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

const shopBaseUrl = (tenantId: string) =>
  `${apiBaseUrl}/v1/tenants/${tenantId}/channels/shops`;

export const listShops = async (
  tenantId: string,
  query: ShopListQuery
): Promise<ShopListResponse> => {
  if (useMocks) {
    return getMockShopList(query);
  }
  return requestJson<ShopListResponse>(
    `${shopBaseUrl(tenantId)}${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取店铺列表失败'
  );
};

export const getShop = async (tenantId: string, shopId: string): Promise<Shop> => {
  if (useMocks) {
    return getMockShop(shopId);
  }
  return requestJson<Shop>(
    `${shopBaseUrl(tenantId)}/${shopId}`,
    { headers: buildAuthHeaders() },
    '获取店铺详情失败'
  );
};

export const createShop = async (tenantId: string, payload: ShopCreatePayload): Promise<Shop> => {
  if (useMocks) {
    return createMockShop(payload);
  }
  return requestJson<Shop>(
    `${shopBaseUrl(tenantId)}`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建店铺失败'
  );
};

export const updateShop = async (
  tenantId: string,
  shopId: string,
  payload: ShopUpdatePayload
): Promise<Shop> => {
  if (useMocks) {
    return updateMockShop(shopId, payload);
  }
  return requestJson<Shop>(
    `${shopBaseUrl(tenantId)}/${shopId}`,
    {
      method: 'PATCH',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新店铺失败'
  );
};

export const deleteShop = async (tenantId: string, shopId: string): Promise<Shop> => {
  if (useMocks) {
    return deleteMockShop(shopId);
  }
  return requestJson<Shop>(
    `${shopBaseUrl(tenantId)}/${shopId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '禁用店铺失败'
  );
};

export const getShopAuthStatus = async (tenantId: string, shopId: string): Promise<Shop> => {
  if (useMocks) {
    return getMockShopAuthStatus(shopId);
  }
  return requestJson<Shop>(
    `${shopBaseUrl(tenantId)}/${shopId}/auth/status`,
    { headers: buildAuthHeaders() },
    '获取授权状态失败'
  );
};

export const startShopAuthorize = async (
  tenantId: string,
  shopId: string,
  payload: ShopAuthorizePayload
): Promise<ShopAuthorizeResponse> => {
  if (useMocks) {
    return startMockShopAuthorize(shopId, payload);
  }
  return requestJson<ShopAuthorizeResponse>(
    `${shopBaseUrl(tenantId)}/${shopId}/auth/authorize`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '发起授权失败'
  );
};

export const submitShopAuthCallback = async (
  tenantId: string,
  shopId: string,
  payload: ShopAuthCallbackPayload
): Promise<Shop> => {
  if (useMocks) {
    return submitMockShopAuth(shopId, payload);
  }
  return requestJson<Shop>(
    `${shopBaseUrl(tenantId)}/${shopId}/auth/callback`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '保存授权信息失败'
  );
};

export const requestTokenRefresh = async (
  tenantId: string,
  shopId: string
): Promise<TokenRefreshRequestResponse> => {
  if (useMocks) {
    return requestMockTokenRefresh(shopId);
  }
  return requestJson<TokenRefreshRequestResponse>(
    `${shopBaseUrl(tenantId)}/${shopId}/tokens/refresh`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '请求刷新 Token 失败'
  );
};
