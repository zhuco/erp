import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import { getMockPurchaseSuggestions } from '@/api/mocks';

export type PurchaseSuggestionReason = 'sales_velocity' | 'safety_stock' | 'low_stock';

export type PurchaseSuggestion = {
  sku_id: string;
  sku_code?: string | null;
  suggested_qty: number;
  reason: PurchaseSuggestionReason | string;
  computed_at: string;
};

export type PurchaseSuggestionListQuery = {
  sku_id?: string;
  reason?: string;
  offset?: number;
  limit?: number;
};

export type PurchaseSuggestionListResponse = {
  items: PurchaseSuggestion[];
  total: number;
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

const buildQueryString = (query: PurchaseSuggestionListQuery): string => {
  const params = new URLSearchParams();
  if (query.sku_id) {
    params.set('sku_id', query.sku_id);
  }
  if (query.reason) {
    params.set('reason', query.reason);
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

export const listPurchaseSuggestions = async (
  tenantId: string,
  query: PurchaseSuggestionListQuery
): Promise<PurchaseSuggestionListResponse> => {
  if (useMocks) {
    return getMockPurchaseSuggestions(query);
  }
  return requestJson<PurchaseSuggestionListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/purchase-suggestions${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    'Failed to load purchase suggestions'
  );
};
