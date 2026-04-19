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
  it('only indexes the five preserved entry points', () => {
    const entries = buildFunctionSearchEntries(appMenuItems);

    expect(entries.map((entry) => entry.path)).toEqual([
      '/dashboard',
      '/channels',
      '/orders/review',
      '/orders/recheck',
      '/sales/settings',
    ]);
  });

  it('supports common order-review aliases and direct matching for the remaining pages', () => {
    const entries = buildFunctionSearchEntries(appMenuItems);

    expect(searchFunctionEntries(entries, '单审')[0]).toMatchObject({
      path: '/orders/review',
    });
    expect(searchFunctionEntries(entries, '复核')[0]).toMatchObject({
      path: '/orders/recheck',
    });
    expect(searchFunctionEntries(entries, '渠道')[0]).toMatchObject({
      path: '/channels',
    });
  });

  it('ignores removed pages when building recent and common recommendations', () => {
    const entries = buildFunctionSearchEntries(appMenuItems);
    const recommendations = buildFunctionRecommendations(entries, {
      recentPaths: ['/orders/review', '/orders', '/channels'],
      shortcutPaths: ['/sales/settings', '/integrations/center'],
    });

    expect(recommendations).toEqual([
      expect.objectContaining({
        path: '/orders/review',
        source: 'recent',
      }),
      expect.objectContaining({
        path: '/channels',
        source: 'recent',
      }),
      expect.objectContaining({
        path: '/sales/settings',
        source: 'shortcut',
      }),
    ]);
  });

  it('groups the reduced results by their surviving top-level modules', () => {
    const groups = groupFunctionEntriesByModule([
      {
        path: '/orders/review',
        name: '审核订单',
        moduleName: '销售',
        groupName: '订单中心',
      },
      {
        path: '/orders/recheck',
        name: '订单复核',
        moduleName: '销售',
        groupName: '订单中心',
      },
      {
        path: '/channels',
        name: '渠道目录',
        moduleName: '渠道',
      },
    ]);

    expect(groups[0]).toMatchObject({
      label: '销售',
    });
    expect(groups[0]?.items.map((item) => item.path)).toEqual(['/orders/review', '/orders/recheck']);
    expect(groups[1]).toMatchObject({
      label: '渠道',
    });
    expect(groups[1]?.items.map((item) => item.path)).toEqual(['/channels']);
  });

  it('groups recommendations by source for the reduced navigation shell', () => {
    const groups = groupFunctionRecommendationsBySource([
      {
        path: '/orders/review',
        name: '审核订单',
        moduleName: '销售',
        groupName: '订单中心',
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
        items: [expect.objectContaining({ path: '/orders/review' })],
      }),
      expect.objectContaining({
        label: '常用功能',
        items: [expect.objectContaining({ path: '/sales/settings' })],
      }),
    ]);
  });

  it('keeps recent function paths deduplicated and ordered by latest visit', () => {
    expect(
      mergeRecentFunctionPaths(['/channels', '/dashboard', '/orders/review'], '/dashboard', 3),
    ).toEqual(['/dashboard', '/channels', '/orders/review']);
    expect(
      mergeRecentFunctionPaths(['/channels', '/dashboard'], '/orders/recheck', 2),
    ).toEqual(['/orders/recheck', '/channels']);
  });
});
