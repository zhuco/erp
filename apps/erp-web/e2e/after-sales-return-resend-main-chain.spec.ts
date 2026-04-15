import type { Page, TestInfo } from '@playwright/test';
import { expect, test, loginAndSelectTenant } from './fixtures';

const capturePage = async (page: Page, testInfo: TestInfo, name: string) => {
  await page.screenshot({ path: testInfo.outputPath(`${name}.png`), fullPage: true });
};

test('after-sales return inspect resend main chain smoke', async ({ page }, testInfo) => {
  await page.goto('/after-sales');
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('after-sales-title')).toBeVisible();

  const receiveAction = page.getByTestId('after-sales-receive-action').first();
  if ((await receiveAction.count()) > 0) {
    const receiveHref = await receiveAction.getAttribute('data-after-sales-receive-href');
    expect(receiveHref).toBeTruthy();
    await page.goto(receiveHref!);
    await expect(page.getByTestId('after-sales-receive-title')).toBeVisible();

    const warehouseSelect = page.getByTestId('after-sales-receive-warehouse');
    await warehouseSelect.click();
    const option = page.locator('.ant-select-dropdown:visible .ant-select-item-option:visible').first();
    await expect(option).toBeVisible();
    await option.click();

    await page.getByTestId('after-sales-receive-submit').click();
    await expect(page.getByTestId('after-sales-detail-title')).toBeVisible();
    await expect(page.getByTestId('after-sales-closure-evidence')).toBeVisible();
  }

  await page.goto('/after-sales/inspections');
  await expect(page.getByTestId('after-sales-inspections-title')).toBeVisible();
  const createInspection = page.getByTestId('after-sales-inspections-create');
  if ((await createInspection.count()) > 0) {
    await createInspection.click();
    const modal = page.locator('.ant-modal:visible');
    await expect(modal).toBeVisible();
    await modal.getByTestId('after-sales-inspection-after-sales').fill('AS-E2E-MAIN-01');
    await modal.getByTestId('after-sales-inspection-sku').fill('SKU-E2E-MAIN-01');
    await modal.getByTestId('after-sales-inspections-submit').click();
    await expect(page.getByText('AS-E2E-MAIN-01')).toBeVisible();
  }

  await page.goto('/after-sales/exchanges');
  await expect(page.getByTestId('after-sales-exchanges-title')).toBeVisible();
  await expect(page.getByTestId('after-sales-exchanges-table')).toBeVisible();

  await capturePage(page, testInfo, 'after-sales-return-resend-main-chain');
});
