import { describe, expect, it } from 'vitest';

import { appMenuItems, flattenMenuItems } from './menu';

describe('appMenuItems', () => {
  it('uses the industrial ERP top-level information architecture order', () => {
    expect(appMenuItems.map((item) => item.name)).toEqual([
      '首页',
      '渠道',
      '销售',
      '商品',
      '采购',
      '计划',
      '仓储',
      '物流',
      '客户',
      '财务',
      '营销',
      '分析',
      '集成',
      '系统',
    ]);
  });

  it('exposes canonical menu paths for the migrated domains', () => {
    const leafPaths = flattenMenuItems(appMenuItems).map((item) => item.path);

    expect(leafPaths).toEqual(
      expect.arrayContaining([
        '/channels/shops',
        '/channels/product-match',
        '/channels/shop-orders',
        '/channels/sync-jobs',
        '/orders/recheck',
        '/sales/shop-order-download',
        '/sales/shop-product-match-query',
        '/sales/shop-inventory-sync',
        '/shop-products',
        '/integrations/center',
        '/integrations/sync-jobs',
        '/integrations/print-devices',
        '/integrations/print-mappings',
        '/procurement/suppliers',
        '/wms/warehouses',
        '/logistics/couriers',
      ]),
    );
  });

  it('removes legacy misplaced routes and hidden demo entries from the menu tree', () => {
    const leafPaths = flattenMenuItems(appMenuItems).map((item) => item.path);

    expect(leafPaths).not.toContain('/channels/mappings');

    expect(leafPaths).not.toEqual(
      expect.arrayContaining([
        '/sales/shops',
        '/sales/product-match',
        '/sales/shop-orders',
        '/channels/mappings',
        '/mappings',
        '/sales/sync-job-runs',
        '/system/integrations',
        '/system/sync-jobs',
        '/system/print-devices',
        '/system/print-mappings',
        '/suppliers',
        '/warehouses',
        '/couriers',
        '/system/ag-grid-enterprise',
      ]),
    );
  });

  it('groups integration and print operations under the platform module', () => {
    const platformModule = appMenuItems.find((item) => item.name === '集成');
    expect(platformModule).toBeDefined();

    const leafPaths = flattenMenuItems(platformModule?.children ?? []).map((item) => item.path);
    expect(leafPaths).toEqual(
      expect.arrayContaining([
        '/integrations/center',
        '/integrations/sync-jobs',
        '/integrations/print-devices',
        '/integrations/print-mappings',
        '/print-jobs',
        '/print-templates',
      ]),
    );
  });

  it('organizes the sales module into grouped business sections and includes order review', () => {
    const salesModule = appMenuItems.find((item) => item.name === '销售');
    expect(salesModule).toBeDefined();

    const salesGroups = salesModule?.children ?? [];
    expect(salesGroups.map((item) => item.name)).toEqual(
      expect.arrayContaining(['订单中心', '网店订单', '履约中心', '结算规则', '业务设置']),
    );

    const orderCenterGroup = salesGroups.find((item) => item.name === '订单中心');
    expect(flattenMenuItems(orderCenterGroup?.children ?? []).map((item) => item.path)).toEqual([
      '/sales/new-order',
      '/sales/import-order',
      '/orders/review',
      '/orders/recheck',
      '/orders',
      '/orders/exceptions',
    ]);

    const ecommerceGroup = salesGroups.find((item) => item.name === '网店订单');
    expect(flattenMenuItems(ecommerceGroup?.children ?? []).map((item) => item.path)).toEqual([
      '/sales/shop-order-download',
      '/sales/logistics-upload',
      '/channels/shop-orders',
      '/shop-products',
      '/channels/product-match',
      '/sales/shop-product-match-query',
      '/sales/shop-inventory-sync',
    ]);
  });

  it('keeps channel menu focused on base records without shop-order collaboration', () => {
    const channelModule = appMenuItems.find((item) => item.name === '渠道');
    expect(channelModule).toBeDefined();

    const channelPaths = flattenMenuItems(channelModule?.children ?? []).map((item) => item.path);
    expect(channelPaths).not.toContain('/channels/shop-orders');
  });

  it('organizes the warehouse module by operational stage for easier scanning', () => {
    const warehouseModule = appMenuItems.find((item) => item.name === '仓储');
    expect(warehouseModule).toBeDefined();

    expect((warehouseModule?.children ?? []).map((item) => item.name)).toEqual(
      expect.arrayContaining(['仓网档案', '收货入仓', '发货出仓', '库存控制', '策略引擎']),
    );
  });
});
