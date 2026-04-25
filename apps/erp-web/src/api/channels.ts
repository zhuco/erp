import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockChannel,
  deleteMockChannel,
  getMockChannelList,
  updateMockChannel,
} from '@/api/mocks';
import type { PlatformCode } from '@/features/platform';

export type ChannelType = 'ONLINE' | 'OFFLINE';

export type ChannelStatus = 'ACTIVE' | 'DISABLED';

export type Channel = {
  channel_id: string;
  tenant_id: string;
  channel_type?: ChannelType;
  platform?: PlatformCode | null;
  name: string;
  contact_name?: string | null;
  contact_phone?: string | null;
  contact_address?: string | null;
  department?: string | null;
  owner?: string | null;
  allow_manual_order: boolean;
  allow_manual_after_sales: boolean;
  default_warehouse_id?: string | null;
  commission_formula?: string | null;
  status: ChannelStatus;
  created_at?: string | null;
  updated_at?: string | null;
};

export type ChannelListQuery = {
  keyword?: string;
  status?: ChannelStatus;
  offset?: number;
  limit?: number;
};

export type ChannelListResponse = {
  items: Channel[];
  total: number;
};

export type ChannelCreatePayload = {
  channel_type?: ChannelType;
  platform?: PlatformCode | null;
  name: string;
  contact_name?: string | null;
  contact_phone?: string | null;
  contact_address?: string | null;
  department?: string | null;
  owner?: string | null;
  allow_manual_order?: boolean;
  allow_manual_after_sales?: boolean;
  default_warehouse_id?: string | null;
  commission_formula?: string | null;
  status?: ChannelStatus;
};

export type ChannelUpdatePayload = {
  channel_type?: ChannelType;
  platform?: PlatformCode | null;
  name?: string;
  contact_name?: string | null;
  contact_phone?: string | null;
  contact_address?: string | null;
  department?: string | null;
  owner?: string | null;
  allow_manual_order?: boolean;
  allow_manual_after_sales?: boolean;
  default_warehouse_id?: string | null;
  commission_formula?: string | null;
  status?: ChannelStatus;
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

const buildQueryString = (query: ChannelListQuery): string => {
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

export const listChannels = async (
  tenantId: string,
  query: ChannelListQuery,
): Promise<ChannelListResponse> => {
  if (useMocks) {
    return getMockChannelList(query);
  }
  return requestJson<ChannelListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/channels${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取渠道列表失败',
  );
};

export const createChannel = async (
  tenantId: string,
  payload: ChannelCreatePayload,
): Promise<Channel> => {
  if (useMocks) {
    return createMockChannel(payload);
  }
  return requestJson<Channel>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/channels`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders(),
      },
      body: JSON.stringify(payload),
    },
    '创建渠道失败',
  );
};

export const updateChannel = async (
  tenantId: string,
  channelId: string,
  payload: ChannelUpdatePayload,
): Promise<Channel> => {
  if (useMocks) {
    return updateMockChannel(channelId, payload);
  }
  return requestJson<Channel>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/channels/${channelId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders(),
      },
      body: JSON.stringify(payload),
    },
    '更新渠道失败',
  );
};

export const deleteChannel = async (tenantId: string, channelId: string): Promise<Channel> => {
  if (useMocks) {
    return deleteMockChannel(channelId);
  }
  return requestJson<Channel>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/channels/${channelId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders(),
    },
    '删除渠道失败',
  );
};
