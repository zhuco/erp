import dayjs from 'dayjs';

import type {
  LogisticsWarehouseStrategy,
  LogisticsWarehouseStrategyCreatePayload,
  LogisticsWarehouseStrategyHistory,
  LogisticsWarehouseStrategyHistoryListResponse,
  LogisticsWarehouseStrategyListQuery,
  LogisticsWarehouseStrategyListResponse,
  LogisticsWarehouseStrategyUpdatePayload,
  LogisticsWarehouseSuggestion,
  LogisticsWarehouseSuggestionListQuery,
  LogisticsWarehouseSuggestionListResponse
} from '../logisticsOptimization';
import { applyPagination } from './data';

const mockStrategies: LogisticsWarehouseStrategy[] = [
  {
    strategy_id: 'STR-SPEED',
    tenant_id: 'TENANT-DEFAULT',
    name: '时效优先策略',
    priority: 'SPEED',
    enabled: true,
    updated_by: '物流主管',
    created_at: dayjs().subtract(10, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  },
  {
    strategy_id: 'STR-COST',
    tenant_id: 'TENANT-DEFAULT',
    name: '成本优先策略',
    priority: 'COST',
    enabled: true,
    updated_by: '物流主管',
    created_at: dayjs().subtract(9, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'day').toISOString()
  }
];

const mockHistory: LogisticsWarehouseStrategyHistory[] = [
  {
    record_id: 'HIS-1',
    tenant_id: 'TENANT-DEFAULT',
    strategy_id: 'STR-SPEED',
    summary: '启用时效优先策略',
    updated_by: '系统管理员',
    updated_at: dayjs().subtract(5, 'day').toISOString()
  }
];

const mockSuggestions: LogisticsWarehouseSuggestion[] = [
  {
    suggestion_id: 'SUG-1001',
    tenant_id: 'TENANT-DEFAULT',
    order_id: 'ORD-100230',
    destination: '上海市浦东新区',
    recommended_warehouse: '华东一号仓',
    eta_hours: 16,
    cost_cent: 1200,
    savings_cent: 240,
    strategy_id: 'STR-SPEED',
    applied: false,
    applied_at: null,
    created_at: dayjs().subtract(6, 'hour').toISOString()
  },
  {
    suggestion_id: 'SUG-1002',
    tenant_id: 'TENANT-DEFAULT',
    order_id: 'ORD-100231',
    destination: '广东省深圳市',
    recommended_warehouse: '华南中心仓',
    eta_hours: 20,
    cost_cent: 980,
    savings_cent: 180,
    strategy_id: 'STR-COST',
    applied: true,
    applied_at: dayjs().subtract(4, 'hour').toISOString(),
    created_at: dayjs().subtract(8, 'hour').toISOString()
  }
];

export const listMockLogisticsWarehouseStrategies = (
  query: LogisticsWarehouseStrategyListQuery
): LogisticsWarehouseStrategyListResponse => {
  const filtered = query.enabled == null
    ? mockStrategies
    : mockStrategies.filter((item) => item.enabled === query.enabled);
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length
  };
};

export const createMockLogisticsWarehouseStrategy = (
  payload: LogisticsWarehouseStrategyCreatePayload
): LogisticsWarehouseStrategy => {
  const now = dayjs().toISOString();
  const record: LogisticsWarehouseStrategy = {
    strategy_id: `STR-${Date.now()}`,
    tenant_id: 'TENANT-DEFAULT',
    name: payload.name,
    priority: payload.priority ?? 'SPEED',
    enabled: payload.enabled ?? true,
    updated_by: payload.updated_by ?? '物流主管',
    created_at: now,
    updated_at: now
  };
  mockStrategies.unshift(record);
  mockHistory.unshift({
    record_id: `HIS-${Date.now()}`,
    tenant_id: 'TENANT-DEFAULT',
    strategy_id: record.strategy_id,
    summary: payload.history_summary ?? `新增策略：${record.name}`,
    updated_by: record.updated_by ?? null,
    updated_at: now
  });
  return record;
};

export const updateMockLogisticsWarehouseStrategy = (
  strategyId: string,
  payload: LogisticsWarehouseStrategyUpdatePayload
): LogisticsWarehouseStrategy => {
  const target = mockStrategies.find((item) => item.strategy_id === strategyId);
  const now = dayjs().toISOString();
  if (!target) {
    const fallback: LogisticsWarehouseStrategy = {
      strategy_id: strategyId,
      tenant_id: 'TENANT-DEFAULT',
      name: payload.name ?? '新策略',
      priority: payload.priority ?? 'SPEED',
      enabled: payload.enabled ?? true,
      updated_by: payload.updated_by ?? '物流主管',
      created_at: now,
      updated_at: now
    };
    mockStrategies.unshift(fallback);
    return fallback;
  }
  Object.assign(target, {
    name: payload.name ?? target.name,
    priority: payload.priority ?? target.priority,
    enabled: payload.enabled ?? target.enabled,
    updated_by: payload.updated_by ?? target.updated_by,
    updated_at: now
  });
  mockHistory.unshift({
    record_id: `HIS-${Date.now()}`,
    tenant_id: 'TENANT-DEFAULT',
    strategy_id: target.strategy_id,
    summary: payload.history_summary ?? '更新仓配策略',
    updated_by: target.updated_by ?? null,
    updated_at: now
  });
  return target;
};

export const listMockLogisticsWarehouseStrategyHistory = (
  strategyId: string
): LogisticsWarehouseStrategyHistoryListResponse => ({
  items: mockHistory.filter((item) => item.strategy_id === strategyId).map((item) => ({ ...item }))
});

export const listMockLogisticsWarehouseSuggestions = (
  query: LogisticsWarehouseSuggestionListQuery
): LogisticsWarehouseSuggestionListResponse => {
  const filtered = query.applied == null
    ? mockSuggestions
    : mockSuggestions.filter((item) => item.applied === query.applied);
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length
  };
};

export const applyMockLogisticsWarehouseSuggestion = (
  suggestionId: string
): LogisticsWarehouseSuggestion => {
  const target = mockSuggestions.find((item) => item.suggestion_id === suggestionId);
  const now = dayjs().toISOString();
  if (!target) {
    const fallback: LogisticsWarehouseSuggestion = {
      suggestion_id: suggestionId,
      tenant_id: 'TENANT-DEFAULT',
      order_id: 'ORD-UNKNOWN',
      destination: '-',
      recommended_warehouse: '-',
      eta_hours: 0,
      cost_cent: 0,
      savings_cent: 0,
      strategy_id: null,
      applied: true,
      applied_at: now,
      created_at: now
    };
    mockSuggestions.unshift(fallback);
    return fallback;
  }
  target.applied = true;
  target.applied_at = now;
  return target;
};
