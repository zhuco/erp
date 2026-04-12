import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockLogisticsInterface,
  createMockLogisticsInterfaceApproval,
  listMockLogisticsInterfaceApprovals,
  listMockLogisticsInterfaces,
  updateMockLogisticsInterface
} from '@/api/mocks';

export type LogisticsInterfaceStatus = 'HEALTHY' | 'WARNING' | 'DOWN';
export type LogisticsInterfaceApprovalStatus = 'APPROVED' | 'PENDING';

export type LogisticsInterface = {
  interface_id: string;
  tenant_id: string;
  carrier_code: string;
  carrier_name: string;
  api_base_url: string;
  template_name: string;
  status: LogisticsInterfaceStatus | string;
  last_checked_at?: string | null;
  retry_policy: string;
  alert_channel: string;
  approval_status: LogisticsInterfaceApprovalStatus | string;
  approved_by?: string | null;
  approved_at?: string | null;
  notes?: string | null;
  created_at: string;
  updated_at: string;
};

export type LogisticsInterfaceApproval = {
  approval_id: string;
  tenant_id: string;
  interface_id: string;
  action: string;
  operator: string;
  comment?: string | null;
  created_at: string;
};

export type LogisticsInterfaceListQuery = {
  keyword?: string;
  status?: string;
  approval_status?: string;
  offset?: number;
  limit?: number;
};

export type LogisticsInterfaceListResponse = {
  items: LogisticsInterface[];
  total: number;
};

export type LogisticsInterfaceCreatePayload = {
  carrier_code: string;
  carrier_name: string;
  api_base_url: string;
  template_name: string;
  status?: LogisticsInterfaceStatus;
  last_checked_at?: string | null;
  retry_policy: string;
  alert_channel: string;
  approval_status?: LogisticsInterfaceApprovalStatus;
  approved_by?: string | null;
  approved_at?: string | null;
  notes?: string | null;
};

export type LogisticsInterfaceUpdatePayload = {
  carrier_code?: string;
  carrier_name?: string;
  api_base_url?: string;
  template_name?: string;
  status?: LogisticsInterfaceStatus;
  last_checked_at?: string | null;
  retry_policy?: string;
  alert_channel?: string;
  approval_status?: LogisticsInterfaceApprovalStatus;
  approved_by?: string | null;
  approved_at?: string | null;
  notes?: string | null;
};

export type LogisticsInterfaceApprovalCreatePayload = {
  action: string;
  operator: string;
  comment?: string | null;
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

const buildQueryString = (query: LogisticsInterfaceListQuery): string => {
  const params = new URLSearchParams();
  if (query.keyword) {
    params.set('keyword', query.keyword);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.approval_status) {
    params.set('approval_status', query.approval_status);
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

export const listLogisticsInterfaces = async (
  tenantId: string,
  query: LogisticsInterfaceListQuery
): Promise<LogisticsInterfaceListResponse> => {
  if (useMocks) {
    return listMockLogisticsInterfaces(query);
  }
  return requestJson<LogisticsInterfaceListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/interfaces${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取物流接口配置失败'
  );
};

export const createLogisticsInterface = async (
  tenantId: string,
  payload: LogisticsInterfaceCreatePayload
): Promise<LogisticsInterface> => {
  if (useMocks) {
    return createMockLogisticsInterface(payload);
  }
  return requestJson<LogisticsInterface>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/interfaces`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建物流接口配置失败'
  );
};

export const updateLogisticsInterface = async (
  tenantId: string,
  interfaceId: string,
  payload: LogisticsInterfaceUpdatePayload
): Promise<LogisticsInterface> => {
  if (useMocks) {
    return updateMockLogisticsInterface(interfaceId, payload);
  }
  return requestJson<LogisticsInterface>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/interfaces/${interfaceId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新物流接口配置失败'
  );
};

export const listLogisticsInterfaceApprovals = async (
  tenantId: string,
  interfaceId: string
): Promise<LogisticsInterfaceApproval[]> => {
  if (useMocks) {
    return listMockLogisticsInterfaceApprovals(interfaceId);
  }
  return requestJson<LogisticsInterfaceApproval[]>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/interfaces/${interfaceId}/approvals`,
    {
      headers: buildAuthHeaders()
    },
    '获取接口审批记录失败'
  );
};

export const createLogisticsInterfaceApproval = async (
  tenantId: string,
  interfaceId: string,
  payload: LogisticsInterfaceApprovalCreatePayload
): Promise<LogisticsInterfaceApproval> => {
  if (useMocks) {
    return createMockLogisticsInterfaceApproval(interfaceId, payload);
  }
  return requestJson<LogisticsInterfaceApproval>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/interfaces/${interfaceId}/approvals`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '提交接口审批记录失败'
  );
};
