import { expect, test, loginAndSelectTenant } from './fixtures';

test('product units page supports rules and create drawer', async ({ page }) => {
  await page.goto('/products/units');
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('product-units-title')).toBeVisible();
  await expect(page.getByTestId('product-units-rules-form')).toBeVisible();
  await expect(page.getByTestId('product-units-filter-form')).toBeVisible();

  const listTable = page.getByTestId('product-units-table');
  await expect(listTable).toBeVisible();

  const createButton = page.getByTestId('product-units-create-button');
  await expect(createButton).toBeVisible();
  await createButton.click();

  const drawer = page.getByTestId('product-units-drawer');
  await expect(drawer).toBeVisible();
});
