import { expect, test, loginAndSelectTenant } from './fixtures';

test('planning forecast and replenishment can generate purchase plans', async ({ page }) => {
  await page.goto('/planning/forecast');
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('planning-forecast-page')).toBeVisible();
  await page.getByLabel('SKU ID').fill('00000000-0000-0000-0000-000000000101');
  await page.getByLabel('SKU Code').fill('SKU-PLAN-E2E-01');
  await page.getByLabel('Warehouse ID').fill('00000000-0000-0000-0000-000000000201');
  await page.getByLabel('Forecast Qty').fill('360');
  await page.getByTestId('planning-forecast-save').click();
  await expect(page.getByText('Forecast saved')).toBeVisible();

  await page.goto('/planning/replenishment-rules');
  await expect(page.getByTestId('planning-replenishment-page')).toBeVisible();
  await page.getByLabel('Warehouse ID').fill('00000000-0000-0000-0000-000000000201');
  await page.getByLabel('Category').fill('BASIC');
  await page.getByLabel('Safety Days').fill('14');
  await page.getByLabel('Cycle Days').fill('7');
  await page.getByLabel('Min Qty').fill('120');
  await page.getByTestId('planning-replenishment-save').click();
  await expect(page.getByText('Replenishment rule saved')).toBeVisible();

  await page.getByTestId('planning-generate-purchase').click();
  await expect(page.getByText(/Generated \d+ purchase plan\(s\)/)).toBeVisible();

  await page.goto('/planning/purchase-plans');
  await expect(page.getByTestId('planning-purchase-page')).toBeVisible();
  await expect(page.getByTestId('planning-purchase-generate')).toBeVisible();
  await expect(page.getByTestId('planning-purchase-table')).toBeVisible();
});

test('planning transfer promo and execution pages are API-driven', async ({ page }) => {
  await page.goto('/planning/transfer-suggestions');
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('planning-transfer-page')).toBeVisible();
  await page.getByTestId('planning-transfer-generate').click();
  await expect(page.getByText(/Generated \d+ transfer suggestion\(s\)/)).toBeVisible();

  await page.goto('/planning/promo-stocking');
  await expect(page.getByTestId('planning-promo-page')).toBeVisible();
  await page.getByLabel('Campaign Name').fill('E2E Promo Campaign');
  await page.getByTestId('planning-promo-generate').click();
  await expect(page.getByText('Promo stocking campaign generated')).toBeVisible();

  await page.goto('/planning/plan-execution');
  await expect(page.getByTestId('planning-execution-page')).toBeVisible();
  await expect(page.getByTestId('planning-execution-table')).toBeVisible();
});
