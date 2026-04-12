import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type ChannelFieldMapping = {
  mapping_id: string;
  tenant_id: string;
  platform: string;
  entity_type: string;
  platform_field: string;
  internal_field: string;
  default_value?: string | null;
  transform_rule?: Record<string, unknown> | null;
  enabled: boolean;
  created_at?: string | null;
  updated_at?: string | null;
};

export type ChannelFieldMappingListQuery = {
  platform?: string;
  entity_type?: string;
  platform_field?: string;
  internal_field?: string;
  enabled?: boolean;
  offset?: number;
  limit?: number;
};

export type ChannelFieldMappingListResponse = {
  items: ChannelFieldMapping[];
  total: number;
};

export type ChannelFieldMappingPayload = {
  platform: string;
  entity_type: string;
  platform_field: string;
  internal_field: string;
  default_value?: string | null;
  transform_rule?: Record<string, unknown> | null;
  enabled?: boolean;
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

const buildQueryString = (query: ChannelFieldMappingListQuery): string => {
  const params = new URLSearchParams();
  if (query.platform) {
    params.set('platform', query.platform);
  }
  if (query.entity_type) {
    params.set('entity_type', query.entity_type);
  }
  if (query.platform_field) {
    params.set('platform_field', query.platform_field);
  }
  if (query.internal_field) {
    params.set('internal_field', query.internal_field);
  }
  if (query.enabled != null) {
    params.set('enabled', query.enabled ? 'true' : 'false');
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

export const listChannelFieldMappings = async (
  tenantId: string,
  query: ChannelFieldMappingListQuery
): Promise<ChannelFieldMappingListResponse> => {
  if (useMocks) {
    return { items: [], total: 0 };
  }
  return requestJson<ChannelFieldMappingListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/channel-field-mappings${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取字段映射失败'
  );
};

export const createChannelFieldMapping = async (
  tenantId: string,
  payload: ChannelFieldMappingPayload
): Promise<ChannelFieldMapping> => {
  if (useMocks) {
    throw createApiError('Mock not supported', 500);
  }
  return requestJson<ChannelFieldMapping>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/channel-field-mappings`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建字段映射失败'
  );
};

export const updateChannelFieldMapping = async (
  tenantId: string,
  mappingId: string,
  payload: ChannelFieldMappingPayload
): Promise<ChannelFieldMapping> => {
  if (useMocks) {
    throw createApiError('Mock not supported', 500);
  }
  return requestJson<ChannelFieldMapping>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/channel-field-mappings/${mappingId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新字段映射失败'
  );
};

export const deleteChannelFieldMapping = async (
  tenantId: string,
  mappingId: string
): Promise<ChannelFieldMapping> => {
  if (useMocks) {
    throw createApiError('Mock not supported', 500);
  }
  return requestJson<ChannelFieldMapping>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/channel-field-mappings/${mappingId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除字段映射失败'
  );
};
