import { describe, expect, it } from 'vitest';

import type { AppMenuItem } from '@/app/layout/menu';

import { buildDashboardQuickLinks, buildDashboardWorkbenchCards } from './dashboardWorkbenchModel';

const buildMenuItems = (paths: string[]): AppMenuItem[] =>
  paths.map((path) => ({
    path,
    name: path,
  }));

describe('dashboardWorkbenchModel', () => {
  it('根据当前可访问功能生成推荐岗位工作台，并按优先级排序', () => {
    const cards = buildDashboardWorkbenchCards(
      buildMenuItems([
        '/orders/review',
        '/orders/exceptions',
        '/orders/review-rules',
        '/shipments',
        '/wms/pack',
        '/inventory',
      ]),
    );

    expect(cards.map((card) => card.key)).toEqual(['review', 'fulfillment', 'warehouse']);
    expect(cards[0]).toMatchObject({
      title: '审单台',
      primaryAction: {
        label: '进入审单',
        path: '/orders/review',
      },
    });
  });

  it('会过滤掉当前无权访问的次级入口，只保留可达动作', () => {
    const cards = buildDashboardWorkbenchCards(buildMenuItems(['/procurement/orders']));

    expect(cards).toEqual([
      expect.objectContaining({
        key: 'procurement',
        actions: [],
        primaryAction: expect.objectContaining({
          path: '/procurement/orders',
        }),
      }),
    ]);
  });

  it('在可访问功能不足时返回空列表，避免展示错误岗位入口', () => {
    expect(buildDashboardWorkbenchCards(buildMenuItems(['/reports']))).toEqual([]);
  });

  it('快捷入口只保留当前可访问功能，并按固定优先级输出', () => {
    const links = buildDashboardQuickLinks(
      buildMenuItems(['/inventory', '/orders/review', '/system/settings', '/channels']),
    );

    expect(links).toEqual([
      { label: '/orders/review', path: '/orders/review' },
      { label: '/inventory', path: '/inventory' },
      { label: '/channels', path: '/channels' },
      { label: '/system/settings', path: '/system/settings' },
    ]);
  });
});
