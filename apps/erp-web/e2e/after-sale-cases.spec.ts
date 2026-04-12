import type { Page, TestInfo } from '@playwright/test';
import { expect, test, loginAndSelectTenant } from './fixtures';

const capturePage = async (page: Page, testInfo: TestInfo, name: string) => {
  await page.screenshot({ path: testInfo.outputPath(`${name}.png`), fullPage: true });
};

test('after-sale list to detail', async ({ page }, testInfo) => {
  await page.goto('/after-sale-cases');

  await loginAndSelectTenant(page);

  await expect(page.getByRole('heading', { name: '售后 Case 列表' })).toBeVisible();
  await capturePage(page, testInfo, 'after-sale-cases-list');

  const firstCaseLink = page.getByTestId('after-sale-case-link').first();
  await expect(firstCaseLink).toBeVisible();
  const caseId = await firstCaseLink.getAttribute('href');
  await firstCaseLink.click();

  await expect(page.getByRole('heading', { name: '售后 Case 详情' })).toBeVisible();
  await expect(page.getByText('Raw 摘要')).toBeVisible();
  await capturePage(page, testInfo, 'after-sale-cases-detail');

  if (caseId) {
    await expect(page).toHaveURL(new RegExp(`${caseId}$`));
  }
});
