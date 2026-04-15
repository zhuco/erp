import { expect, loginAndSelectTenant, test } from './fixtures';

test('finance reconcile center shows core operation panels', async ({ page }) => {
  await page.goto('/finance/reconcile');
  await loginAndSelectTenant(page);

  await expect(page.getByText('对账管理').first()).toBeVisible();
  await expect(page.getByText('账单导入').first()).toBeVisible();
  await expect(page.getByText('运行对账').first()).toBeVisible();
  await expect(page.getByText('对账任务').first()).toBeVisible();
  await expect(page.getByText('差异明细').first()).toBeVisible();
  await expect(page.getByRole('button', { name: '运行对账' }).first()).toBeVisible();
});

test('finance profit report shows filter and grid', async ({ page }) => {
  await page.goto('/finance/profit');
  await loginAndSelectTenant(page);

  await expect(page.getByText('利润分析').first()).toBeVisible();
  await expect(page.getByText('利润过滤').first()).toBeVisible();
  await expect(page.getByTestId('finance-profit-grid')).toBeVisible();
});
