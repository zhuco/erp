import type { PlatformCode } from '@/features/platform';

type BuildShopAuthCallbackUrlInput = {
  origin: string;
  routerMode: 'browser' | 'hash';
  routerBasePath: string;
  shopId: string;
};

export type ShopAuthCallbackState = {
  shopId?: string;
  code?: string;
  state?: string;
  error?: string;
  errorDescription?: string;
  hasCode: boolean;
  hasError: boolean;
};

const normalizeBasePath = (value: string) => {
  if (!value || value === '/') {
    return '/';
  }

  const withLeadingSlash = value.startsWith('/') ? value : `/${value}`;
  return withLeadingSlash.endsWith('/')
    ? withLeadingSlash.slice(0, withLeadingSlash.length - 1)
    : withLeadingSlash;
};

export const buildShopAuthCallbackUrl = ({
  origin,
  routerMode,
  routerBasePath,
  shopId,
}: BuildShopAuthCallbackUrlInput): string => {
  const normalizedOrigin = origin.replace(/\/+$/, '');
  const normalizedBasePath = normalizeBasePath(routerBasePath);
  const params = new URLSearchParams({
    auth_callback: '1',
    shop_id: shopId,
  });

  if (routerMode === 'hash') {
    const baseSegment = normalizedBasePath === '/' ? '' : normalizedBasePath;
    return `${normalizedOrigin}${baseSegment}#/channels/shops?${params.toString()}`;
  }

  const browserPath =
    normalizedBasePath === '/' ? '/channels/shops' : `${normalizedBasePath}/channels/shops`;
  return `${normalizedOrigin}${browserPath}?${params.toString()}`;
};

export const parseShopAuthCallbackSearch = (
  search: string,
): ShopAuthCallbackState | null => {
  const params = new URLSearchParams(search.startsWith('?') ? search : `?${search}`);
  if (params.get('auth_callback') !== '1') {
    return null;
  }

  const code = params.get('code') ?? undefined;
  const state = params.get('state') ?? undefined;
  const error = params.get('error') ?? undefined;
  const errorDescription = params.get('error_description') ?? undefined;
  const shopId = params.get('shop_id') ?? undefined;

  return {
    shopId,
    code,
    state,
    error,
    errorDescription,
    hasCode: Boolean(code),
    hasError: Boolean(error),
  };
};

export const buildCreateChannelRoute = (platform?: PlatformCode): string => {
  const params = new URLSearchParams();
  params.set('open_create', '1');
  if (platform) {
    params.set('platform', platform);
  }
  return `/channels?${params.toString()}`;
};
