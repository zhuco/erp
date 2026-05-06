import { describe, expect, it } from 'vitest';

import {
  appRouteRegistry,
  findP0PlaceholderRoutes,
  findUnregisteredMenuPaths,
  p0RouteRegistrations,
} from './routeRegistry';

describe('routeRegistry', () => {
  it('确保菜单叶子路径都已登记到路由注册表', () => {
    expect(findUnregisteredMenuPaths()).toEqual([]);
  });

  it('确保 P0 路由不使用占位页', () => {
    expect(findP0PlaceholderRoutes()).toEqual([]);
  });

  it('为当前试点入口保留中文标题和成熟度', () => {
    expect(p0RouteRegistrations.map((route) => route.path)).toEqual(
      expect.arrayContaining([
        '/dashboard',
        '/channels',
        '/sales/order-query',
        '/orders/review',
        '/orders/recheck',
        '/sales/settings',
      ]),
    );
    expect(appRouteRegistry.every((route) => route.title.length > 0)).toBe(true);
  });
});

