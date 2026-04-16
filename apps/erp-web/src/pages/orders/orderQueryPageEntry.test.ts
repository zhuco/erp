import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const currentFilePath = fileURLToPath(import.meta.url);
const ordersDir = path.dirname(currentFilePath);
const routerFilePath = path.resolve(ordersDir, '../../app/router.tsx');

describe('order query page entry', () => {
  it('uses OrderQueryPage as the /orders route page', () => {
    const routerSource = readFileSync(routerFilePath, 'utf8');

    expect(routerSource).toContain(
      "const OrderQueryPage = lazyNamedPage(() => import('@/pages/orders/OrderQueryPage'), 'OrderQueryPage');",
    );
    expect(routerSource).toContain('<OrderQueryPage />');
  });

  it('exports OrderQueryPage from the orders page directory and removes OrdersPage', async () => {
    const pageFilePath = path.resolve(ordersDir, 'OrderQueryPage.tsx');
    const pageSource = readFileSync(pageFilePath, 'utf8');

    expect(existsSync(pageFilePath)).toBe(true);
    expect(existsSync(path.resolve(ordersDir, 'OrdersPage.tsx'))).toBe(false);
    expect(pageSource).toContain('export const OrderQueryPage = (');
  });
});
