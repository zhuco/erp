import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  applyMockWmsSlottingSuggestion,
  assignMockWmsLaborTask,
  checkInMockWmsDockAppointment,
  claimMockWmsReplenishmentTask,
  completeMockWmsLaborTask,
  completeMockWmsReplenishmentTask,
  departMockWmsDockAppointment,
  listMockWmsDockAppointments,
  listMockWmsLaborTasks,
  listMockWmsReplenishmentTasks,
  listMockWmsSlottingSuggestions,
  rejectMockWmsSlottingSuggestion,
  startMockWmsDockLoading,
  startMockWmsLaborTask
} from './mocks/wmsExecution';

export type WmsLaborTaskStatus = 'UNASSIGNED' | 'ASSIGNED' | 'IN_PROGRESS' | 'DONE';
export type WmsSlottingSuggestionStatus = 'PENDING' | 'APPLIED' | 'REJECTED';
export type WmsReplenishmentTaskStatus = 'PENDING' | 'IN_PROGRESS' | 'DONE';
export type WmsDockAppointmentStatus = 'SCHEDULED' | 'CHECKED_IN' | 'LOADING' | 'DEPARTED';

export type WmsLaborTask = {
  task_id: string;
  warehouse_id: string;
  zone_code: string;
  task_type: 'PICK' | 'PUTAWAY' | 'COUNT';
  assignee?: string | null;
  planned_qty: number;
  finished_qty: number;
  status: WmsLaborTaskStatus;
  updated_at: string;
};

export type WmsSlottingSuggestion = {
  suggestion_id: string;
  warehouse_id: string;
  sku_id: string;
  from_location_id: string;
  to_location_id: string;
  reason: string;
  priority: 'HIGH' | 'MEDIUM' | 'LOW';
  status: WmsSlottingSuggestionStatus;
  updated_at: string;
};

export type WmsReplenishmentTask = {
  task_id: string;
  warehouse_id: string;
  sku_id: string;
  from_location_id: string;
  to_location_id: string;
  qty: number;
  assignee?: string | null;
  status: WmsReplenishmentTaskStatus;
  updated_at: string;
};

export type WmsDockAppointment = {
  appointment_id: string;
  warehouse_id: string;
  dock_no: string;
  carrier_name: string;
  vehicle_no: string;
  eta_at: string;
  status: WmsDockAppointmentStatus;
  updated_at: string;
};

export type WmsExecutionListResponse<T> = {
  items: T[];
  total: number;
};

export type WmsLaborTaskQuery = {
  warehouse_id?: string;
  status?: WmsLaborTaskStatus;
  offset?: number;
  limit?: number;
};

export type WmsSlottingSuggestionQuery = {
  warehouse_id?: string;
  status?: WmsSlottingSuggestionStatus;
  offset?: number;
  limit?: number;
};

export type WmsReplenishmentTaskQuery = {
  warehouse_id?: string;
  status?: WmsReplenishmentTaskStatus;
  offset?: number;
  limit?: number;
};

export type WmsDockAppointmentQuery = {
  warehouse_id?: string;
  status?: WmsDockAppointmentStatus;
  offset?: number;
  limit?: number;
};

const buildQueryString = (query: Record<string, string | number | undefined>): string => {
  const params = new URLSearchParams();
  Object.entries(query).forEach(([key, value]) => {
    if (value !== undefined) {
      params.set(key, String(value));
    }
  });
  const serialized = params.toString();
  return serialized ? `?${serialized}` : '';
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

export const listWmsLaborTasks = async (
  tenantId: string,
  query: WmsLaborTaskQuery
): Promise<WmsExecutionListResponse<WmsLaborTask>> => {
  if (useMocks) {
    void tenantId;
    return listMockWmsLaborTasks(query);
  }
  return requestJson<WmsExecutionListResponse<WmsLaborTask>>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/labor/tasks${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取 labor 任务失败'
  );
};

export const assignWmsLaborTask = async (
  tenantId: string,
  taskId: string,
  payload: { assignee: string }
): Promise<WmsLaborTask> => {
  if (useMocks) {
    void tenantId;
    return assignMockWmsLaborTask(taskId, payload);
  }
  return requestJson<WmsLaborTask>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/labor/tasks/${taskId}/assign`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '分配 labor 任务失败'
  );
};

export const startWmsLaborTask = async (tenantId: string, taskId: string): Promise<WmsLaborTask> => {
  if (useMocks) {
    void tenantId;
    return startMockWmsLaborTask(taskId);
  }
  return requestJson<WmsLaborTask>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/labor/tasks/${taskId}/start`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '开始 labor 任务失败'
  );
};

