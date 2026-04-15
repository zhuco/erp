import type { Locator, Page } from '@playwright/test';

import { expect, test, loginAndSelectTenant } from './fixtures';

const openCreateModal = async (page: Page): Promise<Locator> => {
  await page.goto('/products');
  await loginAndSelectTenant(page);

  const createButton = page.getByTestId('products-create-button');
  await expect(createButton).toBeVisible();
  await expect(createButton).toBeEnabled();
  await createButton.click();

  const createModal = page.getByRole('dialog', { name: /新建/ });
  await expect(createModal).toBeVisible();
  return createModal;
};

const selectIndustryProfile = async (page: Page, modal: Locator, profile: string) => {
  const select = modal.getByTestId('products-industry-profile');
  await select.click();
  await page
    .locator('.ant-select-dropdown:not(.ant-select-dropdown-hidden)')
    .getByText(profile, { exact: true })
    .click();
};

const selectFieldOption = async (
  page: Page,
  modal: Locator,
  label: string,
  option: string,
) => {
  await modal.getByLabel(label).click();
  await page
    .locator('.ant-select-dropdown:not(.ant-select-dropdown-hidden)')
    .getByText(option, { exact: true })
    .click();
};

test('商品管理支持列表展示与新建弹窗', async ({ page }) => {
  await page.goto('/products');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('products-data-grid');
  await expect(grid).toBeVisible();

  const createButton = page.getByTestId('products-create-button');
  await expect(createButton).toBeVisible();
  if (await createButton.isEnabled()) {
    await createButton.click();
    const createModal = page.getByRole('dialog', { name: /新建/ });
    await expect(createModal).toBeVisible();
    await expect(createModal.getByRole('tab')).toHaveCount(4);
    await page.keyboard.press('Escape');
    await expect(createModal).toBeHidden({ timeout: 20_000 });
  }
});

test('商品管理支持搜索聚焦与快速重置', async ({ page }) => {
  await page.goto('/products');
  await loginAndSelectTenant(page);

  const searchInput = page.getByTestId('products-search').locator('input');
  await expect(searchInput).toBeVisible();
  await expect(searchInput).toBeFocused();

  const grid = page.getByTestId('products-data-grid');
  await expect(grid).toBeVisible();
  await expect(
    grid.locator('[role="rowgroup"]').nth(1).locator('[role="row"]').first(),
  ).toBeVisible({
    timeout: 20_000,
  });

  await searchInput.fill('示例');
  await searchInput.press('Enter');
  await expect(searchInput).toHaveValue('示例');
  await page.getByTestId('products-search-reset').click();
  await expect(searchInput).toHaveValue('');
});

test('商品管理支持快捷筛选设置与工具栏功能', async ({ page }) => {
  await page.goto('/products');
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('products-quick-filters')).toBeVisible();
  await page.getByTestId('products-quick-filter-settings-open').click();

  const settingsModal = page.getByRole('dialog', { name: '快捷筛选设置' });
  await expect(settingsModal).toBeVisible();

  const options = settingsModal.getByTestId('products-quick-filter-settings-options');
  const inactiveOption = options.getByRole('checkbox', { name: '停用货品' });
  await expect(inactiveOption).toBeChecked();
  await inactiveOption.click();
  await expect(inactiveOption).not.toBeChecked();

  await page.getByTestId('products-quick-filter-settings-save').click();
  await expect(settingsModal).toBeHidden({ timeout: 10_000 });
  await expect(page.getByTestId('products-quick-filter-item-INACTIVE')).toBeHidden();

  const downloadPromise = page.waitForEvent('download');
  await page.getByTestId('products-export-button').click();
  const download = await downloadPromise;
  expect(download.suggestedFilename()).toMatch(/^products-\d{4}-\d{2}-\d{2}\d{6}\.csv$/);

  await page.getByTestId('products-help-button').click();
  const helpModal = page.getByRole('dialog', { name: '货品功能帮助' });
  await expect(helpModal).toBeVisible();
  await expect(helpModal).toContainText('工具栏“导出”会导出当前结果或已勾选货品');
  await page.getByTestId('products-help-modal-close').click();
  await expect(helpModal).toBeHidden({ timeout: 10_000 });
});

test('商品管理支持行业模板切换与行业字段联动', async ({ page }) => {
  const createModal = await openCreateModal(page);

  await expect(createModal.getByTestId('products-industry-profile')).toBeVisible();

  await selectIndustryProfile(page, createModal, '食品');
  await expect(createModal.getByTestId('products-food-section')).toBeVisible();
  await expect(createModal.getByLabel('生产日期管理')).toBeVisible();
  await expect(createModal.getByLabel('保质期单位')).toBeVisible();
  await expect(createModal.getByLabel('临期预警天数')).toBeVisible();

  await selectIndustryProfile(page, createModal, '鞋服');
  await expect(createModal.getByTestId('products-apparel-section')).toBeVisible();
  await expect(createModal.getByLabel('尺码体系')).toBeVisible();
  await expect(createModal.getByTestId('products-apparel-spec-hint')).toContainText(
    '颜色 / 尺码',
  );

  await selectIndustryProfile(page, createModal, '数码');
  await expect(createModal.getByTestId('products-digital-section')).toBeVisible();
  await expect(createModal.getByLabel('序列号追踪模式')).toBeVisible();
  await expect(createModal.getByLabel('出库绑定序列号')).toBeVisible();
  await expect(createModal.getByLabel('入库唯一校验')).toBeVisible();
});

