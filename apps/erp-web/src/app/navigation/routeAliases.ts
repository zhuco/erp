export type RouteAlias = {
  legacyPath: string;
  canonicalPath: string;
};

export const routeAliases: RouteAlias[] = [];

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
