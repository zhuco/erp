import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import { getMockStatsReport } from '@/api/mocks';

export type StatsType =
  | 'sales'
  | 'shipping'
  | 'sales-daily'
  | 'sales-trends'
  | 'slow-moving'
  | 'best-sellers'
  | 'sales-cycle';

export type StatsQuery = {
  start_at: string;
  end_at: string;
  compare_start_at?: string;
  compare_end_at?: string;
  dimension?: string;
  metrics?: string[];
};

export type StatsBreakdownItem = {
  dimension: string;
  dimension_label?: string | null;
  metrics: Record<string, number>;
};

export type StatsReport = {
  stats_type: StatsType;
  current: Record<string, number>;
  compare?: Record<string, number>;
  breakdown?: StatsBreakdownItem[];
  updated_at: string;
};

export type StatsInfraHealth = {
  analytics_backend: 'POSTGRES' | 'CLICKHOUSE' | string;
  clickhouse_ready: boolean;
  clickhouse_endpoint?: string | null;
  sso_ready: boolean;
  sso_issuer?: string | null;
  checked_at: string;
};

const buildQueryString = (query: StatsQuery): string => {
  const params = new URLSearchParams();
  params.set('start_at', query.start_at);
  params.set('end_at', query.end_at);
  if (query.compare_start_at) {
    params.set('compare_start_at', query.compare_start_at);
  }
  if (query.compare_end_at) {
    params.set('compare_end_at', query.compare_end_at);
  }
  if (query.dimension) {
    params.set('dimension', query.dimension);
  }
  if (query.metrics && query.metrics.length > 0) {
    params.set('metrics', query.metrics.join(','));
  }
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

export const getStatsReport = async (
  tenantId: string,
  statsType: StatsType,
  query: StatsQuery
): Promise<StatsReport> => {
  if (useMocks) {
    return getMockStatsReport(statsType, query);
  }
  return requestJson<StatsReport>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/stats/${statsType}${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取统计数据失败'
  );
};

export const getStatsInfraHealth = async (tenantId: string): Promise<StatsInfraHealth> =>
  useMocks
    ? {
        analytics_backend: 'POSTGRES',
        clickhouse_ready: false,
        clickhouse_endpoint: null,
        sso_ready: false,
        sso_issuer: null,
        checked_at: new Date().toISOString()
      }
    : requestJson<StatsInfraHealth>(
        `${apiBaseUrl}/v1/tenants/${tenantId}/stats/health`,
        { headers: buildAuthHeaders() },
        '获取统计基础设施状态失败'
      );

export const getSalesStats = (tenantId: string, query: StatsQuery) =>
  getStatsReport(tenantId, 'sales', query);

export const getShippingStats = (tenantId: string, query: StatsQuery) =>
  getStatsReport(tenantId, 'shipping', query);

export const getSalesDailyStats = (tenantId: string, query: StatsQuery) =>
  getStatsReport(tenantId, 'sales-daily', query);

export const getSalesTrendStats = (tenantId: string, query: StatsQuery) =>
  getStatsReport(tenantId, 'sales-trends', query);

export const getSlowMovingStats = (tenantId: string, query: StatsQuery) =>
  getStatsReport(tenantId, 'slow-moving', query);

export const getBestSellerStats = (tenantId: string, query: StatsQuery) =>
  getStatsReport(tenantId, 'best-sellers', query);

export const getSalesCycleStats = (tenantId: string, query: StatsQuery) =>
  getStatsReport(tenantId, 'sales-cycle', query);
