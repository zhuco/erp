import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import { createMockMapping, getMockMappingList, updateMockMapping } from '@/api/mocks';
import type { PlatformCode } from '@/features/platform';

export type ChannelMapping = {
  mapping_id: string;
  tenant_id: string;
  platform: PlatformCode;
  shop_id: string;
  platform_sku_id: string;
  outer_sku_id?: string | null;
  sku_id: string;
  sku_code: string;
  barcode?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export type MappingListQuery = {
  platform?: PlatformCode;
  shop_id?: string;
  platform_sku_id?: string;
  sku_code?: string;
  barcode?: string;
  offset?: number;
  limit?: number;
};

export type MappingListResponse = {
  items: ChannelMapping[];
  total: number;
};

export type MappingPayload = {
  platform: PlatformCode;
  shop_id: string;
  platform_sku_id: string;
  outer_sku_id?: string | null;
  sku_id: string;
  sku_code?: string | null;
  barcode?: string | null;
};

export type MappingBatchPayload = {
  items: MappingPayload[];
};

export type MappingConflict = {
  platform: PlatformCode;
  shop_id: string;
  platform_sku_id: string;
  incoming_sku_id: string;
  existing_mapping_id: string;
  existing_sku_id: string;
  existing_sku_code?: string | null;
  existing_barcode?: string | null;
};

export type MappingBatchResponse = {
  items: ChannelMapping[];
  conflicts: MappingConflict[];
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

const buildQueryString = (query: MappingListQuery): string => {
  const params = new URLSearchParams();
  if (query.platform) {
    params.set('platform', query.platform);
  }
  if (query.shop_id) {
    params.set('shop_id', query.shop_id);
  }
  if (query.platform_sku_id) {
    params.set('platform_sku_id', query.platform_sku_id);
  }
  if (query.sku_code) {
    params.set('sku_code', query.sku_code);
  }
  if (query.barcode) {
    params.set('barcode', query.barcode);
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  if (query.offset != null) {
    params.set('offset', String(query.offset));
  }
  const serialized = params.toString();
  return serialized ? `?${serialized}` : '';
};

export const listMappings = async (
  tenantId: string,
  query: MappingListQuery
): Promise<MappingListResponse> => {
  if (useMocks) {
    return getMockMappingList(query);
  }
  return requestJson<MappingListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/mappings${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取渠道映射失败'
  );
};

export const createMapping = async (
  tenantId: string,
  payload: MappingPayload
): Promise<ChannelMapping> => {
  if (useMocks) {
    return createMockMapping(payload);
  }
  return requestJson<ChannelMapping>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/mappings`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建渠道映射失败'
  );
};

export const updateMapping = async (
  tenantId: string,
  mappingId: string,
  payload: MappingPayload
): Promise<ChannelMapping> => {
  if (useMocks) {
    return updateMockMapping(mappingId, payload);
  }
  return requestJson<ChannelMapping>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/mappings/${mappingId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新渠道映射失败'
  );
};

export const batchCreateMappings = async (
  tenantId: string,
  payload: MappingBatchPayload
): Promise<MappingBatchResponse> => {
  if (useMocks) {
    throw createApiError('Mock not supported', 500);
  }
  return requestJson<MappingBatchResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/mappings/batch`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '批量创建渠道映射失败'
  );
};
