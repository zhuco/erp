import { expect, test, loginAndSelectTenant } from './fixtures';
import type { Locator, Page } from '@playwright/test';

const selectOptionByText = async (
  page: Page,
  trigger: Locator,
  optionText: RegExp | string
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

test('shop product matching governance supports failure triage and manual remediation', async ({
  page
}) => {
  await page.goto('/shop-products');
  await loginAndSelectTenant(page);

  const targetRow = page.locator('tbody tr').filter({ hasText: 'TB-001-RED' }).first();
  await expect(targetRow).toBeVisible();
  await expect(targetRow).toContainText(/CONFLICT|冲突/);
  await expect(targetRow).toContainText(/无候选 SKU|NO_CANDIDATE/);
  await checkRow(targetRow);

  const autoMatchButton = page.getByRole('button', { name: /自动匹配/ }).first();
  await autoMatchButton.click();
  await page.getByText(/按勾选匹配|Match Selected/).first().click();

  const warningDialog = page.getByRole('dialog').filter({ hasText: /无可用候选SKU|NO_CANDIDATE/ }).first();
  await expect(warningDialog).toBeVisible();
  await warningDialog.getByRole('button').last().click();

  const taskDrawer = page.locator('.ant-drawer-content-wrapper:visible');
  if (await taskDrawer.count()) {
    const closeButton = taskDrawer.locator('.ant-drawer-close').first();
    if (await closeButton.count()) {
      await closeButton.click();
    } else {
      await page.keyboard.press('Escape');
    }
    await expect(taskDrawer).toHaveCount(0);
  }

  await checkRow(targetRow);
  await page.getByRole('button', { name: /手动匹配/ }).first().click();

  const manualDialog = page.getByRole('dialog').filter({ hasText: /手动匹配/ }).first();
  await expect(manualDialog).toBeVisible();

  await selectOptionByText(page, manualDialog.locator('.ant-select').first(), /SKU|sku/i);
  await manualDialog.locator('.ant-modal-footer .ant-btn-primary').click();
  await expect(manualDialog).toBeHidden({ timeout: 10_000 });

  const remediatedRow = page.locator('tbody tr').filter({ hasText: 'TB-001-RED' }).first();
  await expect(remediatedRow).toBeVisible();
  await expect(remediatedRow).toContainText(/MANUAL|手动/);
  await expect(remediatedRow).not.toContainText(/无候选 SKU|NO_CANDIDATE/);
});
