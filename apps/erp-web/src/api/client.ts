import createClient from 'openapi-fetch';

import { useSessionStore } from '@/features/sessionStore';
import { apiBaseUrl } from '@/utils/env';

import type { paths } from './generated/schema';
import { createApiError, handleApiError } from './errors';

export const baseUrl = apiBaseUrl;
const REQUEST_TIMEOUT_MS = 15000;

export const buildAuthHeaders = (): Record<string, string> => {
  const token = useSessionStore.getState().token;
  if (!token) {
    return {};
  }
  return {
    Authorization: `Bearer ${token}`
  };
};

const authFetch: typeof fetch = async (input, init) => {
  const headers = new Headers(init?.headers ?? {});
  const authHeaders = buildAuthHeaders();
  if (authHeaders.Authorization && !headers.has('Authorization')) {
    headers.set('Authorization', authHeaders.Authorization);
  }
  const controller = new AbortController();
  const timeout = window.setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
  const upstreamSignal = init?.signal;
  if (upstreamSignal) {
    if (upstreamSignal.aborted) {
      controller.abort();
    } else {
      upstreamSignal.addEventListener('abort', () => controller.abort(), { once: true });
    }
  }
  try {
    return await fetch(input, {
      ...init,
      headers,
      signal: controller.signal
    });
  } catch (error) {
    if (controller.signal.aborted && !upstreamSignal?.aborted) {
      throw createApiError('请求超时，请稍后重试', 408);
    }
    throw error;
  } finally {
    window.clearTimeout(timeout);
  }
};

export const apiClient = createClient<paths>({
  baseUrl,
  fetch: authFetch
});

export const fetchWithAuth = authFetch;

type ApiResponse<T> = {
  data?: T;
  error?: unknown;
  response: Response;
};

export const unwrapApiResponse = <T>(result: ApiResponse<T>, message: string): T => {
  if (result.error) {
    handleApiError(result.response?.status);
    throw createApiError(message, result.response?.status, result.error);
  }
  if (result.data === undefined) {
    handleApiError(result.response?.status);
    throw createApiError(message, result.response?.status);
  }
  return result.data;
};
