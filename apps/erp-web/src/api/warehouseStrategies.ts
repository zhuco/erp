import { apiBaseUrl } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type ShipmentPreprocessStrategy = {
  strategy_id: string;
  name: string;
  enabled: boolean;
  priority: number;
  effective_from?: string | null;
  effective_to?: string | null;
  conditions_json: unknown;
  rules_json: unknown;
  auto_freeze: boolean;
  auto_waybill_reserve: boolean;
  sender_address_id?: string | null;
  created_at: string;
  updated_at: string;
};

export type ShipmentPreprocessHit = {
  hit_id: string;
  strategy_id?: string | null;
  shipment_id?: string | null;
  order_id?: string | null;
  hit_reason?: string | null;
  result_json?: unknown;
  success: boolean;
  created_at: string;
};

export type ShipmentPreprocessPreviewResult = {
  warehouse_id?: string | null;
  carrier_code?: string | null;
  tracking_no?: string | null;
  weight_gram?: number | null;
  sender_address_id?: string | null;
  auto_freeze: boolean;
  auto_waybill_reserve: boolean;
};

export type ShipmentPreprocessPreviewResponse = {
  matched: boolean;
  strategy_id?: string | null;
  hit_reason?: string | null;
  result: ShipmentPreprocessPreviewResult;
};

export type PickRecommendStrategy = {
  strategy_id: string;
  name: string;
  enabled: boolean;
  priority: number;
  method: string;
  prefer_location_priority: boolean;
  prefer_pick_zone: boolean;
  prefer_sufficient: boolean;
  created_at: string;
  updated_at: string;
};

export type PickRecommendHit = {
  hit_id: string;
  strategy_id?: string | null;
  pick_task_id?: string | null;
  sku_id?: string | null;
  from_location_id?: string | null;
  reason?: string | null;
  success: boolean;
  created_at: string;
};

export type StockCountStrategy = {
  strategy_id: string;
  name: string;
  enabled: boolean;
  strategy_type: string;
  config_json: unknown;
  created_at: string;
  updated_at: string;
};

export type StockCountSuggestion = {
  suggestion_id: string;
  warehouse_id: string;
  strategy_id?: string | null;
  strategy_name?: string | null;
  reason?: string | null;
  item_count: number;
  items_json: unknown;
  status: string;
  created_at: string;
  updated_at: string;
};

export type ShipmentPreprocessStrategyListQuery = {
  enabled?: boolean;
  offset?: number;
  limit?: number;
};

export type ShipmentPreprocessStrategyListResponse = {
  items: ShipmentPreprocessStrategy[];
  total: number;
};

export type ShipmentPreprocessHitListQuery = {
  strategy_id?: string;
  shipment_id?: string;
  order_id?: string;
  offset?: number;
  limit?: number;
};

export type ShipmentPreprocessHitListResponse = {
  items: ShipmentPreprocessHit[];
  total: number;
};

export type PickRecommendStrategyListQuery = {
  enabled?: boolean;
  offset?: number;
  limit?: number;
};

export type PickRecommendStrategyListResponse = {
  items: PickRecommendStrategy[];
  total: number;
};

export type PickRecommendHitListQuery = {
  strategy_id?: string;
  pick_task_id?: string;
  offset?: number;
  limit?: number;
};

export type PickRecommendHitListResponse = {
  items: PickRecommendHit[];
  total: number;
};

export type StockCountStrategyListQuery = {
  enabled?: boolean;
  offset?: number;
  limit?: number;
};

export type StockCountStrategyListResponse = {
  items: StockCountStrategy[];
  total: number;
};

export type StockCountSuggestionListQuery = {
  warehouse_id?: string;
  status?: string;
  offset?: number;
  limit?: number;
};

export type StockCountSuggestionListResponse = {
  items: StockCountSuggestion[];
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

export const previewShipmentPreprocess = async (
  tenantId: string,
  orderId: string,
  payload: {
    warehouse_id?: string;
    carrier_code?: string;
    tracking_no?: string;
    weight_gram?: number | null;
  }
): Promise<ShipmentPreprocessPreviewResponse> =>
  requestJson<ShipmentPreprocessPreviewResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/orders/${orderId}/shipment-preprocess`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '获取发货预处理失败'
  );

export const listShipmentPreprocessStrategies = async (
  tenantId: string,
  query: ShipmentPreprocessStrategyListQuery
): Promise<ShipmentPreprocessStrategyListResponse> =>
  requestJson<ShipmentPreprocessStrategyListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/shipment-preprocess-strategies${buildQueryString(
      query
    )}`,
    { headers: buildAuthHeaders() },
    '加载发货预处理策略失败'
  );

