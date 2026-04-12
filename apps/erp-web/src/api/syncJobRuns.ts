import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import { getMockSyncJobRunList } from '@/api/mocks';

export type SyncJobRun = {
  run_id: string;
  tenant_id: string;
  job_id: string;
  shop_id: string;
  job_type: string;
  status: string;
  item_count: number;
  duration_ms: number;
  error: string;
  started_at: string;
  finished_at: string;
};

export type SyncJobRunListQuery = {
  job_id?: string;
  shop_id?: string;
  job_type?: string;
  status?: string;
  started_from?: string;
  started_to?: string;
  offset?: number;
  limit?: number;
};

export type SyncJobRunListResponse = {
  items: SyncJobRun[];
  total: number;
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

const buildQueryString = (query: SyncJobRunListQuery): string => {
  const params = new URLSearchParams();
  if (query.job_id) {
    params.set('job_id', query.job_id);
  }
  if (query.shop_id) {
    params.set('shop_id', query.shop_id);
  }
  if (query.job_type) {
    params.set('job_type', query.job_type);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.started_from) {
    params.set('started_from', query.started_from);
  }
  if (query.started_to) {
    params.set('started_to', query.started_to);
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

export const listSyncJobRuns = async (
  tenantId: string,
  query: SyncJobRunListQuery
): Promise<SyncJobRunListResponse> => {
  if (useMocks) {
    return getMockSyncJobRunList(query);
  }
  return requestJson<SyncJobRunListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/sync-job-runs${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取同步运行记录失败'
  );
};
