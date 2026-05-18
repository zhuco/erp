import { apiBaseUrl, useMocks } from '@/utils/env';
import { apiClient, buildAuthHeaders, fetchWithAuth, unwrapApiResponse } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockSku,
  getMockSkuDetail,
  getMockSkuList,
  searchMockSkus,
  updateMockSku,
} from '@/api/mocks';

import type { components, paths } from './generated/schema';

export type Sku = components['schemas']['Sku'];
export type SkuDetailResponse = components['schemas']['SkuDetailResponse'];
export type SkuListResponse = components['schemas']['SkuListResponse'];
export type SkuListQuery = NonNullable<
  paths['/v1/tenants/{tenant_id}/skus']['get']['parameters']['query']
>;
export type SkuKind = 'NORMAL' | 'KIT';
export type SkuSearchItem = components['schemas']['SkuSearchItem'];
export type SkuSearchResponse = components['schemas']['SkuSearchResponse'];
export type SkuCreatePayload = components['schemas']['SkuCreateRequest'];
export type SkuUpdatePayload = components['schemas']['SkuUpdateRequest'];
export type SkuImportPrecheckRequest = components['schemas']['SkuImportPrecheckRequest'];
export type SkuImportRequest = components['schemas']['SkuImportRequest'];
export type SkuImportPrecheckResponse = components['schemas']['SkuImportPrecheckResponse'];
export type SkuImportUploadResponse = {
  file_id: string;
  file_name: string;
  file_type: string;
};
export type SkuImportDuplicateStrategy = 'skip' | 'overwrite';
export type SkuImportJobCreateRequest = {
  file_id: string;
  file_name?: string;
  file_type?: string;
  duplicate_strategy?: SkuImportDuplicateStrategy;
};
export type SkuImportJobResponse = {
  job_id: string;
  status: string;
  progress: number;
  total_rows: number;
  success_rows: number;
  failed_rows: number;
  file_name?: string | null;
  file_type?: string | null;
  duplicate_strategy?: string | null;
  failure_log_url?: string | null;
  created_at: string;
  updated_at: string;
};
export type SkuImportFailureRow = {
  row: number;
  sku_code?: string | null;
  field?: string | null;
  message: string;
};
export type SkuImportFailureListResponse = {
  items: SkuImportFailureRow[];
  total: number;
};
export type SkuImportFailureListQuery = {
  offset?: number;
  limit?: number;
};
export type SkuCostLedger = components['schemas']['SkuCostLedger'];
export type SkuCostLedgerListResponse = components['schemas']['SkuCostLedgerListResponse'];
export type SkuCostAdjustRequest = components['schemas']['SkuCostAdjustRequest'];
export type SkuCostLedgerListQuery = {
  sku_id?: string;
  start_at?: string;
  end_at?: string;
  offset?: number;
  limit?: number;
};

export const listSkus = async (
  tenantId: string,
  query: SkuListQuery
): Promise<SkuListResponse> => {
  if (useMocks) {
    return getMockSkuList(query);
  }
  const result = await apiClient.GET('/v1/tenants/{tenant_id}/skus', {
    params: {
      path: { tenant_id: tenantId },
      query
    }
  });
  return unwrapApiResponse(result, '获取 SKU 列表失败');
};

export const createSku = async (tenantId: string, payload: SkuCreatePayload): Promise<Sku> => {
  if (useMocks) {
    return createMockSku(payload);
  }
  return requestJson<Sku>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/skus`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建 SKU 失败'
  );
};

export const updateSku = async (
  tenantId: string,
  skuId: string,
  payload: SkuUpdatePayload
): Promise<Sku> => {
  if (useMocks) {
    return updateMockSku(skuId, payload);
  }
  return requestJson<Sku>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/skus/${skuId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新 SKU 失败'
  );
};

export const getSkuDetail = async (
  tenantId: string,
  skuId: string
): Promise<SkuDetailResponse> => {
  if (useMocks) {
    return getMockSkuDetail(skuId);
  }
  const result = await apiClient.GET('/v1/tenants/{tenant_id}/skus/{sku_id}', {
    params: {
      path: { tenant_id: tenantId, sku_id: skuId }
    }
  });
  return unwrapApiResponse(result, '获取 SKU 详情失败');
};

export const deleteSku = async (tenantId: string, skuId: string): Promise<Sku> => {
  const result = await apiClient.DELETE('/v1/tenants/{tenant_id}/skus/{sku_id}', {
    params: {
      path: { tenant_id: tenantId, sku_id: skuId }
    }
  });
  return unwrapApiResponse(result, '删除 SKU 失败');
};

export const copySku = async (tenantId: string, skuId: string): Promise<Sku> => {
  const result = await apiClient.POST('/v1/tenants/{tenant_id}/skus/{sku_id}/copy', {
    params: {
      path: { tenant_id: tenantId, sku_id: skuId }
    }
  });
  return unwrapApiResponse(result, '复制 SKU 失败');
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

const buildSearchQuery = (keyword: string, skuKind?: SkuKind) => {
  const params = new URLSearchParams();
  if (keyword.trim()) {
    params.set('keyword', keyword.trim());
  }
  if (skuKind) {
    params.set('sku_kind', skuKind);
  }
  const serialized = params.toString();
  return serialized ? `?${serialized}` : '';
};

const normalizeSearchResponse = (
  payload: SkuSearchResponse | { data: SkuSearchItem[] }
): SkuSearchResponse => {
  if ('items' in payload) {
    return payload;
  }
  return { items: payload.data };
};

export const searchSkus = async (
  tenantId: string,
  keyword: string,
  skuKind?: SkuKind
): Promise<SkuSearchResponse> => {
  if (useMocks) {
    return searchMockSkus(keyword, skuKind);
  }
  const response = await requestJson<SkuSearchResponse | { data: SkuSearchItem[] }>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/skus/search${buildSearchQuery(keyword, skuKind)}`,
    { headers: buildAuthHeaders() },
    '搜索 SKU 失败'
  );
  return normalizeSearchResponse(response);
};

