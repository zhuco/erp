import type { Page, TestInfo } from '@playwright/test';
import { expect, test, loginAndSelectTenant } from './fixtures';

const capturePage = async (page: Page, testInfo: TestInfo, name: string) => {
  await page.screenshot({ path: testInfo.outputPath(`${name}.png`), fullPage: true });
};

test('after-sales exchanges page renders', async ({ page }, testInfo) => {
  await page.goto('/after-sales/exchanges');

  await loginAndSelectTenant(page);

  await expect(page.getByTestId('after-sales-exchanges-title')).toBeVisible();
  await expect(page.locator('.ant-table').first()).toBeVisible();
  await capturePage(page, testInfo, 'after-sales-exchanges');
});

test('after-sales refund docs page renders', async ({ page }, testInfo) => {
  await page.goto('/refund-docs');

  await loginAndSelectTenant(page);

  await expect(page.getByTestId('refund-docs-title')).toBeVisible();
  await expect(page.locator('.ant-table').first()).toBeVisible();
  await capturePage(page, testInfo, 'after-sales-refund-docs');
});

test('after-sales settings page renders', async ({ page }, testInfo) => {
  await page.goto('/after-sales/settings');

  await loginAndSelectTenant(page);

  await expect(page.getByTestId('after-sales-settings-title')).toBeVisible();
  await expect(page.locator('.ant-table').first()).toBeVisible();
  await capturePage(page, testInfo, 'after-sales-settings');
});
