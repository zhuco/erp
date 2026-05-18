export type RouteAlias = {
  legacyPath: string;
  canonicalPath: string;
};

export const routeAliases: RouteAlias[] = [
  {
    legacyPath: '/orders',
    canonicalPath: '/sales/order-query',
  },
  {
    legacyPath: '/orders/exceptions',
    canonicalPath: '/orders/recheck',
  },
  {
    legacyPath: '/products',
    canonicalPath: '/goods/products/query',
  },
  {
    legacyPath: '/kits',
    canonicalPath: '/goods/kits/query',
  },
];

const routeAliasMap = new Map(routeAliases.map((item) => [item.legacyPath, item.canonicalPath]));
const legacyKitDetailPattern = /^\/kits\/([^/?#]+)$/;

export const getCanonicalPath = (pathname: string): string => routeAliasMap.get(pathname) ?? pathname;

export const getLegacyRedirectTarget = (
  pathname: string,
  search = '',
  hash = ''
): string | null => {
  const legacyKitDetailMatch = pathname.match(legacyKitDetailPattern);
  if (legacyKitDetailMatch) {
    const kitSkuId = decodeURIComponent(legacyKitDetailMatch[1] ?? '');
    if (!kitSkuId) {
      return null;
    }
    return `/goods/kits/${encodeURIComponent(kitSkuId)}/edit`;
  }
  const canonicalPath = routeAliasMap.get(pathname);
  if (!canonicalPath) {
    return null;
  }
  return `${canonicalPath}${search}${hash}`;
};

export const toRelativeRoutePath = (pathname: string): string => pathname.replace(/^\/+/, '');
