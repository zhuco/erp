import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockMemberLevel,
  deleteMockMemberLevel,
  getMockMemberLevelList,
  updateMockMemberLevel
} from '@/api/mocks';

export type MemberLevel = {
  level_id: string;
  tenant_id: string;
  name: string;
  level_rank: number;
  discount_rate?: number | null;
  rules?: string | null;
  status?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export type MemberLevelListQuery = {
  keyword?: string;
  status?: string;
  offset?: number;
  limit?: number;
};

export type MemberLevelListResponse = {
  items: MemberLevel[];
  total: number;
};

export type MemberLevelCreatePayload = {
  name: string;
  level_rank?: number;
  discount_rate?: number;
  rules?: string;
  status?: string;
};

export type MemberLevelUpdatePayload = {
  name?: string;
  level_rank?: number;
  discount_rate?: number;
  rules?: string;
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

const buildQueryString = (query: MemberLevelListQuery): string => {
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

export const listMemberLevels = async (
  tenantId: string,
  query: MemberLevelListQuery
): Promise<MemberLevelListResponse> => {
  if (useMocks) {
    return getMockMemberLevelList(query);
  }
  return requestJson<MemberLevelListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/member-levels${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取会员等级失败'
  );
};

export const createMemberLevel = async (
  tenantId: string,
  payload: MemberLevelCreatePayload
): Promise<MemberLevel> => {
  if (useMocks) {
    return createMockMemberLevel(payload);
  }
  return requestJson<MemberLevel>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/member-levels`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建会员等级失败'
  );
};

export const updateMemberLevel = async (
  tenantId: string,
  levelId: string,
  payload: MemberLevelUpdatePayload
): Promise<MemberLevel> => {
  if (useMocks) {
    return updateMockMemberLevel(levelId, payload);
  }
  return requestJson<MemberLevel>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/member-levels/${levelId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新会员等级失败'
  );
};

export const deleteMemberLevel = async (
  tenantId: string,
  levelId: string
): Promise<MemberLevel> => {
  if (useMocks) {
    return deleteMockMemberLevel(levelId);
  }
  return requestJson<MemberLevel>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/member-levels/${levelId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除会员等级失败'
  );
};
