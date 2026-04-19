import { apiBaseUrl, useMocks } from '@/utils/env';
import { fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type FrontendRouteDiagnosticEventType =
  | 'loading-ready'
  | 'loading-timeout'
  | 'loading-error'
  | 'prefetch-ready'
  | 'prefetch-error'
  | 'prefetch-skipped';

export type FrontendRouteDiagnosticStatus = 'ready' | 'timeout' | 'error' | 'loading' | 'idle';

export type FrontendRouteDiagnosticReport = {
  session_id: string;
  path: string;
  route_label?: string;
  event_type: FrontendRouteDiagnosticEventType;
  status: FrontendRouteDiagnosticStatus;
  duration_ms?: number;
  prefetch_status?: 'prefetching' | 'ready' | 'error' | 'skipped';
  prefetch_source?: 'hover' | 'open' | 'search' | 'recommendation' | 'programmatic';
  prefetch_duration_ms?: number;
  error_summary?: string;
  detail?: string;
  event_at: number;
  created_at?: string;
  page_url: string;
  build_version: string;
  user_agent: string;
};

export type FrontendRouteDiagnosticsQuery = {
  pathKeyword?: string;
  status?: FrontendRouteDiagnosticStatus | 'all';
  eventType?: FrontendRouteDiagnosticEventType | 'all';
  windowHours?: number;
  limit?: number;
};

export type FrontendRouteDiagnosticsSummary = {
  totalEvents: number;
  uniqueRoutes: number;
  readyEvents: number;
  timeoutEvents: number;
  errorEvents: number;
  slowEvents: number;
  prefetchHits: number;
  affectedSessions: number;
  measuredFirstLoads: number;
  successfulFirstLoads: number;
  firstLoadAvgDurationMs?: number;
  prefetchShortCircuitCount: number;
};

export type FrontendRouteDiagnosticsPathSummary = {
  path: string;
  routeLabel?: string;
  totalEvents: number;
  readyEvents: number;
  timeoutEvents: number;
  errorEvents: number;
  slowEvents: number;
  prefetchHits: number;
  avgDurationMs?: number;
  maxDurationMs?: number;
  measuredFirstLoads: number;
  successfulFirstLoads: number;
  firstLoadAvgDurationMs?: number;
  prefetchShortCircuitCount: number;
  lastSeenAt: string;
};

export type FrontendRouteDiagnosticsResponse = {
  summary: FrontendRouteDiagnosticsSummary;
  paths: FrontendRouteDiagnosticsPathSummary[];
  events: FrontendRouteDiagnosticReport[];
};

const REQUEST_TIMEOUT_MS = 15000;
const MOCK_STORAGE_PREFIX = 'erp.mock.frontend-route-diagnostics';
const MOCK_RECORD_LIMIT = 400;
const ROUTE_SLOW_THRESHOLD_MS = 1200;
const terminalLoadingEventTypes = new Set<FrontendRouteDiagnosticEventType>([
  'loading-ready',
  'loading-timeout',
  'loading-error',
]);
const prefetchShortCircuitDetails = new Set(['already-ready', 'inflight']);

type FirstLoadPathMetrics = {
  measuredFirstLoads: number;
  successfulFirstLoads: number;
  firstLoadDurationTotalMs: number;
  firstLoadDurationSamples: number;
};

type FirstLoadMetrics = {
  measuredFirstLoads: number;
  successfulFirstLoads: number;
  firstLoadAvgDurationMs?: number;
  byPath: Map<string, FirstLoadPathMetrics>;
};

type PathAccumulator = FrontendRouteDiagnosticsPathSummary & {
  durationSumMs: number;
  durationSampleCount: number;
};

const buildQueryString = (query: FrontendRouteDiagnosticsQuery) => {
  const params = new URLSearchParams();
  if (query.pathKeyword?.trim()) {
    params.set('path_keyword', query.pathKeyword.trim());
  }
  if (query.status && query.status !== 'all') {
    params.set('status', query.status);
  }
  if (query.eventType && query.eventType !== 'all') {
    params.set('event_type', query.eventType);
  }
  if (typeof query.windowHours === 'number' && Number.isFinite(query.windowHours)) {
    params.set('window_hours', String(Math.max(1, Math.round(query.windowHours))));
  }
  if (typeof query.limit === 'number' && Number.isFinite(query.limit)) {
    params.set('limit', String(Math.max(1, Math.round(query.limit))));
  }
  const serialized = params.toString();
  return serialized ? `?${serialized}` : '';
};

const requestJson = async <T>(input: RequestInfo, init: RequestInit, message: string) => {
  const controller = new AbortController();
  const timeout = window.setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
  try {
    const response = await fetchWithAuth(input, { ...init, signal: controller.signal });
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
  } catch (error) {
    if (error instanceof DOMException && error.name === 'AbortError') {
      handleApiError(408);
      throw createApiError('前端路由观测请求超时，请稍后重试', 408);
    }
    throw error;
  } finally {
    window.clearTimeout(timeout);
  }
};

const getMockStorageKey = (tenantId: string) => `${MOCK_STORAGE_PREFIX}.${tenantId}`;

const readMockReports = (tenantId: string): FrontendRouteDiagnosticReport[] => {
  if (typeof window === 'undefined') {
    return [];
  }
  try {
    const raw = window.localStorage.getItem(getMockStorageKey(tenantId));
    if (!raw) {
      return [];
    }
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      return [];
    }
    return parsed.filter(
      (item): item is FrontendRouteDiagnosticReport =>
        Boolean(item) && typeof item === 'object' && typeof item.path === 'string',
    );
  } catch {
    return [];
  }
};

