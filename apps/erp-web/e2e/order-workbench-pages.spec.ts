import { expect, loginAndSelectTenant, test } from './fixtures';

test('销售对账作业台可以独立打开', async ({ page }) => {
  await page.goto('/orders/reconciliations');
  await loginAndSelectTenant(page);

  await expect(page.getByRole('heading', { name: '销售对账' })).toBeVisible();
  await expect(page.locator('main').getByRole('tab', { name: '销售对账' })).toBeVisible();
});

test('异常 SLA 作业台可以独立打开', async ({ page }) => {
  await page.goto('/orders/exception-sla');
  await loginAndSelectTenant(page);

  await expect(page.getByRole('heading', { name: '异常 SLA' })).toBeVisible();
  await expect(page.getByTestId('order-exception-sla-board')).toBeVisible();
});

test('拆单合单作业台可以独立打开', async ({ page }) => {
  await page.goto('/orders/split-merge');
  await loginAndSelectTenant(page);

  await expect(page).toHaveURL(/\/orders\/split-merge$/);
  await expect(page.getByRole('heading', { name: '拆单合单' })).toBeVisible();
  await expect(page.getByTestId('order-review-approve')).toHaveCount(0);
  await expect(page.getByTestId('order-review-split')).toBeVisible();
  await expect(page.getByRole('button', { name: /合\s*并/ })).toBeVisible();
});
