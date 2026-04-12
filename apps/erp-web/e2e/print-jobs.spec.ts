import { expect, test, loginAndSelectTenant } from './fixtures';

test('create print job and list', async ({ page }) => {
  await page.goto('/orders');

  await loginAndSelectTenant(page);

  await page.getByTestId('order-detail-link').first().click();
  await expect(page).toHaveURL(/\/orders\//);
  const shipTrigger = page.getByTestId('order-detail-ship');
  await expect(shipTrigger).toBeVisible();
  await shipTrigger.click();

  await expect(page.getByTestId('ship-drawer')).toBeVisible();
  await page.getByTestId('ship-carrier-input').fill('SF');
  await page.getByTestId('ship-tracking-input').fill('TRACK-PRINT-001');
  await page.getByTestId('ship-submit').click();

  await expect(page.getByTestId('ship-status-card')).toBeVisible();

  const printButton = page.getByTestId('ship-print-label');
  await expect(printButton).toBeVisible();
  await printButton.click();
  await expect(printButton).toBeEnabled();
  await expect(page.locator('.ant-message')).toContainText(/打印任务/);

  const shipDrawer = page.getByTestId('ship-drawer');
  const drawerClose = shipDrawer.locator('.ant-drawer-close');
  if ((await drawerClose.count()) > 0) {
    await drawerClose.click();
  } else {
    await page.keyboard.press('Escape');
  }
  await expect(shipDrawer).toBeHidden();

  await page.goto('/print-jobs');
  await expect(page.getByTestId('print-jobs-title')).toBeVisible();
  await expect(page.locator('.ant-table-tbody tr').first()).toBeVisible();
});
