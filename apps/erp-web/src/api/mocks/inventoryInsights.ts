import dayjs from 'dayjs';

import type {
  InventoryAbcItem,
  InventoryAlertLevel,
  InventoryAlertListQuery,
  InventoryAlertListResponse,
  InventoryAlertRecord,
  InventoryAlertRule,
  InventoryAlertRuleListQuery,
  InventoryAlertRuleListResponse,
  InventoryAlertRulePayload,
  InventoryAlertSummary,
  InventoryAlertType,
  InventoryAmountAnalysis,
  InventoryAmountRanking,
  InventoryInsightQuery,
  InventorySlowMovingItem,
  InventoryTrendPoint,
  InventoryTurnoverAnalysis,
  InventoryTurnoverBucket
} from '../inventoryInsights';

type WarehouseOption = {
  id: string;
  name: string;
};

const warehouses: WarehouseOption[] = [
  { id: 'WH-DEFAULT', name: '默认仓' },
  { id: 'WH-EAST', name: '华东仓' },
  { id: 'WH-SOUTH', name: '华南仓' },
  { id: 'WH-NORTH', name: '华北仓' }
];

const brands = ['得力', '晨光', '良品铺子', '三只松鼠', '南极人', '洁柔'];
const categories = ['办公文具', '休闲零食', '家居日用', '服饰配件', '美妆护理'];
const alertTypes: InventoryAlertType[] = [
  'OUT_OF_STOCK',
  'LOW_STOCK',
  'HIGH_STOCK',
  'NO_SALES'
];
const alertLevels: InventoryAlertLevel[] = ['HIGH', 'MEDIUM', 'LOW'];

const suggestionMap: Record<InventoryAlertType, string> = {
  OUT_OF_STOCK: '紧急补货/调拨',
  LOW_STOCK: '建议补货',
  HIGH_STOCK: '安排促销/调拨',
  NO_SALES: '清仓或组合促销'
};

const buildAlertRecord = (index: number): InventoryAlertRecord => {
  const warehouse = warehouses[index % warehouses.length];
  const alertType = alertTypes[index % alertTypes.length];
  const alertLevel = alertLevels[index % alertLevels.length];
  const baseQty = 5 + (index % 18) * 3;
  const availableQty = alertType === 'OUT_OF_STOCK' ? 0 : baseQty;
  const turnoverDays = 25 + (index % 8) * 15;
  const stockAgeDays = 40 + (index % 9) * 20;
  const sales30d = alertType === 'NO_SALES' ? 0 : 20 + (index % 10) * 3;
  const price = 18 + (index % 5) * 7;
  return {
    alert_id: `ALERT-${1000 + index}`,
    sku_id: `SKU-${2000 + index}`,
    sku_code: `SKU-${2000 + index}`,
    sku_name: `示例货品-${index + 1}`,
    spec: `${(index % 4) + 1}件/箱`,
    warehouse_id: warehouse.id,
    warehouse_name: warehouse.name,
    brand: brands[index % brands.length] ?? '品牌',
    category: categories[index % categories.length] ?? '分类',
    available_qty: availableQty,
    available_days: alertType === 'HIGH_STOCK' ? turnoverDays + 30 : turnoverDays,
    sales_30d: sales30d,
    alert_type: alertType,
    alert_level: alertLevel,
    suggestion: suggestionMap[alertType],
    inventory_amount: availableQty * price,
    turnover_days: turnoverDays,
    stock_age_days: stockAgeDays
  };
};

const mockAlerts: InventoryAlertRecord[] = Array.from({ length: 48 }, (_, index) =>
  buildAlertRecord(index)
);

