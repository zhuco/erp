import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockMarketingPriceItem,
  createMockMarketingPriceList,
  deleteMockMarketingPriceItem,
  deleteMockMarketingPriceList,
  getMockMarketingPriceList,
  listMockMarketingPriceLists,
  updateMockMarketingPriceItem,
  updateMockMarketingPriceList
} from '@/api/mocks';

export type MarketingPriceList = {
  price_list_id: string;
  tenant_id: string;
  name: string;
  channel_id?: string | null;
  currency: string;
  includes_tax: boolean;
  status: string;
  version: number;
  effective_from?: string | null;
  effective_to?: string | null;
  notes?: string | null;
  created_by?: string | null;
  created_at: string;
  updated_at: string;
};

export type MarketingPriceItem = {
  item_id: string;
  tenant_id: string;
  price_list_id: string;
  sku_id: string;
  base_price_cent?: number | null;
  channel_price_cent?: number | null;
  member_price_cent?: number | null;
  created_at: string;
  updated_at: string;
};

export type MarketingPriceListListQuery = {
  keyword?: string;
  status?: string;
  channel_id?: string;
  currency?: string;
  offset?: number;
  limit?: number;
};

export type MarketingPriceListListResponse = {
  items: MarketingPriceList[];
  total: number;
};

export type MarketingPriceListDetail = {
  price_list: MarketingPriceList;
  items: MarketingPriceItem[];
};

export type MarketingPriceListCreatePayload = {
  name: string;
  channel_id?: string;
  currency?: string;
  includes_tax?: boolean;
  status?: string;
  version?: number;
  effective_from?: string;
  effective_to?: string;
  notes?: string;
};

export type MarketingPriceListUpdatePayload = {
  name?: string;
  channel_id?: string;
  currency?: string;
  includes_tax?: boolean;
  status?: string;
  version?: number;
  effective_from?: string;
  effective_to?: string;
  notes?: string;
};

export type MarketingPriceItemCreatePayload = {
  sku_id: string;
  base_price_cent?: number;
  channel_price_cent?: number;
  member_price_cent?: number;
};

export type MarketingPriceItemUpdatePayload = {
  base_price_cent?: number;
  channel_price_cent?: number;
  member_price_cent?: number;
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

const buildQueryString = (query: MarketingPriceListListQuery): string => {
  const params = new URLSearchParams();
  if (query.keyword) {
    params.set('keyword', query.keyword);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.channel_id) {
    params.set('channel_id', query.channel_id);
  }
  if (query.currency) {
    params.set('currency', query.currency);
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

export const listMarketingPriceLists = async (
  tenantId: string,
  query: MarketingPriceListListQuery
): Promise<MarketingPriceListListResponse> => {
  if (useMocks) {
    return listMockMarketingPriceLists(query);
  }
  return requestJson<MarketingPriceListListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/price-lists${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取渠道价表失败'
  );
};

export const getMarketingPriceList = async (
  tenantId: string,
  priceListId: string
): Promise<MarketingPriceListDetail> => {
  if (useMocks) {
    return getMockMarketingPriceList(priceListId);
  }
  return requestJson<MarketingPriceListDetail>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/price-lists/${priceListId}`,
    { headers: buildAuthHeaders() },
    '获取渠道价表失败'
  );
};

export const createMarketingPriceList = async (
  tenantId: string,
  payload: MarketingPriceListCreatePayload
): Promise<MarketingPriceList> => {
  if (useMocks) {
    return createMockMarketingPriceList(payload);
  }
  return requestJson<MarketingPriceList>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/price-lists`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建渠道价表失败'
  );
};

export const updateMarketingPriceList = async (
  tenantId: string,
  priceListId: string,
  payload: MarketingPriceListUpdatePayload
): Promise<MarketingPriceList> => {
  if (useMocks) {
    return updateMockMarketingPriceList(priceListId, payload);
  }
  return requestJson<MarketingPriceList>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/price-lists/${priceListId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新渠道价表失败'
  );
};

export const deleteMarketingPriceList = async (
  tenantId: string,
  priceListId: string
): Promise<MarketingPriceList> => {
  if (useMocks) {
    return deleteMockMarketingPriceList(priceListId);
  }
  return requestJson<MarketingPriceList>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/price-lists/${priceListId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除渠道价表失败'
  );
};

export const createMarketingPriceItem = async (
  tenantId: string,
  priceListId: string,
  payload: MarketingPriceItemCreatePayload
): Promise<MarketingPriceItem> => {
  if (useMocks) {
    return createMockMarketingPriceItem(priceListId, payload);
  }
  return requestJson<MarketingPriceItem>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/price-lists/${priceListId}/items`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '新增价表明细失败'
  );
};

export const updateMarketingPriceItem = async (
  tenantId: string,
  itemId: string,
  payload: MarketingPriceItemUpdatePayload
): Promise<MarketingPriceItem> => {
  if (useMocks) {
    return updateMockMarketingPriceItem(itemId, payload);
  }
  return requestJson<MarketingPriceItem>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/price-items/${itemId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新价表明细失败'
  );
};

export const deleteMarketingPriceItem = async (
  tenantId: string,
  itemId: string
): Promise<MarketingPriceItem> => {
  if (useMocks) {
    return deleteMockMarketingPriceItem(itemId);
  }
  return requestJson<MarketingPriceItem>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/marketing/price-items/${itemId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除价表明细失败'
  );
};
