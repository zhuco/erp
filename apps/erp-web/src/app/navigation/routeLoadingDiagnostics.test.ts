import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  ROUTE_LOADING_TIMEOUT_MS,
  clearRouteLoadingDiagnostics,
  getRouteLoadingSnapshot,
  markRouteLoadingError,
  markRouteLoadingReady,
  markRouteLoadingStart,
  markRoutePrefetchReady,
  markRoutePrefetchSkipped,
  markRoutePrefetchStart,
  subscribeRouteDiagnosticEvents,
} from './routeLoadingDiagnostics';

describe('routeLoadingDiagnostics', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-04-13T12:00:00.000Z'));
    clearRouteLoadingDiagnostics();
  });

  afterEach(() => {
    clearRouteLoadingDiagnostics();
    vi.useRealTimers();
  });

  it('records loading duration when a route becomes ready', () => {
    markRouteLoadingStart('/orders/review');

    vi.advanceTimersByTime(420);
    vi.setSystemTime(new Date('2026-04-13T12:00:00.420Z'));
    markRouteLoadingReady('/orders/review', { routeLabel: 'OrderReviewPage' });

    expect(getRouteLoadingSnapshot('/orders/review')).toMatchObject({
      path: '/orders/review',
      status: 'ready',
      routeLabel: 'OrderReviewPage',
      loadCount: 1,
      readyCount: 1,
      lastDurationMs: 420,
      firstTerminalStatus: 'ready',
      firstTerminalDurationMs: 420,
      lastError: undefined,
    });
  });

  it('marks a route as timeout when loading exceeds the threshold', () => {
    markRouteLoadingStart('/orders/exceptions');

    vi.advanceTimersByTime(ROUTE_LOADING_TIMEOUT_MS + 1);

    expect(getRouteLoadingSnapshot('/orders/exceptions')).toMatchObject({
      path: '/orders/exceptions',
      status: 'timeout',
    });
  });

  it('keeps the latest prefetch result and error summary for diagnostics', () => {
    markRoutePrefetchStart('/orders/review', 'search');
    vi.advanceTimersByTime(180);
    vi.setSystemTime(new Date('2026-04-13T12:00:00.180Z'));
    markRoutePrefetchReady('/orders/review');
    markRouteLoadingStart('/orders/review');
    vi.advanceTimersByTime(420);
    vi.setSystemTime(new Date('2026-04-13T12:00:00.600Z'));
    markRouteLoadingReady('/orders/review', { routeLabel: 'OrderReviewPage' });

    expect(getRouteLoadingSnapshot('/orders/review')).toMatchObject({
      path: '/orders/review',
      readyCount: 1,
      prefetchReadyCount: 1,
      prefetchHitCount: 1,
      lastPrefetchSource: 'search',
      lastPrefetchStatus: 'ready',
      lastPrefetchDurationMs: 180,
      firstTerminalStatus: 'ready',
      firstTerminalDurationMs: 420,
    });

    markRouteLoadingError('/orders/review', new Error('chunk load failed'));
    markRoutePrefetchSkipped('/orders/review', 'network-constrained');

    expect(getRouteLoadingSnapshot('/orders/review')).toMatchObject({
      path: '/orders/review',
      status: 'error',
      lastError: 'chunk load failed',
      lastPrefetchStatus: 'skipped',
      lastPrefetchReason: 'network-constrained',
    });
  });

  it('publishes terminal diagnostic events with the latest route snapshot', () => {
    const received: Array<{ eventType: string; status: string; durationMs?: number }> = [];
    const unsubscribe = subscribeRouteDiagnosticEvents((event, snapshot) => {
      received.push({
        eventType: event.type,
        status: snapshot.status,
        durationMs: snapshot.lastDurationMs,
      });
    });

    markRouteLoadingStart('/orders/review');
    vi.advanceTimersByTime(420);
    vi.setSystemTime(new Date('2026-04-13T12:00:00.420Z'));
    markRouteLoadingReady('/orders/review', { routeLabel: 'OrderReviewPage' });

    unsubscribe();

    expect(received.at(-1)).toEqual({
      eventType: 'loading-ready',
      status: 'ready',
      durationMs: 420,
    });
  });

  it('preserves the first-open outcome when the first attempt times out before eventual recovery', () => {
    markRouteLoadingStart('/orders/exceptions');

    vi.advanceTimersByTime(ROUTE_LOADING_TIMEOUT_MS + 1);
    vi.setSystemTime(new Date('2026-04-13T12:00:08.001Z'));
    markRouteLoadingReady('/orders/exceptions', { routeLabel: 'OrderExceptionsPage' });

    expect(getRouteLoadingSnapshot('/orders/exceptions')).toMatchObject({
      path: '/orders/exceptions',
      status: 'ready',
      readyCount: 1,
      timeoutCount: 1,
      firstTerminalStatus: 'timeout',
      firstTerminalDurationMs: ROUTE_LOADING_TIMEOUT_MS,
    });
  });
});