export const completeWmsLaborTask = async (
  tenantId: string,
  taskId: string
): Promise<WmsLaborTask> => {
  if (useMocks) {
    void tenantId;
    return completeMockWmsLaborTask(taskId);
  }
  return requestJson<WmsLaborTask>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/labor/tasks/${taskId}/complete`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '完成 labor 任务失败'
  );
};

export const listWmsSlottingSuggestions = async (
  tenantId: string,
  query: WmsSlottingSuggestionQuery
): Promise<WmsExecutionListResponse<WmsSlottingSuggestion>> => {
  if (useMocks) {
    void tenantId;
    return listMockWmsSlottingSuggestions(query);
  }
  return requestJson<WmsExecutionListResponse<WmsSlottingSuggestion>>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/slotting/suggestions${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取 slotting 建议失败'
  );
};

export const applyWmsSlottingSuggestion = async (
  tenantId: string,
  suggestionId: string
): Promise<WmsSlottingSuggestion> => {
  if (useMocks) {
    void tenantId;
    return applyMockWmsSlottingSuggestion(suggestionId);
  }
  return requestJson<WmsSlottingSuggestion>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/slotting/suggestions/${suggestionId}/apply`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '应用 slotting 建议失败'
  );
};

export const rejectWmsSlottingSuggestion = async (
  tenantId: string,
  suggestionId: string
): Promise<WmsSlottingSuggestion> => {
  if (useMocks) {
    void tenantId;
    return rejectMockWmsSlottingSuggestion(suggestionId);
  }
  return requestJson<WmsSlottingSuggestion>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/slotting/suggestions/${suggestionId}/reject`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '驳回 slotting 建议失败'
  );
};

export const listWmsReplenishmentTasks = async (
  tenantId: string,
  query: WmsReplenishmentTaskQuery
): Promise<WmsExecutionListResponse<WmsReplenishmentTask>> => {
  if (useMocks) {
    void tenantId;
    return listMockWmsReplenishmentTasks(query);
  }
  return requestJson<WmsExecutionListResponse<WmsReplenishmentTask>>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/replenishment/tasks${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取补货任务失败'
  );
};

export const claimWmsReplenishmentTask = async (
  tenantId: string,
  taskId: string,
  payload: { assignee: string }
): Promise<WmsReplenishmentTask> => {
  if (useMocks) {
    void tenantId;
    return claimMockWmsReplenishmentTask(taskId, payload);
  }
  return requestJson<WmsReplenishmentTask>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/replenishment/tasks/${taskId}/claim`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '认领补货任务失败'
  );
};

export const completeWmsReplenishmentTask = async (
  tenantId: string,
  taskId: string
): Promise<WmsReplenishmentTask> => {
  if (useMocks) {
    void tenantId;
    return completeMockWmsReplenishmentTask(taskId);
  }
  return requestJson<WmsReplenishmentTask>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/replenishment/tasks/${taskId}/complete`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '完成补货任务失败'
  );
};

export const listWmsDockAppointments = async (
  tenantId: string,
  query: WmsDockAppointmentQuery
): Promise<WmsExecutionListResponse<WmsDockAppointment>> => {
  if (useMocks) {
    void tenantId;
    return listMockWmsDockAppointments(query);
  }
  return requestJson<WmsExecutionListResponse<WmsDockAppointment>>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/dock/appointments${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取月台预约失败'
  );
};

export const checkInWmsDockAppointment = async (
  tenantId: string,
  appointmentId: string
): Promise<WmsDockAppointment> => {
  if (useMocks) {
    void tenantId;
    return checkInMockWmsDockAppointment(appointmentId);
  }
  return requestJson<WmsDockAppointment>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/dock/appointments/${appointmentId}/check-in`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '月台签到失败'
  );
};

export const startWmsDockLoading = async (
  tenantId: string,
  appointmentId: string
): Promise<WmsDockAppointment> => {
  if (useMocks) {
    void tenantId;
    return startMockWmsDockLoading(appointmentId);
  }
  return requestJson<WmsDockAppointment>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/dock/appointments/${appointmentId}/start-loading`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '月台装车开始失败'
  );
};

export const departWmsDockAppointment = async (
  tenantId: string,
  appointmentId: string
): Promise<WmsDockAppointment> => {
  if (useMocks) {
    void tenantId;
    return departMockWmsDockAppointment(appointmentId);
  }
  return requestJson<WmsDockAppointment>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/dock/appointments/${appointmentId}/depart`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '月台离场失败'
  );
};