let mockRules: InventoryAlertRule[] = [
  {
    rule_id: 'RULE-1',
    name: '默认缺货规则',
    warehouse_id: null,
    category: null,
    brand: null,
    sku_keyword: null,
    available_qty_threshold: 3,
    available_days_threshold: 5,
    alert_level: 'HIGH',
    enabled: true,
    created_at: dayjs().subtract(12, 'day').toISOString(),
    updated_at: dayjs().subtract(3, 'day').toISOString()
  },
  {
    rule_id: 'RULE-2',
    name: '华东仓低库存预警',
    warehouse_id: 'WH-EAST',
    category: '办公文具',
    brand: '得力',
    sku_keyword: null,
    available_qty_threshold: 10,
    available_days_threshold: 12,
    alert_level: 'MEDIUM',
    enabled: true,
    created_at: dayjs().subtract(8, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  }
];

const applyPagination = <T,>(items: T[], offset = 0, limit = 20): T[] => {
  const start = Math.max(0, offset);
  return items.slice(start, start + limit);
};

const normalizeQueryText = (value?: string | null) => value?.trim().toLowerCase() ?? '';

const filterAlerts = (
  query: InventoryInsightQuery,
  includeAlertType = true
): InventoryAlertRecord[] => {
  const keyword = normalizeQueryText(query.keyword);
  return mockAlerts.filter((item) => {
    if (query.warehouse_id && item.warehouse_id !== query.warehouse_id) {
      return false;
    }
    if (query.brand && item.brand !== query.brand) {
      return false;
    }
    if (query.category && item.category !== query.category) {
      return false;
    }
    if (includeAlertType && query.alert_type && item.alert_type !== query.alert_type) {
      return false;
    }
    if (query.alert_level && query.alert_level.length > 0) {
      if (!query.alert_level.includes(item.alert_level)) {
        return false;
      }
    }
    if (keyword) {
      const skuCode = item.sku_code?.toLowerCase() ?? '';
      const skuName = item.sku_name?.toLowerCase() ?? '';
      if (!skuCode.includes(keyword) && !skuName.includes(keyword)) {
        return false;
      }
    }
    return true;
  });
};

const summarizeAlerts = (items: InventoryAlertRecord[]): InventoryAlertSummary => ({
  out_of_stock: items.filter((item) => item.alert_type === 'OUT_OF_STOCK').length,
  low_stock: items.filter((item) => item.alert_type === 'LOW_STOCK').length,
  high_stock: items.filter((item) => item.alert_type === 'HIGH_STOCK').length,
  no_sales: items.filter((item) => item.alert_type === 'NO_SALES').length,
  updated_at: dayjs().toISOString()
});

const buildRankings = (
  items: InventoryAlertRecord[],
  resolver: (item: InventoryAlertRecord) => { id: string; name: string }
): InventoryAmountRanking[] => {
  const map = new Map<string, InventoryAmountRanking>();
  items.forEach((item) => {
    const resolved = resolver(item);
    const existing = map.get(resolved.id);
    const amount = item.inventory_amount ?? 0;
    const qty = item.available_qty ?? 0;
    if (existing) {
      existing.inventory_amount += amount;
      existing.inventory_qty += qty;
    } else {
      map.set(resolved.id, {
        dimension_id: resolved.id,
        dimension_name: resolved.name,
        inventory_qty: qty,
        inventory_amount: amount,
        share: 0
      });
    }
  });
  const rows = Array.from(map.values()).sort(
    (a, b) => b.inventory_amount - a.inventory_amount
  );
  const total = rows.reduce((sum, row) => sum + row.inventory_amount, 0) || 1;
  rows.forEach((row) => {
    row.share = Number((row.inventory_amount / total).toFixed(4));
  });
  return rows;
};

const buildTrend = (base: number): InventoryTrendPoint[] => {
  const points: InventoryTrendPoint[] = [];
  for (let index = 13; index >= 0; index -= 1) {
    const date = dayjs().subtract(index, 'day');
    const factor = 0.92 + index * 0.004;
    points.push({
      date: date.format('MM-DD'),
      inventory_qty: Math.round((base / 50) * factor),
      inventory_amount: Math.round(base * factor)
    });
  }
  return points;
};

const buildMonthlyTrend = (base: number): { period: string; inventory_amount: number }[] => {
  const points: { period: string; inventory_amount: number }[] = [];
  for (let index = 5; index >= 0; index -= 1) {
    const period = dayjs().subtract(index, 'month').format('YYYY-MM');
    const factor = 0.86 + index * 0.03;
    points.push({
      period,
      inventory_amount: Math.round(base * factor)
    });
  }
  return points;
};

const buildBuckets = (
  items: InventoryAlertRecord[],
  ranges: Array<{ label: string; min: number; max?: number }>,
  resolver: (item: InventoryAlertRecord) => number
): InventoryTurnoverBucket[] => {
  const totalAmount = items.reduce((sum, item) => sum + (item.inventory_amount ?? 0), 0) || 1;
  return ranges.map((range) => {
    const filtered = items.filter((item) => {
      const value = resolver(item);
      if (range.max == null) {
        return value >= range.min;
      }
      return value >= range.min && value <= range.max;
    });
    const amount = filtered.reduce((sum, item) => sum + (item.inventory_amount ?? 0), 0);
    return {
      label: range.label,
      sku_count: filtered.length,
      inventory_amount: amount,
      share: Number((amount / totalAmount).toFixed(4))
    };
  });
};

const buildAbc = (items: InventoryAlertRecord[]): InventoryAbcItem[] => {
  const sorted = [...items].sort(
    (a, b) => (b.inventory_amount ?? 0) - (a.inventory_amount ?? 0)
  );
  const total = sorted.reduce((sum, item) => sum + (item.inventory_amount ?? 0), 0) || 1;
  let cumulative = 0;
  const buckets: Record<'A' | 'B' | 'C', { count: number; amount: number }> = {
    A: { count: 0, amount: 0 },
    B: { count: 0, amount: 0 },
    C: { count: 0, amount: 0 }
  };
  sorted.forEach((item) => {
    const amount = item.inventory_amount ?? 0;
    cumulative += amount / total;
    const level = cumulative <= 0.7 ? 'A' : cumulative <= 0.9 ? 'B' : 'C';
    buckets[level].count += 1;
    buckets[level].amount += amount;
  });
  return (['A', 'B', 'C'] as const).map((level) => ({
    level,
    sku_count: buckets[level].count,
    inventory_amount: Math.round(buckets[level].amount),
    share: Number((buckets[level].amount / total).toFixed(4)),
    metric: '库存占用金额'
  }));
};

export const getMockInventoryAlertSummary = (
  query: InventoryInsightQuery
): InventoryAlertSummary => summarizeAlerts(filterAlerts(query, false));

export const listMockInventoryAlerts = (
  query: InventoryAlertListQuery
): InventoryAlertListResponse => {
  const filtered = filterAlerts(query, true);
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length
  };
};

