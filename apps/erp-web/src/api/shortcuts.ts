import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders } from '@/api/client';
import { ApiError, createApiError, handleApiError, isErrorResponse } from '@/api/errors';

export type UserShortcut = {
  path: string;
  name?: string;
};

export type UserShortcutsResponse = {
  shortcuts: UserShortcut[];
};

const REQUEST_TIMEOUT_MS = 15000;

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

export const getUserShortcuts = async (tenantId: string): Promise<UserShortcutsResponse> => {
  if (useMocks) {
    return { shortcuts: [] };
  }
  return requestJson<UserShortcutsResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/me/shortcuts`,
    { headers: buildAuthHeaders() },
    '获取快捷入口失败'
  );
};

export const updateUserShortcuts = async (
  tenantId: string,
  shortcuts: UserShortcut[]
): Promise<UserShortcutsResponse> => {
  if (useMocks) {
    return { shortcuts };
  }
  return requestJson<UserShortcutsResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/me/shortcuts`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify({ shortcuts })
    },
    '更新快捷入口失败'
  );
};
