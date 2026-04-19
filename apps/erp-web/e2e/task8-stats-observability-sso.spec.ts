import { expect, loginAndSelectTenant, test } from './fixtures';

test('task8 stats and observability pages show analytics and sso health', async ({ page }) => {
  await page.goto('/insights/revenue/overview');
  await loginAndSelectTenant(page);

  await expect(page).toHaveURL(/\/insights\/revenue\/overview(\?.*)?$/);
  await expect(page.getByTestId('insights-revenue-overview-page')).toBeVisible();
  await expect(page.getByTestId('stats-infra-health')).toBeVisible();
  await expect(page.getByTestId('stats-infra-health')).toContainText('\u5206\u6790\u5f15\u64ce');

  await page.goto('/stats/sales?source=legacy#summary');
  await expect(page).toHaveURL(/\/insights\/revenue\/overview\?source=legacy#summary$/);
  await expect(page.getByTestId('insights-revenue-overview-page')).toBeVisible();

  await page.goto('/insights/revenue/trend-radar');
  await expect(page.getByTestId('insights-revenue-trend-radar-page')).toBeVisible();
  await expect(page.getByTestId('stats-infra-health')).toBeVisible();

  await page.goto('/stats/shipping');
  await expect(page).toHaveURL(/\/insights\/fulfillment\/lead-time(\?.*)?$/);
  await expect(page.getByTestId('insights-fulfillment-lead-time-page')).toBeVisible();
  await expect(page.getByTestId('stats-infra-health')).toBeVisible();

  await page.goto('/reports');
  await expect(page).toHaveURL(/\/insights\/workbench\/report-center(\?.*)?$/);
  await expect(page.getByTestId('insights-workbench-report-center-page')).toBeVisible();

  await page.goto('/system/observability');
  await expect(page.getByTestId('observability-stats-infra-health')).toBeVisible();
});
