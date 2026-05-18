import { describe, expect, it } from 'vitest';

import { resolveRouteTitle } from './routeMeta';

describe('routeMeta', () => {
  const menuTitleMap = new Map<string, string>([
    ['/goods/products/query', '货品查询'],
    ['/goods/kits/query', '组合装查询'],
  ]);

  it('resolves goods workbench edit pages to dedicated dynamic titles', () => {
    expect(resolveRouteTitle('/goods/products/product-1/edit', menuTitleMap)).toBe('货品修改');
    expect(resolveRouteTitle('/goods/kits/kit-1/edit', menuTitleMap)).toBe('组合装修改');
  });
});
