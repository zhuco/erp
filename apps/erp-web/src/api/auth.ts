import { ApiError, createApiError, handleApiError, isErrorResponse } from '@/api/errors';
import { buildAuthHeaders } from '@/api/client';
import { defaultPermissions } from '@/features/rbac';
import { apiBaseUrl, useMocks } from '@/utils/env';

export type LoginRequest = {
  tenant_id: string;
  email: string;
  password: string;
};

export type LoginResponse = {
  access_token: string;
  token_type: string;
  expires_in: number;
};

export type CurrentUserResponse = {
  user: {
    user_id: string;
    tenant_id: string;
    email: string;
    status: string;
    created_at: string;
  };
  roles: Array<{
    role_id: string;
    tenant_id: string;
    name: string;
    created_at: string;
  }>;
  permissions: string[];
  data_scopes: {
    shop_ids: string[];
    warehouse_ids: string[];
  };
};

export type CurrentUserProfile = CurrentUserResponse['user'];

export type CurrentUserUpdateRequest = {
  email: string;
};

export type CurrentUserPasswordRequest = {
  current_password: string;
  new_password: string;
};

const REQUEST_TIMEOUT_MS = 15000;
const DEFAULT_TENANT_ID = '11111111-1111-1111-1111-111111111111';
const DEFAULT_SHOP_IDS = ['SHOP-taobao-1', 'SHOP-jd-1', 'SHOP-pdd-1'];
const MOCK_TOKEN = 'mock-token';
let mockTenantId = DEFAULT_TENANT_ID;
let mockUserEmail = 'admin@example.com';

const buildMockCurrentUser = (): CurrentUserResponse => {
  const now = new Date().toISOString();
  return {
    user: {
      user_id: '00000000-0000-0000-0000-000000000001',
      tenant_id: mockTenantId,
      email: mockUserEmail,
      status: 'ACTIVE',
      created_at: now
    },
    roles: [
      {
        role_id: 'ROLE-ADMIN',
        tenant_id: mockTenantId,
        name: 'Admin',
        created_at: now
      }
    ],
    // mock 登录态默认继承完整默认权限，包含一期新增动作粒度权限。
    permissions: [...defaultPermissions],
    data_scopes: {
      shop_ids: DEFAULT_SHOP_IDS,
      warehouse_ids: []
    }
  };
};

const requestJson = async <T>(input: RequestInfo, init: RequestInit, message: string) => {
  const controller = new AbortController();
  const timeout = window.setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
  try {
    const response = await fetch(input, { ...init, signal: controller.signal });
    let payload: unknown = undefined;
    try {
      payload = await response.json();
    } catch {
      payload = undefined;
    }
    if (!response.ok) {
      handleApiError(response.status);
      const errorMessage = isErrorResponse(payload)
        ? payload.message
        : response.statusText || message;
      throw createApiError(errorMessage, response.status, payload);
    }
    return payload as T;
  } catch (error) {
    if (error instanceof ApiError) {
      throw error;
    }
    if (error instanceof DOMException && error.name === 'AbortError') {
      handleApiError(408);
      throw createApiError('请求超时，请稍后重试', 408);
    }
    throw createApiError(message, undefined);
  } finally {
    window.clearTimeout(timeout);
  }
};

export const login = async (payload: LoginRequest): Promise<LoginResponse> => {
  if (useMocks) {
    mockTenantId = payload.tenant_id || DEFAULT_TENANT_ID;
    return {
      access_token: MOCK_TOKEN,
      token_type: 'Bearer',
      expires_in: 3600
    };
  }
  return requestJson<LoginResponse>(
    `${apiBaseUrl}/auth/login`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    },
    '登录失败'
  );
};

export const fetchCurrentUser = async (): Promise<CurrentUserResponse> => {
  if (useMocks) {
    return buildMockCurrentUser();
  }
  return requestJson<CurrentUserResponse>(
    `${apiBaseUrl}/me`,
    {
      headers: buildAuthHeaders()
    },
    '获取当前用户信息失败'
  );
};

export const updateCurrentUser = async (
  payload: CurrentUserUpdateRequest
): Promise<CurrentUserProfile> => {
  if (useMocks) {
    mockUserEmail = payload.email;
    return buildMockCurrentUser().user;
  }
  return requestJson<CurrentUserProfile>(
    `${apiBaseUrl}/me`,
    {
      method: 'PUT',
      headers: {
        ...buildAuthHeaders(),
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    },
    '更新个人信息失败'
  );
};

export const updateCurrentUserPassword = async (
  payload: CurrentUserPasswordRequest
): Promise<CurrentUserProfile> => {
  if (useMocks) {
    return buildMockCurrentUser().user;
  }
  return requestJson<CurrentUserProfile>(
    `${apiBaseUrl}/me/password`,
    {
      method: 'PUT',
      headers: {
        ...buildAuthHeaders(),
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    },
    '更新密码失败'
  );
};
