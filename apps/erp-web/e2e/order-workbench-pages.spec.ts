import { expect, loginAndSelectTenant, test } from './fixtures';

test('订单查询作业台可以独立打开', async ({ page }) => {
  await page.goto('/sales/order-query');
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('sales-order-query-workbench-page')).toBeVisible();
  await expect(page.getByText('订单查询').first()).toBeVisible();
  await expect(page.getByTestId('order-review-data-grid')).toBeVisible();
});