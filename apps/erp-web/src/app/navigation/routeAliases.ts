export type RouteAlias = {
  legacyPath: string;
  canonicalPath: string;
};

export const routeAliases: RouteAlias[] = [
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
];

const routeAliasMap = new Map(routeAliases.map((item) => [item.legacyPath, item.canonicalPath]));

export const getCanonicalPath = (pathname: string): string => routeAliasMap.get(pathname) ?? pathname;

export const getLegacyRedirectTarget = (
  pathname: string,
  search = '',
  hash = ''
): string | null => {
  const canonicalPath = routeAliasMap.get(pathname);
  if (!canonicalPath) {
    return null;
  }
  return `${canonicalPath}${search}${hash}`;
};

export const toRelativeRoutePath = (pathname: string): string => pathname.replace(/^\/+/, '');
