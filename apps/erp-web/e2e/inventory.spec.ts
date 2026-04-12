import type { Page } from '@playwright/test';
import { expect, test, loginAndSelectTenant } from './fixtures';

const readCellNumber = async (page: Page, skuCode: string, field: 'reserved' | 'available') => {
  const cell = page
    .getByTestId('inventory-table')
    .locator('.erp-data-grid-row', { hasText: skuCode })
    .first()
    .locator(`.ag-cell[col-id="${field}"]`)
    .first();
  await expect(cell).toBeVisible();
  const raw = (await cell.textContent())?.trim() ?? '0';
  return Number.parseInt(raw, 10);
};

test('inventory list supports search and reset', async ({ page }) => {
  await page.goto('/inventory/warehouse');
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('inventory-title')).toBeVisible();
  await expect(page.getByTestId('inventory-table')).toBeVisible();
  await expect(page.getByTestId('inventory-table').locator('.erp-data-grid-row').first()).toBeVisible();

  await page.getByTestId('inventory-sku-code-input').fill('SKU-ERP-1005');
  await page.getByTestId('inventory-search-submit').click();

  const firstSkuAfterSearch = page
    .getByTestId('inventory-table')
    .locator('.erp-data-grid-row .ag-cell[col-id="sku_code"]')
    .first();
  await expect(firstSkuAfterSearch).toHaveText('SKU-ERP-1005');

  await page.getByTestId('inventory-search-reset').click();
  const firstSkuAfterReset = page
    .getByTestId('inventory-table')
    .locator('.erp-data-grid-row .ag-cell[col-id="sku_code"]')
    .first();
  await expect(firstSkuAfterReset).toHaveText('SKU-ERP-1000');
});

test('inventory reserve action updates row values', async ({ page }) => {
  await page.goto('/inventory/warehouse');
  await loginAndSelectTenant(page);

  const targetSku = 'SKU-ERP-1000';
  const reservedBefore = await readCellNumber(page, targetSku, 'reserved');
  const availableBefore = await readCellNumber(page, targetSku, 'available');

  const reserveButton = page
    .getByTestId('inventory-selected-reserve-button');
  await reserveButton.click();

  const actionModal = page.getByRole('dialog', { name: /预占库存/ });
  await expect(actionModal).toBeVisible();
  await actionModal.locator('[data-testid="inventory-action-qty"] input').fill('2');
  await actionModal.getByTestId('inventory-action-ref-type').fill('E2E');
  await actionModal.getByTestId('inventory-action-ref-id').fill(`E2E-${Date.now()}`);
  await actionModal.getByTestId('inventory-action-submit').click();
  await expect(actionModal).toBeHidden();

  const reservedAfter = await readCellNumber(page, targetSku, 'reserved');
  const availableAfter = await readCellNumber(page, targetSku, 'available');
  expect(reservedAfter).toBe(reservedBefore + 2);
  expect(availableAfter).toBe(availableBefore - 2);
});

