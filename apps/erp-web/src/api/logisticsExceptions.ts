import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  assignMockLogisticsExceptionWorkOrder,
  createMockLogisticsExceptionType,
  createMockLogisticsExceptionWorkOrder,
  escalateMockLogisticsExceptionWorkOrder,
  listMockLogisticsExceptionTypes,
  listMockLogisticsExceptionWorkOrders,
  resolveMockLogisticsExceptionWorkOrder,
  updateMockLogisticsExceptionType
} from '@/api/mocks';

export type LogisticsExceptionSeverity = 'LOW' | 'MEDIUM' | 'HIGH';
export type LogisticsExceptionStatus = 'OPEN' | 'IN_PROGRESS' | 'ESCALATED' | 'RESOLVED';

export type LogisticsExceptionType = {
  type_code: string;
  name: string;
  sla_hours: number;
  severity: LogisticsExceptionSeverity;
  enabled: boolean;
  created_at: string;
  updated_at: string;
};

export type LogisticsExceptionWorkOrder = {
  ticket_id: string;
  order_id: string;
  tracking_no: string;
  carrier_code: string;
  exception_type: string;
  status: LogisticsExceptionStatus | string;
  assignee?: string | null;
  created_at: string;
  due_at?: string | null;
  escalation_level: number;
  resolution?: string | null;
  resolved_at?: string | null;
};

export type LogisticsExceptionTypeCreatePayload = {
  type_code: string;
  name: string;
  sla_hours?: number;
  severity?: LogisticsExceptionSeverity;
  enabled?: boolean;
};

export type LogisticsExceptionTypeUpdatePayload = {
  name?: string;
  sla_hours?: number;
  severity?: LogisticsExceptionSeverity;
  enabled?: boolean;
};

export type LogisticsExceptionTypeListQuery = {
  enabled?: boolean;
};

export type LogisticsExceptionWorkOrderListQuery = {
  status?: string;
  exception_type?: string;
  assignee?: string;
  offset?: number;
  limit?: number;
};

export type LogisticsExceptionWorkOrderListResponse = {
  items: LogisticsExceptionWorkOrder[];
  total: number;
};

export type LogisticsExceptionWorkOrderCreatePayload = {
  order_id: string;
  tracking_no: string;
  carrier_code: string;
  exception_type: string;
  status?: LogisticsExceptionStatus;
  assignee?: string | null;
  due_at?: string | null;
};

export type LogisticsExceptionWorkOrderAssignPayload = {
  assignee: string;
};

export type LogisticsExceptionWorkOrderResolvePayload = {
  resolution: string;
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

const buildQueryString = (query: LogisticsExceptionWorkOrderListQuery): string => {
  const params = new URLSearchParams();
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.exception_type) {
    params.set('exception_type', query.exception_type);
  }
  if (query.assignee) {
    params.set('assignee', query.assignee);
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

export const listLogisticsExceptionTypes = async (
  tenantId: string,
  query: LogisticsExceptionTypeListQuery
): Promise<LogisticsExceptionType[]> => {
  if (useMocks) {
    return listMockLogisticsExceptionTypes(query);
  }
  const params = query.enabled == null ? '' : `?enabled=${query.enabled ? 'true' : 'false'}`;
  return requestJson<LogisticsExceptionType[]>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/exception-types${params}`,
    { headers: buildAuthHeaders() },
    '获取异常类型失败'
  );
};

export const createLogisticsExceptionType = async (
  tenantId: string,
  payload: LogisticsExceptionTypeCreatePayload
): Promise<LogisticsExceptionType> => {
  if (useMocks) {
    return createMockLogisticsExceptionType(payload);
  }
  return requestJson<LogisticsExceptionType>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/exception-types`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建异常类型失败'
  );
};

export const updateLogisticsExceptionType = async (
  tenantId: string,
  typeCode: string,
  payload: LogisticsExceptionTypeUpdatePayload
): Promise<LogisticsExceptionType> => {
  if (useMocks) {
    return updateMockLogisticsExceptionType(typeCode, payload);
  }
  return requestJson<LogisticsExceptionType>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/exception-types/${typeCode}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新异常类型失败'
  );
};

export const listLogisticsExceptionWorkOrders = async (
  tenantId: string,
  query: LogisticsExceptionWorkOrderListQuery
): Promise<LogisticsExceptionWorkOrderListResponse> => {
  if (useMocks) {
    return listMockLogisticsExceptionWorkOrders(query);
  }
  return requestJson<LogisticsExceptionWorkOrderListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/exception-work-orders${buildQueryString(
      query
    )}`,
    { headers: buildAuthHeaders() },
    '获取异常工单失败'
  );
};

export const createLogisticsExceptionWorkOrder = async (
  tenantId: string,
  payload: LogisticsExceptionWorkOrderCreatePayload
): Promise<LogisticsExceptionWorkOrder> => {
  if (useMocks) {
    return createMockLogisticsExceptionWorkOrder(payload);
  }
  return requestJson<LogisticsExceptionWorkOrder>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/exception-work-orders`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建异常工单失败'
  );
};

export const assignLogisticsExceptionWorkOrder = async (
  tenantId: string,
  ticketId: string,
  payload: LogisticsExceptionWorkOrderAssignPayload
): Promise<LogisticsExceptionWorkOrder> => {
  if (useMocks) {
    return assignMockLogisticsExceptionWorkOrder(ticketId, payload);
  }
  return requestJson<LogisticsExceptionWorkOrder>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/exception-work-orders/${ticketId}/assign`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '分派异常工单失败'
  );
};

export const resolveLogisticsExceptionWorkOrder = async (
  tenantId: string,
  ticketId: string,
  payload: LogisticsExceptionWorkOrderResolvePayload
): Promise<LogisticsExceptionWorkOrder> => {
  if (useMocks) {
    return resolveMockLogisticsExceptionWorkOrder(ticketId, payload);
  }
  return requestJson<LogisticsExceptionWorkOrder>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/exception-work-orders/${ticketId}/resolve`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '处理异常工单失败'
  );
};

export const escalateLogisticsExceptionWorkOrder = async (
  tenantId: string,
  ticketId: string
): Promise<LogisticsExceptionWorkOrder> => {
  if (useMocks) {
    return escalateMockLogisticsExceptionWorkOrder(ticketId);
  }
  return requestJson<LogisticsExceptionWorkOrder>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/exception-work-orders/${ticketId}/escalate`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '升级异常工单失败'
  );
};
