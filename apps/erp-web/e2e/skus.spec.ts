import { expect, test, loginAndSelectTenant } from './fixtures';

test('scan barcode opens sku drawer', async ({ page }) => {
  await page.goto('/skus');

  await loginAndSelectTenant(page);

  await expect(page.getByTestId('skus-title')).toBeVisible();
  await expect(page.getByText('SKU-ERP-1000')).toBeVisible();
});