export const getMockInventoryAmountAnalysis = (
  query: InventoryInsightQuery
): InventoryAmountAnalysis => {
  const filtered = filterAlerts(query, false);
  const warehouseRanking = buildRankings(filtered, (item) => ({
    id: item.warehouse_id ?? 'unknown',
    name: item.warehouse_name ?? item.warehouse_id ?? '未知仓库'
  }));
  const brandShare = buildRankings(filtered, (item) => ({
    id: item.brand ?? 'unknown',
    name: item.brand ?? '未知品牌'
  }));
  const categoryShare = buildRankings(filtered, (item) => ({
    id: item.category ?? 'unknown',
    name: item.category ?? '未知分类'
  }));
  const skuShare = buildRankings(filtered, (item) => ({
    id: item.sku_id,
    name: item.sku_name ?? item.sku_code ?? item.sku_id
  }));
  const totalAmount = filtered.reduce((sum, item) => sum + (item.inventory_amount ?? 0), 0);
  return {
    warehouse_ranking: warehouseRanking,
    brand_share: brandShare,
    category_share: categoryShare,
    sku_share: skuShare.slice(0, 10),
    trend: buildMonthlyTrend(totalAmount)
  };
};

export const getMockInventoryTurnoverAnalysis = (
  query: InventoryInsightQuery
): InventoryTurnoverAnalysis => {
  const filtered = filterAlerts(query, false);
  const turnoverRanges = [
    { label: '0-60天', min: 0, max: 60 },
    { label: '61-90天', min: 61, max: 90 },
    { label: '91-180天', min: 91, max: 180 },
    { label: '>180天', min: 181 }
  ];
  const ageRanges = [
    { label: '0-30天', min: 0, max: 30 },
    { label: '31-60天', min: 31, max: 60 },
    { label: '61-120天', min: 61, max: 120 },
    { label: '>120天', min: 121 }
  ];
  const turnoverBuckets = buildBuckets(filtered, turnoverRanges, (item) => item.turnover_days ?? 0);
  const ageBuckets = buildBuckets(filtered, ageRanges, (item) => item.stock_age_days ?? 0);
  const slowMovingMap = new Map<string, { total: number; count: number }>();
  filtered.forEach((item) => {
    const brand = item.brand ?? '未知品牌';
    const entry = slowMovingMap.get(brand) ?? { total: 0, count: 0 };
    entry.total += item.turnover_days ?? 0;
    entry.count += 1;
    slowMovingMap.set(brand, entry);
  });
  const slowMoving: InventorySlowMovingItem[] = Array.from(slowMovingMap.entries())
    .map(([name, value]) => ({
      dimension_name: name,
      avg_turnover_days: Math.round(value.total / Math.max(value.count, 1)),
      sku_count: value.count
    }))
    .sort((a, b) => b.avg_turnover_days - a.avg_turnover_days)
    .slice(0, 6);
  const totalAmount = filtered.reduce((sum, item) => sum + (item.inventory_amount ?? 0), 0);
  const history = buildTrend(totalAmount);
  const deadStock = filtered
    .filter((item) => item.alert_type === 'NO_SALES')
    .slice(0, 12)
    .map((item) => ({
      sku_id: item.sku_id,
      sku_code: item.sku_code ?? item.sku_id,
      sku_name: item.sku_name ?? item.sku_code ?? item.sku_id,
      warehouse_name: item.warehouse_name ?? item.warehouse_id ?? '-',
      dead_days: item.stock_age_days ?? 0,
      inventory_qty: item.available_qty,
      inventory_amount: item.inventory_amount ?? 0
    }));
  return {
    turnover_buckets: turnoverBuckets,
    slow_moving: slowMoving,
    history,
    age_buckets: ageBuckets,
    dead_stock: deadStock,
    abc_classes: buildAbc(filtered)
  };
};

