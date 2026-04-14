import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import { getMockSyncJobList, runMockSyncJob } from '@/api/mocks';

export type SyncJob = {
  job_id: string;
  tenant_id: string;
  shop_id: string;
  job_type: string;
  cursor?: string | null;
  last_time?: string | null;
  status: string;
  last_error: string;
  created_at: string;
  updated_at: string;
};

export type SyncJobListQuery = {
  shop_id?: string;
  job_type?: string;
  status?: string;
  offset?: number;
  limit?: number;
};

export type SyncJobListResponse = {
  items: SyncJob[];
  total: number;
};

export type SyncJobRunPayload = {
  job_type: string;
  cursor?: string;
  last_time?: string;
};

export type PlatformOrderManualDownloadPayload = {
  cursor?: string;
  watermark?: string;
  last_time?: string;
};

export type PlatformOrderTimeRangeDownloadPayload = {
  start_time: string;
  end_time: string;
  cursor?: string;
};

export type PlatformOrderNoDownloadPayload = {
  order_nos: string[];
};

export type PlatformOrderDownloadResponse = {
  job: SyncJob;
  criteria: Record<string, unknown>;
};

export type PlatformOrderNoDownloadItem = {
  platform_order_id: string;
  success: boolean;
  order_id?: string | null;
  message?: string | null;
};

export type PlatformOrderNoDownloadResponse = {
  total: number;
  success_count: number;
  failure_count: number;
  items: PlatformOrderNoDownloadItem[];
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

const buildQueryString = (query: SyncJobListQuery): string => {
  const params = new URLSearchParams();
  if (query.shop_id) {
    params.set('shop_id', query.shop_id);
  }
  if (query.job_type) {
    params.set('job_type', query.job_type);
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

const buildShopBasePath = (tenantId: string, shopId: string) =>
  `${apiBaseUrl}/v1/tenants/${tenantId}/channels/shops/${shopId}`;

export const listSyncJobs = async (
  tenantId: string,
  query: SyncJobListQuery
): Promise<SyncJobListResponse> => {
  if (useMocks) {
    return getMockSyncJobList(query);
  }
  return requestJson<SyncJobListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/sync-jobs${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取同步任务失败'
  );
};

export const runSyncJob = async (
  tenantId: string,
  shopId: string,
  payload: SyncJobRunPayload
): Promise<SyncJob> => {
  if (useMocks) {
    return runMockSyncJob(shopId, payload);
  }
  return requestJson<SyncJob>(
    `${buildShopBasePath(tenantId, shopId)}/sync-jobs/run`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '触发同步任务失败'
  );
};

export const downloadPlatformOrdersManual = async (
  tenantId: string,
  shopId: string,
  payload: PlatformOrderManualDownloadPayload
): Promise<PlatformOrderDownloadResponse> => {
  if (useMocks) {
    const job = runMockSyncJob(shopId, {
      job_type: 'order_sync',
      cursor: payload.cursor,
      last_time: payload.watermark ?? payload.last_time
    });
    return {
      job,
      criteria: {
        mode: 'MANUAL',
        cursor: payload.cursor ?? null,
        watermark: payload.watermark ?? payload.last_time ?? null
      }
    };
  }

  return requestJson<PlatformOrderDownloadResponse>(
    `${buildShopBasePath(tenantId, shopId)}/platform-orders/download/manual`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '触发手动下载失败'
  );
};

export const downloadPlatformOrdersByTimeRange = async (
  tenantId: string,
  shopId: string,
  payload: PlatformOrderTimeRangeDownloadPayload
): Promise<PlatformOrderDownloadResponse> => {
  if (useMocks) {
    const job = runMockSyncJob(shopId, {
      job_type: 'order_sync',
      cursor: payload.cursor,
      last_time: payload.start_time
    });
    return {
      job,
      criteria: {
        mode: 'TIME_RANGE',
        start_time: payload.start_time,
        end_time: payload.end_time,
        cursor: payload.cursor ?? null
      }
    };
  }

  return requestJson<PlatformOrderDownloadResponse>(
    `${buildShopBasePath(tenantId, shopId)}/platform-orders/download/by-time-range`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '按时间段下载失败'
  );
};

export const downloadPlatformOrdersByOrderNos = async (
  tenantId: string,
  shopId: string,
  payload: PlatformOrderNoDownloadPayload
): Promise<PlatformOrderNoDownloadResponse> => {
  if (useMocks) {
    const orderNos = (payload.order_nos ?? []).map((value) => value.trim()).filter(Boolean);
    const items = orderNos.map((orderNo, index) => ({
      platform_order_id: orderNo,
      success: true,
      order_id: `ORDER-${shopId}-${index + 1}`,
      message: null
    }));
    return {
      total: items.length,
      success_count: items.length,
      failure_count: 0,
      items
    };
  }

  return requestJson<PlatformOrderNoDownloadResponse>(
    `${buildShopBasePath(tenantId, shopId)}/platform-orders/download/by-order-nos`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '按订单号下载失败'
  );
};
