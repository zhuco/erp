import { expect, test, loginAndSelectTenant } from './fixtures';

test('product change control supports create drawer', async ({ page }) => {
  await page.goto('/products/change-control');
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('product-change-title')).toBeVisible();
  await expect(page.getByTestId('product-change-steps')).toBeVisible();
  await expect(page.getByTestId('product-change-filter-form')).toBeVisible();

  const listTable = page.getByTestId('product-change-table');
  await expect(listTable).toBeVisible();

  const createButton = page.getByTestId('product-change-create-button');
  await expect(createButton).toBeVisible();
  await createButton.click();

  const drawer = page.getByTestId('product-change-drawer');
  await expect(drawer).toBeVisible();
  await expect(drawer.getByTestId('product-change-drawer-form')).toBeVisible();
});
