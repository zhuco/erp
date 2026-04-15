import type { Locator, Page } from '@playwright/test';
import dayjs from 'dayjs';

import { expect, test, loginAndSelectTenant } from './fixtures';

const openSkuBatchCreateDrawer = async (page: Page): Promise<Locator> => {
  await page.goto('/sku-batches');
  await loginAndSelectTenant(page);

  const createButton = page.getByTestId('sku-batches-create-button');
  await expect(createButton).toBeVisible();
  await createButton.click();

  const drawer = page.getByRole('dialog', { name: /\u65b0\u5efa\u6279\u6b21/ });
  await expect(drawer).toBeVisible();
  return drawer;
};

const openSkuBatchesPage = async (page: Page) => {
  await page.goto('/sku-batches');
  await loginAndSelectTenant(page);
  await expect(page.getByText('SKU \u6279\u6b21\u53f0\u8d26')).toBeVisible();
};

const openSkuSerialGenerateDrawer = async (page: Page): Promise<Locator> => {
  await page.goto('/sku-serials');
  await loginAndSelectTenant(page);

  const generateButton = page.getByTestId('sku-serials-generate-button');
  await expect(generateButton).toBeVisible();
  await generateButton.click();

  const drawer = page.getByRole('dialog', { name: /\u751f\u6210\u5e8f\u5217\u53f7/ });
  await expect(drawer).toBeVisible();
  return drawer;
};

const selectAntOption = async (page: Page, trigger: Locator, optionText: string) => {
  const selector = trigger.locator('.ant-select-selector');
  const clickable = (await selector.count()) > 0 ? selector.first() : trigger;
  await clickable.click();
  const dropdown = page.locator('.ant-select-dropdown:not(.ant-select-dropdown-hidden)').last();
  await expect(dropdown).toBeVisible();
  await dropdown.locator('.ant-select-item-option-content', { hasText: optionText }).first().click();
};

test('\u98df\u54c1\u6cbb\u7406 SKU \u7f3a\u5c11\u5931\u6548\u65e5\u671f\u65f6\u7981\u7528\u4fdd\u5b58\u5e76\u663e\u793a\u539f\u56e0', async ({ page }) => {
  const drawer = await openSkuBatchCreateDrawer(page);

  await selectAntOption(page, drawer.getByTestId('sku-batches-drawer-sku'), 'SKU-FOOD-GOV-001');
  await expect(page.getByTestId('sku-batches-food-governance-alert')).toBeVisible();
  await expect(page.getByTestId('sku-batches-save-reason-alert')).toContainText('\u5931\u6548\u65e5\u671f');

  const batchNo = `FOOD-BATCH-${Date.now()}`;
  await drawer.getByTestId('sku-batches-drawer-batch-no').fill(batchNo);
  await expect(drawer.getByTestId('sku-batches-save-button')).toBeDisabled();
  await expect(page.getByText(batchNo, { exact: false })).toHaveCount(0);
});

test('\u98df\u54c1\u6cbb\u7406 SKU \u586b\u5199\u5931\u6548\u65e5\u671f\u540e\u5c55\u793a\u63a8\u7b97\u751f\u4ea7\u65e5\u671f', async ({ page }) => {
  const drawer = await openSkuBatchCreateDrawer(page);
  const expireAt = dayjs().startOf('month').date(25).hour(0).minute(0).second(0);

  await selectAntOption(page, drawer.getByTestId('sku-batches-drawer-sku'), 'SKU-FOOD-GOV-001');
  const expireInput = drawer.getByRole('textbox', { name: /\u5931\u6548\u65e5\u671f/ });
  await expireInput.click();
  const pickerDropdown = page.locator('.ant-picker-dropdown').last();
  await expect(pickerDropdown).toBeVisible();
  await pickerDropdown.locator('.ant-picker-cell-in-view .ant-picker-cell-inner', { hasText: '25' }).last().click();
  const confirmButton = pickerDropdown.locator('.ant-picker-ok button');
  if (await confirmButton.count()) {
    await confirmButton.click();
  } else {
    await page.mouse.click(0, 0);
  }

  await expect(page.getByTestId('sku-batches-production-date-preview')).toBeVisible();
  await expect(page.getByTestId('sku-batches-production-date-preview')).toContainText(
    expireAt.subtract(180, 'day').format('YYYY-MM-DD'),
  );
  await expect(page.getByTestId('sku-batches-production-date-preview')).toContainText('\u5269\u4f59\u4fdd\u8d28\u671f');
});

test('\u98df\u54c1\u7981\u6279\u6b21 SKU \u7981\u7528\u4fdd\u5b58\u5e76\u5c55\u793a\u6cbb\u7406\u963b\u65ad\u539f\u56e0', async ({ page }) => {
  const drawer = await openSkuBatchCreateDrawer(page);

  await selectAntOption(page, drawer.getByTestId('sku-batches-drawer-sku'), 'SKU-FOOD-NOBATCH-001');
  await expect(page.getByTestId('sku-batches-batch-disabled-alert')).toBeVisible();
  await expect(page.getByTestId('sku-batches-save-reason-alert')).toContainText('\u672a\u542f\u7528\u6279\u6b21\u7ba1\u7406');

  const batchNo = `FOOD-DISABLED-${Date.now()}`;
  await drawer.getByTestId('sku-batches-drawer-batch-no').fill(batchNo);
  await expect(drawer.getByTestId('sku-batches-save-button')).toBeDisabled();
  await expect(page.getByText(batchNo, { exact: false })).toHaveCount(0);
});