export const precheckSkuImport = async (
  tenantId: string,
  payload: SkuImportPrecheckRequest
): Promise<SkuImportPrecheckResponse> => {
  const result = await apiClient.POST('/v1/tenants/{tenant_id}/skus/import/precheck', {
    params: {
      path: { tenant_id: tenantId }
    },
    body: payload
  });
  return unwrapApiResponse(result, 'SKU 导入预检失败');
};

export const createSkuImport = async (
  tenantId: string,
  payload: SkuImportRequest
): Promise<components['schemas']['ExportJob']> => {
  const result = await apiClient.POST('/v1/tenants/{tenant_id}/skus/import', {
    params: {
      path: { tenant_id: tenantId }
    },
    body: payload
  });
  return unwrapApiResponse(result, '创建 SKU 导入任务失败');
};

export const downloadSkuImportTemplate = async (tenantId: string): Promise<Blob> => {
  const response = await fetchWithAuth(
    `${apiBaseUrl}/v1/tenants/${tenantId}/skus/import/template`,
    { headers: buildAuthHeaders() }
  );
  if (!response.ok) {
    handleApiError(response.status);
    throw createApiError('下载 SKU 导入模板失败', response.status);
  }
  return response.blob();
};

export const uploadSkuImportFile = async (
  tenantId: string,
  file: File
): Promise<SkuImportUploadResponse> => {
  const formData = new FormData();
  formData.append('file', file);
  const response = await fetchWithAuth(`${apiBaseUrl}/v1/tenants/${tenantId}/skus/import/files`, {
    method: 'POST',
    body: formData
  });
  let payload: unknown = undefined;
  try {
    payload = await response.json();
  } catch {
    payload = undefined;
  }
  if (!response.ok) {
    handleApiError(response.status);
    throw createApiError('上传 SKU 导入文件失败', response.status, payload);
  }
  return payload as SkuImportUploadResponse;
};

export const createSkuImportJob = async (
  tenantId: string,
  payload: SkuImportJobCreateRequest
): Promise<SkuImportJobResponse> => {
  const response = await fetchWithAuth(`${apiBaseUrl}/v1/tenants/${tenantId}/skus/import/jobs`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...buildAuthHeaders()
    },
    body: JSON.stringify(payload)
  });
  let result: unknown = undefined;
  try {
    result = await response.json();
  } catch {
    result = undefined;
  }
  if (!response.ok) {
    handleApiError(response.status);
    throw createApiError('创建 SKU 导入任务失败', response.status, result);
  }
  return result as SkuImportJobResponse;
};

export const getSkuImportJob = async (
  tenantId: string,
  jobId: string
): Promise<SkuImportJobResponse> => {
  const response = await fetchWithAuth(
    `${apiBaseUrl}/v1/tenants/${tenantId}/skus/import/jobs/${jobId}`,
    { headers: buildAuthHeaders() }
  );
  let result: unknown = undefined;
  try {
    result = await response.json();
  } catch {
    result = undefined;
  }
  if (!response.ok) {
    handleApiError(response.status);
    throw createApiError('获取 SKU 导入任务失败', response.status, result);
  }
  return result as SkuImportJobResponse;
};

export const listSkuImportFailures = async (
  tenantId: string,
  jobId: string,
  query: SkuImportFailureListQuery
): Promise<SkuImportFailureListResponse> => {
  const params = new URLSearchParams();
  if (query.offset != null) {
    params.set('offset', query.offset.toString());
  }
  if (query.limit != null) {
    params.set('limit', query.limit.toString());
  }
  const search = params.toString();
  const response = await fetchWithAuth(
    `${apiBaseUrl}/v1/tenants/${tenantId}/skus/import/jobs/${jobId}/failures${
      search ? `?${search}` : ''
    }`,
    { headers: buildAuthHeaders() }
  );
  let result: unknown = undefined;
  try {
    result = await response.json();
  } catch {
    result = undefined;
  }
  if (!response.ok) {
    handleApiError(response.status);
    throw createApiError('获取 SKU 导入失败明细失败', response.status, result);
  }
  return result as SkuImportFailureListResponse;
};

export const downloadSkuImportFailureLog = async (
  tenantId: string,
  jobId: string
): Promise<Blob> => {
  const response = await fetchWithAuth(
    `${apiBaseUrl}/v1/tenants/${tenantId}/skus/import/jobs/${jobId}/failures/file`,
    { headers: buildAuthHeaders() }
  );
  if (!response.ok) {
    handleApiError(response.status);
    throw createApiError('下载 SKU 导入失败日志失败', response.status);
  }
  return response.blob();
};

export const listSkuCostLedgers = async (
  tenantId: string,
  query: SkuCostLedgerListQuery
): Promise<SkuCostLedgerListResponse> => {
  const result = await apiClient.GET('/v1/tenants/{tenant_id}/sku-cost-ledgers', {
    params: {
      path: { tenant_id: tenantId },
      query
    }
  });
  return unwrapApiResponse(result, '获取 SKU 成本流水失败');
};

export const adjustSkuCost = async (
  tenantId: string,
  payload: SkuCostAdjustRequest
): Promise<SkuCostLedger> => {
  const result = await apiClient.POST('/v1/tenants/{tenant_id}/sku-cost-ledgers/adjust', {
    params: {
      path: { tenant_id: tenantId }
    },
    body: payload
  });
  return unwrapApiResponse(result, '调整 SKU 成本失败');
};
