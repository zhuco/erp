import { describe, expect, it } from 'vitest';

import { appMenuItems, flattenMenuItems } from './menu';

describe('appMenuItems', () => {
  it('only exposes the approved five-page UI shell', () => {
    expect(flattenMenuItems(appMenuItems).map((item) => item.path)).toEqual([
      '/dashboard',
      '/channels',
      '/orders/review',
      '/orders/recheck',
      '/sales/settings',
    ]);
  });

  it('keeps only home, channel directory, and the reduced sales module', () => {
    expect(appMenuItems).toHaveLength(3);
    expect(appMenuItems[0]).toMatchObject({
      path: '/dashboard',
    });

    const channelModule = appMenuItems.find((item) =>
      flattenMenuItems([item]).some((leaf) => leaf.path === '/channels'),
    );
    expect(flattenMenuItems(channelModule ? [channelModule] : []).map((item) => item.path)).toEqual([
      '/channels',
    ]);

    const salesModule = appMenuItems.find((item) =>
      flattenMenuItems([item]).some((leaf) => leaf.path === '/orders/review'),
    );
    expect(flattenMenuItems(salesModule ? [salesModule] : []).map((item) => item.path)).toEqual([
      '/orders/review',
      '/orders/recheck',
      '/sales/settings',
    ]);
  });
});
