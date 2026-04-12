import { apiClient, unwrapApiResponse } from '@/api/client';

import type { components, paths } from './generated/schema';

export type ExportJob = components['schemas']['ExportJob'];
export type ExportJobListResponse = components['schemas']['ExportJobListResponse'];
export type ExportJobCreateRequest = components['schemas']['ExportJobCreateRequest'];
export type ExportJobListQuery = NonNullable<
  paths['/exports/jobs']['get']['parameters']['query']
>;

export const listExportJobs = async (
  query: ExportJobListQuery
): Promise<ExportJobListResponse> => {
  const result = await apiClient.GET('/exports/jobs', { params: { query } });
  return unwrapApiResponse(result, '获取导出任务失败');
};

export const createExportJob = async (
  payload: ExportJobCreateRequest
): Promise<ExportJob> => {
  const result = await apiClient.POST('/exports/jobs', { body: payload });
  return unwrapApiResponse(result, '创建导出任务失败');
};

export const getExportJob = async (jobId: string): Promise<ExportJob> => {
  const result = await apiClient.GET('/exports/jobs/{id}', {
    params: { path: { id: jobId } }
  });
  return unwrapApiResponse(result, '获取导出任务失败');
};