const writeMockReports = (tenantId: string, reports: FrontendRouteDiagnosticReport[]) => {
  if (typeof window === 'undefined') {
    return;
  }
  window.localStorage.setItem(
    getMockStorageKey(tenantId),
    JSON.stringify(reports.slice(0, MOCK_RECORD_LIMIT)),
  );
};

const normalizeReport = (report: FrontendRouteDiagnosticReport): FrontendRouteDiagnosticReport => ({
  ...report,
  created_at: report.created_at ?? new Date().toISOString(),
});

const getReportCreatedAt = (report: FrontendRouteDiagnosticReport) =>
  report.created_at ? Date.parse(report.created_at) : report.event_at;

const matchesWindow = (report: FrontendRouteDiagnosticReport, windowHours: number) => {
  const createdAt = report.created_at ? Date.parse(report.created_at) : Number.NaN;
  if (Number.isNaN(createdAt)) {
    return true;
  }
  return createdAt >= Date.now() - windowHours * 60 * 60 * 1000;
};

const matchesQuery = (report: FrontendRouteDiagnosticReport, query: FrontendRouteDiagnosticsQuery) => {
  const pathKeyword = query.pathKeyword?.trim().toLowerCase();
  if (pathKeyword && !report.path.toLowerCase().includes(pathKeyword)) {
    return false;
  }
  if (query.status && query.status !== 'all' && report.status !== query.status) {
    return false;
  }
  if (query.eventType && query.eventType !== 'all' && report.event_type !== query.eventType) {
    return false;
  }
  const windowHours = query.windowHours ?? 24;
  return matchesWindow(report, windowHours);
};

const getSeverityRank = (item: FrontendRouteDiagnosticsPathSummary) => {
  if (item.errorEvents > 0) {
    return 0;
  }
  if (item.timeoutEvents > 0) {
    return 1;
  }
  if (item.slowEvents > 0) {
    return 2;
  }
  if (item.readyEvents > 0) {
    return 3;
  }
  return 4;
};