export const createShipmentPreprocessStrategy = async (
  tenantId: string,
  payload: Partial<ShipmentPreprocessStrategy>
): Promise<ShipmentPreprocessStrategy> =>
  requestJson<ShipmentPreprocessStrategy>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/shipment-preprocess-strategies`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', ...buildAuthHeaders() },
      body: JSON.stringify(payload)
    },
    '创建发货预处理策略失败'
  );

export const updateShipmentPreprocessStrategy = async (
  tenantId: string,
  strategyId: string,
  payload: Partial<ShipmentPreprocessStrategy>
): Promise<ShipmentPreprocessStrategy> =>
  requestJson<ShipmentPreprocessStrategy>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/shipment-preprocess-strategies/${strategyId}`,
    {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', ...buildAuthHeaders() },
      body: JSON.stringify(payload)
    },
    '更新发货预处理策略失败'
  );

export const deleteShipmentPreprocessStrategy = async (
  tenantId: string,
  strategyId: string
): Promise<ShipmentPreprocessStrategy> =>
  requestJson<ShipmentPreprocessStrategy>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/shipment-preprocess-strategies/${strategyId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除发货预处理策略失败'
  );

export const listShipmentPreprocessHits = async (
  tenantId: string,
  query: ShipmentPreprocessHitListQuery
): Promise<ShipmentPreprocessHitListResponse> =>
  requestJson<ShipmentPreprocessHitListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/shipment-preprocess-hits${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '加载发货预处理命中失败'
  );

export const listPickRecommendStrategies = async (
  tenantId: string,
  query: PickRecommendStrategyListQuery
): Promise<PickRecommendStrategyListResponse> =>
  requestJson<PickRecommendStrategyListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/pick-recommend-strategies${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '加载货位推荐策略失败'
  );

export const createPickRecommendStrategy = async (
  tenantId: string,
  payload: Partial<PickRecommendStrategy>
): Promise<PickRecommendStrategy> =>
  requestJson<PickRecommendStrategy>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/pick-recommend-strategies`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', ...buildAuthHeaders() },
      body: JSON.stringify(payload)
    },
    '创建货位推荐策略失败'
  );

export const updatePickRecommendStrategy = async (
  tenantId: string,
  strategyId: string,
  payload: Partial<PickRecommendStrategy>
): Promise<PickRecommendStrategy> =>
  requestJson<PickRecommendStrategy>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/pick-recommend-strategies/${strategyId}`,
    {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', ...buildAuthHeaders() },
      body: JSON.stringify(payload)
    },
    '更新货位推荐策略失败'
  );

export const deletePickRecommendStrategy = async (
  tenantId: string,
  strategyId: string
): Promise<PickRecommendStrategy> =>
  requestJson<PickRecommendStrategy>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/pick-recommend-strategies/${strategyId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除货位推荐策略失败'
  );

export const listPickRecommendHits = async (
  tenantId: string,
  query: PickRecommendHitListQuery
): Promise<PickRecommendHitListResponse> =>
  requestJson<PickRecommendHitListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/pick-recommend-hits${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '加载货位推荐命中失败'
  );

export const listStockCountStrategies = async (
  tenantId: string,
  query: StockCountStrategyListQuery
): Promise<StockCountStrategyListResponse> =>
  requestJson<StockCountStrategyListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/stock-count-strategies${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '加载盘点策略失败'
  );

export const createStockCountStrategy = async (
  tenantId: string,
  payload: Partial<StockCountStrategy>
): Promise<StockCountStrategy> =>
  requestJson<StockCountStrategy>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/stock-count-strategies`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', ...buildAuthHeaders() },
      body: JSON.stringify(payload)
    },
    '创建盘点策略失败'
  );

export const updateStockCountStrategy = async (
  tenantId: string,
  strategyId: string,
  payload: Partial<StockCountStrategy>
): Promise<StockCountStrategy> =>
  requestJson<StockCountStrategy>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/stock-count-strategies/${strategyId}`,
    {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', ...buildAuthHeaders() },
      body: JSON.stringify(payload)
    },
    '更新盘点策略失败'
  );

export const deleteStockCountStrategy = async (
  tenantId: string,
  strategyId: string
): Promise<StockCountStrategy> =>
  requestJson<StockCountStrategy>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/stock-count-strategies/${strategyId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除盘点策略失败'
  );

export const refreshStockCountSuggestions = async (
  tenantId: string,
  payload: { warehouse_id?: string }
): Promise<{ created: number }> =>
  requestJson<{ created: number }>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/stock-count-suggestions`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', ...buildAuthHeaders() },
      body: JSON.stringify(payload)
    },
    '刷新盘点建议失败'
  );

export const listStockCountSuggestions = async (
  tenantId: string,
  query: StockCountSuggestionListQuery
): Promise<StockCountSuggestionListResponse> =>
  requestJson<StockCountSuggestionListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/stock-count-suggestions${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '加载盘点建议失败'
  );

export const applyStockCountSuggestion = async (
  tenantId: string,
  suggestionId: string
): Promise<{ count_id: string }> =>
  requestJson<{ count_id: string }>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/stock-count-suggestions/${suggestionId}/apply`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '生成盘点单失败'
  );
