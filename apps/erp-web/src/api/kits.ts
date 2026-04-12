import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import { getMockKitDetail, getMockKitList, saveMockKitBom } from '@/api/mocks';
import type { SkuKind, SkuSearchItem } from '@/api/skus';

export type KitComponent = {
  component_sku_id: string;
  component_sku_code: string;
  component_name?: string | null;
  component_barcode?: string | null;
  qty: number;
  expire_at?: string | null;
};

export type KitDetail = {
  kit_sku: SkuSearchItem & { sku_kind?: SkuKind };
  components: KitComponent[];
  updated_at?: string | null;
};

export type KitSummary = {
  kit_sku_id: string;
  kit_sku_code: string;
  kit_name?: string | null;
  component_count: number;
  updated_at?: string | null;
};

export type KitListResponse = {
  items: KitSummary[];
  total: number;
};

export type KitListQuery = {
  keyword?: string;
  offset?: number;
  limit?: number;
};

export type ProductKitListQuery = {
  offset?: number;
  limit?: number;
};

export type SaveKitPayload = {
  kit_sku_id: string;
  components: Array<{ component_sku_id: string; qty: number; expire_at?: string | null }>;
};

type KitListApiResponse = {
  data?: KitSummary[];
  items?: KitSummary[];
  total?: number;
};

type KitDetailApiResponse = {
  data?: KitDetail;
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

const buildQueryString = (query: KitListQuery) => {
  const params = new URLSearchParams();
  if (query.keyword?.trim()) {
    params.set('keyword', query.keyword.trim());
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

const buildProductQueryString = (query: ProductKitListQuery) => {
  const params = new URLSearchParams();
  if (query.offset != null) {
    params.set('offset', String(query.offset));
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  const serialized = params.toString();
  return serialized ? `?${serialized}` : '';
};

const normalizeKitList = (payload: KitListApiResponse | KitListResponse): KitListResponse => {
  if ('items' in payload) {
    return {
      items: payload.items ?? [],
      total: payload.total ?? payload.items?.length ?? 0
    };
  }
  return {
    items: payload.data ?? [],
    total: payload.total ?? payload.data?.length ?? 0
  };
};

const normalizeKitDetail = (payload: KitDetail | KitDetailApiResponse): KitDetail => {
  if ('kit_sku' in payload) {
    return payload;
  }
  if (payload.data) {
    return payload.data;
  }
  throw createApiError('获取组合装详情失败');
};

export const listKits = async (
  tenantId: string,
  query: KitListQuery
): Promise<KitListResponse> => {
  if (useMocks) {
    return getMockKitList(query);
  }
  const response = await requestJson<KitListApiResponse | KitListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/kits${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取组合装列表失败'
  );
  return normalizeKitList(response);
};

export const listProductKits = async (
  tenantId: string,
  productId: string,
  query: ProductKitListQuery
): Promise<KitListResponse> => {
  if (useMocks) {
    return getMockKitList(query);
  }
  const response = await requestJson<KitListApiResponse | KitListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/products/${productId}/kits${buildProductQueryString(
      query
    )}`,
    { headers: buildAuthHeaders() },
    '获取组合装失败'
  );
  return normalizeKitList(response);
};

export const getKitDetail = async (
  tenantId: string,
  kitSkuId: string
): Promise<KitDetail> => {
  if (useMocks) {
    return getMockKitDetail(kitSkuId);
  }
  const response = await requestJson<KitDetail | KitDetailApiResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/kits/${kitSkuId}`,
    { headers: buildAuthHeaders() },
    '获取组合装详情失败'
  );
  return normalizeKitDetail(response);
};

export const saveKitBom = async (
  tenantId: string,
  payload: SaveKitPayload
): Promise<KitDetail> => {
  if (useMocks) {
    return saveMockKitBom(payload);
  }
  const response = await requestJson<KitDetail | KitDetailApiResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/kits/${payload.kit_sku_id}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify({
        components: payload.components
      })
    },
    '保存组合装失败'
  );
  return normalizeKitDetail(response);
};
