import { expect, loginAndSelectTenant, test } from './fixtures';

test('channels workbench can open shop access page and display shop list', async ({ page }) => {
  await page.goto('/channels');
  await loginAndSelectTenant(page);

  const directoryGrid = page.getByTestId('channels-workbench-grid');
  await expect(directoryGrid).toBeVisible();
  await expect(page.getByTestId('sales-shop-config-grid')).toHaveCount(0);

  await expect(page.getByTestId('channels-open-shops')).toBeVisible();
  await page.getByTestId('channels-open-shops').click();

  await expect(page).toHaveURL(/\/channels\/shops(\?.*)?$/);
  const salesGrid = page.getByTestId('sales-shop-config-grid');
  await expect(salesGrid).toBeVisible();
  await expect(page.getByTestId('channels-workbench-grid')).not.toBeVisible();

  const rows = salesGrid.locator('.ag-center-cols-container .ag-row');
  await expect(rows.first()).toBeVisible();

  await page.screenshot({
    path: 'test-results/channels-shops-list-visibility.png',
    fullPage: true
  });
});
