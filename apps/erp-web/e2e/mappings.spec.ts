import { expect, test, loginAndSelectTenant } from './fixtures';
import { selectAntdOptionByText } from './utils/select';

test('order mapping missing flow', async ({ page }) => {
  await page.goto('/orders');
  await loginAndSelectTenant(page);

  const detailButton = page.getByTestId('order-detail-link').first();
  await expect(detailButton).toBeVisible();
  await detailButton.click();

  const mappingButton = page.getByTestId('order-item-mapping-link').first();
  await expect(mappingButton).toBeVisible();
  await mappingButton.click();

  await expect(page).toHaveURL(/\/mappings/);
  await expect(page.getByTestId('mapping-platform-sku')).toHaveValue('PLAT-SKU-100');
  await expect(page.getByTestId('mapping-outer-sku')).toHaveValue('OUTER-200');
  await expect(page.getByTestId('mapping-shop-id')).toHaveValue(/SHOP-/);

  const skuSelect = page.getByTestId('mapping-sku-select');
  await selectAntdOptionByText(page, skuSelect, /SKU-ERP-/);
  await page.getByTestId('mapping-save-button').click();

  await expect(page).toHaveURL(/\/orders\//);
  await expect(page.getByTestId('order-item-mapping-link').first()).toBeVisible();
});
