import { expect, test, loginAndSelectTenant } from './fixtures';

test('product packaging specs page supports selecting SKU and saving default spec', async ({
  page
}) => {
  await page.goto('/products/packaging-specs');
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('product-packaging-specs-page')).toBeVisible();

  const skuSelect = page.getByTestId('packaging-specs-sku-select');
  await expect(skuSelect).toBeVisible();
  await skuSelect.click();
  await skuSelect.locator('input').fill('SKU-ERP-1000');

  const dropdown = page.locator('.ant-select-dropdown:visible').last();
  await expect(dropdown).toBeVisible();
  const firstOption = dropdown.locator('.ant-select-item-option:visible').first();
  await expect(firstOption).toBeVisible();
  await firstOption.click();

  const saveButton = page.getByTestId('packaging-specs-save-button');
  await expect(saveButton).toBeVisible();
  await saveButton.click();

  await expect(page.getByText('Packaging spec saved')).toBeVisible();
  const table = page.getByTestId('packaging-specs-table');
  await expect(table).toBeVisible();
  await expect(table.locator('tbody tr').first()).toBeVisible();
  await expect(table).toContainText('DEFAULT');
});

test('inventory summary governance panel supports ATP and reconciliation interactions', async ({
  page
}) => {
  await page.goto('/inventory');
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('inventory-summary-title')).toBeVisible();
  const summaryTable = page.getByTestId('inventory-summary-table');
  await expect(summaryTable).toBeVisible();

  const firstRow = summaryTable.locator('.erp-data-grid-row').first();
  await expect(firstRow).toBeVisible();
  await firstRow.click();

  await page.getByRole('button', { name: /Inventory Governance/ }).click();
  const governancePanel = page.getByTestId('inventory-governance-panel');
  await expect(governancePanel).toBeVisible();

  const governanceEmpty = page.getByTestId('inventory-governance-empty');
  if (await governanceEmpty.isVisible()) {
    await summaryTable.getByRole('row', { name: /SKU-ERP-1000/ }).first().click();
    await expect(governanceEmpty).toBeHidden();
  }

  const requestQtyInput = page.getByRole('spinbutton', { name: 'Request qty' });
  await expect(requestQtyInput).toBeVisible();
  await requestQtyInput.fill('8');
  await page.getByRole('button', { name: 'Recalculate ATP' }).click();

  await expect(governancePanel).toContainText('ATP Qty');

  const expectedQtyInput = page.getByRole('spinbutton', { name: 'Expected on-hand qty' });
  await expect(expectedQtyInput).toBeVisible();
  await expectedQtyInput.fill('5');
  await page.getByRole('button', { name: 'Check Reconciliation' }).click();

  await expect(page.getByText(/Reconciliation (passed|found mismatch)/)).toBeVisible();
  await expect(governancePanel).toContainText('Status');
  await expect(governancePanel).toContainText(/MATCH|MISMATCH/);
});

