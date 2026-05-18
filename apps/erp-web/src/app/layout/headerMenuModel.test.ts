import { describe, expect, it } from 'vitest';

import { appMenuItems } from './menu';
import { buildHeaderModules, resolveActiveHeaderModuleKey } from './headerMenuModel';

describe('headerMenuModel', () => {
  it('builds top-level modules and includes the new goods module', () => {
    const modules = buildHeaderModules(appMenuItems);
    const goodsModule = modules.find((module) => module.defaultPath === '/goods/products/new');

    expect(modules.map((module) => module.defaultPath)).toEqual(
      expect.arrayContaining(['/dashboard', '/channels', '/sales/order-query', '/goods/products/new']),
    );
    expect(goodsModule?.label).toBe('货品');
    expect(goodsModule?.leafPaths).toEqual([
      '/goods/products/new',
      '/goods/products/query',
      '/goods/kits/new',
      '/goods/kits/query',
    ]);
  });

  it('keeps the goods module grouped into product and kit sections', () => {
    const modules = buildHeaderModules(appMenuItems);
    const goodsModule = modules.find((module) => module.defaultPath === '/goods/products/new');

    expect(goodsModule?.groups).toHaveLength(2);
    expect(goodsModule?.groups?.map((group) => group.label)).toEqual(['货品', '组合装']);
    expect(goodsModule?.groups[0]).toMatchObject({
      label: '货品',
    });
    expect(goodsModule?.groups[0]?.items.map((item) => item.path)).toEqual([
      '/goods/products/new',
      '/goods/products/query',
    ]);
    expect(goodsModule?.groups[1]?.items.map((item) => item.path)).toEqual([
      '/goods/kits/new',
      '/goods/kits/query',
    ]);
  });

  it('resolves grouped directory and goods children to their shared top module', () => {
    const modules = buildHeaderModules(appMenuItems);
    const salesModule = modules.find((module) => module.defaultPath === '/sales/order-query');
    const goodsModule = modules.find((module) => module.defaultPath === '/goods/products/new');

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
    expect(resolveActiveHeaderModuleKey(modules, '/goods/products/new')).toBe(goodsModule?.key);
    expect(resolveActiveHeaderModuleKey(modules, '/goods/products/query')).toBe(goodsModule?.key);
    expect(resolveActiveHeaderModuleKey(modules, '/goods/products/product-1/edit')).toBe(goodsModule?.key);
    expect(resolveActiveHeaderModuleKey(modules, '/goods/kits/new')).toBe(goodsModule?.key);
    expect(resolveActiveHeaderModuleKey(modules, '/goods/kits/query')).toBe(goodsModule?.key);
    expect(resolveActiveHeaderModuleKey(modules, '/goods/kits/kit-1/edit')).toBe(goodsModule?.key);
    expect(resolveActiveHeaderModuleKey(modules, '/dashboard')).toBe('dashboard');
  });
});
