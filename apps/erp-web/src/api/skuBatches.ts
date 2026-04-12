import { apiClient, unwrapApiResponse } from '@/api/client';
import { useMocks } from '@/utils/env';
import {
  createMockSkuBatch,
  getMockSkuBatchList,
  updateMockSkuBatch
} from '@/api/mocks';

import type { components, paths } from './generated/schema';

export type SkuBatch = components['schemas']['SkuBatch'];
export type SkuBatchListResponse = components['schemas']['SkuBatchListResponse'];
export type SkuBatchListQuery = NonNullable<
  paths['/v1/tenants/{tenant_id}/sku-batches']['get']['parameters']['query']
>;
export type SkuBatchCreateRequest = components['schemas']['SkuBatchCreateRequest'];
export type SkuBatchUpdateRequest = components['schemas']['SkuBatchUpdateRequest'];

export const listSkuBatches = async (
  tenantId: string,
  query: SkuBatchListQuery
): Promise<SkuBatchListResponse> => {
  if (useMocks) {
    return getMockSkuBatchList(query);
  }
  const result = await apiClient.GET('/v1/tenants/{tenant_id}/sku-batches', {
    params: {
      path: { tenant_id: tenantId },
      query
    }
  });
  return unwrapApiResponse(result, '获取批次失败');
};

export const createSkuBatch = async (
  tenantId: string,
  payload: SkuBatchCreateRequest
): Promise<SkuBatch> => {
  if (useMocks) {
    return createMockSkuBatch(payload);
  }
  const result = await apiClient.POST('/v1/tenants/{tenant_id}/sku-batches', {
    params: {
      path: { tenant_id: tenantId }
    },
    body: payload
  });
  return unwrapApiResponse(result, '创建批次失败');
};

export const updateSkuBatch = async (
  tenantId: string,
  batchId: string,
  payload: SkuBatchUpdateRequest
): Promise<SkuBatch> => {
  if (useMocks) {
    return updateMockSkuBatch(batchId, payload);
  }
  const result = await apiClient.PUT('/v1/tenants/{tenant_id}/sku-batches/{batch_id}', {
    params: {
      path: { tenant_id: tenantId, batch_id: batchId }
    },
    body: payload
  });
  return unwrapApiResponse(result, '更新批次失败');
};