const buildFirstLoadMetrics = (reports: FrontendRouteDiagnosticReport[]): FirstLoadMetrics => {
  const firstTerminalBySessionPath = new Map<string, FrontendRouteDiagnosticReport>();

  [...reports]
    .sort((left, right) => getReportCreatedAt(left) - getReportCreatedAt(right))
    .forEach((report) => {
      if (!terminalLoadingEventTypes.has(report.event_type)) {
        return;
      }
      const sessionId = report.session_id.trim();
      const path = report.path.trim();
      if (!sessionId || !path) {
        return;
      }
      const key = `${sessionId}::${path}`;
      if (!firstTerminalBySessionPath.has(key)) {
        firstTerminalBySessionPath.set(key, report);
      }
    });

  const byPath = new Map<string, FirstLoadPathMetrics>();
  let measuredFirstLoads = 0;
  let successfulFirstLoads = 0;
  let successfulDurationTotalMs = 0;
  let successfulDurationSamples = 0;

  firstTerminalBySessionPath.forEach((report) => {
    measuredFirstLoads += 1;
    const pathMetrics = byPath.get(report.path) ?? {
      measuredFirstLoads: 0,
      successfulFirstLoads: 0,
      firstLoadDurationTotalMs: 0,
      firstLoadDurationSamples: 0,
    };
    pathMetrics.measuredFirstLoads += 1;

    if (report.status === 'ready') {
      successfulFirstLoads += 1;
      pathMetrics.successfulFirstLoads += 1;
      if (typeof report.duration_ms === 'number' && Number.isFinite(report.duration_ms)) {
        successfulDurationTotalMs += report.duration_ms;
        successfulDurationSamples += 1;
        pathMetrics.firstLoadDurationTotalMs += report.duration_ms;
        pathMetrics.firstLoadDurationSamples += 1;
      }
    }

    byPath.set(report.path, pathMetrics);
  });

  return {
    measuredFirstLoads,
    successfulFirstLoads,
    firstLoadAvgDurationMs: successfulDurationSamples
      ? Math.round(successfulDurationTotalMs / successfulDurationSamples)
      : undefined,
    byPath,
  };
};

const isPrefetchShortCircuit = (report: FrontendRouteDiagnosticReport) =>
  report.event_type === 'prefetch-skipped' && prefetchShortCircuitDetails.has(report.detail ?? '');