export const listMockInventoryAlertRules = (
  query: InventoryAlertRuleListQuery
): InventoryAlertRuleListResponse => ({
  items: applyPagination(mockRules, query.offset, query.limit),
  total: mockRules.length
});

export const createMockInventoryAlertRule = (
  payload: InventoryAlertRulePayload
): InventoryAlertRule => {
  const now = dayjs().toISOString();
  const rule: InventoryAlertRule = {
    rule_id: `RULE-${Date.now()}`,
    ...payload,
    created_at: now,
    updated_at: now
  };
  mockRules = [rule, ...mockRules];
  return rule;
};

export const updateMockInventoryAlertRule = (
  ruleId: string,
  payload: InventoryAlertRulePayload
): InventoryAlertRule => {
  const now = dayjs().toISOString();
  const index = mockRules.findIndex((item) => item.rule_id === ruleId);
  if (index < 0) {
    const rule: InventoryAlertRule = {
      rule_id: ruleId,
      ...payload,
      created_at: now,
      updated_at: now
    };
    mockRules = [rule, ...mockRules];
    return rule;
  }
  const updated = {
    ...mockRules[index],
    ...payload,
    updated_at: now
  };
  mockRules = [
    updated,
    ...mockRules.filter((item) => item.rule_id !== ruleId)
  ];
  return updated;
};

export const deleteMockInventoryAlertRule = (ruleId: string) => {
  mockRules = mockRules.filter((rule) => rule.rule_id !== ruleId);
};
