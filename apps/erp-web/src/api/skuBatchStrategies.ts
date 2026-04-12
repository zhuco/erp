import { apiBaseUrl } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type SkuBatchStrategy = {
  strategy_id: string;
  tenant_id: string;
  sku_id?: string | null;
  name: string;
  enabled: boolean;
  expire_threshold_days: number;
  fifo_enabled: boolean;
  created_at?: string | null;
  updated_at?: string | null;
};

export type SkuBatchStrategyListQuery = {
  sku_id?: string;
  enabled?: boolean;
  offset?: number;
  limit?: number;
};

export type SkuBatchStrategyListResponse = {
  items: SkuBatchStrategy[];
  total: number;
};

export type SkuBatchStrategyPayload = {
  sku_id?: string | null;
  name: string;
  enabled?: boolean;
  expire_threshold_days?: number;
  fifo_enabled?: boolean;
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

const buildQueryString = (query: SkuBatchStrategyListQuery): string => {
  const params = new URLSearchParams();
  if (query.sku_id) {
    params.set('sku_id', query.sku_id);
  }
  if (query.enabled != null) {
    params.set('enabled', query.enabled ? 'true' : 'false');
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

export const listSkuBatchStrategies = async (
  tenantId: string,
  query: SkuBatchStrategyListQuery
): Promise<SkuBatchStrategyListResponse> =>
  requestJson<SkuBatchStrategyListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/sku-batch-strategies${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取批次策略失败'
  );

export const createSkuBatchStrategy = async (
  tenantId: string,
  payload: SkuBatchStrategyPayload
): Promise<SkuBatchStrategy> =>
  requestJson<SkuBatchStrategy>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/sku-batch-strategies`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建批次策略失败'
  );

export const updateSkuBatchStrategy = async (
  tenantId: string,
  strategyId: string,
  payload: SkuBatchStrategyPayload
): Promise<SkuBatchStrategy> =>
  requestJson<SkuBatchStrategy>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/sku-batch-strategies/${strategyId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新批次策略失败'
  );

export const deleteSkuBatchStrategy = async (
  tenantId: string,
  strategyId: string
): Promise<SkuBatchStrategy> =>
  requestJson<SkuBatchStrategy>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/sku-batch-strategies/${strategyId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除批次策略失败'
  );
