import { describe, expect, it } from 'vitest';

import { appMenuItems } from './menu';
import { buildHeaderModules, resolveActiveHeaderModuleKey } from './headerMenuModel';

describe('headerMenuModel', () => {
  it('builds only the surviving top-level modules from the reduced menu tree', () => {
    const modules = buildHeaderModules(appMenuItems);
    const salesModule = modules.find((module) => module.defaultPath === '/sales/order-query');

    expect(modules.map((module) => module.defaultPath)).toEqual([
      '/dashboard',
      '/channels',
      '/sales/order-query',
    ]);
    expect(salesModule?.leafPaths).toEqual([
      '/sales/order-query',
      '/orders/review',
      '/orders/recheck',
      '/sales/settings',
    ]);
  });

  it('treats direct-entry modules as standalone navigation items', () => {
    const modules = buildHeaderModules(appMenuItems);
    const homeModule = modules.find((module) => module.defaultPath === '/dashboard');
    const channelModule = modules.find((module) => module.defaultPath === '/channels');

    expect(homeModule?.groups).toEqual([]);
    expect(channelModule?.groups).toEqual([]);
  });

  it('resolves active modules from the preserved routes only', () => {
    const modules = buildHeaderModules(appMenuItems);
    const salesModule = modules.find((module) => module.defaultPath === '/sales/order-query');

    expect(resolveActiveHeaderModuleKey(modules, '/sales/order-query')).toBe(salesModule?.key);
    expect(resolveActiveHeaderModuleKey(modules, '/orders/review')).toBe(salesModule?.key);
    expect(resolveActiveHeaderModuleKey(modules, '/orders/recheck')).toBe(salesModule?.key);
    expect(resolveActiveHeaderModuleKey(modules, '/sales/settings')).toBe(salesModule?.key);
    expect(resolveActiveHeaderModuleKey(modules, '/channels')).toBe('channels');
    expect(resolveActiveHeaderModuleKey(modules, '/dashboard')).toBe('dashboard');
  });
});
