import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type PlanningListResponse<T> = {
  items: T[];
  total: number;
};

export type PlanningDemandForecast = {
  forecast_id: string;
  sku_id: string;
  sku_code?: string | null;
  warehouse_id: string;
  forecast_qty: number;
  model: string;
  confidence: string;
  version: string;
  updated_at: string;
};

export type PlanningDemandForecastUpsertPayload = {
  sku_id: string;
  sku_code?: string | null;
  warehouse_id: string;
  forecast_qty: number;
  model?: string | null;
  confidence?: string | null;
  version?: string | null;
};

export type PlanningReplenishmentRule = {
  rule_id: string;
  warehouse_id: string;
  category: string;
  safety_stock_days: number;
  replenishment_cycle_days: number;
  min_replenishment_qty: number;
  status: string;
  updated_at: string;
};

export type PlanningReplenishmentRuleUpsertPayload = {
  warehouse_id: string;
  category: string;
  safety_stock_days: number;
  replenishment_cycle_days: number;
  min_replenishment_qty: number;
  status?: string | null;
};

export type PlanningPurchasePlan = {
  plan_id: string;
  supplier: string;
  warehouse_id: string;
  items: number;
  total_qty: number;
  status: string;
  eta?: string | null;
  source_rule_id?: string | null;
  created_at: string;
};

export type PlanningTransferSuggestion = {
  suggestion_id: string;
  from_warehouse_id: string;
  to_warehouse_id: string;
  sku_code: string;
  qty: number;
  reason: string;
  priority: string;
  status: string;
  created_at: string;
};

export type PlanningPromoStockingCampaign = {
  campaign_id: string;
  campaign_name: string;
  period_start: string;
  period_end: string;
  locked_budget_cent: number;
  status: string;
  created_at: string;
};

export type PlanningPromoStockingGeneratePayload = {
  campaign_name: string;
  period_start: string;
  period_end: string;
  locked_budget_cent?: number | null;
};

export type PlanningPlanExecution = {
  execution_id: string;
  plan_type: string;
  related_plan: string;
  owner: string;
  progress: number;
  status: string;
  variance: string;
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

type MockPlanningStore = {
  forecasts: PlanningDemandForecast[];
  rules: PlanningReplenishmentRule[];
  purchasePlans: PlanningPurchasePlan[];
  transferSuggestions: PlanningTransferSuggestion[];
  promoCampaigns: PlanningPromoStockingCampaign[];
};

const mockStoreByTenant = new Map<string, MockPlanningStore>();

const nextMockId = (prefix: string) => `${prefix}-${Date.now()}-${Math.floor(Math.random() * 1000)}`;

const getMockStore = (tenantId: string): MockPlanningStore => {
  const existing = mockStoreByTenant.get(tenantId);
  if (existing) {
    return existing;
  }
  const now = new Date().toISOString();
  const seeded: MockPlanningStore = {
    forecasts: [
      {
        forecast_id: nextMockId('FC'),
        sku_id: '00000000-0000-0000-0000-000000000011',
        sku_code: 'SKU-PLAN-001',
        warehouse_id: '00000000-0000-0000-0000-000000000021',
        forecast_qty: 480,
        model: 'SEASONAL',
        confidence: 'HIGH',
        version: '2026-W15',
        updated_at: now
      }
    ],
    rules: [
      {
        rule_id: nextMockId('RULE'),
        warehouse_id: '00000000-0000-0000-0000-000000000021',
        category: 'BASIC',
        safety_stock_days: 15,
        replenishment_cycle_days: 7,
        min_replenishment_qty: 120,
        status: 'ACTIVE',
        updated_at: now
      }
    ],
    purchasePlans: [],
    transferSuggestions: [],
    promoCampaigns: []
  };
  mockStoreByTenant.set(tenantId, seeded);
  return seeded;
};

const listResponse = <T>(items: T[]): PlanningListResponse<T> => ({ items, total: items.length });

export const listPlanningDemandForecasts = async (
  tenantId: string
): Promise<PlanningListResponse<PlanningDemandForecast>> => {
  if (useMocks) {
    return listResponse(getMockStore(tenantId).forecasts);
  }
  return requestJson<PlanningListResponse<PlanningDemandForecast>>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/planning/forecasts`,
    { headers: buildAuthHeaders() },
    'Failed to list planning demand forecasts'
  );
};

export const upsertPlanningDemandForecast = async (
  tenantId: string,
  payload: PlanningDemandForecastUpsertPayload
): Promise<PlanningDemandForecast> => {
  if (useMocks) {
    const store = getMockStore(tenantId);
    const now = new Date().toISOString();
    const matched = store.forecasts.find(
      (item) => item.sku_id === payload.sku_id && item.warehouse_id === payload.warehouse_id
    );
    if (matched) {
      matched.sku_code = payload.sku_code ?? matched.sku_code;
      matched.forecast_qty = payload.forecast_qty;
      matched.model = payload.model ?? matched.model;
      matched.confidence = payload.confidence ?? matched.confidence;
      matched.version = payload.version ?? matched.version;
      matched.updated_at = now;
      return { ...matched };
    }
    const record: PlanningDemandForecast = {
      forecast_id: nextMockId('FC'),
      sku_id: payload.sku_id,
      sku_code: payload.sku_code ?? null,
      warehouse_id: payload.warehouse_id,
      forecast_qty: payload.forecast_qty,
      model: payload.model ?? 'SEASONAL',
      confidence: payload.confidence ?? 'MEDIUM',
      version: payload.version ?? 'LATEST',
      updated_at: now
    };
    store.forecasts.unshift(record);
    return record;
  }

  return requestJson<PlanningDemandForecast>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/planning/forecasts`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    'Failed to upsert planning demand forecast'
  );
};

