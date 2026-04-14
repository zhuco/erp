import { describe, expect, it } from 'vitest';

import { getCanonicalPath, getLegacyRedirectTarget, routeAliases } from './routeAliases';

describe('routeAliases', () => {
  it('maps legacy business routes to the new canonical information architecture', () => {
    expect(routeAliases).toEqual(
      expect.arrayContaining([
        { legacyPath: '/mappings', canonicalPath: '/channels/mappings' },
        { legacyPath: '/sales/shops', canonicalPath: '/channels/shops' },
        { legacyPath: '/sales/product-match', canonicalPath: '/channels/product-match' },
        { legacyPath: '/sales/shop-orders', canonicalPath: '/channels/shop-orders' },
        { legacyPath: '/sales/sync-job-runs', canonicalPath: '/channels/sync-jobs' },
        { legacyPath: '/system/integrations', canonicalPath: '/integrations/center' },
        { legacyPath: '/system/sync-jobs', canonicalPath: '/integrations/sync-jobs' },
        { legacyPath: '/system/print-devices', canonicalPath: '/integrations/print-devices' },
        { legacyPath: '/system/print-mappings', canonicalPath: '/integrations/print-mappings' },
        { legacyPath: '/suppliers', canonicalPath: '/procurement/suppliers' },
        { legacyPath: '/warehouses', canonicalPath: '/wms/warehouses' },
        { legacyPath: '/couriers', canonicalPath: '/logistics/couriers' }
      ])
    );
  });

  it('preserves search and hash when redirecting legacy routes', () => {
    expect(getLegacyRedirectTarget('/mappings', '?platform=tmall', '#filters')).toBe(
      '/channels/mappings?platform=tmall#filters'
    );
  });

  it('normalizes legacy paths to canonical paths and leaves other paths unchanged', () => {
    expect(getCanonicalPath('/system/integrations')).toBe('/integrations/center');
    expect(getCanonicalPath('/orders')).toBe('/orders');
    expect(getLegacyRedirectTarget('/channels/mappings', '?platform=tmall')).toBeNull();
  });
});
