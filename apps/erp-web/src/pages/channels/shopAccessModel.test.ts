import { describe, expect, it } from 'vitest';

import {
  buildCreateChannelRoute,
  buildShopAuthCallbackUrl,
  parseShopAuthCallbackSearch,
} from './shopAccessModel';

describe('shopAccessModel', () => {
  it('builds browser callback url with router base path', () => {
    const url = buildShopAuthCallbackUrl({
      origin: 'https://erp.example.com',
      routerMode: 'browser',
      routerBasePath: '/erp',
      shopId: 'SHOP-1',
    });

    expect(url).toBe(
      'https://erp.example.com/erp/channels/shops?auth_callback=1&shop_id=SHOP-1',
    );
  });

  it('builds hash callback url with router base path', () => {
    const url = buildShopAuthCallbackUrl({
      origin: 'https://erp.example.com',
      routerMode: 'hash',
      routerBasePath: '/erp',
      shopId: 'SHOP-1',
    });

    expect(url).toBe(
      'https://erp.example.com/erp#/channels/shops?auth_callback=1&shop_id=SHOP-1',
    );
  });

  it('parses oauth callback with code', () => {
    const parsed = parseShopAuthCallbackSearch(
      '?auth_callback=1&shop_id=SHOP-1&code=auth-code&state=abc',
    );

    expect(parsed).toEqual({
      shopId: 'SHOP-1',
      code: 'auth-code',
      state: 'abc',
      error: undefined,
      errorDescription: undefined,
      hasCode: true,
      hasError: false,
    });
  });

  it('parses oauth callback with error', () => {
    const parsed = parseShopAuthCallbackSearch(
      '?auth_callback=1&shop_id=SHOP-1&error=access_denied&error_description=user_cancelled',
    );

    expect(parsed).toEqual({
      shopId: 'SHOP-1',
      code: undefined,
      state: undefined,
      error: 'access_denied',
      errorDescription: 'user_cancelled',
      hasCode: false,
      hasError: true,
    });
  });

  it('returns null for non-callback search', () => {
    expect(parseShopAuthCallbackSearch('?shop_id=SHOP-1')).toBeNull();
  });

  it('builds create channel route with platform', () => {
    expect(buildCreateChannelRoute('pdd')).toBe('/channels?open_create=1&platform=pdd');
  });

  it('builds create channel route without platform', () => {
    expect(buildCreateChannelRoute(undefined)).toBe('/channels?open_create=1');
  });
});
