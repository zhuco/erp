import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders } from '@/api/client';
import { ApiError, createApiError, handleApiError, isErrorResponse } from '@/api/errors';

export type DeadLetterStatus = 'PENDING' | 'REPLAYED';

export type DeadLetter = {
  dead_letter_id: string;
  tenant_id: string;
  outbox_id: string;
  event_type: string;
  payload: Record<string, unknown>;
  error: string;
  attempts: number;
  classification: string;
  next_action: string;
  status: DeadLetterStatus;
  created_at: string;
  replayed_at?: string | null;
};

export type OutboxStatus = 'PENDING' | 'FAILED' | 'DISPATCHING' | 'SENT' | 'DEAD';

export type OutboxEvent = {
  tenant_id: string;
  outbox_id: string;
  event_type: string;
  status: OutboxStatus;
  attempts: number;
  dedup_key?: string | null;
  last_error: string;
  next_retry_at: string;
  last_attempt_at: string;
  published_at?: string | null;
  created_at: string;
  updated_at: string;
};

export type DeadLetterListQuery = {
  status?: DeadLetterStatus;
  cursor?: string;
  limit?: number;
};

export type OutboxEventListQuery = {
  status?: OutboxStatus;
  event_type?: string;
  cursor?: string;
  limit?: number;
};

export type CursorListResponse<T> = {
  data: T[];
  next_cursor?: string | null;
};

type OutboxRequeueResponse = {
  outbox_id: string;
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

const buildQueryString = (query: Record<string, string | undefined>) => {
  const params = new URLSearchParams();
  Object.entries(query).forEach(([key, value]) => {
    if (!value) {
      return;
    }
    params.set(key, value);
  });
  const serialized = params.toString();
  return serialized ? `?${serialized}` : '';
};

export const listDeadLetters = async (
  tenantId: string,
  query: DeadLetterListQuery
): Promise<CursorListResponse<DeadLetter>> => {
  if (useMocks) {
    return { data: [], next_cursor: null };
  }
  return requestJson<CursorListResponse<DeadLetter>>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/dead-letters${buildQueryString({
      status: query.status,
      cursor: query.cursor,
      limit: query.limit ? String(query.limit) : undefined
    })}`,
    {
      headers: buildAuthHeaders()
    },
    '获取失败任务失败'
  );
};

export const replayDeadLetter = async (
  tenantId: string,
  deadLetterId: string
): Promise<OutboxRequeueResponse> => {
  if (useMocks) {
    return { outbox_id: deadLetterId };
  }
  return requestJson<OutboxRequeueResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/dead-letters/${deadLetterId}/replay`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '重放失败任务失败'
  );
};

export const listOutboxEvents = async (
  tenantId: string,
  query: OutboxEventListQuery
): Promise<CursorListResponse<OutboxEvent>> => {
  if (useMocks) {
    return { data: [], next_cursor: null };
  }
  return requestJson<CursorListResponse<OutboxEvent>>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/outbox-events${buildQueryString({
      status: query.status,
      event_type: query.event_type,
      cursor: query.cursor,
      limit: query.limit ? String(query.limit) : undefined
    })}`,
    {
      headers: buildAuthHeaders()
    },
    '获取Outbox队列失败'
  );
};

export const requeueOutboxEvent = async (
  tenantId: string,
  outboxId: string
): Promise<OutboxRequeueResponse> => {
  if (useMocks) {
    return { outbox_id: outboxId };
  }
  return requestJson<OutboxRequeueResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/outbox-events/${outboxId}/requeue`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '重试Outbox事件失败'
  );
};
