import type { Page, TestInfo } from '@playwright/test';
import { expect, test, loginAndSelectTenant } from './fixtures';

const capturePage = async (page: Page, testInfo: TestInfo, name: string) => {
  await page.screenshot({ path: testInfo.outputPath(`${name}.png`), fullPage: true });
};

const selectOption = async (page: Page, label: string) => {
  const dropdown = page.locator('.ant-select-dropdown:visible').first();
  await expect(dropdown).toBeVisible();
  const roleOption = dropdown.getByRole('option', { name: label, exact: true }).first();
  if ((await roleOption.count()) > 0) {
    await roleOption.scrollIntoViewIfNeeded();
    await roleOption.click({ force: true });
    return;
  }
  const fallbackOption = dropdown
    .locator('.ant-select-item-option-content', { hasText: label })
    .first();
  await fallbackOption.scrollIntoViewIfNeeded();
  await fallbackOption.click({ force: true });
};

test('customer service tickets create and search', async ({ page }, testInfo) => {
  await page.goto('/customers/service-tickets');

  await loginAndSelectTenant(page);

  await expect(page.getByTestId('customer-service-title')).toBeVisible();
  await expect(page.getByTestId('customer-service-table')).toBeVisible();

  await page.getByTestId('customer-service-create').click();

  const drawer = page.locator('.ant-drawer:visible');
  await expect(drawer).toBeVisible();

  await drawer.getByTestId('customer-service-form-name').fill('E2E客户');
  await drawer.getByTestId('customer-service-form-phone').fill('13800138000');

  await drawer.getByTestId('customer-service-form-channel').click();
  await selectOption(page, '在线');

  await drawer.getByTestId('customer-service-form-category').click();
  await selectOption(page, '售后');

  await drawer.getByTestId('customer-service-form-priority').click();
  await selectOption(page, '高');

  await drawer.getByTestId('customer-service-form-status').click();
  await selectOption(page, '处理中');

  await drawer.getByTestId('customer-service-form-order').fill('ORDER-E2E-01');
  await drawer.getByTestId('customer-service-form-handler').fill('客服-E2E');
  await drawer.getByTestId('customer-service-form-description').fill('E2E 客服工单描述');

  await drawer.getByTestId('customer-service-form-save').click();

  await expect(page.locator('.ant-drawer:visible')).toHaveCount(0);

  await page.getByTestId('customer-service-filter-keyword').fill('E2E客户');
  await page.getByTestId('customer-service-filter-submit').click();

  await expect(page.getByText('E2E客户')).toBeVisible();

  await capturePage(page, testInfo, 'customer-service-tickets');
});
