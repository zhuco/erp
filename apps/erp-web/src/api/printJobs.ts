import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockPrintJob,
  dispatchMockPrintJob,
  getMockPrintJobDetail,
  getMockPrintJobList,
  updateMockPrintJobStatus
} from '@/api/mocks';
import type { PrintShippingLabelPayload } from '@/features/printing/types';

export type PrintJobStatus = 'PENDING' | 'RUNNING' | 'DONE' | 'FAILED';
export type PrintJobDispatchStatus = 'QUEUED' | 'DISPATCHED' | 'PRINTED' | 'FAILED';

export type PrintJobEvent = {
  status: PrintJobStatus;
  error?: string | null;
  created_at: string;
  actor_type?: string | null;
};

export type PrintJobSummary = {
  job_id: string;
  job_type?: string | null;
  ref_type?: string | null;
  ref_id?: string | null;
  status: PrintJobStatus;
  dispatch_status?: PrintJobDispatchStatus | null;
  output_url?: string | null;
  attempts?: number | null;
  last_error?: string | null;
  device_fingerprint?: string | null;
  device_name?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export type PrintJobDetail = PrintJobSummary & {
  events: PrintJobEvent[];
  payload?: PrintShippingLabelPayload | Record<string, unknown> | null;
};

export type PrintJobListResponse = {
  items: PrintJobSummary[];
  total: number;
};

export type PrintJobListQuery = {
  status?: PrintJobStatus;
  job_type?: string;
  ref_type?: string;
  ref_id?: string;
  offset?: number;
  limit?: number;
};

export type CreatePrintJobPayload = {
  job_type: string;
  ref_type: string;
  ref_id: string;
  template_id?: string;
  device_fingerprint?: string;
  payload?: PrintShippingLabelPayload | Record<string, unknown>;
};

export type UpdatePrintJobPayload = {
  status?: PrintJobStatus;
  output_url?: string | null;
  last_error?: string | null;
  payload?: Record<string, unknown> | null;
};

type PrintJobListApiResponse = {
  data?: PrintJobSummary[];
  items?: PrintJobSummary[];
  total?: number;
};

type PrintJobDetailApiResponse = {
  data?: PrintJobDetail;
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

const buildQueryString = (query: PrintJobListQuery) => {
  const params = new URLSearchParams();
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.job_type) {
    params.set('job_type', query.job_type);
  }
  if (query.ref_type) {
    params.set('ref_type', query.ref_type);
  }
  if (query.ref_id) {
    params.set('ref_id', query.ref_id);
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

const normalizeListResponse = (
  payload: PrintJobListApiResponse | PrintJobListResponse
): PrintJobListResponse => {
  if ('items' in payload) {
    return {
      items: payload.items ?? [],
      total: payload.total ?? payload.items?.length ?? 0
    };
  }
  return {
    items: payload.data ?? [],
    total: payload.total ?? payload.data?.length ?? 0
  };
};

const normalizeDetailResponse = (
  payload: PrintJobDetail | PrintJobDetailApiResponse
): PrintJobDetail => {
  if ('job_id' in payload) {
    return payload;
  }
  if (payload.data) {
    return payload.data;
  }
  throw createApiError('获取打印任务失败');
};

export const listPrintJobs = async (
  tenantId: string,
  query: PrintJobListQuery
): Promise<PrintJobListResponse> => {
  if (useMocks) {
    return getMockPrintJobList(query);
  }
  const response = await requestJson<PrintJobListApiResponse | PrintJobListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/print-jobs${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取打印任务失败'
  );
  return normalizeListResponse(response);
};

export const getPrintJobDetail = async (
  tenantId: string,
  printJobId: string
): Promise<PrintJobDetail> => {
  if (useMocks) {
    return getMockPrintJobDetail(printJobId);
  }
  const response = await requestJson<PrintJobDetail | PrintJobDetailApiResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/print-jobs/${printJobId}`,
    { headers: buildAuthHeaders() },
    '获取打印任务详情失败'
  );
  return normalizeDetailResponse(response);
};

export const createPrintJob = async (
  tenantId: string,
  payload: CreatePrintJobPayload
): Promise<PrintJobDetail> => {
  if (useMocks) {
    return createMockPrintJob(payload);
  }
  const response = await requestJson<PrintJobDetail | PrintJobDetailApiResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/print-jobs`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建打印任务失败'
  );
  return normalizeDetailResponse(response);
};

export const updatePrintJobStatus = async (
  tenantId: string,
  printJobId: string,
  payload: UpdatePrintJobPayload
): Promise<PrintJobDetail> => {
  if (useMocks) {
    return updateMockPrintJobStatus(printJobId, payload);
  }
  const response = await requestJson<PrintJobDetail | PrintJobDetailApiResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/print-jobs/${printJobId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新打印任务失败'
  );
  return normalizeDetailResponse(response);
};

export const retryPrintJob = async (
  tenantId: string,
  printJobId: string
): Promise<PrintJobDetail> => {
  if (useMocks) {
    return updateMockPrintJobStatus(printJobId, { status: 'PENDING' });
  }
  const response = await requestJson<PrintJobDetail | PrintJobDetailApiResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/print-jobs/${printJobId}/retry`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      }
    },
    '重试打印任务失败'
  );
  return normalizeDetailResponse(response);
};

export const dispatchPrintJob = async (
  tenantId: string,
  printJobId: string
): Promise<PrintJobDetail> => {
  if (useMocks) {
    return dispatchMockPrintJob(printJobId);
  }
  const response = await requestJson<PrintJobDetail | PrintJobDetailApiResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/print-jobs/${printJobId}/dispatch`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      }
    },
    '派发打印任务失败'
  );
  return normalizeDetailResponse(response);
};
