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
  it('builds searchable function entries with top-level module labels', () => {
    const entries = buildFunctionSearchEntries(appMenuItems);
    const orderReviewEntry = entries.find((entry) => entry.path === '/orders/review');

    expect(orderReviewEntry).toBeDefined();
    expect(orderReviewEntry).toMatchObject({
      path: '/orders/review',
      name: '\u8ba2\u5355\u5ba1\u6838',
      moduleName: '\u9500\u552e',
      groupName: '\u8ba2\u5355\u4e2d\u5fc3',
    });
  });

  it('supports fuzzy Chinese keyword matching for function lookup', () => {
    const entries = buildFunctionSearchEntries(appMenuItems);
    const results = searchFunctionEntries(entries, '\u5355\u5ba1');

    expect(results[0]).toMatchObject({
      path: '/orders/review',
      name: '\u8ba2\u5355\u5ba1\u6838',
    });
  });

  it('matches module names and paths when searching', () => {
    const entries = buildFunctionSearchEntries(appMenuItems);

    expect(searchFunctionEntries(entries, '\u96c6\u6210').map((entry) => entry.path)).toEqual(
      expect.arrayContaining(['/integrations/center', '/integrations/sync-jobs']),
    );
    expect(searchFunctionEntries(entries, 'print').map((entry) => entry.path)).toEqual(
      expect.arrayContaining(['/integrations/print-devices', '/print-templates']),
    );
  });

  it('matches grouped business categories when searching', () => {
    const entries = buildFunctionSearchEntries(appMenuItems);

    expect(searchFunctionEntries(entries, '\u8ba2\u5355\u4e2d\u5fc3').map((entry) => entry.path)).toEqual(
      expect.arrayContaining(['/orders', '/orders/review', '/orders/exceptions']),
    );
  });

  it('builds recent and common recommendations with recent items first', () => {
    const entries = buildFunctionSearchEntries(appMenuItems);
    const recommendations = buildFunctionRecommendations(entries, {
      recentPaths: ['/orders/review', '/orders', '/not-found'],
      shortcutPaths: ['/orders/review', '/integrations/center'],
    });

    expect(recommendations).toEqual([
      expect.objectContaining({
        path: '/orders/review',
        source: 'recent',
      }),
      expect.objectContaining({
        path: '/orders',
        source: 'recent',
      }),
      expect.objectContaining({
        path: '/integrations/center',
        source: 'shortcut',
      }),
    ]);
  });

  it('groups matched functions by module to keep search results clustered by business domain', () => {
    const groups = groupFunctionEntriesByModule([
      {
        path: '/orders/review',
        name: '\u8ba2\u5355\u5ba1\u6838',
        moduleName: '\u9500\u552e',
        groupName: '\u8ba2\u5355\u4e2d\u5fc3',
      },
      {
        path: '/orders',
        name: '\u8ba2\u5355\u67e5\u8be2',
        moduleName: '\u9500\u552e',
        groupName: '\u8ba2\u5355\u4e2d\u5fc3',
      },
      {
        path: '/channels/directory',
        name: '\u6e20\u9053\u76ee\u5f55',
        moduleName: '\u6e20\u9053',
        groupName: '\u5e73\u53f0\u63a5\u5165',
      },
    ]);

    expect(groups).toEqual([
      expect.objectContaining({
        label: '\u9500\u552e',
        items: [
          expect.objectContaining({ path: '/orders/review' }),
          expect.objectContaining({ path: '/orders' }),
        ],
      }),
      expect.objectContaining({
        label: '\u6e20\u9053',
        items: [expect.objectContaining({ path: '/channels/directory' })],
      }),
    ]);
  });

  it('groups recommendations by source so the dropdown can render recent and common sections', () => {
    const groups = groupFunctionRecommendationsBySource([
      {
        path: '/orders/review',
        name: '\u8ba2\u5355\u5ba1\u6838',
        moduleName: '\u9500\u552e',
        groupName: '\u8ba2\u5355\u4e2d\u5fc3',
        source: 'recent',
      },
      {
        path: '/integrations/center',
        name: '\u96c6\u6210\u4e2d\u5fc3',
        moduleName: '\u96c6\u6210',
        source: 'shortcut',
      },
    ]);

    expect(groups).toEqual([
      expect.objectContaining({
        label: '\u6700\u8fd1\u4f7f\u7528',
        items: [expect.objectContaining({ path: '/orders/review' })],
      }),
      expect.objectContaining({
        label: '\u5e38\u7528\u529f\u80fd',
        items: [expect.objectContaining({ path: '/integrations/center' })],
      }),
    ]);
  });

  it('keeps recent function paths deduplicated and ordered by latest visit', () => {
    expect(
      mergeRecentFunctionPaths(['/orders', '/dashboard', '/orders/review'], '/dashboard', 3),
    ).toEqual(['/dashboard', '/orders', '/orders/review']);
    expect(mergeRecentFunctionPaths(['/orders', '/dashboard'], '/orders/review', 2)).toEqual([
      '/orders/review',
      '/orders',
    ]);
  });
});
