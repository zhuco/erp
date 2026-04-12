import { expect, test, loginAndSelectTenant } from './fixtures';

test('customer segments supports create and filter flow', async ({ page }) => {
  await page.goto('/customers/segments');
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('customer-segments-title')).toBeVisible();
  await expect(page.getByTestId('customer-segments-table')).toBeVisible();
  await expect(page.locator('[role="grid"]').first()).toBeVisible();

  const segmentName = `E2E-SEG-${Date.now()}`;
  await page.getByTestId('customer-segments-create').click();

  const drawer = page.locator('.ant-drawer:visible').last();
  await expect(drawer).toBeVisible();
  await drawer.getByTestId('customer-segments-form-name').fill(segmentName);
  await drawer.getByTestId('customer-segments-form-type').click();
  await page.locator('.ant-select-dropdown:visible .ant-select-item-option').first().click();
  await drawer.getByTestId('customer-segments-form-save').click();

  await expect(page.locator('.ant-drawer:visible')).toHaveCount(0);
  await page.getByTestId('customer-segments-filter-keyword').fill(segmentName);
  await page.getByTestId('customer-segments-filter-submit').click();
  await expect(
    page.locator('[role="grid"] [role="row"]', { hasText: segmentName }).first()
  ).toBeVisible();
});

test('customer lifecycle supports create and filter flow', async ({ page }) => {
  await page.goto('/customers/lifecycle');
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('customer-lifecycle-title')).toBeVisible();
  await expect(page.getByTestId('customer-lifecycle-table')).toBeVisible();
  await expect(page.locator('[role="grid"]').first()).toBeVisible();

  const strategy = `E2E-LIFECYCLE-${Date.now()}`;
  await page.getByTestId('customer-lifecycle-create').click();

  const drawer = page.locator('.ant-drawer:visible').last();
  await expect(drawer).toBeVisible();
  await drawer.getByTestId('customer-lifecycle-form-stage').click();
  await page.locator('.ant-select-dropdown:visible .ant-select-item-option').first().click();
  await drawer.getByTestId('customer-lifecycle-form-strategy').fill(strategy);
  await drawer.getByTestId('customer-lifecycle-form-save').click();

  await expect(page.locator('.ant-drawer:visible')).toHaveCount(0);
  await page.getByTestId('customer-lifecycle-filter-keyword').fill(strategy);
  await page.getByTestId('customer-lifecycle-filter-submit').click();
  await expect(
    page.locator('[role="grid"] [role="row"]', { hasText: strategy }).first()
  ).toBeVisible();
});

test('customer loyalty and stats pages load core tables', async ({ page }) => {
  await page.goto('/customers/loyalty');
  await loginAndSelectTenant(page);

  await expect(page.locator('main .ant-tabs [role="tab"]').first()).toBeVisible();
  await expect(page.locator('main .ant-table:visible').first()).toBeVisible();
  await page.locator('main .ant-tabs [role="tab"]').nth(1).click();
  await expect(page.locator('main .ant-table:visible').first()).toBeVisible();

  await page.goto('/stats/sales');
  await expect(page.locator('.ant-card').first()).toBeVisible();
  await expect(page.locator('form .ant-btn-primary').first()).toBeVisible();
  await expect(page.locator('main .ant-table:visible').first()).toBeVisible();
});
