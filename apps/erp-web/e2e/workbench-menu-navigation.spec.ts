import { expect, loginAndSelectTenant, test } from './fixtures';

const findOrderReviewMenuItem = async (page: import('@playwright/test').Page) => {
  const moduleButtons = page.locator('.workbench-module-item');
  const reviewMenuItem = page
    .locator(
      '.ant-dropdown:visible [data-menu-id*="/orders/review"], .ant-dropdown:visible [data-key="/orders/review"]'
    )
    .first();
  const count = await moduleButtons.count();

  for (let index = 0; index < count; index += 1) {
    await page.keyboard.press('Escape');
    const button = moduleButtons.nth(index);
    if (await button.isDisabled()) {
      continue;
    }

    await button.click();

    try {
      await reviewMenuItem.waitFor({ state: 'visible', timeout: 500 });
      return reviewMenuItem;
    } catch {
      // continue scanning modules
    }
  }

  return null;
};

test.describe('workbench menu navigation', () => {
  test('can navigate to order review page from module menu', async ({ page }) => {
    await page.goto('/dashboard');
    await loginAndSelectTenant(page);
    await expect(page.locator('.workbench-shell')).toBeVisible();

    const reviewMenuItem = await findOrderReviewMenuItem(page);
    expect(reviewMenuItem).not.toBeNull();
    if (!reviewMenuItem) {
      return;
    }

    await reviewMenuItem.click();

    await expect(page).toHaveURL(/\/orders\/review(\?.*)?$/);
    await expect(page.getByTestId('order-review-data-grid')).toBeVisible();
  });

  test('can navigate to order review page from strategy preview', async ({ page }) => {
    await page.goto('/system/strategy-layout-preview');
    await loginAndSelectTenant(page);
    await page.goto('/system/strategy-layout-preview');
    await expect(page.getByTestId('strategy-layout-preview-page')).toBeVisible();

    const reviewMenuItem = await findOrderReviewMenuItem(page);
    expect(reviewMenuItem).not.toBeNull();
    if (!reviewMenuItem) {
      return;
    }

    await reviewMenuItem.click();

    await expect(page).toHaveURL(/\/orders\/review(\?.*)?$/);
    await expect(page.getByTestId('order-review-data-grid')).toBeVisible();
  });
});
