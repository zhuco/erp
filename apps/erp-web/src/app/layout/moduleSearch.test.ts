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
  it('indexes directory children together with the preserved sales workbench pages', () => {
    const entries = buildFunctionSearchEntries(appMenuItems);

    expect(entries.map((entry) => entry.path)).toEqual([
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

  it('supports direct matching for directory entries and sales workbench routes', () => {
    const entries = buildFunctionSearchEntries(appMenuItems);

    expect(searchFunctionEntries(entries, '渠道目录')[0]).toMatchObject({
      path: '/channels',
    });
    expect(searchFunctionEntries(entries, '物流目录')[0]).toMatchObject({
      path: '/couriers',
    });
    expect(searchFunctionEntries(entries, '记号目录')[0]).toMatchObject({
      path: '/tags',
    });
    expect(searchFunctionEntries(entries, '仓库目录')[0]).toMatchObject({
      path: '/warehouses',
    });
    expect(searchFunctionEntries(entries, '物流费目录')[0]).toMatchObject({
      path: '/logistics-fees',
    });
    expect(searchFunctionEntries(entries, '供应商目录')[0]).toMatchObject({
      path: '/suppliers',
    });
    expect(searchFunctionEntries(entries, '单审')[0]).toMatchObject({
      path: '/orders/review',
    });
  });

  it('ignores removed pages when building recent and common recommendations', () => {
    const entries = buildFunctionSearchEntries(appMenuItems);
    const recommendations = buildFunctionRecommendations(entries, {
      recentPaths: ['/channels', '/suppliers', '/orders'],
      shortcutPaths: ['/sales/settings', '/integrations/center'],
    });

    expect(recommendations).toEqual([
      expect.objectContaining({
        path: '/channels',
        source: 'recent',
      }),
      expect.objectContaining({
        path: '/suppliers',
        source: 'recent',
      }),
      expect.objectContaining({
        path: '/sales/settings',
        source: 'shortcut',
      }),
    ]);
  });

  it('groups directory results under the shared directory module', () => {
    const groups = groupFunctionEntriesByModule([
      {
        path: '/channels',
        name: '渠道目录',
        moduleName: '目录',
        groupName: '基础目录',
      },
      {
        path: '/couriers',
        name: '物流目录',
        moduleName: '目录',
        groupName: '基础目录',
      },
      {
        path: '/suppliers',
        name: '供应商目录',
        moduleName: '目录',
        groupName: '基础目录',
      },
      {
        path: '/orders/review',
        name: '订单审核',
        moduleName: '销售',
        groupName: '订单中心',
      },
    ]);

    expect(groups[0]).toMatchObject({
      label: '目录',
    });
    expect(groups[0]?.items.map((item) => item.path)).toEqual([
      '/channels',
      '/couriers',
      '/suppliers',
    ]);
    expect(groups[1]).toMatchObject({
      label: '销售',
    });
    expect(groups[1]?.items.map((item) => item.path)).toEqual(['/orders/review']);
  });

  it('groups recommendations by source for the upgraded navigation shell', () => {
    const groups = groupFunctionRecommendationsBySource([
      {
        path: '/channels',
        name: '渠道目录',
        moduleName: '目录',
        groupName: '基础目录',
        source: 'recent',
      },
      {
        path: '/sales/settings',
        name: '销售设置',
        moduleName: '销售',
        source: 'shortcut',
      },
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
    expect(
      mergeRecentFunctionPaths(['/channels', '/dashboard', '/sales/order-query'], '/dashboard', 3),
    ).toEqual(['/dashboard', '/channels', '/sales/order-query']);
    expect(
      mergeRecentFunctionPaths(['/channels', '/dashboard'], '/suppliers', 2),
    ).toEqual(['/suppliers', '/channels']);
  });
});
