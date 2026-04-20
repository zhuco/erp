import { describe, expect, it } from 'vitest';

import { getCanonicalPath, getLegacyRedirectTarget, routeAliases } from './routeAliases';

describe('routeAliases', () => {
  it('keeps only the legacy redirects that remain part of the unified data workbench surface', () => {
    expect(routeAliases).toEqual([
      {
        legacyPath: '/orders',
        canonicalPath: '/sales/order-query',
      },
      {
        legacyPath: '/orders/exceptions',
        canonicalPath: '/orders/recheck',
      },
    ]);
  });

  it('leaves removed legacy routes unresolved instead of redirecting them into hidden pages', () => {
    expect(getLegacyRedirectTarget('/mappings', '?platform=tmall', '#filters')).toBeNull();
    expect(getLegacyRedirectTarget('/stats/sales')).toBeNull();
  });

  it('redirects retained legacy order routes to the new unified workbench pages', () => {
    expect(getLegacyRedirectTarget('/orders', '?tab=main', '#summary')).toBe(
      '/sales/order-query?tab=main#summary',
    );
    expect(getLegacyRedirectTarget('/orders/exceptions')).toBe('/orders/recheck');
  });

  it('returns original paths for the five preserved routes', () => {
    expect(getCanonicalPath('/dashboard')).toBe('/dashboard');
    expect(getCanonicalPath('/channels')).toBe('/channels');
    expect(getCanonicalPath('/sales/order-query')).toBe('/sales/order-query');
    expect(getCanonicalPath('/orders/review')).toBe('/orders/review');
    expect(getCanonicalPath('/orders/recheck')).toBe('/orders/recheck');
    expect(getCanonicalPath('/sales/settings')).toBe('/sales/settings');
  });
});
