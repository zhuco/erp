import { describe, expect, it } from 'vitest';

import { buildWorkbenchWarmupPaths } from './workbenchWarmup';

describe('workbenchWarmup', () => {
  it('优先预热当前模块当前分组的兄弟功能，再补充推荐功能', () => {
    const paths = buildWorkbenchWarmupPaths({
      currentPath: '/orders/review',
      activeModuleKey: 'sales',
      modules: [
        {
          key: 'sales',
          defaultPath: '/orders',
          leafPaths: ['/orders', '/sales/new-order', '/orders/review', '/orders/exceptions'],
          groups: [
            {
              key: 'orders',
              items: [
                { path: '/orders' },
                { path: '/sales/new-order' },
                { path: '/orders/review' },
                { path: '/orders/exceptions' },
              ],
            },
          ],
        },
        {
          key: 'system',
          defaultPath: '/system/observability',
          leafPaths: ['/system/observability'],
          groups: [{ key: 'ops', items: [{ path: '/system/observability' }] }],
        },
      ],
      recommendedPaths: ['/orders/review', '/system/observability', '/orders'],
      moduleLimit: 3,
      limit: 4,
    });

    expect(paths).toEqual([
      '/orders',
      '/sales/new-order',
      '/orders/exceptions',
      '/system/observability',
    ]);
  });

  it('当前模块没有可预热兄弟页时回退到推荐功能', () => {
    const paths = buildWorkbenchWarmupPaths({
      currentPath: '/dashboard',
      activeModuleKey: 'dashboard',
      modules: [
        {
          key: 'dashboard',
          defaultPath: '/dashboard',
          leafPaths: ['/dashboard'],
          groups: [],
        },
      ],
      recommendedPaths: ['/orders/review', '/system/observability'],
      limit: 3,
    });

    expect(paths).toEqual(['/orders/review', '/system/observability']);
  });

  it('会去重并跳过当前页面，且遵守总量限制', () => {
    const paths = buildWorkbenchWarmupPaths({
      currentPath: '/orders',
      activeModuleKey: 'sales',
      modules: [
        {
          key: 'sales',
          defaultPath: '/orders',
          leafPaths: ['/orders', '/orders/review', '/orders/exceptions', '/shipments'],
          groups: [
            {
              key: 'orders',
              items: [{ path: '/orders' }, { path: '/orders/review' }],
            },
          ],
        },
      ],
      recommendedPaths: [
        '/orders/review',
        '/orders/exceptions',
        '/shipments',
        '/system/observability',
      ],
      moduleLimit: 2,
      limit: 3,
    });

    expect(paths).toEqual(['/orders/review', '/orders/exceptions', '/shipments']);
  });
});
