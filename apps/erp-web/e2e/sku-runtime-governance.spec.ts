import type { Locator, Page } from '@playwright/test';

import { expect, test, loginAndSelectTenant } from './fixtures';

const openSkuBatchCreateDrawer = async (page: Page): Promise<Locator> => {
  await page.goto('/sku-batches');
  await loginAndSelectTenant(page);

  const createButton = page.getByTestId('sku-batches-create-button');
  await expect(createButton).toBeVisible();
  await createButton.click();

  const drawer = page.getByRole('dialog', { name: /新建批次/ });
  await expect(drawer).toBeVisible();
  return drawer;
};

const openSkuSerialGenerateDrawer = async (page: Page): Promise<Locator> => {
  await page.goto('/sku-serials');
  await loginAndSelectTenant(page);

  const generateButton = page.getByTestId('sku-serials-generate-button');
  await expect(generateButton).toBeVisible();
  await generateButton.click();

  const drawer = page.getByRole('dialog', { name: /生成序列号/ });
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

test('食品治理 SKU 缺少失效日期时禁用保存并显示原因', async ({ page }) => {
  const drawer = await openSkuBatchCreateDrawer(page);

  await selectAntOption(page, drawer.getByTestId('sku-batches-drawer-sku'), 'SKU-FOOD-GOV-001');
  await expect(page.getByTestId('sku-batches-food-governance-alert')).toBeVisible();
  await expect(page.getByTestId('sku-batches-save-reason-alert')).toContainText('失效日期');

  const batchNo = `FOOD-BATCH-${Date.now()}`;
  await drawer.getByTestId('sku-batches-drawer-batch-no').fill(batchNo);
  await expect(drawer.getByTestId('sku-batches-save-button')).toBeDisabled();
  await expect(page.getByText(batchNo, { exact: false })).toHaveCount(0);
});

test('食品禁批次 SKU 禁用保存并展示治理阻断原因', async ({ page }) => {
  const drawer = await openSkuBatchCreateDrawer(page);

  await selectAntOption(page, drawer.getByTestId('sku-batches-drawer-sku'), 'SKU-FOOD-NOBATCH-001');
  await expect(page.getByTestId('sku-batches-batch-disabled-alert')).toBeVisible();
  await expect(page.getByTestId('sku-batches-save-reason-alert')).toContainText('未启用批次管理');

  const batchNo = `FOOD-DISABLED-${Date.now()}`;
  await drawer.getByTestId('sku-batches-drawer-batch-no').fill(batchNo);
  await expect(drawer.getByTestId('sku-batches-save-button')).toBeDisabled();
  await expect(page.getByText(batchNo, { exact: false })).toHaveCount(0);
});

test('数码禁序列号 SKU 禁用生成并展示阻断原因', async ({ page }) => {
  const drawer = await openSkuSerialGenerateDrawer(page);

  await selectAntOption(page, drawer.getByTestId('sku-serials-generate-sku'), 'SKU-DIGITAL-NOSERIAL-001');
  await expect(page.getByTestId('sku-serials-digital-governance-alert')).toBeVisible();
  await expect(page.getByTestId('sku-serials-generate-reason-alert')).toContainText('未启用序列号管理');

  const prefix = `NOSN${Date.now()}`;
  await drawer.getByTestId('sku-serials-generate-prefix').fill(prefix);
  await drawer.getByRole('spinbutton').fill('2');
  await expect(drawer.getByTestId('sku-serials-confirm-generate-button')).toBeDisabled();
  await expect(page.getByText(prefix, { exact: false })).toHaveCount(0);
});

test('数码治理 SKU 自动带出建议前缀后可成功生成', async ({ page }) => {
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
