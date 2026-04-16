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
        { legacyPath: '/couriers', canonicalPath: '/logistics/couriers' },
        { legacyPath: '/stats/sales', canonicalPath: '/insights/revenue/overview' },
        { legacyPath: '/stats/sales-daily', canonicalPath: '/insights/revenue/daily-brief' },
        { legacyPath: '/stats/sales-trends', canonicalPath: '/insights/revenue/trend-radar' },
        { legacyPath: '/stats/shipping', canonicalPath: '/insights/fulfillment/lead-time' },
        { legacyPath: '/stats/sales-cycle', canonicalPath: '/insights/fulfillment/order-cycle' },
        { legacyPath: '/stats/best-sellers', canonicalPath: '/insights/catalog/hot-sku' },
        { legacyPath: '/stats/slow-moving', canonicalPath: '/insights/catalog/slow-sku' },
        { legacyPath: '/reports', canonicalPath: '/insights/workbench/report-center' },
        { legacyPath: '/exports', canonicalPath: '/insights/workbench/export-queue' },
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
    expect(getCanonicalPath('/stats/sales')).toBe('/insights/revenue/overview');
    expect(getCanonicalPath('/orders')).toBe('/orders');
    expect(getLegacyRedirectTarget('/channels/mappings', '?platform=tmall')).toBeNull();
  });
});
