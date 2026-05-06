import { describe, expect, it } from 'vitest';

import { appMenuItems, flattenMenuItems } from './menu';
import { findUnregisteredMenuPaths } from '../routeRegistry';

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
      '/inventory',
      '/wms/waves',
      '/wms/ship',
      '/logistics/exceptions',
      '/after-sales/exchanges',
      '/finance/profit',
      '/planning/plan-execution',
      '/stats/sales',
      '/integrations/center',
      '/system/observability',
    ]);
  });

  it('keeps the P0 modules grouped at the top level', () => {
    expect(appMenuItems.map((item) => item.name)).toEqual([
      '首页',
      '目录',
      '销售',
      '履约',
      '经营',
      '报表',
    ]);
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

    const fulfillmentModule = appMenuItems.find((item) => item.name === '履约');
    expect(flattenMenuItems(fulfillmentModule ? [fulfillmentModule] : []).map((item) => item.path)).toEqual([
      '/inventory',
      '/wms/waves',
      '/wms/ship',
      '/logistics/exceptions',
      '/after-sales/exchanges',
    ]);
  });

  it('keeps every visible menu leaf registered as a reachable route', () => {
    expect(findUnregisteredMenuPaths()).toEqual([]);
  });
});
