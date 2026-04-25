import { describe, expect, it } from 'vitest';

import { appMenuItems, flattenMenuItems } from './menu';

describe('appMenuItems', () => {
  it('exposes the directory module as a grouped first-level menu with second-level master-data entries', () => {
    expect(flattenMenuItems(appMenuItems).map((item) => item.path)).toEqual([
      '/dashboard',
      '/channels',
      '/couriers',
      '/tags',
      '/warehouses',
      '/logistics-fees',
      '/suppliers',
      '/sales/order-query',
      '/orders/review',
      '/orders/recheck',
      '/sales/settings',
    ]);
  });

  it('keeps only home, directory, and the sales module at the top level', () => {
    expect(appMenuItems).toHaveLength(3);
    expect(appMenuItems[0]).toMatchObject({
      path: '/dashboard',
    });

    const directoryModule = appMenuItems.find((item) => item.name === '目录');
    expect(directoryModule?.path).toBeUndefined();
    expect(flattenMenuItems(directoryModule ? [directoryModule] : []).map((item) => item.path)).toEqual([
      '/channels',
      '/couriers',
      '/tags',
      '/warehouses',
      '/logistics-fees',
      '/suppliers',
    ]);

    const salesModule = appMenuItems.find((item) =>
      flattenMenuItems([item]).some((leaf) => leaf.path === '/sales/order-query'),
    );
    expect(flattenMenuItems(salesModule ? [salesModule] : []).map((item) => item.path)).toEqual([
      '/sales/order-query',
      '/orders/review',
      '/orders/recheck',
      '/sales/settings',
    ]);
  });
});
