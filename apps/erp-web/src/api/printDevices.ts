import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type PrintDevice = {
  device_id: string;
  tenant_id: string;
  name: string;
  fingerprint: string;
  status: string;
  is_default: boolean;
  created_at: string;
  updated_at: string;
};

export type PrintDeviceListQuery = {
  fingerprint?: string;
  status?: string;
  cursor?: string;
  limit?: number;
};

export type PrintDeviceListResponse = {
  data: PrintDevice[];
  next_cursor?: string | null;
};

export type PrintDeviceCreatePayload = {
  name: string;
  fingerprint: string;
  status?: string;
  is_default?: boolean;
};

export type PrintDeviceRegisterPayload = {
  device_fingerprint: string;
  name?: string;
  status?: string;
};

export type PrintDeviceUpdatePayload = {
  name?: string;
  status?: string;
  is_default?: boolean;
};

const requestJson = async <T>(input: RequestInfo, init: RequestInit, message: string) => {
  const response = await fetch(input, init);
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

const buildQueryString = (query: PrintDeviceListQuery): string => {
  const params = new URLSearchParams();
  if (query.fingerprint) {
    params.set('fingerprint', query.fingerprint);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.cursor) {
    params.set('cursor', query.cursor);
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  const serialized = params.toString();
  return serialized ? `?${serialized}` : '';
};

export const listPrintDevices = async (
  tenantId: string,
  query: PrintDeviceListQuery
): Promise<PrintDeviceListResponse> => {
  if (useMocks) {
    return { data: [], next_cursor: null };
  }
  return requestJson<PrintDeviceListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/print-devices${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取打印设备失败'
  );
};

export const createPrintDevice = async (
  tenantId: string,
  payload: PrintDeviceCreatePayload
): Promise<PrintDevice> => {
  if (useMocks) {
    return {
      device_id: `${Date.now()}`,
      tenant_id: tenantId,
      name: payload.name,
      fingerprint: payload.fingerprint,
      status: payload.status ?? 'ACTIVE',
      is_default: payload.is_default ?? false,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<PrintDevice>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/print-devices`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建打印设备失败'
  );
};

export const registerPrintDevice = async (
  tenantId: string,
  payload: PrintDeviceRegisterPayload
): Promise<PrintDevice> => {
  if (useMocks) {
    return {
      device_id: `${Date.now()}`,
      tenant_id: tenantId,
      name: payload.name ?? payload.device_fingerprint,
      fingerprint: payload.device_fingerprint,
      status: payload.status ?? 'ACTIVE',
      is_default: false,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<PrintDevice>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/print-devices/register`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '注册打印设备失败'
  );
};

export const updatePrintDevice = async (
  tenantId: string,
  deviceId: string,
  payload: PrintDeviceUpdatePayload
): Promise<PrintDevice> => {
  if (useMocks) {
    return {
      device_id: deviceId,
      tenant_id: tenantId,
      name: payload.name ?? 'mock',
      fingerprint: 'mock',
      status: payload.status ?? 'ACTIVE',
      is_default: payload.is_default ?? false,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<PrintDevice>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/print-devices/${deviceId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新打印设备失败'
  );
};
