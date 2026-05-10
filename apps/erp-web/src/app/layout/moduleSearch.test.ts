import { describe, expect, it } from 'vitest';

import { appMenuItems } from './menu';
import {
  buildFunctionRecommendations,
  buildFunctionSearchEntries,
  groupFunctionEntriesByModule,
  groupFunctionRecommendationsBySource,
  mergeRecentFunctionPaths,
  searchFunctionEntries,
} from './moduleSearch';

describe('moduleSearch', () => {
  it('indexes expanded directory pages together with preserved sales workbench pages', () => {
    const entries = buildFunctionSearchEntries(appMenuItems);

    expect(entries.map((entry) => entry.path)).toEqual([
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

  it('supports direct matching for expanded directory entries and order review', () => {
    const entries = buildFunctionSearchEntries(appMenuItems);

    expect(searchFunctionEntries(entries, '渠道目录')[0]).toMatchObject({ path: '/channels' });
    expect(searchFunctionEntries(entries, '物流目录')[0]).toMatchObject({ path: '/couriers' });
    expect(searchFunctionEntries(entries, '标签目录')[0]).toMatchObject({ path: '/tags' });
    expect(searchFunctionEntries(entries, '仓库目录')[0]).toMatchObject({ path: '/warehouses' });
    expect(searchFunctionEntries(entries, '物流费目录')[0]).toMatchObject({ path: '/logistics-fees' });
    expect(searchFunctionEntries(entries, '供应商目录')[0]).toMatchObject({ path: '/suppliers' });
    expect(searchFunctionEntries(entries, '客户分类目录')[0]).toMatchObject({ path: '/customers/categories' });
    expect(searchFunctionEntries(entries, '会员等级目录')[0]).toMatchObject({ path: '/customers/member-levels' });
    expect(searchFunctionEntries(entries, '客户标签目录')[0]).toMatchObject({ path: '/customers/tags' });
    expect(searchFunctionEntries(entries, '容器目录')[0]).toMatchObject({ path: '/wms/containers' });
    expect(searchFunctionEntries(entries, '寄方目录')[0]).toMatchObject({ path: '/wms/sender-addresses' });
    expect(searchFunctionEntries(entries, '库区目录')[0]).toMatchObject({ path: '/wms/warehouse-areas' });
    expect(searchFunctionEntries(entries, '货位目录')[0]).toMatchObject({ path: '/wms/warehouse-locations' });
    expect(searchFunctionEntries(entries, '权限目录')[0]).toMatchObject({ path: '/system/permissions' });
    expect(searchFunctionEntries(entries, '角色目录')[0]).toMatchObject({ path: '/system/roles' });
    expect(searchFunctionEntries(entries, '订单审核')[0]).toMatchObject({ path: '/orders/review' });
  });

  it('ignores removed pages when building recent and common recommendations', () => {
    const entries = buildFunctionSearchEntries(appMenuItems);
    const recommendations = buildFunctionRecommendations(entries, {
      recentPaths: ['/channels', '/suppliers', '/orders'],
      shortcutPaths: ['/sales/settings', '/integrations/center'],
    });

    expect(recommendations).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ path: '/channels', source: 'recent' }),
        expect.objectContaining({ path: '/suppliers', source: 'recent' }),
        expect.objectContaining({ path: '/sales/settings', source: 'shortcut' }),
        expect.objectContaining({ path: '/integrations/center', source: 'shortcut' }),
      ]),
    );
  });

  it('groups directory search results under the shared directory module', () => {
    const groups = groupFunctionEntriesByModule([
      { path: '/channels', name: '渠道目录', moduleName: '目录', groupName: '基础目录' },
      { path: '/couriers', name: '物流目录', moduleName: '目录', groupName: '基础目录' },
      { path: '/suppliers', name: '供应商目录', moduleName: '目录', groupName: '基础目录' },
      { path: '/customers/categories', name: '客户分类目录', moduleName: '目录', groupName: '客户目录' },
      { path: '/customers/member-levels', name: '会员等级目录', moduleName: '目录', groupName: '客户目录' },
      { path: '/customers/tags', name: '客户标签目录', moduleName: '目录', groupName: '客户目录' },
      { path: '/wms/containers', name: '容器目录', moduleName: '目录', groupName: 'WMS 主数据' },
      { path: '/system/permissions', name: '权限目录', moduleName: '目录', groupName: '系统目录' },
      { path: '/orders/review', name: '订单审核', moduleName: '销售', groupName: '订单中心' },
    ]);

    expect(groups[0]).toMatchObject({ label: '目录' });
    expect(groups[0]?.items.map((item) => item.path)).toEqual([
      '/channels',
      '/couriers',
      '/suppliers',
      '/customers/categories',
      '/customers/member-levels',
      '/customers/tags',
      '/wms/containers',
      '/system/permissions',
    ]);
    expect(groups[1]).toMatchObject({ label: '销售' });
    expect(groups[1]?.items.map((item) => item.path)).toEqual(['/orders/review']);
  });

  it('groups recommendations by source for the upgraded navigation shell', () => {
    const groups = groupFunctionRecommendationsBySource([
      { path: '/channels', name: '渠道目录', moduleName: '目录', groupName: '基础目录', source: 'recent' },
      { path: '/sales/settings', name: '销售设置', moduleName: '销售', source: 'shortcut' },
    ]);

    expect(groups).toEqual([
      expect.objectContaining({
        label: '最近使用',
        items: [expect.objectContaining({ path: '/channels' })],
      }),
      expect.objectContaining({
        label: '常用功能',
        items: [expect.objectContaining({ path: '/sales/settings' })],
      }),
    ]);
  });

  it('keeps recent function paths deduplicated and ordered by latest visit', () => {
    expect(mergeRecentFunctionPaths(['/channels', '/dashboard', '/sales/order-query'], '/dashboard', 3)).toEqual([
      '/dashboard',
      '/channels',
      '/sales/order-query',
    ]);
    expect(mergeRecentFunctionPaths(['/channels', '/dashboard'], '/suppliers', 2)).toEqual([
      '/suppliers',
      '/channels',
    ]);
  });
});
