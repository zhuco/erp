import { expect, loginAndSelectTenant, test } from './fixtures';

test('shop product matching governance supports apparel dictionary triage in workbench flow', async ({
  page,
}) => {
  await page.goto('/shop-products');
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('shop-products-page')).toBeVisible();
  await expect(page.getByTestId('shop-products-left-panel')).toBeVisible();
  await expect(page.getByTestId('shop-products-detail-panel')).toBeVisible();
  await expect(page.getByTestId('shop-products-detail-template')).toBeVisible();
  await expect(page.getByTestId('shop-products-bottom-panel')).toBeVisible();
  await expect(page.getByTestId('shop-products-toolbar-selected-count')).toContainText('已选 0');

  const grid = page.getByTestId('shop-products-grid');
  const gridRows = grid.locator('.ag-center-cols-container .ag-row');
  await expect(gridRows.first()).toBeVisible();
  await expect(page.getByTestId('shop-products-detail-panel')).toContainText('TB-SKU-001');
  await expect(page.getByTestId('shop-products-detail-panel')).toContainText(/CONFLICT|COLOR_DICT_MISS/);

  await page.getByTestId('shop-products-filter-keyword').fill('TB-SKU-002');
  await page.getByTestId('shop-products-toolbar-search-submit').click();
  await expect(page.getByTestId('shop-products-detail-panel')).toContainText('TB-SKU-002');
  await expect(page.getByTestId('shop-products-detail-status-hint')).toContainText(/ERP|匹配|match/i);
  await expect(page.getByTestId('shop-products-toolbar-selected-count')).toContainText('已选 0');

  await page.getByTestId('shop-products-filter-keyword').fill('');
  await page.getByTestId('shop-products-toolbar-search-submit').click();
  await expect(grid.locator('.ag-center-cols-container .ag-row').first()).toBeVisible();

  await page.getByTestId('shop-products-auto-match-button').click();
  await page.getByTestId('shop-products-auto-match-filtered').click();

  const warningDialog = page
    .getByRole('dialog')
    .filter({ hasText: /COLOR_DICT_MISS|Auto match failures/i })
    .first();
  if (await warningDialog.count()) {
    await expect(warningDialog).toBeVisible();
    await warningDialog.locator('.ant-modal-footer .ant-btn-primary').click();
    await expect(warningDialog).toBeHidden({ timeout: 10_000 });
  }

  const bottomPanel = page.getByTestId('shop-products-bottom-panel');
  await bottomPanel.getByRole('tab').nth(1).click();
  await expect(bottomPanel.locator('.shop-products-bottom-failure-item').first()).toBeVisible();
  await expect(page.getByTestId('shop-products-detail-panel')).toContainText('TB-SKU-002');
  await expect(page.getByTestId('shop-products-detail-status-hint')).toContainText(/ERP|匹配|match/i);
});
