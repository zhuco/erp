import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockAfterSalesInspection,
  createMockAfterSalesIntercept,
  createMockAfterSalesSlaRule,
  deleteMockAfterSalesSlaRule,
  escalateMockAfterSalesIntercept,
  listMockAfterSalesInspections,
  listMockAfterSalesIntercepts,
  listMockAfterSalesSlaRules,
  retryMockAfterSalesIntercept,
  updateMockAfterSalesInspection,
  updateMockAfterSalesSlaRule
} from '@/api/mocks';

import type { AfterSalesType } from '@/api/afterSalesOps';

export type AfterSalesInterceptStatus = 'PROCESSING' | 'SUCCESS' | 'FAILED' | 'ESCALATED';
export type AfterSalesInspectionResult = 'PENDING' | 'RESALE' | 'REPAIR' | 'SCRAP';
export type AfterSalesSlaStatus = 'ACTIVE' | 'DISABLED';

export type AfterSalesIntercept = {
  intercept_id: string;
  order_id: string;
  after_sales_id?: string | null;
  carrier_name?: string | null;
  tracking_no?: string | null;
  status: AfterSalesInterceptStatus;
  reason?: string | null;
  requested_at: string;
  completed_at?: string | null;
  created_at: string;
  updated_at: string;
};

export type AfterSalesInspection = {
  inspection_id: string;
  after_sales_id: string;
  sku_id: string;
  warehouse_id?: string | null;
  qty: number;
  result_code: AfterSalesInspectionResult;
  inspector_name?: string | null;
  inspected_at?: string | null;
  created_at: string;
  updated_at: string;
};

export type AfterSalesSlaRule = {
  rule_id: string;
  after_sales_type: AfterSalesType;
  response_hours: number;
  resolution_hours: number;
  escalation_role?: string | null;
  status: AfterSalesSlaStatus;
  created_at: string;
  updated_at: string;
};

export type AfterSalesInterceptListQuery = {
  status?: AfterSalesInterceptStatus;
  order_id?: string;
  tracking_no?: string;
  requested_from?: string;
  requested_to?: string;
  offset?: number;
  limit?: number;
};

export type AfterSalesInspectionListQuery = {
  after_sales_id?: string;
  sku_id?: string;
  warehouse_id?: string;
  result_code?: AfterSalesInspectionResult;
  inspected_from?: string;
  inspected_to?: string;
  offset?: number;
  limit?: number;
};

export type AfterSalesSlaRuleListQuery = {
  after_sales_type?: AfterSalesType;
  status?: AfterSalesSlaStatus;
  offset?: number;
  limit?: number;
};

export type AfterSalesInterceptListResponse = {
  items: AfterSalesIntercept[];
  total: number;
  limit: number;
  offset: number;
};

export type AfterSalesInspectionListResponse = {
  items: AfterSalesInspection[];
  total: number;
  limit: number;
  offset: number;
};

export type AfterSalesSlaRuleListResponse = {
  items: AfterSalesSlaRule[];
  total: number;
  limit: number;
  offset: number;
};

export type AfterSalesInterceptCreatePayload = {
  order_id: string;
  after_sales_id?: string;
  carrier_name?: string;
  tracking_no?: string;
  status?: AfterSalesInterceptStatus;
  reason?: string;
  requested_at?: string;
};

export type AfterSalesInspectionCreatePayload = {
  after_sales_id: string;
  sku_id: string;
  warehouse_id?: string | null;
  qty: number;
  result_code?: AfterSalesInspectionResult;
  inspector_name?: string | null;
  inspected_at?: string | null;
};

export type AfterSalesInspectionUpdatePayload = {
  warehouse_id?: string | null;
  qty?: number | null;
  result_code?: AfterSalesInspectionResult;
  inspector_name?: string | null;
  inspected_at?: string | null;
};

export type AfterSalesSlaRulePayload = {
  after_sales_type: AfterSalesType;
  response_hours: number;
  resolution_hours: number;
  escalation_role?: string | null;
  status?: AfterSalesSlaStatus;
};

export type AfterSalesSlaRuleUpdatePayload = {
  after_sales_type?: AfterSalesType;
  response_hours?: number | null;
  resolution_hours?: number | null;
  escalation_role?: string | null;
  status?: AfterSalesSlaStatus;
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

const createIdempotencyKey = (prefix: string): string => {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return `${prefix}-${crypto.randomUUID()}`;
  }
  return `${prefix}-${Date.now()}-${Math.random().toString(16).slice(2)}`;
};

