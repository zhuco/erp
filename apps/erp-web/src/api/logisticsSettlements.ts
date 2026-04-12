import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  confirmMockLogisticsSettlementBatch,
  getMockLogisticsSettlementRule,
  listMockLogisticsSettlementBatches,
  updateMockLogisticsSettlementRule
} from '@/api/mocks';

export type LogisticsSettlementStatus = 'DRAFT' | 'CONFIRMED' | 'PAID';

export type LogisticsSettlementRule = {
  tenant_id: string;
  weight_rounding: string;
  min_fee_cent: number;
  oversize_fee_cent: number;
  overtime_fee_cent: number;
  created_at?: string;
  updated_at?: string;
};

export type LogisticsSettlementBatch = {
  batch_id: string;
  tenant_id: string;
  carrier: string;
  period_start: string;
  period_end: string;
  total_shipments: number;
  expected_cent: number;
  actual_cent: number;
  diff_cent: number;
  status: LogisticsSettlementStatus | string;
  deductions: string[];
  payment_plan_id?: string | null;
  created_at: string;
};

export type LogisticsSettlementBatchListQuery = {
  status?: string;
  offset?: number;
  limit?: number;
};

export type LogisticsSettlementBatchListResponse = {
  items: LogisticsSettlementBatch[];
  total: number;
};

export type LogisticsSettlementRuleUpdatePayload = {
  weight_rounding: string;
  min_fee_cent: number;
  oversize_fee_cent: number;
  overtime_fee_cent: number;
};

export type LogisticsSettlementBatchConfirmPayload = {
  payment_plan_id?: string | null;
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

const buildQueryString = (query: LogisticsSettlementBatchListQuery): string => {
  const params = new URLSearchParams();
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

export const getLogisticsSettlementRule = async (
  tenantId: string
): Promise<LogisticsSettlementRule> => {
  if (useMocks) {
    return getMockLogisticsSettlementRule();
  }
  return requestJson<LogisticsSettlementRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/settlements/rules`,
    { headers: buildAuthHeaders() },
    '获取结算规则失败'
  );
};

export const updateLogisticsSettlementRule = async (
  tenantId: string,
  payload: LogisticsSettlementRuleUpdatePayload
): Promise<LogisticsSettlementRule> => {
  if (useMocks) {
    return updateMockLogisticsSettlementRule(payload);
  }
  return requestJson<LogisticsSettlementRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/settlements/rules`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新结算规则失败'
  );
};

export const listLogisticsSettlementBatches = async (
  tenantId: string,
  query: LogisticsSettlementBatchListQuery
): Promise<LogisticsSettlementBatchListResponse> => {
  if (useMocks) {
    return listMockLogisticsSettlementBatches(query);
  }
  return requestJson<LogisticsSettlementBatchListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/settlements/batches${buildQueryString(
      query
    )}`,
    { headers: buildAuthHeaders() },
    '获取结算批次失败'
  );
};

export const confirmLogisticsSettlementBatch = async (
  tenantId: string,
  batchId: string,
  payload: LogisticsSettlementBatchConfirmPayload
): Promise<LogisticsSettlementBatch> => {
  if (useMocks) {
    return confirmMockLogisticsSettlementBatch(batchId, payload);
  }
  return requestJson<LogisticsSettlementBatch>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/logistics/settlements/batches/${batchId}/confirm`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '确认结算批次失败'
  );
};
