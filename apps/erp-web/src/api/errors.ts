import { message } from 'antd';

import { useSessionStore } from '@/features/sessionStore';

import type { components } from './generated/schema';

export type ErrorResponse = components['schemas']['ErrorResponse'];

export const isErrorResponse = (payload: unknown): payload is ErrorResponse => {
  if (!payload || typeof payload !== 'object') {
    return false;
  }
  const record = payload as Record<string, unknown>;
  return typeof record.code === 'string' && typeof record.message === 'string';
};

type ApiErrorOptions = {
  status?: number;
  payload?: unknown;
};

export class ApiError extends Error {
  status?: number;
  code?: string;
  details?: Record<string, unknown> | null;
  requestId?: string | null;
  payload?: unknown;

  constructor(message: string, options: ApiErrorOptions = {}) {
    super(message);
    this.name = 'ApiError';
    this.status = options.status;
    this.payload = options.payload;
    if (isErrorResponse(options.payload)) {
      this.code = options.payload.code;
      this.details = options.payload.details ?? null;
      this.requestId = options.payload.request_id ?? null;
    }
  }
}

export const createApiError = (
  fallbackMessage: string,
  status?: number,
  payload?: unknown
): ApiError => {
  const message = isErrorResponse(payload) ? payload.message || fallbackMessage : fallbackMessage;
  return new ApiError(message, { status, payload });
};

const redirectTo = (path: string) => {
  if (window.location.pathname !== path) {
    window.location.assign(path);
  }
};

export const handleApiError = (status?: number) => {
  if (!status) {
    return;
  }
  if (status === 408) {
    message.error('请求超时，请重试');
    return;
  }
  if (status === 401) {
    useSessionStore.getState().resetSession();
    message.warning('登录已过期，请重新登录');
    redirectTo('/login');
    return;
  }
  if (status === 403) {
    message.error('无权限访问');
    redirectTo('/403');
  }
  if (status >= 500) {
    message.error('服务异常，请稍后重试');
  }
};