test('商品管理支持鞋服规格批量生成并在保存后回显行业摘要', async ({ page }) => {
  const createModal = await openCreateModal(page);
  const productName = `鞋服模板商品-${Date.now()}`;

  await createModal.getByLabel('名称').fill(productName);
  await selectIndustryProfile(page, createModal, '鞋服');

  await expect(createModal.getByTestId('products-apparel-composer')).toBeVisible();
  await createModal.getByTestId('products-apparel-colors-input').fill('黑色,白色');
  await createModal.getByTestId('products-apparel-sizes-input').fill('41,42');
  await createModal.getByTestId('products-apparel-generate-button').click();

  await expect(createModal.locator("input[value='黑色 / 41']")).toBeVisible();
  await expect(createModal.locator("input[value='白色 / 42']")).toBeVisible();

  await page.locator('.ant-modal-footer .ant-btn-primary').last().click();
  await expect(createModal).toBeHidden({ timeout: 20_000 });

  const searchInput = page.getByTestId('products-search').locator('input');
  await searchInput.fill(productName);
  await searchInput.press('Enter');

  const productRow = page.getByRole('row', { name: new RegExp(productName) }).first();
  await expect(productRow).toBeVisible({ timeout: 20_000 });
  await productRow.click();

  await expect(page.getByText('鞋服规格策略')).toBeVisible();
  await expect(page.getByText('规格 4 条')).toBeVisible();
});

test('商品管理支持食品模板保存后回显食品效期策略', async ({ page }) => {
  const createModal = await openCreateModal(page);
  const productName = `食品模板商品-${Date.now()}`;

  await createModal.getByLabel('名称').fill(productName);
  await selectIndustryProfile(page, createModal, '食品');

  await createModal.getByLabel('保质期(天)').fill('180');
  await createModal.getByLabel('禁售天数').fill('7');
  await createModal.getByLabel('临期预警天数').fill('15');

  await page.locator('.ant-modal-footer .ant-btn-primary').last().click();
  await expect(createModal).toBeHidden({ timeout: 20_000 });

  const searchInput = page.getByTestId('products-search').locator('input');
  await searchInput.fill(productName);
  await searchInput.press('Enter');

  const productRow = page.getByRole('row', { name: new RegExp(productName) }).first();
  await expect(productRow).toBeVisible({ timeout: 20_000 });
  await productRow.click();

  await expect(page.getByText('食品效期策略')).toBeVisible();
  await expect(page.getByText('生产日期必填')).toBeVisible();
  await expect(page.getByText('保质期 180天')).toBeVisible();
  await expect(page.getByText('临期预警 15 天')).toBeVisible();
  await expect(page.getByText('禁售提前 7 天')).toBeVisible();
});

test('商品管理支持数码模板保存后回显序列号策略并再次编辑正确回填', async ({ page }) => {
  const createModal = await openCreateModal(page);
  const productName = `数码模板商品-${Date.now()}`;

  await createModal.getByLabel('名称').fill(productName);
  await selectIndustryProfile(page, createModal, '数码');

  const serialPushPlatformSwitch = createModal.locator('#serial_push_platform');
  await expect(createModal.locator('#serial_enabled')).toHaveAttribute('aria-checked', 'true');
  await expect(serialPushPlatformSwitch).toBeEnabled();
  await serialPushPlatformSwitch.click();
  await expect(serialPushPlatformSwitch).toHaveAttribute('aria-checked', 'true');
  await selectFieldOption(page, createModal, '序列号追踪模式', '设备追踪');
  await createModal.getByLabel('出库绑定序列号').click();
  await createModal.getByLabel('入库唯一校验').click();

  await page.locator('.ant-modal-footer .ant-btn-primary').last().click();
  await expect(createModal).toBeHidden({ timeout: 20_000 });

  const searchInput = page.getByTestId('products-search').locator('input');
  await searchInput.fill(productName);
  await searchInput.press('Enter');

  const productRow = page.getByRole('row', { name: new RegExp(productName) }).first();
  await expect(productRow).toBeVisible({ timeout: 20_000 });
  await productRow.click();

  await expect(page.getByText('序列号策略')).toBeVisible();
  await expect(page.getByText('追踪模式 设备追踪')).toBeVisible();
  await expect(page.getByText('出库需绑定序列号')).toBeVisible();
  await expect(page.getByText('入库唯一校验')).toBeVisible();
  await expect(page.getByText('平台回传已启用')).toBeVisible();

  await productRow.dblclick();
  const editModal = page.getByRole('dialog', { name: '编辑货品' });
  await expect(editModal).toBeVisible();
  await expect(editModal.locator('#serial_enabled')).toHaveAttribute('aria-checked', 'true');
  await expect(editModal.locator('#serial_push_platform')).toHaveAttribute('aria-checked', 'true');
  await expect(editModal.getByLabel('出库绑定序列号')).toBeChecked();
  await expect(editModal.getByLabel('入库唯一校验')).toBeChecked();
  await expect(editModal.getByText('设备追踪', { exact: true })).toBeVisible();
});

