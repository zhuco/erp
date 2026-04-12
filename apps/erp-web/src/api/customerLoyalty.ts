import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  getMockCustomerPointLedgers,
  getMockCustomerStoredValueLedgers
} from '@/api/mocks';

export type CustomerPointLedger = {
  ledger_id: string;
  customer_id: string;
  customer_name: string;
  change_points: number;
  balance_points: number;
  reason: string;
  ref_type?: string | null;
  ref_id?: string | null;
  created_at: string;
};

export type CustomerStoredValueLedger = {
  ledger_id: string;
  customer_id: string;
  customer_name: string;
  change_cent: number;
  balance_cent: number;
  reason: string;
  ref_type?: string | null;
  ref_id?: string | null;
  created_at: string;
};

export type CustomerPointLedgerListQuery = {
  keyword?: string;
  customer_id?: string;
  created_from?: string;
  created_to?: string;
  offset?: number;
  limit?: number;
};

export type CustomerStoredValueLedgerListQuery = {
  keyword?: string;
  customer_id?: string;
  created_from?: string;
  created_to?: string;
  offset?: number;
  limit?: number;
};

export type CustomerPointLedgerListResponse = {
  items: CustomerPointLedger[];
  total: number;
};

export type CustomerStoredValueLedgerListResponse = {
  items: CustomerStoredValueLedger[];
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

const buildQueryString = (
  query: CustomerPointLedgerListQuery | CustomerStoredValueLedgerListQuery
): string => {
  const params = new URLSearchParams();
  if (query.keyword) {
    params.set('keyword', query.keyword);
  }
  if (query.customer_id) {
    params.set('customer_id', query.customer_id);
  }
  if (query.created_from) {
    params.set('created_from', query.created_from);
  }
  if (query.created_to) {
    params.set('created_to', query.created_to);
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

export const listCustomerPointLedgers = async (
  tenantId: string,
  query: CustomerPointLedgerListQuery
): Promise<CustomerPointLedgerListResponse> => {
  if (useMocks) {
    return getMockCustomerPointLedgers(query);
  }
  return requestJson<CustomerPointLedgerListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/crm/points-ledgers${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取积分流水失败'
  );
};

export const listCustomerStoredValueLedgers = async (
  tenantId: string,
  query: CustomerStoredValueLedgerListQuery
): Promise<CustomerStoredValueLedgerListResponse> => {
  if (useMocks) {
    return getMockCustomerStoredValueLedgers(query);
  }
  return requestJson<CustomerStoredValueLedgerListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/crm/stored-value-ledgers${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取储值流水失败'
  );
};
