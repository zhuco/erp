import { expect, test, loginAndSelectTenant } from './fixtures';

test('outbound picking/packing/shipping pages render', async ({ page }) => {
  await page.goto('/wms/pick-tasks');

  await loginAndSelectTenant(page);

  await expect(page.getByTestId('pick-tasks-title')).toBeVisible();
  await expect(page.getByTestId('pick-tasks-table').getByRole('grid')).toBeVisible();

  await page.goto('/wms/pack');

  await expect(page.getByTestId('pack-title')).toBeVisible();
  await expect(page.getByTestId('pack-table').getByRole('grid')).toBeVisible();

  await page.goto('/wms/ship');

  await expect(page.getByTestId('ship-title')).toBeVisible();
  await expect(page.getByTestId('ship-table').getByRole('grid')).toBeVisible();

  await page.goto('/wms/outbound-confirm');

  await expect(page.getByTestId('outbound-confirm-page')).toBeVisible();
  await expect(page.getByTestId('outbound-confirm-input')).toBeVisible();
  await expect(page.getByTestId('outbound-confirm-recent')).toBeVisible();
});