test('\u6570\u7801\u7981\u5e8f\u5217\u53f7 SKU \u7981\u7528\u751f\u6210\u5e76\u5c55\u793a\u963b\u65ad\u539f\u56e0', async ({ page }) => {
  const drawer = await openSkuSerialGenerateDrawer(page);

  await selectAntOption(page, drawer.getByTestId('sku-serials-generate-sku'), 'SKU-DIGITAL-NOSERIAL-001');
  await expect(page.getByTestId('sku-serials-digital-governance-alert')).toBeVisible();
  await expect(page.getByTestId('sku-serials-generate-reason-alert')).toContainText('\u672a\u542f\u7528\u5e8f\u5217\u53f7\u7ba1\u7406');

  const prefix = `NOSN${Date.now()}`;
  await drawer.getByTestId('sku-serials-generate-prefix').fill(prefix);
  await drawer.getByRole('spinbutton').fill('2');
  await expect(drawer.getByTestId('sku-serials-confirm-generate-button')).toBeDisabled();
  await expect(page.getByText(prefix, { exact: false })).toHaveCount(0);
});

test('\u6570\u7801\u6cbb\u7406 SKU \u81ea\u52a8\u5e26\u51fa\u5efa\u8bae\u524d\u7f00\u540e\u53ef\u6210\u529f\u751f\u6210', async ({ page }) => {
  const drawer = await openSkuSerialGenerateDrawer(page);

  await selectAntOption(page, drawer.getByTestId('sku-serials-generate-sku'), 'SKU-DIGITAL-GOV-001');
  await expect(page.getByTestId('sku-serials-digital-governance-alert')).toBeVisible();
  await expect(page.getByTestId('sku-serials-prefix-suggestion')).toBeVisible();

  const suggestedPrefix = await drawer.getByTestId('sku-serials-generate-prefix').inputValue();
  expect(suggestedPrefix.trim()).not.toBe('');

  await drawer.getByRole('spinbutton').fill('2');
  await expect(drawer.getByTestId('sku-serials-confirm-generate-button')).toBeEnabled();
  await drawer.getByTestId('sku-serials-confirm-generate-button').click();

  await expect(drawer).toBeHidden({ timeout: 20_000 });
  await expect(page.getByText(suggestedPrefix, { exact: false }).first()).toBeVisible({ timeout: 20_000 });
});

test('\u98df\u54c1\u6279\u6b21\u9875\u53ef\u89c1\u4e34\u671f\u4e0e\u7981\u552e\u72b6\u6001\u6458\u8981', async ({ page }) => {
  await openSkuBatchesPage(page);
  const normalExpectedProductionDate = dayjs().add(40, 'day').subtract(180, 'day').format('YYYY-MM-DD');
  const nearExpectedProductionDate = dayjs().add(10, 'day').subtract(180, 'day').format('YYYY-MM-DD');

  await expect(page.getByTestId('sku-batches-summary-normal')).toBeVisible();
  await expect(page.getByTestId('sku-batches-summary-near-expiry')).toBeVisible();
  await expect(page.getByTestId('sku-batches-summary-forbid-sale')).toBeVisible();
  await expect(page.getByText('BATCH-FOOD-NEAR-001')).toBeVisible();
  await expect(page.getByText('BATCH-FOOD-FORBID-001')).toBeVisible();
  await expect(page.getByText('\u4e34\u671f').first()).toBeVisible();
  await expect(page.getByText('\u7981\u552e').first()).toBeVisible();
  await expect(page.getByRole('row', { name: /BATCH-FOOD-NORMAL-001/ }).first()).toContainText(normalExpectedProductionDate);
  await expect(page.getByRole('row', { name: /BATCH-FOOD-NEAR-001/ }).first()).toContainText(nearExpectedProductionDate);
  await expect(page.getByRole('row', { name: /BATCH-FOOD-NORMAL-001/ }).first()).toContainText('40 \u5929');
});

test('\u98df\u54c1\u6279\u6b21\u9875\u652f\u6301\u6309\u6cbb\u7406\u72b6\u6001\u7b5b\u9009\u7981\u552e\u6279\u6b21', async ({ page }) => {
  await openSkuBatchesPage(page);

  await selectAntOption(page, page.getByTestId('sku-batches-governance-status-filter'), '\u7981\u552e');
  await expect(page.getByText('BATCH-FOOD-FORBID-001')).toBeVisible();
  await expect(page.getByText('BATCH-FOOD-NORMAL-001')).toHaveCount(0);
  await expect(page.getByRole('row', { name: /BATCH-FOOD-FORBID-001/ }).first()).toContainText('\u51bb\u7ed3\u5e93\u5b58\u5e76\u590d\u6838');
});

test('\u6570\u7801\u6cbb\u7406 SKU \u5c55\u793a\u6cbb\u7406\u8981\u6c42\u6e05\u5355', async ({ page }) => {
  const drawer = await openSkuSerialGenerateDrawer(page);

  await selectAntOption(page, drawer.getByTestId('sku-serials-generate-sku'), 'SKU-DIGITAL-GOV-001');
  await expect(page.getByTestId('sku-serials-policy-checklist')).toBeVisible();
  await expect(page.getByTestId('sku-serials-policy-inbound-unique')).toBeVisible();
  await expect(page.getByTestId('sku-serials-policy-outbound-required')).toBeVisible();
  await expect(page.getByTestId('sku-serials-policy-platform-push')).toBeVisible();
});
