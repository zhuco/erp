import { describe, expect, it } from 'vitest';

import { flattenMenuItems, appMenuItems } from './menu';
import { isReducedUiPathAllowed } from './WorkbenchLayout';

describe('WorkbenchLayout reduced UI shell', () => {
  it('allows core shell routes and canonical edit routes', () => {
    expect(isReducedUiPathAllowed('/')).toBe(true);
    expect(isReducedUiPathAllowed('/dashboard')).toBe(true);
    expect(isReducedUiPathAllowed('/channels')).toBe(true);
    expect(isReducedUiPathAllowed('/sales/order-query')).toBe(true);
    expect(isReducedUiPathAllowed('/orders/review')).toBe(true);
    expect(isReducedUiPathAllowed('/orders/recheck')).toBe(true);
    expect(isReducedUiPathAllowed('/goods/products/product-1/edit')).toBe(true);
    expect(isReducedUiPathAllowed('/goods/kits/kit-1/edit')).toBe(true);

    expect(isReducedUiPathAllowed('/orders')).toBe(false);
    expect(isReducedUiPathAllowed('/orders/exceptions')).toBe(false);
    expect(isReducedUiPathAllowed('/channels/shops')).toBe(false);
    expect(isReducedUiPathAllowed('/products')).toBe(false);
  });

  it('allows every menu leaf route to render inside the workbench shell', () => {
    const menuPaths = flattenMenuItems(appMenuItems).map((item) => item.path);
    expect(menuPaths.length).toBeGreaterThan(20);
    expect(menuPaths.filter((path) => !isReducedUiPathAllowed(path))).toEqual([]);
  });

});
