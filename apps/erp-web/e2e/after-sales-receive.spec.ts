import type { Page, TestInfo } from '@playwright/test';
import { expect, test, loginAndSelectTenant } from './fixtures';

const capturePage = async (page: Page, testInfo: TestInfo, name: string) => {
  await page.screenshot({ path: testInfo.outputPath(`${name}.png`), fullPage: true });
};

test('after-sales receive flow', async ({ page }, testInfo) => {
  await page.goto('/after-sales');

  await loginAndSelectTenant(page);

  await expect(page.getByTestId('after-sales-title')).toBeVisible();
  await capturePage(page, testInfo, 'after-sales-list');

  const receiveButton = page.getByTestId('after-sales-receive-action').first();
  await expect(receiveButton).toBeVisible();
  await receiveButton.click();

  await expect(page.getByTestId('after-sales-receive-title')).toBeVisible();
  await capturePage(page, testInfo, 'after-sales-receive');

  const warehouseSelect = page.getByTestId('after-sales-receive-warehouse');
  await warehouseSelect.click();
  const dropdown = page.locator('.ant-select-dropdown:visible').first();
  const option = dropdown.locator('.ant-select-item-option:visible').first();
  await expect(option).toBeVisible();
  await option.click();

  await page.getByTestId('after-sales-receive-submit').click();

  await expect(page.getByTestId('after-sales-detail-title')).toBeVisible();
  await capturePage(page, testInfo, 'after-sales-detail');
});
