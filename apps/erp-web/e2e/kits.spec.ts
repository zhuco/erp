import { expect, test, loginAndSelectTenant } from './fixtures';
import { selectAntdOptionByText } from './utils/select';

test('kit editor add component', async ({ page }) => {
  await page.goto('/kits');

  await loginAndSelectTenant(page);

  await expect(page.getByTestId('kits-title')).toBeVisible();

  const editButton = page.getByTestId('kit-edit-link').first();
  await expect(editButton).toBeVisible();
  await editButton.click();

  await expect(page.getByTestId('kit-editor-title')).toBeVisible();

  const componentSelect = page.getByTestId('kit-component-select');
  await selectAntdOptionByText(page, componentSelect, /SKU-ERP-1004/);

  const qtyInput = page.getByTestId('kit-component-qty');
  await qtyInput.fill('2');

  await page.getByTestId('kit-component-add').click();

  await expect(page.getByText('SKU-ERP-1004')).toBeVisible();
});
