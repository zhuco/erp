import { describe, expect, it } from 'vitest';

import {
  buildGoodsCompactWorkbenchPath,
  buildGoodsKitCreatePath,
  buildGoodsKitEditPath,
  buildGoodsProductCreatePath,
  buildGoodsProductEditPath,
  buildGoodsQueryPath,
  createGoodsEditSearchParams,
} from './goodsRoutes';

describe('goodsRoutes', () => {
  it('builds product edit routes with preserved query context and compact workbench defaults', () => {
    expect(
      buildGoodsProductEditPath(
        'product-1',
        '?keyword=abc&page=2&pageSize=50&rightCollapsed=0&bottomCollapsed=0',
      ),
    ).toBe(
      '/goods/products/product-1/edit?keyword=abc&page=2&pageSize=50&rightCollapsed=1&bottomCollapsed=1',
    );
  });

  it('builds kit edit routes with active selection retention and compact defaults', () => {
    expect(buildGoodsKitEditPath('kit-1', '?keyword=kit&activeId=kit-1')).toBe(
      '/goods/kits/kit-1/edit?keyword=kit&activeId=kit-1&rightCollapsed=1&bottomCollapsed=1',
    );
  });

  it('builds product create routes with preserved query context and compact defaults', () => {
    expect(
      buildGoodsProductCreatePath(
        '?keyword=abc&page=2&pageSize=50&rightCollapsed=0&bottomCollapsed=0',
      ),
    ).toBe(
      '/goods/products/new?keyword=abc&page=2&pageSize=50&rightCollapsed=1&bottomCollapsed=1',
    );
  });

  it('builds kit create routes with preserved query context and compact defaults', () => {
    expect(buildGoodsKitCreatePath('?keyword=kit&activeId=kit-1')).toBe(
      '/goods/kits/new?keyword=kit&activeId=kit-1&rightCollapsed=1&bottomCollapsed=1',
    );
  });

  it('removes legacy edit markers before redirecting to the new edit route', () => {
    expect(
      createGoodsEditSearchParams('?keyword=kit&edit=KIT-001&page=3'),
    ).toBe('?keyword=kit&page=3&rightCollapsed=1&bottomCollapsed=1');
  });

  it('builds compact workbench defaults when create or edit pages are opened without layout query params', () => {
    expect(buildGoodsCompactWorkbenchPath('/goods/products/new', '')).toBe(
      '/goods/products/new?rightCollapsed=1&bottomCollapsed=1',
    );
  });

  it('only fills missing compact layout params instead of overriding explicit user layout state', () => {
    expect(
      buildGoodsCompactWorkbenchPath(
        '/goods/kits/SKU-1/edit',
        '?keyword=kit&rightCollapsed=0',
      ),
    ).toBe('/goods/kits/SKU-1/edit?keyword=kit&rightCollapsed=0&bottomCollapsed=1');
  });

  it('builds query return routes without mutating preserved query context', () => {
    expect(
      buildGoodsQueryPath('/goods/products/query', '?keyword=abc&activeId=product-1'),
    ).toBe('/goods/products/query?keyword=abc&activeId=product-1&rightCollapsed=0&bottomCollapsed=0');
  });

  it('removes edit workbench compact layout params when returning to query pages', () => {
    expect(
      buildGoodsQueryPath(
        '/goods/kits/query',
        '?keyword=kit&activeId=kit-1&rightCollapsed=1&bottomCollapsed=1',
      ),
    ).toBe('/goods/kits/query?keyword=kit&activeId=kit-1&rightCollapsed=0&bottomCollapsed=0');
  });
});
