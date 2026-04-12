import { expect, test, loginAndSelectTenant } from './fixtures';

test('release gate smoke covers integration, inventory, after-sales and profit chains', async ({
  page
}) => {
  await page.goto('/system/integrations');
  await loginAndSelectTenant(page);
  await expect(page.getByTestId('system-integrations-overview-section')).toBeVisible();

  await page.goto('/inventory');
  await expect(page.getByTestId('inventory-summary-title')).toBeVisible();
  await expect(page.getByTestId('inventory-summary-table')).toBeVisible();

  await page.goto('/after-sales/exchanges');
  await expect(page.getByTestId('after-sales-exchanges-title')).toBeVisible();

  await page.goto('/logistics/exceptions');
  await expect(page.getByTestId('logistics-exceptions-page')).toBeVisible();

  await page.goto('/finance/profit');
  await expect(page.getByTestId('finance-profit-grid')).toBeVisible();
});
