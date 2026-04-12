import { expect, loginAndSelectTenant, test } from './fixtures';

test('wms site execution chain works for labor/slotting/replenishment/dock', async ({ page }) => {
  await page.goto('/wms/labor');
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('wms-labor-title')).toBeVisible();
  await expect(page.getByTestId('wms-labor-table').locator('table')).toBeVisible();
  const laborAssign = page.getByTestId('wms-labor-assign').first();
  if (await laborAssign.isVisible()) {
    await laborAssign.click();
    await page.getByTestId('wms-labor-start').first().click();
    await page.getByTestId('wms-labor-complete').first().click();
  }

  await page.goto('/wms/slotting');
  await expect(page.getByTestId('wms-slotting-title')).toBeVisible();
  await expect(page.getByTestId('wms-slotting-table').locator('table')).toBeVisible();
  const slottingApply = page.getByTestId('wms-slotting-apply').first();
  if (await slottingApply.isVisible()) {
    await slottingApply.click();
  }

  await page.goto('/wms/replenishment');
  await expect(page.getByTestId('wms-replenishment-title')).toBeVisible();
  await expect(page.getByTestId('wms-replenishment-table').locator('table')).toBeVisible();
  const replenishmentClaim = page.getByTestId('wms-replenishment-claim').first();
  if (await replenishmentClaim.isVisible()) {
    await replenishmentClaim.click();
    await page.getByTestId('wms-replenishment-complete').first().click();
  }

  await page.goto('/wms/dock-yard');
  await expect(page.getByTestId('wms-dock-title')).toBeVisible();
  await expect(page.getByTestId('wms-dock-table').locator('table')).toBeVisible();
  const dockCheckin = page.getByTestId('wms-dock-checkin').first();
  if (await dockCheckin.isVisible()) {
    await dockCheckin.click();
    await page.getByTestId('wms-dock-loading').first().click();
    await page.getByTestId('wms-dock-depart').first().click();
  }
});
