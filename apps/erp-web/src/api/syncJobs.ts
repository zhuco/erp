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
    `${apiBaseUrl}/v1/tenants/${tenantId}/channels/shops/${shopId}/sync-jobs/run`,
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
