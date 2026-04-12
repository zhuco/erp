import { apiClient, unwrapApiResponse } from '@/api/client';
import { useMocks } from '@/utils/env';
import {
  generateMockSkuSerials,
  getMockSkuSerialList,
  printMockSkuSerials
} from '@/api/mocks';

import type { components, paths } from './generated/schema';

export type SkuSerial = components['schemas']['SkuSerial'];
export type SkuSerialListResponse = components['schemas']['SkuSerialListResponse'];
export type SkuSerialListQuery = NonNullable<
  paths['/v1/tenants/{tenant_id}/sku-serials']['get']['parameters']['query']
>;
export type SkuSerialGenerateRequest = {
  sku_id: string;
  prefix?: string;
  rule_id?: string;
  count: number;
};
export type SkuSerialPrintRequest = components['schemas']['SkuSerialPrintRequest'];
export type PrintJob = components['schemas']['PrintJob'];

export const listSkuSerials = async (
  tenantId: string,
  query: SkuSerialListQuery
): Promise<SkuSerialListResponse> => {
  if (useMocks) {
    return getMockSkuSerialList(query);
  }
  const result = await apiClient.GET('/v1/tenants/{tenant_id}/sku-serials', {
    params: {
      path: { tenant_id: tenantId },
      query
    }
  });
  return unwrapApiResponse(result, '获取唯一码失败');
};

export const generateSkuSerials = async (
  tenantId: string,
  payload: SkuSerialGenerateRequest
): Promise<SkuSerial[]> => {
  if (useMocks) {
    return generateMockSkuSerials(payload);
  }
  const result = await apiClient.POST('/v1/tenants/{tenant_id}/sku-serials/generate', {
    params: {
      path: { tenant_id: tenantId }
    },
    body: payload
  });
  return unwrapApiResponse(result, '生成唯一码失败');
};

export const printSkuSerials = async (
  tenantId: string,
  payload: SkuSerialPrintRequest
): Promise<PrintJob> => {
  if (useMocks) {
    return printMockSkuSerials(payload);
  }
  const result = await apiClient.POST('/v1/tenants/{tenant_id}/sku-serials/print', {
    params: {
      path: { tenant_id: tenantId }
    },
    body: payload
  });
  return unwrapApiResponse(result, '打印唯一码失败');
};
