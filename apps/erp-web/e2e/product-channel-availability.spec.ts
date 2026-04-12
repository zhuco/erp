import { expect, test, loginAndSelectTenant } from './fixtures';

test('product channel availability page renders rules and list', async ({ page }) => {
  await page.goto('/products/channel-availability');
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('product-channel-title')).toBeVisible();
  await expect(page.getByTestId('product-channel-filter-form')).toBeVisible();

  const rulesTable = page.getByTestId('product-channel-rules-table');
  await expect(rulesTable).toBeVisible();

  const listTable = page.getByTestId('product-channel-table');
  await expect(listTable).toBeVisible();
});
