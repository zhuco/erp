import { describe, expect, it } from 'vitest';

import type { RouteDiagnosticEvent, RouteLoadingSnapshot } from '@/app/navigation/routeLoadingDiagnostics';

import {
  buildFrontendRouteBackendEfficiencySummary,
  buildFrontendRouteEventRows,
  buildFrontendRouteEfficiencySummary,
  buildFrontendRouteHealthRows,
  buildFrontendRouteHealthSummary,
  buildFrontendRoutePathRankRows,
  buildFrontendRouteTrendRows,
  buildFrontendRouteVersionRows,
} from './frontendRouteObservability';
import type {
  FrontendRouteDiagnosticReport,
  FrontendRouteDiagnosticsSummary,
  FrontendRouteDiagnosticsPathSummary,
} from '@/api/system';

const snapshots: RouteLoadingSnapshot[] = [
  {
    path: '/orders/review',
    status: 'ready',
    routeLabel: 'OrderReviewPage',
    lastDurationMs: 420,
    lastReadyAt: Date.UTC(2026, 3, 13, 4, 0, 0),
    loadCount: 2,
    readyCount: 2,
    firstTerminalStatus: 'ready',
    firstTerminalDurationMs: 420,
    prefetchReadyCount: 1,
    prefetchHitCount: 1,
    prefetchShortCircuitCount: 2,
    lastPrefetchStatus: 'ready',
    lastPrefetchSource: 'search',
    lastPrefetchDurationMs: 180,
  },
  {
    path: '/orders/exceptions',
    status: 'timeout',
    routeLabel: 'OrderExceptionsPage',
    lastDurationMs: 8200,
    lastStartedAt: Date.UTC(2026, 3, 13, 4, 2, 0),
    loadCount: 1,
    timeoutCount: 1,
    firstTerminalStatus: 'timeout',
    firstTerminalDurationMs: 8000,
  },
  {
    path: '/system/observability',
    status: 'error',
    routeLabel: 'ObservabilityPage',
    lastError: 'chunk load failed',
    lastReadyAt: Date.UTC(2026, 3, 13, 4, 4, 0),
    loadCount: 1,
    errorCount: 1,
    firstTerminalStatus: 'error',
  },
];

const events: RouteDiagnosticEvent[] = [
  {
    path: '/orders/exceptions',
    type: 'loading-timeout',
    at: Date.UTC(2026, 3, 13, 4, 2, 8),
  },
  {
    path: '/system/observability',
    type: 'loading-error',
    at: Date.UTC(2026, 3, 13, 4, 4, 0),
    detail: 'chunk load failed',
  },
  {
    path: '/orders/review',
    type: 'prefetch-ready',
    at: Date.UTC(2026, 3, 13, 4, 0, 0),
    detail: 'search',
  },
];

const backendEvents: FrontendRouteDiagnosticReport[] = [
  {
    session_id: 'session-a',
    path: '/orders/review',
    route_label: 'OrderReviewPage',
    event_type: 'loading-ready',
    status: 'ready',
    duration_ms: 420,
    prefetch_status: 'ready',
    prefetch_source: 'search',
    event_at: Date.UTC(2026, 3, 13, 4, 0, 0),
    created_at: '2026-04-13T04:00:00.000Z',
    page_url: 'http://localhost:5173/orders/review',
    build_version: '2026.04.13-a',
    user_agent: 'Vitest',
  },
  {
    session_id: 'session-b',
    path: '/orders/review',
    route_label: 'OrderReviewPage',
    event_type: 'loading-timeout',
    status: 'timeout',
    duration_ms: 3200,
    event_at: Date.UTC(2026, 3, 13, 5, 0, 0),
    created_at: '2026-04-13T05:00:00.000Z',
    page_url: 'http://localhost:5173/orders/review',
    build_version: '2026.04.13-b',
    user_agent: 'Vitest',
  },
  {
    session_id: 'session-c',
    path: '/orders/exceptions',
    route_label: 'OrderExceptionsPage',
    event_type: 'loading-error',
    status: 'error',
    error_summary: 'chunk load failed',
    event_at: Date.UTC(2026, 3, 13, 5, 10, 0),
    created_at: '2026-04-13T05:10:00.000Z',
    page_url: 'http://localhost:5173/orders/exceptions',
    build_version: '2026.04.13-b',
    user_agent: 'Vitest',
  },
];

const backendPaths: FrontendRouteDiagnosticsPathSummary[] = [
  {
    path: '/orders/exceptions',
    routeLabel: 'OrderExceptionsPage',
    totalEvents: 3,
    readyEvents: 0,
    timeoutEvents: 1,
    errorEvents: 1,
    slowEvents: 2,
    prefetchHits: 0,
    avgDurationMs: 2800,
    maxDurationMs: 8200,
    measuredFirstLoads: 1,
    successfulFirstLoads: 0,
    firstLoadAvgDurationMs: undefined,
    prefetchShortCircuitCount: 0,
    lastSeenAt: '2026-04-13T05:10:00.000Z',
  },
  {
    path: '/orders/review',
    routeLabel: 'OrderReviewPage',
    totalEvents: 2,
    readyEvents: 1,
    timeoutEvents: 1,
    errorEvents: 0,
    slowEvents: 1,
    prefetchHits: 1,
    avgDurationMs: 1810,
    maxDurationMs: 3200,
    measuredFirstLoads: 2,
    successfulFirstLoads: 1,
    firstLoadAvgDurationMs: 420,
    prefetchShortCircuitCount: 0,
    lastSeenAt: '2026-04-13T05:00:00.000Z',
  },
];

