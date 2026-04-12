import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockAfterSalesQualityRule,
  createMockAfterSalesReason,
  deleteMockAfterSalesQualityRule,
  deleteMockAfterSalesReason,
  getMockAfterSalesQualityRuleList,
  getMockAfterSalesReasonList,
  updateMockAfterSalesQualityRule,
  updateMockAfterSalesReason
} from '@/api/mocks';

export type AfterSalesReason = {
  reason_code: string;
  label: string;
  category?: string | null;
  status: string;
  sort_order: number;
  created_at: string;
  updated_at: string;
};

export type AfterSalesQualityRule = {
  rule_id: string;
  after_sales_type: string;
  result_code: string;
  status: string;
  sort_order: number;
  created_at: string;
  updated_at: string;
};

export type AfterSalesReasonListQuery = {
  category?: string;
  status?: string;
  offset?: number;
  limit?: number;
};

export type AfterSalesQualityRuleListQuery = {
  after_sales_type?: string;
  status?: string;
  offset?: number;
  limit?: number;
};

export type AfterSalesReasonListResponse = {
  items: AfterSalesReason[];
  total: number;
  limit: number;
  offset: number;
};

export type AfterSalesQualityRuleListResponse = {
  items: AfterSalesQualityRule[];
  total: number;
  limit: number;
  offset: number;
};

export type AfterSalesReasonPayload = {
  reason_code?: string;
  label: string;
  category?: string;
  status?: string;
  sort_order?: number;
};

export type AfterSalesQualityRulePayload = {
  after_sales_type: string;
  result_code: string;
  status?: string;
  sort_order?: number;
};

const buildQueryString = (query: Record<string, unknown>): string => {
  const params = new URLSearchParams();
  Object.entries(query).forEach(([key, value]) => {
    if (value === undefined || value === null || value === '') {
      return;
    }
    params.set(key, String(value));
  });
  const serialized = params.toString();
  return serialized ? `?${serialized}` : '';
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

export const listAfterSalesReasons = async (
  tenantId: string,
  query: AfterSalesReasonListQuery
): Promise<AfterSalesReasonListResponse> => {
  if (useMocks) {
    return getMockAfterSalesReasonList(query);
  }
  return requestJson<AfterSalesReasonListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/reasons${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取售后原因失败'
  );
};

export const createAfterSalesReason = async (
  tenantId: string,
  payload: AfterSalesReasonPayload
): Promise<AfterSalesReason> => {
  if (useMocks) {
    return createMockAfterSalesReason(payload);
  }
  return requestJson<AfterSalesReason>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/reasons`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建售后原因失败'
  );
};

export const updateAfterSalesReason = async (
  tenantId: string,
  reasonCode: string,
  payload: AfterSalesReasonPayload
): Promise<AfterSalesReason> => {
  if (useMocks) {
    return updateMockAfterSalesReason(reasonCode, payload);
  }
  return requestJson<AfterSalesReason>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/reasons/${reasonCode}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新售后原因失败'
  );
};

export const deleteAfterSalesReason = async (
  tenantId: string,
  reasonCode: string
): Promise<void> => {
  if (useMocks) {
    deleteMockAfterSalesReason(reasonCode);
    return;
  }
  await requestJson(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/reasons/${reasonCode}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除售后原因失败'
  );
};

export const listAfterSalesQualityRules = async (
  tenantId: string,
  query: AfterSalesQualityRuleListQuery
): Promise<AfterSalesQualityRuleListResponse> => {
  if (useMocks) {
    return getMockAfterSalesQualityRuleList(query);
  }
  return requestJson<AfterSalesQualityRuleListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/quality-rules${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取质检规则失败'
  );
};

export const createAfterSalesQualityRule = async (
  tenantId: string,
  payload: AfterSalesQualityRulePayload
): Promise<AfterSalesQualityRule> => {
  if (useMocks) {
    return createMockAfterSalesQualityRule(payload);
  }
  return requestJson<AfterSalesQualityRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/quality-rules`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建质检规则失败'
  );
};

export const updateAfterSalesQualityRule = async (
  tenantId: string,
  ruleId: string,
  payload: AfterSalesQualityRulePayload
): Promise<AfterSalesQualityRule> => {
  if (useMocks) {
    return updateMockAfterSalesQualityRule(ruleId, payload);
  }
  return requestJson<AfterSalesQualityRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/quality-rules/${ruleId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新质检规则失败'
  );
};

export const deleteAfterSalesQualityRule = async (
  tenantId: string,
  ruleId: string
): Promise<void> => {
  if (useMocks) {
    deleteMockAfterSalesQualityRule(ruleId);
    return;
  }
  await requestJson(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/quality-rules/${ruleId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除质检规则失败'
  );
};