export const listPlanningReplenishmentRules = async (
  tenantId: string
): Promise<PlanningListResponse<PlanningReplenishmentRule>> => {
  if (useMocks) {
    return listResponse(getMockStore(tenantId).rules);
  }
  return requestJson<PlanningListResponse<PlanningReplenishmentRule>>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/planning/replenishment-rules`,
    { headers: buildAuthHeaders() },
    'Failed to list planning replenishment rules'
  );
};

export const upsertPlanningReplenishmentRule = async (
  tenantId: string,
  payload: PlanningReplenishmentRuleUpsertPayload
): Promise<PlanningReplenishmentRule> => {
  if (useMocks) {
    const store = getMockStore(tenantId);
    const now = new Date().toISOString();
    const matched = store.rules.find(
      (item) =>
        item.warehouse_id === payload.warehouse_id &&
        item.category.toUpperCase() === payload.category.toUpperCase()
    );
    if (matched) {
      matched.safety_stock_days = payload.safety_stock_days;
      matched.replenishment_cycle_days = payload.replenishment_cycle_days;
      matched.min_replenishment_qty = payload.min_replenishment_qty;
      matched.status = payload.status ?? matched.status;
      matched.updated_at = now;
      return { ...matched };
    }
    const record: PlanningReplenishmentRule = {
      rule_id: nextMockId('RULE'),
      warehouse_id: payload.warehouse_id,
      category: payload.category.toUpperCase(),
      safety_stock_days: payload.safety_stock_days,
      replenishment_cycle_days: payload.replenishment_cycle_days,
      min_replenishment_qty: payload.min_replenishment_qty,
      status: payload.status ?? 'ACTIVE',
      updated_at: now
    };
    store.rules.unshift(record);
    return record;
  }
  return requestJson<PlanningReplenishmentRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/planning/replenishment-rules`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    'Failed to upsert planning replenishment rule'
  );
};

export const listPlanningPurchasePlans = async (
  tenantId: string
): Promise<PlanningListResponse<PlanningPurchasePlan>> => {
  if (useMocks) {
    return listResponse(getMockStore(tenantId).purchasePlans);
  }
  return requestJson<PlanningListResponse<PlanningPurchasePlan>>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/planning/purchase-plans`,
    { headers: buildAuthHeaders() },
    'Failed to list planning purchase plans'
  );
};

export const generatePlanningPurchasePlans = async (
  tenantId: string,
  trigger = 'REPLENISHMENT'
): Promise<PlanningListResponse<PlanningPurchasePlan>> => {
  if (useMocks) {
    const store = getMockStore(tenantId);
    const now = new Date().toISOString();
    const created = (store.rules.length ? store.rules : [null]).map((rule, index) => ({
      plan_id: nextMockId('PP'),
      supplier: 'DEFAULT_SUPPLIER',
      warehouse_id: rule?.warehouse_id ?? `MOCK-WH-${index + 1}`,
      items: 1,
      total_qty: Math.max(rule?.min_replenishment_qty ?? 80, 80),
      status: 'PENDING_APPROVAL',
      eta: now.slice(0, 10),
      source_rule_id: rule?.rule_id ?? null,
      created_at: now
    }));
    store.purchasePlans.unshift(...created);
    return listResponse(created);
  }
  return requestJson<PlanningListResponse<PlanningPurchasePlan>>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/planning/purchase-plans/generate`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify({ trigger })
    },
    'Failed to generate planning purchase plans'
  );
};

