import { expect, test, loginAndSelectTenant } from './fixtures';

test('login selects tenant then opens orders', async ({ page }) => {
  await page.goto('/orders');

  await loginAndSelectTenant(page);

  await expect(page.getByTestId('orders-keyword-input')).toBeVisible();
});
