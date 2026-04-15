import { expect, test, loginAndSelectTenant } from './fixtures';
import type { Locator, Page } from '@playwright/test';

const selectOptionByText = async (
  page: Page,
  trigger: Locator,
  optionText: RegExp | string,
): Promise<void> => {
  await trigger.scrollIntoViewIfNeeded();
  await trigger.click();
  const dropdown = page.locator('.ant-select-dropdown:visible').last();
  await expect(dropdown).toBeVisible();
  const visibleOptions = dropdown.locator('[role="option"]:visible, .ant-select-item-option:visible');
  let option = visibleOptions.filter({ hasText: optionText });
  if ((await option.count()) === 0) {
    option = visibleOptions;
  }
  await expect(option.first()).toBeVisible();
  await option.first().click();
};

const checkRow = async (row: Locator): Promise<void> => {
  const checkbox = row.getByRole('checkbox').first();
  await checkbox.scrollIntoViewIfNeeded();
  await checkbox.click({ force: true });
};

test('shop product matching governance supports apparel dictionary triage and manual remediation', async ({
  page,
}) => {
  await page.goto('/shop-products');
  await loginAndSelectTenant(page);

  const targetRow = page.locator('tbody tr').filter({ hasText: 'TB-001-RED' }).first();
  await expect(targetRow).toBeVisible();
  await expect(targetRow).toContainText(/CONFLICT/);
  await expect(targetRow).toContainText(/COLOR_DICT_MISS/);

  await checkRow(targetRow);
  await page.getByTestId('shop-products-auto-match-button').click();
  await page.getByTestId('shop-products-auto-match-selected').click();

  const warningDialog = page.getByRole('dialog').filter({ hasText: /COLOR_DICT_MISS|自动匹配失败/ }).first();
  if (await warningDialog.count()) {
    await expect(warningDialog).toBeVisible();
    await warningDialog.locator('.ant-modal-footer .ant-btn-primary').click();
    await expect(warningDialog).toBeHidden({ timeout: 10_000 });
  }

  await expect(page.locator('.ant-drawer-content-wrapper:visible')).toHaveCount(0);
  await expect(targetRow).toContainText(/CONFLICT/);
  await expect(targetRow).toContainText(/COLOR_DICT_MISS/);

  await checkRow(targetRow);
  await page.getByTestId('shop-products-manual-match-button').click();

  const manualDialog = page
    .getByRole('dialog')
    .filter({ has: page.getByTestId('shop-products-manual-match-sku-select') })
    .first();
  await expect(manualDialog).toBeVisible();

  await selectOptionByText(page, manualDialog.locator('.ant-select').first(), /SKU|sku/i);
  await manualDialog.locator('.ant-modal-footer .ant-btn-primary').click();
  await expect(manualDialog).toBeHidden({ timeout: 10_000 });

  const remediatedRow = page.locator('tbody tr').filter({ hasText: 'TB-001-RED' }).first();
  await expect(remediatedRow).toBeVisible();
  await expect(remediatedRow).toContainText(/MANUAL|手动匹配/);
  await expect(remediatedRow).not.toContainText(/COLOR_DICT_MISS/);
});

