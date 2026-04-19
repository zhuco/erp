import { describe, expect, it } from 'vitest';

import { getCanonicalPath, getLegacyRedirectTarget, routeAliases } from './routeAliases';

describe('routeAliases', () => {
  it('removes legacy redirects for pages that are no longer part of the reduced UI shell', () => {
    expect(routeAliases).toEqual([]);
  });

  it('leaves removed legacy routes unresolved instead of redirecting them into hidden pages', () => {
    expect(getLegacyRedirectTarget('/mappings', '?platform=tmall', '#filters')).toBeNull();
    expect(getLegacyRedirectTarget('/stats/sales')).toBeNull();
  });

  it('returns original paths for the five preserved routes', () => {
    expect(getCanonicalPath('/dashboard')).toBe('/dashboard');
    expect(getCanonicalPath('/channels')).toBe('/channels');
    expect(getCanonicalPath('/orders/review')).toBe('/orders/review');
    expect(getCanonicalPath('/orders/recheck')).toBe('/orders/recheck');
    expect(getCanonicalPath('/sales/settings')).toBe('/sales/settings');
  });
});
