import { describe, expect, it } from 'vitest';

import { appMenuItems } from './menu';
import {
  buildHeaderModules,
  resolveActiveHeaderModuleKey,
} from './headerMenuModel';

describe('headerMenuModel', () => {
  it('builds grouped header modules from the app menu tree', () => {
    const modules = buildHeaderModules(appMenuItems);
    const salesModule = modules.find((module) => module.key === 'sales');

    expect(salesModule).toBeDefined();
    expect(salesModule?.groups.map((group) => group.label)).toEqual(
      expect.arrayContaining(['订单中心', '网店订单', '履约中心', '结算规则', '业务设置']),
    );
    expect(salesModule?.leafPaths).toEqual(
      expect.arrayContaining(['/orders/review', '/orders', '/sales/settings']),
    );
  });

  it('treats direct-entry modules as navigation items without shared submenu groups', () => {
    const modules = buildHeaderModules(appMenuItems);
    const homeModule = modules.find((module) => module.key === 'dashboard');

    expect(homeModule).toBeDefined();
    expect(homeModule?.defaultPath).toBe('/dashboard');
    expect(homeModule?.groups).toEqual([]);
  });

  it('resolves the active top-level module from the current pathname', () => {
    const modules = buildHeaderModules(appMenuItems);

    expect(resolveActiveHeaderModuleKey(modules, '/orders/review')).toBe('sales');
    expect(resolveActiveHeaderModuleKey(modules, '/integrations/print-devices')).toBe(
      'integrations',
    );
    expect(resolveActiveHeaderModuleKey(modules, '/insights/revenue/overview')).toBe('insights');
    expect(resolveActiveHeaderModuleKey(modules, '/dashboard')).toBe('dashboard');
  });
});
