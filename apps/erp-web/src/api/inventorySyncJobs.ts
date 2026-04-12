import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders } from '@/api/client';
import { ApiError, createApiError, handleApiError, isErrorResponse } from '@/api/errors';
import {
  getMockStockSyncJobs,
  retryMockStockSyncJob,
  runMockStockSyncJob
} from '@/api/mocks';

export type StockSyncJobStatus = 'PENDING' | 'SENDING' | 'SENT' | 'FAILED';

export type StockSyncJob = {
  job_id: string;
  shop_id: string;
  sku_id: string;
  warehouse_id: string;
  target_qty: number;
  status: StockSyncJobStatus;
  attempts: number;
  last_error?: string | null;
  success_count?: number;
  failure_count?: number;
  last_duration_ms?: number;
  created_at: string;
  updated_at: string;
};

export type StockSyncJobListQuery = {
  shop_id?: string;
  sku_id?: string;
  warehouse_id?: string;
  status?: StockSyncJobStatus;
  offset?: number;
  limit?: number;
};

export type StockSyncJobListResponse = {
  items: StockSyncJob[];
  total: number;
};

export type StockSyncJobEvent = {
  event_id: string;
  status: string;
  attempt: number;
  payload: Record<string, unknown>;
  created_at: string;
};

export type StockSyncJobEventListResponse = {
  items: StockSyncJobEvent[];
};

type StockSyncJobListApiResponse = {
  items?: StockSyncJob[];
  total?: number;
};

const INVENTORY_REQUEST_TIMEOUT_MS = 15000;
const enableInventoryDiagnostics = import.meta.env.DEV || import.meta.env.MODE === 'test';

const logInventoryRequest = ({
  url,
  method,
  status,
  durationMs,
  errorSummary
}: {
  url: string;
  method: string;
  status?: number;
  durationMs: number;
  errorSummary?: string;
}) => {
  if (!enableInventoryDiagnostics) {
    return;
  }
  const statusLabel = status ?? 'NETWORK_ERROR';
  const summary = errorSummary ? ` error="${errorSummary}"` : '';
  console.info(
    `[inventory] ${method.toUpperCase()} ${url} status=${statusLabel} duration=${durationMs}ms${summary}`
  );
};

const buildQueryString = (query: StockSyncJobListQuery): string => {
  const params = new URLSearchParams();
  if (query.shop_id) {
    params.set('shop_id', query.shop_id);
  }
  if (query.sku_id) {
    params.set('sku_id', query.sku_id);
  }
  if (query.warehouse_id) {
    params.set('warehouse_id', query.warehouse_id);
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

const requestJson = async <T>(input: RequestInfo, init: RequestInit, message: string) => {
  const url = typeof input === 'string' ? input : input.toString();
  const method = init.method ?? 'GET';
  const start = Date.now();
  const controller = new AbortController();
  const timeoutId = globalThis.setTimeout(
    () => controller.abort(),
    INVENTORY_REQUEST_TIMEOUT_MS
  );

  try {
    const response = await fetch(input, { ...init, signal: controller.signal });
    let payload: unknown = undefined;
    try {
      payload = await response.json();
    } catch {
      payload = undefined;
    }
    const durationMs = Date.now() - start;
    if (!response.ok) {
      const errorSummary = isErrorResponse(payload)
        ? payload.message
        : response.statusText || 'request failed';
      logInventoryRequest({
        url,
        method,
        status: response.status,
        durationMs,
        errorSummary
      });
      handleApiError(response.status);
      throw createApiError(message, response.status, payload);
    }
    logInventoryRequest({ url, method, status: response.status, durationMs });
    return payload as T;
  } catch (error) {
    if (error instanceof ApiError) {
      throw error;
    }
    const durationMs = Date.now() - start;
    const isTimeout = error instanceof DOMException && error.name === 'AbortError';
    const status = isTimeout ? 408 : undefined;
    const errorSummary = error instanceof Error ? error.message : 'request failed';
    logInventoryRequest({ url, method, status, durationMs, errorSummary });
    handleApiError(status);
    throw createApiError(isTimeout ? '请求超时，请重试' : message, status);
  } finally {
    globalThis.clearTimeout(timeoutId);
  }
};

export const listInventorySyncJobs = async (
  tenantId: string,
  query: StockSyncJobListQuery
): Promise<StockSyncJobListResponse> => {
  if (useMocks) {
    return getMockStockSyncJobs(query);
  }
  const response = await requestJson<StockSyncJobListApiResponse | StockSyncJobListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inv/sync-jobs${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取库存同步任务失败'
  );
  return {
    items: response.items ?? [],
    total: response.total ?? 0
  };
};

export const retryInventorySyncJob = async (
  tenantId: string,
  jobId: string
): Promise<StockSyncJob> => {
  if (useMocks) {
    return retryMockStockSyncJob(jobId);
  }
  return requestJson<StockSyncJob>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inv/sync-jobs/${jobId}/retry`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '重试库存同步失败'
  );
};

export const runInventorySyncJob = async (
  tenantId: string,
  jobId: string
): Promise<StockSyncJob> => {
  if (useMocks) {
    return runMockStockSyncJob(jobId);
  }
  return requestJson<StockSyncJob>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inv/sync-jobs/${jobId}/run`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '触发库存同步失败'
  );
};

export const listInventorySyncJobEvents = async (
  tenantId: string,
  jobId: string,
  status?: string
): Promise<StockSyncJobEventListResponse> => {
  const query = status ? `?status=${encodeURIComponent(status)}` : '';
  return requestJson<StockSyncJobEventListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/inv/sync-jobs/${jobId}/events${query}`,
    {
      headers: buildAuthHeaders()
    },
    '获取同步任务日志失败'
  );
};
