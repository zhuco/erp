import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  assignMockStocktakeTask,
  cancelMockStocktakeTask,
  claimMockStocktakeTask,
  generateMockStocktakeTasks,
  getMockStocktakeTask,
  getMockStocktakeTaskRule,
  listMockStocktakeTasks,
  printMockStocktakeTasks,
  submitMockStocktakeTask,
  updateMockStocktakeTaskItems,
  updateMockStocktakeTaskRule
} from '@/api/mocks';

export type StocktakeTaskStatus = 'PENDING' | 'IN_PROGRESS' | 'SUBMITTED' | 'CANCELLED';
export type StocktakeTaskSplitDimension = 'AREA' | 'AISLE' | 'LOCATION_TYPE';
export type StocktakeTaskAssignMode = 'CLAIM' | 'ASSIGN';

export type StocktakeTaskRule = {
  split_dimension: StocktakeTaskSplitDimension;
  assignment_mode: StocktakeTaskAssignMode;
};

export type StocktakeTaskSummary = {
  task_id: string;
  count_id: string;
  warehouse_id: string;
  range_desc: string;
  assignee?: string | null;
  status: StocktakeTaskStatus;
  completed_at?: string | null;
  created_at: string;
  updated_at: string;
};

export type StocktakeTaskItem = {
  task_item_id: string;
  sku_id: string;
  barcode?: string | null;
  location_id?: string | null;
  system_qty: number;
  counted_qty: number;
};

export type StocktakeTaskDetail = {
  task: StocktakeTaskSummary;
  items: StocktakeTaskItem[];
  location_enabled: boolean;
};

export type StocktakeTaskListQuery = {
  count_id?: string;
  warehouse_id?: string;
  status?: StocktakeTaskStatus;
  assignee?: string;
  task_id?: string;
  offset?: number;
  limit?: number;
};

export type StocktakeTaskListResponse = {
  items: StocktakeTaskSummary[];
  total: number;
};

export type StocktakeTaskGeneratePayload = {
  count_id: string;
  split_dimension?: StocktakeTaskSplitDimension;
  assignment_mode?: StocktakeTaskAssignMode;
};

export type StocktakeTaskAssignPayload = {
  assignee: string;
};

export type StocktakeTaskItemUpdate = {
  task_item_id: string;
  counted_qty: number;
};

export type StocktakeTaskSubmitPayload = {
  items?: StocktakeTaskItemUpdate[];
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

const buildQueryString = (query: StocktakeTaskListQuery): string => {
  const params = new URLSearchParams();
  if (query.count_id) {
    params.set('count_id', query.count_id);
  }
  if (query.warehouse_id) {
    params.set('warehouse_id', query.warehouse_id);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.assignee) {
    params.set('assignee', query.assignee);
  }
  if (query.task_id) {
    params.set('task_id', query.task_id);
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

export const getStocktakeTaskRule = async (tenantId: string): Promise<StocktakeTaskRule> => {
  if (useMocks) {
    return getMockStocktakeTaskRule();
  }
  return requestJson<StocktakeTaskRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stocktake-tasks/rule`,
    { headers: buildAuthHeaders() },
    '获取盘点任务规则失败'
  );
};

export const updateStocktakeTaskRule = async (
  tenantId: string,
  payload: StocktakeTaskRule
): Promise<StocktakeTaskRule> => {
  if (useMocks) {
    return updateMockStocktakeTaskRule(payload);
  }
  return requestJson<StocktakeTaskRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stocktake-tasks/rule`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新盘点任务规则失败'
  );
};

export const listStocktakeTasks = async (
  tenantId: string,
  query: StocktakeTaskListQuery
): Promise<StocktakeTaskListResponse> => {
  if (useMocks) {
    return listMockStocktakeTasks(query);
  }
  return requestJson<StocktakeTaskListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stocktake-tasks${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取盘点任务失败'
  );
};

export const getStocktakeTask = async (
  tenantId: string,
  taskId: string
): Promise<StocktakeTaskDetail> => {
  if (useMocks) {
    return getMockStocktakeTask(taskId);
  }
  return requestJson<StocktakeTaskDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stocktake-tasks/${taskId}`,
    { headers: buildAuthHeaders() },
    '获取盘点任务详情失败'
  );
};

export const generateStocktakeTasks = async (
  tenantId: string,
  payload: StocktakeTaskGeneratePayload
): Promise<StocktakeTaskListResponse> => {
  if (useMocks) {
    return generateMockStocktakeTasks(payload);
  }
  return requestJson<StocktakeTaskListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stocktake-tasks`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '生成盘点任务失败'
  );
};

export const assignStocktakeTask = async (
  tenantId: string,
  taskId: string,
  payload: StocktakeTaskAssignPayload
): Promise<StocktakeTaskSummary> => {
  if (useMocks) {
    return assignMockStocktakeTask(taskId, payload);
  }
  return requestJson<StocktakeTaskSummary>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stocktake-tasks/${taskId}/assign`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '指派盘点任务失败'
  );
};

export const claimStocktakeTask = async (
  tenantId: string,
  taskId: string
): Promise<StocktakeTaskSummary> => {
  if (useMocks) {
    return claimMockStocktakeTask(taskId);
  }
  return requestJson<StocktakeTaskSummary>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stocktake-tasks/${taskId}/claim`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '领取盘点任务失败'
  );
};

export const cancelStocktakeTask = async (
  tenantId: string,
  taskId: string
): Promise<StocktakeTaskSummary> => {
  if (useMocks) {
    return cancelMockStocktakeTask(taskId);
  }
  return requestJson<StocktakeTaskSummary>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stocktake-tasks/${taskId}/cancel`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '作废盘点任务失败'
  );
};

export const updateStocktakeTaskItems = async (
  tenantId: string,
  taskId: string,
  items: StocktakeTaskItemUpdate[]
): Promise<StocktakeTaskDetail> => {
  if (useMocks) {
    return updateMockStocktakeTaskItems(taskId, items);
  }
  return requestJson<StocktakeTaskDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stocktake-tasks/${taskId}/items`,
    {
      method: 'PATCH',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify({ items })
    },
    '更新盘点任务明细失败'
  );
};

export const submitStocktakeTask = async (
  tenantId: string,
  taskId: string,
  payload: StocktakeTaskSubmitPayload
): Promise<StocktakeTaskDetail> => {
  if (useMocks) {
    return submitMockStocktakeTask(taskId, payload.items);
  }
  return requestJson<StocktakeTaskDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stocktake-tasks/${taskId}/submit`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '提交盘点任务失败'
  );
};

export const printStocktakeTasks = async (
  tenantId: string,
  taskIds: string[]
): Promise<void> => {
  if (useMocks) {
    return printMockStocktakeTasks(taskIds);
  }
  await requestJson<void>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stocktake-tasks/print`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify({ task_ids: taskIds })
    },
    '打印盘点任务失败'
  );
};
