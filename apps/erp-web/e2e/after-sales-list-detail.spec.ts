import type { Page, TestInfo } from '@playwright/test';
import { expect, test, loginAndSelectTenant } from './fixtures';

const capturePage = async (page: Page, testInfo: TestInfo, name: string) => {
  await page.screenshot({ path: testInfo.outputPath(`${name}.png`), fullPage: true });
};

test('after-sales list to detail', async ({ page }, testInfo) => {
  await page.goto('/after-sales');

  await loginAndSelectTenant(page);

  await expect(page.getByTestId('after-sales-title')).toBeVisible();

  const firstLink = page.getByTestId('after-sales-link').first();
  await expect(firstLink).toBeVisible();

  const afterSalesId = (await firstLink.textContent())?.trim();
  await firstLink.click();

  await expect(page.getByTestId('after-sales-detail-title')).toBeVisible();

  if (afterSalesId) {
    await expect(page.getByText(afterSalesId)).toBeVisible();
  }

  await capturePage(page, testInfo, 'after-sales-detail');
});