export const listAfterSalesIntercepts = async (
  tenantId: string,
  query: AfterSalesInterceptListQuery
): Promise<AfterSalesInterceptListResponse> => {
  if (useMocks) {
    return listMockAfterSalesIntercepts(query);
  }
  return requestJson<AfterSalesInterceptListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/intercepts${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取售后拦截列表失败'
  );
};

export const createAfterSalesIntercept = async (
  tenantId: string,
  payload: AfterSalesInterceptCreatePayload
): Promise<AfterSalesIntercept> => {
  if (useMocks) {
    return createMockAfterSalesIntercept(payload);
  }
  return requestJson<AfterSalesIntercept>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/intercepts`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Idempotency-Key': createIdempotencyKey('as-intercept'),
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建售后拦截失败'
  );
};

export const retryAfterSalesIntercept = async (
  tenantId: string,
  interceptId: string
): Promise<AfterSalesIntercept> => {
  if (useMocks) {
    return retryMockAfterSalesIntercept(interceptId);
  }
  return requestJson<AfterSalesIntercept>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/intercepts/${interceptId}/retry`,
    {
      method: 'POST',
      headers: {
        'Idempotency-Key': createIdempotencyKey('as-intercept-retry'),
        ...buildAuthHeaders()
      }
    },
    '重试拦截失败'
  );
};

export const escalateAfterSalesIntercept = async (
  tenantId: string,
  interceptId: string
): Promise<AfterSalesIntercept> => {
  if (useMocks) {
    return escalateMockAfterSalesIntercept(interceptId);
  }
  return requestJson<AfterSalesIntercept>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/intercepts/${interceptId}/escalate`,
    {
      method: 'POST',
      headers: {
        'Idempotency-Key': createIdempotencyKey('as-intercept-escalate'),
        ...buildAuthHeaders()
      }
    },
    '升级拦截失败'
  );
};

export const listAfterSalesInspections = async (
  tenantId: string,
  query: AfterSalesInspectionListQuery
): Promise<AfterSalesInspectionListResponse> => {
  if (useMocks) {
    return listMockAfterSalesInspections(query);
  }
  return requestJson<AfterSalesInspectionListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/inspections${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取售后质检列表失败'
  );
};

export const createAfterSalesInspection = async (
  tenantId: string,
  payload: AfterSalesInspectionCreatePayload
): Promise<AfterSalesInspection> => {
  if (useMocks) {
    return createMockAfterSalesInspection(payload);
  }
  return requestJson<AfterSalesInspection>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/inspections`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Idempotency-Key': createIdempotencyKey('as-inspection'),
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建售后质检失败'
  );
};

export const updateAfterSalesInspection = async (
  tenantId: string,
  inspectionId: string,
  payload: AfterSalesInspectionUpdatePayload
): Promise<AfterSalesInspection> => {
  if (useMocks) {
    return updateMockAfterSalesInspection(inspectionId, payload);
  }
  return requestJson<AfterSalesInspection>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/inspections/${inspectionId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        'Idempotency-Key': createIdempotencyKey('as-inspection-update'),
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新售后质检失败'
  );
};

export const listAfterSalesSlaRules = async (
  tenantId: string,
  query: AfterSalesSlaRuleListQuery
): Promise<AfterSalesSlaRuleListResponse> => {
  if (useMocks) {
    return listMockAfterSalesSlaRules(query);
  }
  return requestJson<AfterSalesSlaRuleListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/sla-rules${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取售后SLA规则失败'
  );
};

export const createAfterSalesSlaRule = async (
  tenantId: string,
  payload: AfterSalesSlaRulePayload
): Promise<AfterSalesSlaRule> => {
  if (useMocks) {
    return createMockAfterSalesSlaRule(payload);
  }
  return requestJson<AfterSalesSlaRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/sla-rules`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Idempotency-Key': createIdempotencyKey('as-sla'),
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建售后SLA规则失败'
  );
};

export const updateAfterSalesSlaRule = async (
  tenantId: string,
  ruleId: string,
  payload: AfterSalesSlaRuleUpdatePayload
): Promise<AfterSalesSlaRule> => {
  if (useMocks) {
    return updateMockAfterSalesSlaRule(ruleId, payload);
  }
  return requestJson<AfterSalesSlaRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/sla-rules/${ruleId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        'Idempotency-Key': createIdempotencyKey('as-sla-update'),
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新售后SLA规则失败'
  );
};

export const deleteAfterSalesSlaRule = async (
  tenantId: string,
  ruleId: string
): Promise<void> => {
  if (useMocks) {
    deleteMockAfterSalesSlaRule(ruleId);
    return;
  }
  await requestJson(
    `${apiBaseUrl}/v1/tenants/${tenantId}/after-sales/sla-rules/${ruleId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除售后SLA规则失败'
  );
};
