import { expect, test, loginAndSelectTenant } from './fixtures';

test('product barcodes page supports list and create drawer', async ({ page }) => {
  await page.goto('/products/barcodes');
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('product-barcodes-title')).toBeVisible();
  await expect(page.getByTestId('product-barcodes-filter-form')).toBeVisible();

  const listTable = page.getByTestId('product-barcodes-table');
  await expect(listTable).toBeVisible();

  const samplesTable = page.getByTestId('product-barcodes-samples-table');
  await expect(samplesTable).toBeVisible();

  const createButton = page.getByTestId('product-barcodes-create-button');
  await expect(createButton).toBeVisible();
  await createButton.click();

  const drawer = page.getByTestId('product-barcodes-drawer');
  await expect(drawer).toBeVisible();
});
