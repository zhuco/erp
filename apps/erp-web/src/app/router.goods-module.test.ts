import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('router goods module contract', () => {
  it('keeps goods create and query pages inside the workbench shell', () => {
    const source = readFileSync(resolve(__dirname, './router.tsx'), 'utf8');

    expect(source).toContain("path: 'goods/products/query'");
    expect(source).toContain("path: 'goods/kits/query'");
    expect(source).toContain("path: 'goods/products/new'");
    expect(source).toContain("path: 'goods/kits/new'");
    expect(source).toContain("path: 'goods/products/:productId/edit'");
    expect(source).toContain("path: 'goods/kits/:kitSkuId/edit'");
    expect(source).not.toContain("path: '/goods/products/new'");
    expect(source).not.toContain("path: '/goods/kits/new'");
  });

  it('registers legacy product and kit redirects through the shared redirect component', () => {
    const source = readFileSync(resolve(__dirname, './router.tsx'), 'utf8');

    expect(source).toContain('const legacyRouteRedirects = routeAliases.map');
    expect(source).toContain('<LegacyRouteRedirect />');
    expect(source).toContain("path: '/kits/:kit_sku_id'");
  });
});
