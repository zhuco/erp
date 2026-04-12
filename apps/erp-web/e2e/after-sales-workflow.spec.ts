import { fileURLToPath } from 'url';
import type { Page, TestInfo } from '@playwright/test';
import { expect, test, loginAndSelectTenant } from './fixtures';

const capturePage = async (page: Page, testInfo: TestInfo, name: string) => {
  await page.screenshot({ path: testInfo.outputPath(`${name}.png`), fullPage: true });
};

test('after-sales intercepts create', async ({ page }, testInfo) => {
  await page.goto('/after-sales/intercepts');

  await loginAndSelectTenant(page);

  await expect(page.getByTestId('after-sales-intercepts-title')).toBeVisible();

  const createButton = page.getByTestId('after-sales-intercepts-create');
  await expect(createButton).toBeVisible();
  await createButton.click();

  await page.getByTestId('after-sales-intercepts-order').fill('ORDER-E2E-01');
  const interceptModal = page.locator('.ant-modal:visible');
  await expect(interceptModal).toBeVisible();
  await interceptModal.getByTestId('after-sales-intercepts-submit').click();

  await expect(page.getByText('ORDER-E2E-01')).toBeVisible();
  await capturePage(page, testInfo, 'after-sales-intercepts');
});

test('after-sales inspections create', async ({ page }, testInfo) => {
  await page.goto('/after-sales/inspections');

  await loginAndSelectTenant(page);

  await expect(page.getByTestId('after-sales-inspections-title')).toBeVisible();

  const createButton = page.getByTestId('after-sales-inspections-create');
  await expect(createButton).toBeVisible();
  await createButton.click();

  const modal = page.locator('.ant-modal:visible');
  await expect(modal).toBeVisible();
  await modal.getByTestId('after-sales-inspection-after-sales').fill('AS-E2E-01');
  await modal.getByTestId('after-sales-inspection-sku').fill('SKU-E2E-01');
  await modal.getByTestId('after-sales-inspections-submit').click();

  await expect(page.getByText('AS-E2E-01')).toBeVisible();
  await capturePage(page, testInfo, 'after-sales-inspections');
});

test('after-sales sla rule create', async ({ page }, testInfo) => {
  await page.goto('/after-sales/sla');

  await loginAndSelectTenant(page);

  await expect(page.getByTestId('after-sales-sla-title')).toBeVisible();

  const createButton = page.getByTestId('after-sales-sla-create');
  await expect(createButton).toBeVisible();
  await createButton.click();

  const modal = page.locator('.ant-modal:visible');
  await expect(modal).toBeVisible();
  await modal.getByTestId('after-sales-sla-type').click();
  const dropdown = page.locator('.ant-select-dropdown:visible').first();
  await dropdown.getByText('换货').click();

  const responseInput = modal.getByTestId('after-sales-sla-response');
  await responseInput.fill('5');
  const resolutionInput = modal.getByTestId('after-sales-sla-resolution');
  await resolutionInput.fill('48');
  await modal.getByTestId('after-sales-sla-role').fill('E2E角色');
  await modal.getByTestId('after-sales-sla-submit').click();

  await expect(page.getByText('E2E角色')).toBeVisible();
  await capturePage(page, testInfo, 'after-sales-sla');
});

test('after-sales bulk import job create', async ({ page }, testInfo) => {
  await page.goto('/after-sales/bulk-edit');

  await loginAndSelectTenant(page);

  await expect(page.getByTestId('after-sales-bulk-title')).toBeVisible();

  const uploadInput = page
    .getByTestId('after-sales-bulk-upload')
    .locator('input[type="file"]');
  const filePath = fileURLToPath(
    new URL('./fixtures/after-sales-import.csv', import.meta.url)
  );
  await uploadInput.setInputFiles(filePath);

  await page.getByRole('button', { name: '开始导入' }).click();

  await expect(page.getByRole('cell', { name: 'after-sales-import.csv' })).toBeVisible();
  await capturePage(page, testInfo, 'after-sales-bulk-edit');
});
