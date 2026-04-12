import dayjs from 'dayjs';

import type { StatsBreakdownItem, StatsQuery, StatsReport, StatsType } from '../stats';

const defaultMetrics: Record<StatsType, string[]> = {
  sales: ['sales_amount_cent', 'order_count', 'avg_order_value_cent', 'refund_rate'],
  shipping: ['shipment_count', 'on_time_rate', 'exception_count', 'avg_outbound_hours'],
  'sales-daily': ['daily_sales_cent', 'daily_order_count', 'daily_refund_rate', 'daily_on_time_rate'],
  'sales-trends': [
    'yoy_growth_rate',
    'mom_growth_rate',
    'moving_avg_7d_cent',
    'season_peak_cent',
    'channel_trend_index'
  ],
  'slow-moving': ['slow_sku_count', 'slow_stock_amount_cent', 'avg_turnover_days', 'dead_stock_ratio'],
  'best-sellers': ['top_sku_sales_cent', 'top_sku_order_count', 'top_sku_share', 'repeat_purchase_rate'],
  'sales-cycle': [
    'avg_order_cycle_hours',
    'avg_ship_cycle_hours',
    'avg_return_cycle_days',
    'on_time_delivery_rate'
  ]
};

const dimensionSeeds: Record<string, string[]> = {
  channel: ['天猫', '京东', '抖音', '拼多多', '私域'],
  warehouse: ['华东仓', '华南仓', '华北仓', '海外仓'],
  category: ['服饰', '美妆', '食品', '家电', '数码']
};

const buildSeed = (value: string) =>
  value.split('').reduce((sum, char) => sum + char.charCodeAt(0), 0);

const resolveRangeDays = (startAt: string, endAt: string): number => {
  const start = dayjs(startAt);
  const end = dayjs(endAt);
  if (!start.isValid() || !end.isValid()) {
    return 7;
  }
  const diff = end.diff(start, 'day');
  return Math.max(1, diff + 1);
};

const normalizeMetricValue = (key: string, value: number) => {
  if (key.includes('rate') || key.includes('ratio')) {
    return Number(value.toFixed(2));
  }
  if (key.includes('hours')) {
    return Number(value.toFixed(2));
  }
  if (key.includes('days')) {
    return Number(value.toFixed(1));
  }
  return Math.round(value);
};

const buildMetricValue = (key: string, seed: number, rangeDays: number) => {
  const base = seed + rangeDays * 19;
  if (key.includes('rate') || key.includes('ratio')) {
    return normalizeMetricValue(key, (base % 8500) / 100);
  }
  if (key.includes('cent') || key.includes('amount')) {
    return normalizeMetricValue(key, (base % 280000) + rangeDays * 4800);
  }
  if (key.includes('hours')) {
    return normalizeMetricValue(key, (base % 3600) / 100);
  }
  if (key.includes('days')) {
    return normalizeMetricValue(key, (base % 900) / 10);
  }
  return normalizeMetricValue(key, (base % 5200) + rangeDays * 28);
};

const buildMetricsRecord = (keys: string[], seed: number, rangeDays: number) => {
  const record: Record<string, number> = {};
  keys.forEach((key, index) => {
    record[key] = buildMetricValue(key, seed + index * 37, rangeDays);
  });
  return record;
};

const buildCompareRecord = (current: Record<string, number>, seed: number) => {
  const compare: Record<string, number> = {};
  Object.keys(current).forEach((key, index) => {
    const value = current[key];
    const sign = (seed + index) % 2 === 0 ? 1 : -1;
    const ratio = 0.05 + ((seed + index) % 15) / 100;
    const compareValue = Math.max(0, value * (1 - sign * ratio));
    compare[key] = normalizeMetricValue(key, compareValue);
  });
  return compare;
};

const buildBreakdown = (
  statsType: StatsType,
  dimension: string,
  keys: string[],
  seed: number,
  rangeDays: number
): StatsBreakdownItem[] => {
  const options = dimensionSeeds[dimension] ?? [];
  return options.map((name, index) => {
    const metricSeed = seed + buildSeed(name) + index * 13;
    return {
      dimension: name,
      dimension_label: name,
      metrics: buildMetricsRecord(keys, metricSeed, rangeDays)
    };
  });
};

export const getMockStatsReport = (statsType: StatsType, query: StatsQuery): StatsReport => {
  const metricKeys =
    query.metrics && query.metrics.length > 0
      ? query.metrics
      : defaultMetrics[statsType];
  const rangeDays = resolveRangeDays(query.start_at, query.end_at);
  const seed = buildSeed(
    `${statsType}-${query.start_at}-${query.end_at}-${metricKeys.join('-')}`
  );
  const current = buildMetricsRecord(metricKeys, seed, rangeDays);
  const compare =
    query.compare_start_at && query.compare_end_at
      ? buildCompareRecord(current, seed)
      : undefined;
  const breakdown = query.dimension
    ? buildBreakdown(statsType, query.dimension, metricKeys, seed, rangeDays)
    : undefined;

  return {
    stats_type: statsType,
    current,
    compare,
    breakdown,
    updated_at: dayjs().toISOString()
  };
};
