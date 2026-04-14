import { describe, expect, it } from 'vitest';

import type { FrontendRouteDiagnosticReport } from './frontendRouteDiagnostics';

import { summarizeFrontendRouteDiagnostics } from './frontendRouteDiagnostics';

const records: FrontendRouteDiagnosticReport[] = [
  {
    session_id: 'session-a',
    path: '/orders/review',
    route_label: 'OrderReviewPage',
    event_type: 'loading-ready',
    status: 'ready',
    duration_ms: 420,
    prefetch_status: 'ready',
    prefetch_source: 'search',
    prefetch_duration_ms: 180,
    event_at: Date.UTC(2026, 3, 13, 4, 0, 0),
    created_at: '2026-04-13T04:00:00.000Z',
    page_url: 'http://localhost:5173/orders/review',
    build_version: '2026.04.13',
    user_agent: 'Vitest',
  },
  {
    session_id: 'session-a',
    path: '/orders/review',
    route_label: 'OrderReviewPage',
    event_type: 'prefetch-skipped',
    status: 'ready',
    detail: 'already-ready',
    event_at: Date.UTC(2026, 3, 13, 4, 1, 0),
    created_at: '2026-04-13T04:01:00.000Z',
    page_url: 'http://localhost:5173/orders/review',
    build_version: '2026.04.13',
    user_agent: 'Vitest',
  },
  {
    session_id: 'session-b',
    path: '/orders/exceptions',
    route_label: 'OrderExceptionsPage',
    event_type: 'loading-timeout',
    status: 'timeout',
    duration_ms: 8200,
    event_at: Date.UTC(2026, 3, 13, 4, 2, 8),
    created_at: '2026-04-13T04:02:08.000Z',
    page_url: 'http://localhost:5173/orders/exceptions',
    build_version: '2026.04.13',
    user_agent: 'Vitest',
  },
  {
    session_id: 'session-c',
    path: '/system/observability',
    route_label: 'ObservabilityPage',
    event_type: 'loading-error',
    status: 'error',
    error_summary: 'chunk load failed',
    event_at: Date.UTC(2026, 3, 13, 4, 4, 0),
    created_at: '2026-04-13T04:04:00.000Z',
    page_url: 'http://localhost:5173/system/observability',
    build_version: '2026.04.13',
    user_agent: 'Vitest',
  },
];

describe('frontendRouteDiagnostics', () => {
  it('summarizes backend route diagnostics for observability dashboards', () => {
    expect(
      summarizeFrontendRouteDiagnostics(records, {
        limit: 20,
      }).summary,
    ).toEqual({
      totalEvents: 4,
      uniqueRoutes: 3,
      readyEvents: 2,
      timeoutEvents: 1,
      errorEvents: 1,
      slowEvents: 1,
      prefetchHits: 1,
      affectedSessions: 3,
      measuredFirstLoads: 3,
      successfulFirstLoads: 1,
      firstLoadAvgDurationMs: 420,
      prefetchShortCircuitCount: 1,
    });
  });

  it('aggregates path health rows by severity and recency', () => {
    const result = summarizeFrontendRouteDiagnostics(records, { limit: 20 });

    expect(result.paths.map((item) => item.path)).toEqual([
      '/system/observability',
      '/orders/exceptions',
      '/orders/review',
    ]);
    expect(result.paths[0]).toMatchObject({
      path: '/system/observability',
      errorEvents: 1,
      lastSeenAt: '2026-04-13T04:04:00.000Z',
    });
    expect(result.paths[1]).toMatchObject({
      path: '/orders/exceptions',
      timeoutEvents: 1,
      maxDurationMs: 8200,
    });
    expect(result.paths[2]).toMatchObject({
      path: '/orders/review',
      readyEvents: 2,
      prefetchHits: 1,
      avgDurationMs: 420,
      measuredFirstLoads: 1,
      successfulFirstLoads: 1,
      firstLoadAvgDurationMs: 420,
      prefetchShortCircuitCount: 1,
    });
  });

  it('supports keyword and status filters for backend monitoring', () => {
    const result = summarizeFrontendRouteDiagnostics(records, {
      limit: 20,
      pathKeyword: 'orders',
      status: 'timeout',
    });

    expect(result.summary.totalEvents).toBe(1);
    expect(result.paths).toHaveLength(1);
    expect(result.paths[0]?.path).toBe('/orders/exceptions');
    expect(result.events).toHaveLength(1);
    expect(result.events[0]).toMatchObject({
      path: '/orders/exceptions',
      status: 'timeout',
      event_type: 'loading-timeout',
    });
  });
});
