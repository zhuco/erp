import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  addMockInstallationAppointment,
  createMockInstallationTask,
  listMockInstallationTasks,
  updateMockInstallationTask
} from '@/api/mocks';

export type LogisticsInstallationStatus = 'SCHEDULED' | 'IN_PROGRESS' | 'DONE' | 'EXCEPTION';
export type LogisticsAppointmentStatus = 'BOOKED' | 'DONE' | 'CANCELLED';

export type LogisticsInstallationAppointment = {
  appointment_id: string;
  task_id: string;
  scheduled_at: string;
  status: LogisticsAppointmentStatus | string;
  created_at: string;
};

export type LogisticsInstallationTask = {
  task_id: string;
  tenant_id: string;
  order_id: string;
  customer_name: string;
  address: string;
  service_type: string;
  assigned_team?: string | null;
  status: LogisticsInstallationStatus | string;
  remedy_ticket_id?: string | null;
  exception_note?: string | null;
  created_at: string;
  updated_at: string;
  appointments?: LogisticsInstallationAppointment[];
};

export type LogisticsInstallationTaskListQuery = {
  status?: string;
  offset?: number;
  limit?: number;
};

export type LogisticsInstallationTaskListResponse = {
  items: LogisticsInstallationTask[];
  total: number;
};

export type LogisticsInstallationTaskCreatePayload = {
  order_id: string;
  customer_name: string;
  address: string;
  service_type: string;
  assigned_team?: string | null;
  scheduled_at?: string | null;
};

export type LogisticsInstallationTaskUpdatePayload = {
  assigned_team?: string | null;
  status?: LogisticsInstallationStatus;
  remedy_ticket_id?: string | null;
  exception_note?: string | null;
};

export type LogisticsInstallationAppointmentCreatePayload = {
  scheduled_at: string;
  status?: LogisticsAppointmentStatus;
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

const buildQueryString = (query: LogisticsInstallationTaskListQuery): string => {
  const params = new URLSearchParams();
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

export const listLogisticsInstallationTasks = async (
  tenantId: string,
  query: LogisticsInstallationTaskListQuery
): Promise<LogisticsInstallationTaskListResponse> => {
  if (useMocks) {
    return listMockInstallationTasks(query);
  }
  return requestJson<LogisticsInstallationTaskListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/installations/tasks${buildQueryString(
      query
    )}`,
    { headers: buildAuthHeaders() },
    '获取安装任务失败'
  );
};

export const createLogisticsInstallationTask = async (
  tenantId: string,
  payload: LogisticsInstallationTaskCreatePayload
): Promise<LogisticsInstallationTask> => {
  if (useMocks) {
    return createMockInstallationTask(payload);
  }
  return requestJson<LogisticsInstallationTask>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/installations/tasks`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建安装任务失败'
  );
};

export const updateLogisticsInstallationTask = async (
  tenantId: string,
  taskId: string,
  payload: LogisticsInstallationTaskUpdatePayload
): Promise<LogisticsInstallationTask> => {
  if (useMocks) {
    return updateMockInstallationTask(taskId, payload);
  }
  return requestJson<LogisticsInstallationTask>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/installations/tasks/${taskId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新安装任务失败'
  );
};

export const addLogisticsInstallationAppointment = async (
  tenantId: string,
  taskId: string,
  payload: LogisticsInstallationAppointmentCreatePayload
): Promise<LogisticsInstallationAppointment> => {
  if (useMocks) {
    return addMockInstallationAppointment(taskId, payload);
  }
  return requestJson<LogisticsInstallationAppointment>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/installations/tasks/${taskId}/appointments`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建预约失败'
  );
};
