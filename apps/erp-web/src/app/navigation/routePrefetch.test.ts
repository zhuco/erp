import { describe, expect, it, vi } from 'vitest';
import { createElement } from 'react';

import { clearRouteLoadingDiagnostics, getRouteLoadingSnapshot } from './routeLoadingDiagnostics';
import {
  attachRoutePrefetch,
  buildRoutePrefetchRegistry,
  clearRoutePrefetchRegistry,
  installRoutePrefetchRegistry,
  prefetchRoutePath,
} from './routePrefetch';

const Guard = ({ children }: { children?: unknown }) => createElement('section', null, children);

describe('routePrefetch', () => {
  it('对已经预热完成的路由不会重复触发 loader', async () => {
    clearRoutePrefetchRegistry();
    clearRouteLoadingDiagnostics();

    const loader = vi.fn(async () => {
      await Promise.resolve();
    });

    const OrdersPage = attachRoutePrefetch(() => null, {
      routeLabel: 'OrdersPage',
      prefetch: loader,
    });

    installRoutePrefetchRegistry([
      {
        path: '/',
        children: [
          {
            path: 'orders',
            element: createElement(Guard, null, createElement(OrdersPage)),
          },
        ],
      },
    ]);

    await expect(prefetchRoutePath('/orders', { source: 'search' })).resolves.toBe(true);
    await expect(prefetchRoutePath('/orders', { source: 'programmatic' })).resolves.toBe(true);

    expect(loader).toHaveBeenCalledTimes(1);
    expect(getRouteLoadingSnapshot('/orders')).toMatchObject({
      prefetchReadyCount: 1,
      prefetchShortCircuitCount: 1,
      lastPrefetchReason: 'already-ready',
    });
  });

  it('collects prefetchers from nested guarded routes', async () => {
    const orderReviewPrefetch = vi.fn().mockResolvedValue(undefined);
    const orderExceptionsPrefetch = vi.fn().mockResolvedValue(undefined);

    const OrderReviewPage = attachRoutePrefetch(() => null, {
      routeLabel: 'OrderReviewPage',
      prefetch: orderReviewPrefetch,
    });
    const OrderExceptionsPage = attachRoutePrefetch(() => null, {
      routeLabel: 'OrderExceptionsPage',
      prefetch: orderExceptionsPrefetch,
    });

    const registry = buildRoutePrefetchRegistry([
      {
        path: '/',
        children: [
          {
            element: createElement(Guard, null, createElement(Guard, null)),
            children: [
              {
                path: 'orders/review',
                element: createElement(Guard, null, createElement(OrderReviewPage)),
              },
              {
                path: 'orders/exceptions',
                element: createElement(Guard, null, createElement(OrderExceptionsPage)),
              },
              {
                path: 'orders/:order_id',
                element: createElement(Guard, null, createElement(OrderReviewPage)),
              },
            ],
          },
        ],
      },
    ]);

    expect(Array.from(registry.keys())).toEqual(['/orders/review', '/orders/exceptions']);
  });

  it('deduplicates concurrent prefetch requests for the same route', async () => {
    clearRoutePrefetchRegistry();
    clearRouteLoadingDiagnostics();

    const loader = vi.fn(async () => {
      await Promise.resolve();
    });

    const OrdersPage = attachRoutePrefetch(() => null, {
      routeLabel: 'OrdersPage',
      prefetch: loader,
    });

    installRoutePrefetchRegistry([
      {
        path: '/',
        children: [
          {
            path: 'orders',
            element: createElement(Guard, null, createElement(OrdersPage)),
          },
        ],
      },
    ]);

    await Promise.all([
      prefetchRoutePath('/orders', { source: 'search' }),
      prefetchRoutePath('/orders', { source: 'programmatic' }),
    ]);

    expect(loader).toHaveBeenCalledTimes(1);
    expect(getRouteLoadingSnapshot('/orders')).toMatchObject({
      prefetchReadyCount: 1,
      prefetchShortCircuitCount: 1,
      lastPrefetchReason: 'inflight',
    });
  });
});
