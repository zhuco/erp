import { expect, test, loginAndSelectTenant } from './fixtures';

test('商品管理支持列设置与新建弹窗', async ({ page }) => {
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
  await expect
    .poll(async () => Number((await grid.getAttribute('data-row-count')) ?? '0'), {
      timeout: 20_000,
    })
    .toBeGreaterThan(0);

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
