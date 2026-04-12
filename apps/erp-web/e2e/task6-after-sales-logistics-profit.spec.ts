import { expect, test, loginAndSelectTenant } from './fixtures';

test('task6 after-sales logistics and profit pages are available with minimal flow', async ({
  page
}) => {
  await page.goto('/after-sales/exchanges');
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('after-sales-exchanges-title')).toBeVisible();
  await expect(page.getByTestId('after-sales-exchanges-table')).toBeVisible();

  await page.goto('/logistics/exceptions');
  await expect(page.getByTestId('logistics-exceptions-page')).toBeVisible();
  await expect(page.locator('.ant-table-tbody tr').first()).toBeVisible();

  await page.goto('/finance/profit');
  await expect(page.getByLabel('SKU')).toBeVisible();
  await expect(page.getByTestId('finance-profit-grid')).toBeVisible();
  await expect(page.getByText('TB-PLAT-10001')).toBeVisible();
});