export const listPlanningTransferSuggestions = async (
  tenantId: string
): Promise<PlanningListResponse<PlanningTransferSuggestion>> => {
  if (useMocks) {
    return listResponse(getMockStore(tenantId).transferSuggestions);
  }
  return requestJson<PlanningListResponse<PlanningTransferSuggestion>>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/planning/transfer-suggestions`,
    { headers: buildAuthHeaders() },
    'Failed to list planning transfer suggestions'
  );
};

export const generatePlanningTransferSuggestions = async (
  tenantId: string,
  strategy = 'BALANCE'
): Promise<PlanningListResponse<PlanningTransferSuggestion>> => {
  if (useMocks) {
    const store = getMockStore(tenantId);
    const now = new Date().toISOString();
    const basePlans = store.purchasePlans.length
      ? store.purchasePlans
      : [
          {
            plan_id: nextMockId('PP'),
            supplier: 'DEFAULT_SUPPLIER',
            warehouse_id: 'MOCK-WH-TARGET',
            items: 1,
            total_qty: 90,
            status: 'PENDING_APPROVAL',
            eta: now.slice(0, 10),
            source_rule_id: null,
            created_at: now
          }
        ];
    const created = basePlans.slice(0, 3).map((plan, index) => ({
      suggestion_id: nextMockId('TS'),
      from_warehouse_id: `MOCK-WH-SRC-${index + 1}`,
      to_warehouse_id: plan.warehouse_id,
      sku_code: `SKU-TRANSFER-${index + 1}`,
      qty: Math.max(Math.floor(plan.total_qty / 3), 20),
      reason: `AUTO_${strategy}`,
      priority: index === 0 ? 'HIGH' : 'MEDIUM',
      status: 'PENDING',
      created_at: now
    }));
    store.transferSuggestions.unshift(...created);
    return listResponse(created);
  }
  return requestJson<PlanningListResponse<PlanningTransferSuggestion>>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/planning/transfer-suggestions/generate`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify({ strategy })
    },
    'Failed to generate planning transfer suggestions'
  );
};

export const listPlanningPromoStocking = async (
  tenantId: string
): Promise<PlanningListResponse<PlanningPromoStockingCampaign>> => {
  if (useMocks) {
    return listResponse(getMockStore(tenantId).promoCampaigns);
  }
  return requestJson<PlanningListResponse<PlanningPromoStockingCampaign>>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/planning/promo-stocking`,
    { headers: buildAuthHeaders() },
    'Failed to list planning promo stocking campaigns'
  );
};

export const generatePlanningPromoStocking = async (
  tenantId: string,
  payload: PlanningPromoStockingGeneratePayload
): Promise<PlanningPromoStockingCampaign> => {
  if (useMocks) {
    const store = getMockStore(tenantId);
    const record: PlanningPromoStockingCampaign = {
      campaign_id: nextMockId('PROMO'),
      campaign_name: payload.campaign_name,
      period_start: payload.period_start,
      period_end: payload.period_end,
      locked_budget_cent: payload.locked_budget_cent ?? 0,
      status: 'READY',
      created_at: new Date().toISOString()
    };
    store.promoCampaigns.unshift(record);
    return record;
  }
  return requestJson<PlanningPromoStockingCampaign>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/planning/promo-stocking/generate`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    'Failed to generate planning promo stocking campaign'
  );
};

export const listPlanningPlanExecutions = async (
  tenantId: string
): Promise<PlanningListResponse<PlanningPlanExecution>> => {
  if (useMocks) {
    const store = getMockStore(tenantId);
    const rows: PlanningPlanExecution[] = [
      ...store.purchasePlans.map((plan) => ({
        execution_id: `EX-${plan.plan_id}`,
        plan_type: 'PURCHASE_PLAN',
        related_plan: plan.plan_id,
        owner: 'PROCUREMENT_TEAM',
        progress: plan.status === 'DONE' ? 100 : 60,
        status: plan.status === 'DONE' ? 'COMPLETED' : 'IN_PROGRESS',
        variance: '-5%'
      })),
      ...store.transferSuggestions.map((item) => ({
        execution_id: `EX-${item.suggestion_id}`,
        plan_type: 'TRANSFER_SUGGESTION',
        related_plan: item.suggestion_id,
        owner: 'WMS_TEAM',
        progress: item.status === 'DONE' ? 100 : 50,
        status: item.status === 'DONE' ? 'COMPLETED' : 'IN_PROGRESS',
        variance: '-2%'
      })),
      ...store.promoCampaigns.map((item) => ({
        execution_id: `EX-${item.campaign_id}`,
        plan_type: 'PROMO_STOCKING',
        related_plan: item.campaign_id,
        owner: 'PLANNING_TEAM',
        progress: item.status === 'DONE' ? 100 : 40,
        status: item.status === 'DONE' ? 'COMPLETED' : 'WARNING',
        variance: '-8%'
      }))
    ];
    return listResponse(rows);
  }
  return requestJson<PlanningListResponse<PlanningPlanExecution>>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/planning/plan-executions`,
    { headers: buildAuthHeaders() },
    'Failed to list planning plan executions'
  );
};
