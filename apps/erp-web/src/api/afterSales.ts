import { apiClient, unwrapApiResponse } from '@/api/client';
import { getMockAfterSaleDetail, getMockAfterSaleRaw, getMockAfterSalesList } from '@/api/mocks';
import { useMocks } from '@/utils/env';

import type { components, paths } from './generated/schema';

export type AfterSaleCaseSummary = components['schemas']['AfterSaleCaseSummary'];
export type AfterSaleCaseDetail = components['schemas']['AfterSaleCaseDetail'];
export type AfterSaleListResponse = components['schemas']['AfterSaleCaseListResponse'];
export type AfterSaleListQuery = NonNullable<
  paths['/v1/tenants/{tenant_id}/after-sale-cases']['get']['parameters']['query']
>;

export const listAfterSales = async (
  tenantId: string,
  query: AfterSaleListQuery
): Promise<AfterSaleListResponse> => {
  if (useMocks) {
    return getMockAfterSalesList(query);
  }
  const result = await apiClient.GET(
    '/v1/tenants/{tenant_id}/after-sale-cases',
    {
      params: {
        path: { tenant_id: tenantId },
        query
      }
    }
  );
  return unwrapApiResponse(result, '获取售后列表失败');
};

export const getAfterSaleCaseDetail = async (
  tenantId: string,
  caseId: string
): Promise<AfterSaleCaseDetail> => {
  if (useMocks) {
    return getMockAfterSaleDetail(caseId);
  }
  const result = await apiClient.GET(
    '/v1/tenants/{tenant_id}/after-sale-cases/{case_id}',
    {
      params: {
        path: { tenant_id: tenantId, case_id: caseId }
      }
    }
  );
  return unwrapApiResponse(result, '获取售后详情失败');
};

export const getAfterSaleCaseRaw = async (
  tenantId: string,
  caseId: string
): Promise<Record<string, unknown>> => {
  if (useMocks) {
    return getMockAfterSaleRaw(caseId);
  }
  const result = await apiClient.GET(
    '/v1/tenants/{tenant_id}/after-sale-cases/{case_id}/raw',
    {
      params: {
        path: { tenant_id: tenantId, case_id: caseId }
      }
    }
  );
  return unwrapApiResponse(result, '获取售后原始数据失败') as Record<string, unknown>;
};
