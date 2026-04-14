import { describe, expect, it } from 'vitest';

import type {
  RouteDiagnosticEvent,
  RouteLoadingSnapshot,
} from '@/app/navigation/routeLoadingDiagnostics';

import {
  buildRouteDiagnosticReport,
  isRouteDiagnosticEventReportable,
} from './routeDiagnosticsReporting';

const snapshot: RouteLoadingSnapshot = {
  path: '/orders/review',
  status: 'ready',
  routeLabel: 'OrderReviewPage',
  lastDurationMs: 420,
  lastError: undefined,
  lastPrefetchStatus: 'ready',
  lastPrefetchSource: 'search',
  lastPrefetchDurationMs: 180,
};

describe('routeDiagnosticsReporting', () => {
  it('only reports terminal loading and prefetch events', () => {
    expect(
      isRouteDiagnosticEventReportable({
        path: '/orders/review',
        type: 'loading-start',
        at: Date.UTC(2026, 3, 13, 4, 0, 0),
      }),
    ).toBe(false);

    expect(
      isRouteDiagnosticEventReportable({
        path: '/orders/review',
        type: 'loading-ready',
        at: Date.UTC(2026, 3, 13, 4, 0, 0),
      }),
    ).toBe(true);

    expect(
      isRouteDiagnosticEventReportable({
        path: '/orders/review',
        type: 'prefetch-ready',
        at: Date.UTC(2026, 3, 13, 4, 0, 0),
      }),
    ).toBe(true);
  });

  it('builds a backend-ready report payload from route diagnostics', () => {
    const event: RouteDiagnosticEvent = {
      path: '/orders/review',
      type: 'loading-ready',
      at: Date.UTC(2026, 3, 13, 4, 0, 0),
    };

    expect(
      buildRouteDiagnosticReport(event, snapshot, {
        sessionId: 'session-1',
        currentUrl: 'http://localhost:5173/orders/review',
        buildVersion: '2026.04.13',
        userAgent: 'Vitest',
      }),
    ).toEqual({
      session_id: 'session-1',
      path: '/orders/review',
      route_label: 'OrderReviewPage',
      event_type: 'loading-ready',
      status: 'ready',
      duration_ms: 420,
      prefetch_status: 'ready',
      prefetch_source: 'search',
      prefetch_duration_ms: 180,
      error_summary: undefined,
      detail: undefined,
      event_at: Date.UTC(2026, 3, 13, 4, 0, 0),
      page_url: 'http://localhost:5173/orders/review',
      build_version: '2026.04.13',
      user_agent: 'Vitest',
    });
  });

  it('preserves timeout and error details for backend filtering', () => {
    const timeoutEvent: RouteDiagnosticEvent = {
      path: '/orders/exceptions',
      type: 'loading-timeout',
      at: Date.UTC(2026, 3, 13, 4, 2, 8),
      detail: 'slow-chunk',
    };

    expect(
      buildRouteDiagnosticReport(
        timeoutEvent,
        {
          path: '/orders/exceptions',
          status: 'timeout',
          routeLabel: 'OrderExceptionsPage',
          lastDurationMs: 8200,
          lastError: 'chunk load failed',
        },
        {
          sessionId: 'session-2',
          currentUrl: 'http://localhost:5173/orders/exceptions',
          buildVersion: '2026.04.13',
          userAgent: 'Vitest',
        },
      ),
    ).toMatchObject({
      event_type: 'loading-timeout',
      status: 'timeout',
      duration_ms: 8200,
      error_summary: 'chunk load failed',
      detail: 'slow-chunk',
    });
  });
});
