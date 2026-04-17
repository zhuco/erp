import { loginAndSelectTenant, test } from './fixtures';
import { expectNoWorkbenchA11yViolations } from './utils/a11y';

const assertWorkbenchA11y = async (
  page: import('@playwright/test').Page,
  route: string,
  readySelector: string,
  exclude: string[] = [],
) => {
  await page.goto(route);
  await loginAndSelectTenant(page);
  await page.goto(route);
  await page.locator(readySelector).first().waitFor({ state: 'visible' });
  await expectNoWorkbenchA11yViolations(page, {
    include: ['.workbench-shell'],
    exclude,
  });
};

test.describe('workbench accessibility gate', () => {
  test('dashboard workbench has no blocking accessibility violations', async ({ page }) => {
    await assertWorkbenchA11y(page, '/dashboard', '.workbench-shell');
  });

  test('order review workbench has no blocking accessibility violations', async ({ page }) => {
    await assertWorkbenchA11y(
      page,
      '/orders/review',
      '[data-testid="order-review-data-grid"]',
      ['.order-review-floating-filter-date-picker-anchor'],
    );
  });

  test('order query workbench has no blocking accessibility violations', async ({ page }) => {
    await assertWorkbenchA11y(page, '/orders', '[data-testid="orders-grid"]');
  });
});
