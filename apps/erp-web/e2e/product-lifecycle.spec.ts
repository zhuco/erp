import { expect, test, loginAndSelectTenant } from './fixtures';

test('product lifecycle page shows steps and list', async ({ page }) => {
  await page.goto('/products/lifecycle');
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('product-lifecycle-title')).toBeVisible();
  await expect(page.getByTestId('product-lifecycle-steps')).toBeVisible();
  await expect(page.getByTestId('product-lifecycle-filter-form')).toBeVisible();

  const listTable = page.getByTestId('product-lifecycle-table');
  await expect(listTable).toBeVisible();
});
