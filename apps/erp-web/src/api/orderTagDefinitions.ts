import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type OrderTagDefinition = {
  tag_id: string;
  tenant_id: string;
  name: string;
  color?: string | null;
  priority: number;
  status: string;
  auto_apply: boolean;
  conditions_json: Record<string, unknown> | unknown[];
  created_at: string;
  updated_at: string;
};

export type OrderTagDefinitionListQuery = {
  keyword?: string;
  status?: string;
  offset?: number;
  limit?: number;
};

export type OrderTagDefinitionListResponse = {
  items: OrderTagDefinition[];
  total: number;
};

export type OrderTagDefinitionCreatePayload = {
  name: string;
  color?: string;
  priority?: number;
  status?: string;
  auto_apply?: boolean;
  conditions_json?: Record<string, unknown> | unknown[];
};

export type OrderTagDefinitionUpdatePayload = {
  name?: string;
  color?: string;
  priority?: number;
  status?: string;
  auto_apply?: boolean;
  conditions_json?: Record<string, unknown> | unknown[];
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

const buildQueryString = (query: OrderTagDefinitionListQuery): string => {
  const params = new URLSearchParams();
  if (query.keyword) {
    params.set('keyword', query.keyword);
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

export const listOrderTagDefinitions = async (
  tenantId: string,
  query: OrderTagDefinitionListQuery
): Promise<OrderTagDefinitionListResponse> => {
  if (useMocks) {
    return { items: [], total: 0 };
  }
  return requestJson<OrderTagDefinitionListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/order-tag-definitions${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取订单标记失败'
  );
};

export const createOrderTagDefinition = async (
  tenantId: string,
  payload: OrderTagDefinitionCreatePayload
): Promise<OrderTagDefinition> => {
  if (useMocks) {
    return {
      tag_id: `${Date.now()}`,
      tenant_id: tenantId,
      name: payload.name,
      color: payload.color ?? null,
      priority: payload.priority ?? 0,
      status: payload.status ?? 'ACTIVE',
      auto_apply: payload.auto_apply ?? false,
      conditions_json: payload.conditions_json ?? [],
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<OrderTagDefinition>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/order-tag-definitions`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建订单标记失败'
  );
};

export const updateOrderTagDefinition = async (
  tenantId: string,
  tagId: string,
  payload: OrderTagDefinitionUpdatePayload
): Promise<OrderTagDefinition> => {
  if (useMocks) {
    return {
      tag_id: tagId,
      tenant_id: tenantId,
      name: payload.name ?? 'mock',
      color: payload.color ?? null,
      priority: payload.priority ?? 0,
      status: payload.status ?? 'ACTIVE',
      auto_apply: payload.auto_apply ?? false,
      conditions_json: payload.conditions_json ?? [],
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<OrderTagDefinition>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/order-tag-definitions/${tagId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新订单标记失败'
  );
};

export const deleteOrderTagDefinition = async (
  tenantId: string,
  tagId: string
): Promise<OrderTagDefinition> => {
  if (useMocks) {
    return {
      tag_id: tagId,
      tenant_id: tenantId,
      name: 'mock',
      color: null,
      priority: 0,
      status: 'ACTIVE',
      auto_apply: false,
      conditions_json: [],
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<OrderTagDefinition>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/order-tag-definitions/${tagId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除订单标记失败'
  );
};
