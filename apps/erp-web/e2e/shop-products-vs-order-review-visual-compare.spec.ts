import fs from 'node:fs';
import path from 'node:path';

import { expect, loginAndSelectTenant, test } from './fixtures';

const OUTPUT_DIR = path.resolve(process.cwd(), 'output/playwright');
const SHOP_PRODUCTS_SHOT = path.join(OUTPUT_DIR, 'shop-products-workbench.png');
const ORDER_REVIEW_SHOT = path.join(OUTPUT_DIR, 'order-review-workbench-latest.png');

const ensureOutputDir = () => {
  fs.mkdirSync(OUTPUT_DIR, { recursive: true });
};

test.describe('shop-products vs order-review visual compare', () => {
  test('captures aligned workbench screenshots for visual comparison', async ({ page }) => {
    ensureOutputDir();
    await page.setViewportSize({ width: 1440, height: 900 });

    await page.goto('/shop-products');
    await loginAndSelectTenant(page);
    await expect(page.getByTestId('shop-products-grid')).toBeVisible();
    await page.waitForTimeout(800);
    await page.screenshot({
      path: SHOP_PRODUCTS_SHOT,
      fullPage: true,
    });

    await page.goto('/orders/review');
    await expect(page.getByTestId('order-review-data-grid')).toBeVisible();
    await page.waitForTimeout(800);
    await page.screenshot({
      path: ORDER_REVIEW_SHOT,
      fullPage: true,
    });
  });
});

