export type StatsMetricDefinition = {
  key: string;
  label: string;
  unit?: string;
  precision?: number;
  format?: (value: number) => string;
};

export type StatsMetricSnapshot = Record<string, number | null | undefined>;

export type StatsDiffItem = {
  key: string;
  label: string;
  metric: StatsMetricDefinition;
  current: number | null;
  compare: number | null;
  diff: number | null;
  diffRate: number | null;
};

const normalizeNumber = (value?: number | null): number | null => {
  if (value == null || Number.isNaN(value)) {
    return null;
  }
  return value;
};

export const buildStatsDiff = (
  metrics: StatsMetricDefinition[],
  current: StatsMetricSnapshot,
  compare?: StatsMetricSnapshot
): StatsDiffItem[] =>
  metrics.map((metric) => {
    const currentValue = normalizeNumber(current[metric.key]);
    const compareValue = normalizeNumber(compare?.[metric.key]);
    const diffValue =
      currentValue != null && compareValue != null ? currentValue - compareValue : null;
    const diffRate =
      diffValue != null && compareValue != null && compareValue !== 0
        ? diffValue / compareValue
        : null;
    return {
      key: metric.key,
      label: metric.label,
      metric,
      current: currentValue,
      compare: compareValue,
      diff: diffValue,
      diffRate
    };
  });

const formatNumber = (value: number, precision: number) =>
  value.toLocaleString('zh-CN', {
    minimumFractionDigits: precision,
    maximumFractionDigits: precision
  });

export const formatStatsValue = (
  value?: number | null,
  metric?: StatsMetricDefinition
): string => {
  if (value == null || Number.isNaN(value)) {
    return '-';
  }
  if (metric?.format) {
    return metric.format(value);
  }
  const precision = metric?.precision ?? (metric?.unit === '%' ? 2 : 0);
  const formatted = formatNumber(value, precision);
  return metric?.unit ? `${formatted}${metric.unit}` : formatted;
};

export const formatDiffValue = (
  value?: number | null,
  metric?: StatsMetricDefinition
): string => {
  if (value == null || Number.isNaN(value)) {
    return '-';
  }
  const sign = value > 0 ? '+' : value < 0 ? '-' : '';
  return `${sign}${formatStatsValue(Math.abs(value), metric)}`;
};

export const formatDiffRate = (rate?: number | null): string => {
  if (rate == null || Number.isNaN(rate)) {
    return '-';
  }
  const sign = rate > 0 ? '+' : rate < 0 ? '-' : '';
  return `${sign}${Math.abs(rate * 100).toFixed(2)}%`;
};
