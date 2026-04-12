import { expect, test, loginAndSelectTenant } from './fixtures';

test('ship order from detail drawer', async ({ page }) => {
  await page.goto('/orders');

  await loginAndSelectTenant(page);

  await page.getByTestId('order-detail-link').first().click();

  const shipTrigger = page.getByTestId('order-detail-ship');
  await expect(shipTrigger).toBeVisible();
  await shipTrigger.click();

  const shipDrawer = page.getByTestId('ship-drawer');
  await expect(shipDrawer).toBeVisible();
  await page.getByTestId('ship-carrier-input').fill('SF');
  await page.getByTestId('ship-tracking-input').fill('TRACK-TEST-001');

  const idempotencyInput = shipDrawer.getByTestId('ship-idempotency-input');
  await expect(idempotencyInput).toHaveValue(/.+/);

  await page.getByTestId('ship-submit').click();

  await expect(page.getByTestId('ship-status-card')).toBeVisible();
  await expect(page.getByText('REQUESTED')).toBeVisible();
});

test('failed shipment retry action visible', async ({ page }) => {
  await page.goto('/shipments');

  await loginAndSelectTenant(page);

  const retryButton = page.getByTestId('shipment-retry-button').first();
  await expect(retryButton).toBeVisible();
  await expect(retryButton).toBeEnabled();
});
