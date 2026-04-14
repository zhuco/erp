import { expect, loginAndSelectTenant, test } from './fixtures';

test('销售对账作业台可以独立打开', async ({ page }) => {
  await page.goto('/orders/reconciliations');
  await loginAndSelectTenant(page);

  await expect(page.getByRole('heading', { name: '销售对账' })).toBeVisible();
  await expect(page.locator('main').getByRole('tab', { name: '销售对账' })).toBeVisible();
});
