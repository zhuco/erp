import { expect, test, loginAndSelectTenant } from './fixtures';

test('release gate cross-chain smoke covers oms wms after-sales planning and observability', async ({
  page
}) => {
  await page.goto('/system/integrations');
  await loginAndSelectTenant(page);
  await expect(page.getByTestId('system-integrations-overview-section')).toBeVisible();

  await page.goto('/orders/review');
  await expect(page.getByTestId('order-review-search')).toBeVisible();

  await page.goto('/wms/ship');
  await expect(page.getByTestId('ship-title')).toBeVisible();
  await expect(page.getByTestId('ship-table')).toBeVisible();

  await page.goto('/after-sales/exchanges');
  await expect(page.getByTestId('after-sales-exchanges-title')).toBeVisible();

  await page.goto('/planning/plan-execution');
  await expect(page.getByTestId('planning-execution-page')).toBeVisible();
  await expect(page.getByTestId('planning-execution-table')).toBeVisible();

  await page.goto('/stats/sales');
  await expect(page.getByTestId('stats-sales-page')).toBeVisible();
  await expect(page.getByTestId('stats-infra-health')).toBeVisible();

  await page.goto('/system/observability');
  await expect(page.getByTestId('observability-stats-infra-health')).toBeVisible();
});
