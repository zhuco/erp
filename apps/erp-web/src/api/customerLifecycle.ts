import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockCustomerLifecycleTask,
  deleteMockCustomerLifecycleTask,
  getMockCustomerLifecycleTasks,
  updateMockCustomerLifecycleTask
} from '@/api/mocks';

export type CustomerLifecycleStage = 'NEW' | 'ACTIVE' | 'DORMANT' | 'CHURNED';

export type CustomerLifecycleTask = {
  task_id: string;
  stage: CustomerLifecycleStage;
  strategy: string;
  trigger_rule?: string | null;
  owner?: string | null;
  status?: string | null;
  last_triggered_at?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export type CustomerLifecycleTaskListQuery = {
  keyword?: string;
  stage?: CustomerLifecycleStage;
  status?: string;
  offset?: number;
  limit?: number;
};

export type CustomerLifecycleTaskListResponse = {
  items: CustomerLifecycleTask[];
  total: number;
};

export type CustomerLifecycleTaskCreatePayload = {
  stage: CustomerLifecycleStage;
  strategy: string;
  trigger_rule?: string;
  owner?: string;
  status?: string;
};

export type CustomerLifecycleTaskUpdatePayload = {
  stage?: CustomerLifecycleStage;
  strategy?: string;
  trigger_rule?: string;
  owner?: string;
  status?: string;
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

const buildQueryString = (query: CustomerLifecycleTaskListQuery): string => {
  const params = new URLSearchParams();
  if (query.keyword) {
    params.set('keyword', query.keyword);
  }
  if (query.stage) {
    params.set('stage', query.stage);
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

export const listCustomerLifecycleTasks = async (
  tenantId: string,
  query: CustomerLifecycleTaskListQuery
): Promise<CustomerLifecycleTaskListResponse> => {
  if (useMocks) {
    return getMockCustomerLifecycleTasks(query);
  }
  return requestJson<CustomerLifecycleTaskListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/crm/lifecycle-tasks${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取生命周期任务失败'
  );
};

export const createCustomerLifecycleTask = async (
  tenantId: string,
  payload: CustomerLifecycleTaskCreatePayload
): Promise<CustomerLifecycleTask> => {
  if (useMocks) {
    return createMockCustomerLifecycleTask(payload);
  }
  return requestJson<CustomerLifecycleTask>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/crm/lifecycle-tasks`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建生命周期任务失败'
  );
};

export const updateCustomerLifecycleTask = async (
  tenantId: string,
  taskId: string,
  payload: CustomerLifecycleTaskUpdatePayload
): Promise<CustomerLifecycleTask> => {
  if (useMocks) {
    return updateMockCustomerLifecycleTask(taskId, payload);
  }
  return requestJson<CustomerLifecycleTask>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/crm/lifecycle-tasks/${taskId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新生命周期任务失败'
  );
};

export const deleteCustomerLifecycleTask = async (
  tenantId: string,
  taskId: string
): Promise<CustomerLifecycleTask> => {
  if (useMocks) {
    return deleteMockCustomerLifecycleTask(taskId);
  }
  return requestJson<CustomerLifecycleTask>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/crm/lifecycle-tasks/${taskId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除生命周期任务失败'
  );
};
