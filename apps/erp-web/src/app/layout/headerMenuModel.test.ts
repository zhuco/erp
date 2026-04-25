import { describe, expect, it } from 'vitest';

import { appMenuItems } from './menu';
import { buildHeaderModules, resolveActiveHeaderModuleKey } from './headerMenuModel';

describe('headerMenuModel', () => {
  it('builds top-level modules for home, directory, and sales', () => {
    const modules = buildHeaderModules(appMenuItems);
    const directoryModule = modules.find((module) => module.defaultPath === '/channels');
    const salesModule = modules.find((module) => module.defaultPath === '/sales/order-query');

    expect(modules.map((module) => module.defaultPath)).toEqual([
      '/dashboard',
      '/channels',
      '/sales/order-query',
    ]);
    expect(directoryModule?.leafPaths).toEqual([
      '/channels',
      '/couriers',
      '/tags',
      '/warehouses',
      '/logistics-fees',
      '/suppliers',
    ]);
    expect(salesModule?.leafPaths).toEqual([
      '/sales/order-query',
      '/orders/review',
      '/orders/recheck',
      '/sales/settings',
    ]);
  });

  it('treats the directory module as a grouped navigation module instead of a direct-entry page', () => {
    const modules = buildHeaderModules(appMenuItems);
    const homeModule = modules.find((module) => module.defaultPath === '/dashboard');
    const directoryModule = modules.find((module) => module.defaultPath === '/channels');

    expect(homeModule?.groups).toEqual([]);
    expect(directoryModule?.groups).toHaveLength(1);
    expect(directoryModule?.groups[0]).toMatchObject({
      label: '基础目录',
    });
    expect(directoryModule?.groups[0]?.items.map((item) => item.path)).toEqual([
      '/channels',
      '/couriers',
      '/tags',
      '/warehouses',
      '/logistics-fees',
      '/suppliers',
    ]);
  });

  it('resolves all directory children to the shared directory top module', () => {
    const modules = buildHeaderModules(appMenuItems);
    const salesModule = modules.find((module) => module.defaultPath === '/sales/order-query');

    expect(resolveActiveHeaderModuleKey(modules, '/channels')).toBe('channels');
    expect(resolveActiveHeaderModuleKey(modules, '/couriers')).toBe('channels');
    expect(resolveActiveHeaderModuleKey(modules, '/tags')).toBe('channels');
    expect(resolveActiveHeaderModuleKey(modules, '/warehouses')).toBe('channels');
    expect(resolveActiveHeaderModuleKey(modules, '/logistics-fees')).toBe('channels');
    expect(resolveActiveHeaderModuleKey(modules, '/suppliers')).toBe('channels');
    expect(resolveActiveHeaderModuleKey(modules, '/sales/order-query')).toBe(salesModule?.key);
    expect(resolveActiveHeaderModuleKey(modules, '/orders/review')).toBe(salesModule?.key);
    expect(resolveActiveHeaderModuleKey(modules, '/orders/recheck')).toBe(salesModule?.key);
    expect(resolveActiveHeaderModuleKey(modules, '/sales/settings')).toBe(salesModule?.key);
    expect(resolveActiveHeaderModuleKey(modules, '/dashboard')).toBe('dashboard');
  });
});
