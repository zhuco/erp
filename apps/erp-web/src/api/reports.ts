import { apiClient, unwrapApiResponse } from '@/api/client';

import type { components, paths } from './generated/schema';

export type DashboardReport = components['schemas']['DashboardReport'];
export type FulfillmentReport = components['schemas']['FulfillmentReport'];
export type InventoryReport = components['schemas']['InventoryReport'];
export type FinanceReport = components['schemas']['FinanceReport'];

export type DashboardQuery = NonNullable<
  paths['/reports/dashboard']['get']['parameters']['query']
>;
export type FulfillmentQuery = NonNullable<
  paths['/reports/fulfillment']['get']['parameters']['query']
>;
export type InventoryQuery = NonNullable<
  paths['/reports/inventory']['get']['parameters']['query']
>;
export type FinanceQuery = NonNullable<
  paths['/reports/finance']['get']['parameters']['query']
>;

export const getDashboardReport = async (query: DashboardQuery): Promise<DashboardReport> => {
  const result = await apiClient.GET('/reports/dashboard', { params: { query } });
  return unwrapApiResponse(result, '获取经营报表失败');
};

export const getFulfillmentReport = async (
  query: FulfillmentQuery
): Promise<FulfillmentReport> => {
  const result = await apiClient.GET('/reports/fulfillment', { params: { query } });
  return unwrapApiResponse(result, '获取履约报表失败');
};

export const getInventoryReport = async (query: InventoryQuery): Promise<InventoryReport> => {
  const result = await apiClient.GET('/reports/inventory', { params: { query } });
  return unwrapApiResponse(result, '获取库存报表失败');
};

export const getFinanceReport = async (query: FinanceQuery): Promise<FinanceReport> => {
  const result = await apiClient.GET('/reports/finance', { params: { query } });
  return unwrapApiResponse(result, '获取财务报表失败');
};
