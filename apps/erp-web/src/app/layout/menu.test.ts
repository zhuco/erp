import { describe, expect, it } from 'vitest';

import { appMenuItems, flattenMenuItems } from './menu';
import { findUnregisteredMenuPaths } from '../routeRegistry';

describe('appMenuItems', () => {
  it('exposes grouped top-level workbench navigation with directory expansions', () => {
    expect(flattenMenuItems(appMenuItems).map((item) => item.path)).toEqual([
      '/dashboard',
      '/channels',
      '/couriers',
      '/tags',
      '/warehouses',
      '/logistics-fees',
      '/suppliers',
      '/customers/categories',
      '/customers/member-levels',
      '/customers/tags',
      '/wms/containers',
      '/wms/sender-addresses',
      '/wms/warehouse-areas',
      '/wms/warehouse-locations',
      '/system/permissions',
      '/system/roles',
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

  it('keeps the upgraded modules grouped at the top level', () => {
    expect(appMenuItems.map((item) => item.name)).toEqual([
      '首页',
      '目录',
      '销售',
      '履约',
      '经营',
      '报表',
    ]);

    const directoryModule = appMenuItems.find((item) => item.name === '目录');
    expect(flattenMenuItems(directoryModule ? [directoryModule] : []).map((item) => item.path)).toEqual([
      '/channels',
      '/couriers',
      '/tags',
      '/warehouses',
      '/logistics-fees',
      '/suppliers',
      '/customers/categories',
      '/customers/member-levels',
      '/customers/tags',
      '/wms/containers',
      '/wms/sender-addresses',
      '/wms/warehouse-areas',
      '/wms/warehouse-locations',
      '/system/permissions',
      '/system/roles',
    ]);
  });

  it('keeps every visible menu leaf registered as a reachable route', () => {
    expect(findUnregisteredMenuPaths()).toEqual([]);
  });
});