export const summarizeFrontendRouteDiagnostics = (
  reports: FrontendRouteDiagnosticReport[],
  query: FrontendRouteDiagnosticsQuery,
): FrontendRouteDiagnosticsResponse => {
  const filteredReports = reports
    .map(normalizeReport)
    .filter((report) => matchesQuery(report, query))
    .sort((left, right) => getReportCreatedAt(right) - getReportCreatedAt(left));

  const totalEvents = filteredReports.length;
  const uniqueRoutes = new Set(filteredReports.map((report) => report.path)).size;
  const affectedSessions = new Set(filteredReports.map((report) => report.session_id)).size;
  const readyEvents = filteredReports.filter((report) => report.status === 'ready').length;
  const timeoutEvents = filteredReports.filter((report) => report.status === 'timeout').length;
  const errorEvents = filteredReports.filter((report) => report.status === 'error').length;
  const slowEvents = filteredReports.filter(
    (report) =>
      report.status === 'timeout' ||
      (typeof report.duration_ms === 'number' && report.duration_ms >= ROUTE_SLOW_THRESHOLD_MS),
  ).length;
  const prefetchHits = filteredReports.filter((report) => report.prefetch_status === 'ready').length;
  const prefetchShortCircuitCount = filteredReports.filter(isPrefetchShortCircuit).length;
  const firstLoadMetrics = buildFirstLoadMetrics(filteredReports);

  const paths = Array.from(
    filteredReports.reduce((map, report) => {
      const current =
        map.get(report.path) ??
        ({
          path: report.path,
          routeLabel: report.route_label,
          totalEvents: 0,
          readyEvents: 0,
          timeoutEvents: 0,
          errorEvents: 0,
          slowEvents: 0,
          prefetchHits: 0,
          avgDurationMs: undefined,
          maxDurationMs: undefined,
          measuredFirstLoads: 0,
          successfulFirstLoads: 0,
          firstLoadAvgDurationMs: undefined,
          prefetchShortCircuitCount: 0,
          lastSeenAt: report.created_at ?? new Date(report.event_at).toISOString(),
          durationSumMs: 0,
          durationSampleCount: 0,
        } satisfies PathAccumulator);
      current.totalEvents += 1;
      if (report.status === 'ready') {
        current.readyEvents += 1;
      }
      if (report.status === 'timeout') {
        current.timeoutEvents += 1;
      }
      if (report.status === 'error') {
        current.errorEvents += 1;
      }
      if (
        report.status === 'timeout' ||
        (typeof report.duration_ms === 'number' && report.duration_ms >= ROUTE_SLOW_THRESHOLD_MS)
      ) {
        current.slowEvents += 1;
      }
      if (report.prefetch_status === 'ready') {
        current.prefetchHits += 1;
      }
      if (isPrefetchShortCircuit(report)) {
        current.prefetchShortCircuitCount += 1;
      }
      if (typeof report.duration_ms === 'number') {
        current.durationSumMs += report.duration_ms;
        current.durationSampleCount += 1;
        current.avgDurationMs = Math.round(current.durationSumMs / current.durationSampleCount);
        current.maxDurationMs = Math.max(current.maxDurationMs ?? 0, report.duration_ms);
      }
      current.routeLabel = current.routeLabel ?? report.route_label;
      const candidateLastSeenAt = report.created_at ?? new Date(report.event_at).toISOString();
      if (candidateLastSeenAt > current.lastSeenAt) {
        current.lastSeenAt = candidateLastSeenAt;
      }
      map.set(report.path, current);
      return map;
    }, new Map<string, PathAccumulator>()),
  )
    .map(([, item]) => {
      const firstLoadPathMetrics = firstLoadMetrics.byPath.get(item.path);
      return {
        path: item.path,
        routeLabel: item.routeLabel,
        totalEvents: item.totalEvents,
        readyEvents: item.readyEvents,
        timeoutEvents: item.timeoutEvents,
        errorEvents: item.errorEvents,
        slowEvents: item.slowEvents,
        prefetchHits: item.prefetchHits,
        avgDurationMs: item.avgDurationMs,
        maxDurationMs: item.maxDurationMs,
        measuredFirstLoads: firstLoadPathMetrics?.measuredFirstLoads ?? 0,
        successfulFirstLoads: firstLoadPathMetrics?.successfulFirstLoads ?? 0,
        firstLoadAvgDurationMs:
          firstLoadPathMetrics && firstLoadPathMetrics.firstLoadDurationSamples > 0
            ? Math.round(
                firstLoadPathMetrics.firstLoadDurationTotalMs /
                  firstLoadPathMetrics.firstLoadDurationSamples,
              )
            : undefined,
        prefetchShortCircuitCount: item.prefetchShortCircuitCount,
        lastSeenAt: item.lastSeenAt,
      } satisfies FrontendRouteDiagnosticsPathSummary;
    })
    .sort((left, right) => {
      const severityDiff = getSeverityRank(left) - getSeverityRank(right);
      if (severityDiff !== 0) {
        return severityDiff;
      }
      return right.lastSeenAt.localeCompare(left.lastSeenAt);
    });

  return {
    summary: {
      totalEvents,
      uniqueRoutes,
      readyEvents,
      timeoutEvents,
      errorEvents,
      slowEvents,
      prefetchHits,
      affectedSessions,
      measuredFirstLoads: firstLoadMetrics.measuredFirstLoads,
      successfulFirstLoads: firstLoadMetrics.successfulFirstLoads,
      firstLoadAvgDurationMs: firstLoadMetrics.firstLoadAvgDurationMs,
      prefetchShortCircuitCount,
    },
    paths,
    events: filteredReports.slice(0, query.limit ?? 20),
  };
};

export const reportFrontendRouteDiagnostics = async (
  tenantId: string,
  payload: { events: FrontendRouteDiagnosticReport[] },
) => {
  if (payload.events.length === 0) {
    return { accepted: 0 };
  }

  if (useMocks) {
    const current = readMockReports(tenantId);
    const next = [...payload.events.map(normalizeReport), ...current];
    writeMockReports(tenantId, next);
    return { accepted: payload.events.length };
  }

  return requestJson<{ accepted: number }>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/system/frontend-route-diagnostics`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(payload),
      keepalive: true,
    },
    '上报前端路由观测失败',
  );
};

export const getFrontendRouteDiagnostics = async (
  tenantId: string,
  query: FrontendRouteDiagnosticsQuery,
): Promise<FrontendRouteDiagnosticsResponse> => {
  if (useMocks) {
    return summarizeFrontendRouteDiagnostics(readMockReports(tenantId), query);
  }

  return requestJson<FrontendRouteDiagnosticsResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/system/frontend-route-diagnostics${buildQueryString(query)}`,
    {
      method: 'GET',
    },
    '获取前端路由观测失败',
  );
};
