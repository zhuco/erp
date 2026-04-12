import { expect, test, loginAndSelectTenant } from './fixtures';

test('task8 stats and observability pages show analytics and sso health', async ({ page }) => {
  await page.goto('/stats/sales');
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('stats-sales-page')).toBeVisible();
  await expect(page.getByTestId('stats-infra-health')).toBeVisible();
  await expect(page.getByTestId('stats-infra-health')).toContainText('分析引擎');

  await page.goto('/stats/sales-trends');
  await expect(page.getByTestId('stats-sales-trends-page')).toBeVisible();
  await expect(page.getByTestId('stats-infra-health')).toBeVisible();

  await page.goto('/stats/shipping');
  await expect(page.getByTestId('stats-shipping-page')).toBeVisible();
  await expect(page.getByTestId('stats-infra-health')).toBeVisible();

  await page.goto('/system/observability');
  await expect(page.getByTestId('observability-stats-infra-health')).toBeVisible();
});
