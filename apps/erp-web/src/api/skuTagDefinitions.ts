import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockSkuTagDefinition,
  deleteMockSkuTagDefinition,
  listMockSkuTagDefinitions,
  updateMockSkuTagDefinition,
} from '@/api/mocks';

export type SkuTagDefinition = {
  tag_id: string;
  tenant_id: string;
  name: string;
  color?: string | null;
  priority: number;
  status: string;
  created_at: string;
  updated_at: string;
};

export type SkuTagDefinitionListQuery = {
  keyword?: string;
  status?: string;
  offset?: number;
  limit?: number;
};

export type SkuTagDefinitionListResponse = {
  items: SkuTagDefinition[];
  total: number;
};

export type SkuTagDefinitionCreatePayload = {
  name: string;
  color?: string;
  priority?: number;
  status?: string;
};

export type SkuTagDefinitionUpdatePayload = {
  name?: string;
  color?: string;
  priority?: number;
  status?: string;
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

const buildQueryString = (query: SkuTagDefinitionListQuery): string => {
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

export const listSkuTagDefinitions = async (
  tenantId: string,
  query: SkuTagDefinitionListQuery
): Promise<SkuTagDefinitionListResponse> => {
  if (useMocks) {
    return listMockSkuTagDefinitions(query);
  }
  return requestJson<SkuTagDefinitionListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/sku-tag-definitions${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取货品标记失败'
  );
};

export const createSkuTagDefinition = async (
  tenantId: string,
  payload: SkuTagDefinitionCreatePayload
): Promise<SkuTagDefinition> => {
  if (useMocks) {
    return createMockSkuTagDefinition(tenantId, payload);
  }
  return requestJson<SkuTagDefinition>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/sku-tag-definitions`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建货品标记失败'
  );
};

export const updateSkuTagDefinition = async (
  tenantId: string,
  tagId: string,
  payload: SkuTagDefinitionUpdatePayload
): Promise<SkuTagDefinition> => {
  if (useMocks) {
    return updateMockSkuTagDefinition(tenantId, tagId, payload);
  }
  return requestJson<SkuTagDefinition>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/sku-tag-definitions/${tagId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新货品标记失败'
  );
};

export const deleteSkuTagDefinition = async (
  tenantId: string,
  tagId: string
): Promise<SkuTagDefinition> => {
  if (useMocks) {
    return deleteMockSkuTagDefinition(tenantId, tagId);
  }
  return requestJson<SkuTagDefinition>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/sku-tag-definitions/${tagId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除货品标记失败'
  );
};