const backendSummary: FrontendRouteDiagnosticsSummary = {
  totalEvents: 5,
  uniqueRoutes: 2,
  readyEvents: 1,
  timeoutEvents: 1,
  errorEvents: 1,
  slowEvents: 2,
  prefetchHits: 1,
  affectedSessions: 3,
  measuredFirstLoads: 3,
  successfulFirstLoads: 1,
  firstLoadAvgDurationMs: 420,
  prefetchShortCircuitCount: 2,
};

describe('frontendRouteObservability', () => {
  it('builds route health summary counts for tracked routes', () => {
    expect(buildFrontendRouteHealthSummary(snapshots, 1200)).toEqual({
      trackedRoutes: 3,
      readyRoutes: 1,
      loadingRoutes: 0,
      timeoutRoutes: 1,
      errorRoutes: 1,
      slowRoutes: 1,
      prefetchedRoutes: 1,
    });
  });

  it('orders health rows by severity and formats route state labels', () => {
    const rows = buildFrontendRouteHealthRows(snapshots, 1200);

    expect(rows.map((row) => row.path)).toEqual([
      '/system/observability',
      '/orders/exceptions',
      '/orders/review',
    ]);
    expect(rows[0]).toMatchObject({
      statusLabel: '加载失败',
      statusTone: 'error',
      errorSummary: 'chunk load failed',
    });
    expect(rows[1]).toMatchObject({
      statusLabel: '加载超时',
      statusTone: 'warning',
      durationText: '8200ms',
    });
    expect(rows[2]).toMatchObject({
      prefetchLabel: '已预热 / 搜索',
      statusLabel: '正常',
      durationText: '420ms',
      firstOpenText: '首开 420ms',
      prefetchHitText: '50% (1/2)',
      prefetchShortCircuitText: '2 次',
    });
  });

  it('builds efficiency summary for first-open quality and prefetch reuse', () => {
    expect(buildFrontendRouteEfficiencySummary(snapshots)).toEqual({
      measuredFirstLoads: 3,
      successfulFirstLoads: 1,
      firstLoadSuccessRate: 33,
      firstLoadAvgDurationMs: 420,
      totalReadyEvents: 2,
      prefetchHitCount: 1,
      prefetchHitRate: 50,
      prefetchShortCircuitCount: 2,
    });
  });

  it('builds backend efficiency summary for historical first-open playback', () => {
    expect(buildFrontendRouteBackendEfficiencySummary(backendSummary)).toEqual({
      measuredFirstLoads: 3,
      successfulFirstLoads: 1,
      firstLoadSuccessRate: 33,
      firstLoadSuccessText: '33% (1/3)',
      firstLoadAvgDurationMs: 420,
      prefetchShortCircuitCount: 2,
    });
  });

  it('formats recent route events into operator-friendly rows', () => {
    const rows = buildFrontendRouteEventRows(events);

    expect(rows).toEqual([
      expect.objectContaining({
        key: '0',
        path: '/orders/exceptions',
        title: '页面加载超时',
        tone: 'warning',
      }),
      expect.objectContaining({
        key: '1',
        path: '/system/observability',
        title: '页面加载失败',
        detail: 'chunk load failed',
        tone: 'error',
      }),
      expect.objectContaining({
        key: '2',
        path: '/orders/review',
        title: '页面预热完成',
        tone: 'success',
      }),
    ]);
  });

  it('builds backend route trend buckets for observability playback', () => {
    const rows = buildFrontendRouteTrendRows(backendEvents, 6, 6);

    expect(rows).toHaveLength(6);
    expect(rows.at(-2)).toMatchObject({
      totalEvents: 1,
      prefetchHits: 1,
    });
    expect(rows.at(-1)).toMatchObject({
      totalEvents: 2,
      timeoutEvents: 1,
      errorEvents: 1,
    });
  });

  it('sorts backend path ranking rows by fault severity', () => {
    const rows = buildFrontendRoutePathRankRows(backendPaths);

    expect(rows.map((row) => row.path)).toEqual(['/orders/exceptions', '/orders/review']);
    expect(rows[0]).toMatchObject({
      riskLabel: '异常优先',
      timeoutEvents: 1,
      errorEvents: 1,
    });
  });

  it('groups backend events by build version for release comparison', () => {
    const rows = buildFrontendRouteVersionRows(backendEvents);

    expect(rows.map((row) => row.buildVersion)).toEqual(['2026.04.13-b', '2026.04.13-a']);
    expect(rows[0]).toMatchObject({
      healthLabel: '需回归',
      totalEvents: 2,
      affectedRoutes: 2,
      timeoutEvents: 1,
      errorEvents: 1,
    });
    expect(rows[1]).toMatchObject({
      healthLabel: '稳定',
      prefetchHits: 1,
      totalEvents: 1,
    });
  });
});

