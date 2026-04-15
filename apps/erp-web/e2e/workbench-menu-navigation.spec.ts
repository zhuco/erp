import { expect, loginAndSelectTenant, test } from './fixtures';

const SALES_MODULE_NAME = '\u9500\u552e';
const ORDER_CENTER_LABEL = '\u8ba2\u5355\u4e2d\u5fc3';
const SEARCH_PLACEHOLDER = '\u641c\u529f\u80fd';
const ORDER_REVIEW_KEYWORD = '\u5355\u5ba1';
const RECENT_LABEL = '\u6700\u8fd1\u4f7f\u7528';
const NO_MATCH_KEYWORD = '\u4e0d\u5b58\u5728\u7684\u529f\u80fd\u5165\u53e3';

const openModulePanel = async (page: import('@playwright/test').Page, moduleName: string) => {
  await page.getByRole('button', { name: moduleName, exact: true }).click();
  await expect(page.getByTestId('workbench-module-panel')).toBeVisible();
};

test.describe('workbench menu navigation', () => {
  test('can open order review from the dashboard role workbench', async ({ page }) => {
    await page.goto('/dashboard');
    await loginAndSelectTenant(page);
    await expect(page.locator('.workbench-shell')).toBeVisible();

    await expect(page.getByTestId('dashboard-role-card-review')).toBeVisible();
    await page.getByTestId('dashboard-role-primary-review').click();

    await expect(page).toHaveURL(/\/orders\/review(\?.*)?$/);
    await expect(page.getByTestId('order-review-data-grid')).toBeVisible();
  });

  test('can navigate to order review page from sales module groups', async ({ page }) => {
    await page.goto('/dashboard');
    await loginAndSelectTenant(page);
    await expect(page.locator('.workbench-shell')).toBeVisible();

    await openModulePanel(page, SALES_MODULE_NAME);
    await expect(page.getByTestId('workbench-module-panel').getByText(ORDER_CENTER_LABEL)).toBeVisible();

    await page
      .getByTestId('workbench-module-panel')
      .getByTestId('module-link-orders-review')
      .click();

    await expect(page).toHaveURL(/\/orders\/review(\?.*)?$/);
    await expect(page.getByTestId('order-review-data-grid')).toBeVisible();
  });

  test('can navigate to order review page from integration center', async ({ page }) => {
    await page.goto('/integrations/center');
    await loginAndSelectTenant(page);
    await page.goto('/integrations/center');
    await expect(page.getByTestId('system-integrations-title')).toBeVisible();

    await openModulePanel(page, SALES_MODULE_NAME);
    await page
      .getByTestId('workbench-module-panel')
      .getByTestId('module-link-orders-review')
      .click();

    await expect(page).toHaveURL(/\/orders\/review(\?.*)?$/);
    await expect(page.getByTestId('order-review-data-grid')).toBeVisible();
  });

  test('can open the shop order query experience from the sales module panel', async ({ page }) => {
    await page.goto('/dashboard');
    await loginAndSelectTenant(page);
    await expect(page.locator('.workbench-shell')).toBeVisible();

    await openModulePanel(page, SALES_MODULE_NAME);
    await page
      .getByTestId('workbench-module-panel')
      .getByTestId('module-link-sales-shop-order-download')
      .click();

    await expect(page).toHaveURL(/\/sales\/shop-order-download(\?.*)?$/);
    await expect(page.locator('.shop-order-query-page')).toBeVisible();
  });

  test('can navigate to order review page from function search', async ({ page }) => {
    await page.goto('/dashboard');
    await loginAndSelectTenant(page);
    await expect(page.locator('.workbench-shell')).toBeVisible();

    const searchInput = page.getByPlaceholder(SEARCH_PLACEHOLDER);
    await searchInput.fill(ORDER_REVIEW_KEYWORD);
    await page.getByTestId('function-search-option-orders-review').click();

    await expect(page).toHaveURL(/\/orders\/review(\?.*)?$/);
    await expect(page.getByTestId('order-review-data-grid')).toBeVisible();
  });

  test('can jump to the first matched function by clicking the search button', async ({ page }) => {
    await page.goto('/dashboard');
    await loginAndSelectTenant(page);
    await expect(page.locator('.workbench-shell')).toBeVisible();

    await page.getByPlaceholder(SEARCH_PLACEHOLDER).fill(ORDER_REVIEW_KEYWORD);
    await page.getByTestId('workbench-function-search-button').click();

    await expect(page).toHaveURL(/\/orders\/review(\?.*)?$/);
    await expect(page.getByTestId('order-review-data-grid')).toBeVisible();
  });

  test('shows recent function recommendations when focusing the empty search box', async ({
    page,
  }) => {
    await page.goto('/orders/review');
    await loginAndSelectTenant(page);
    await expect(page.getByTestId('order-review-data-grid')).toBeVisible();

    await page.goto('/dashboard');
    await expect(page.locator('.workbench-shell')).toBeVisible();

    await page.getByPlaceholder(SEARCH_PLACEHOLDER).click();
    await expect(page.getByTestId('function-search-option-orders-review')).toBeVisible();
    await expect(
      page
        .locator('.workbench-function-search-dropdown .ant-select-item-group')
        .filter({ hasText: RECENT_LABEL }),
    ).toBeVisible();
    await expect(page.getByTestId('function-search-source-orders-review')).toContainText(
      RECENT_LABEL,
    );
  });

  test('groups matched functions in the search dropdown and supports keyboard selection', async ({
    page,
  }) => {
    await page.goto('/dashboard');
    await loginAndSelectTenant(page);
    await expect(page.locator('.workbench-shell')).toBeVisible();

    const searchInput = page.getByPlaceholder(SEARCH_PLACEHOLDER);
    await searchInput.fill(ORDER_REVIEW_KEYWORD);

    await expect(
      page
        .locator('.workbench-function-search-dropdown .ant-select-item-group')
        .filter({ hasText: SALES_MODULE_NAME }),
    ).toBeVisible();

    await searchInput.press('ArrowDown');
    await searchInput.press('Enter');

    await expect(page).toHaveURL(/\/orders\/review(\?.*)?$/);
    await expect(page.getByTestId('order-review-data-grid')).toBeVisible();
  });

  test('uses a borderless dark shell and highlights the active function in the module panel', async ({
    page,
  }) => {
    await page.goto('/orders/review');
    await loginAndSelectTenant(page);
    await expect(page.getByTestId('order-review-data-grid')).toBeVisible();

    await openModulePanel(page, SALES_MODULE_NAME);

    const panelStyle = await page.getByTestId('workbench-module-panel').evaluate((element) => {
      const container = element.parentElement?.parentElement as HTMLElement | null;
      if (!container) {
        return {
          borderTopWidth: 'missing',
          boxShadow: 'missing',
          padding: 'missing',
          backgroundColor: 'missing',
        };
      }

      const style = window.getComputedStyle(container);
      return {
        borderTopWidth: style.borderTopWidth,
        boxShadow: style.boxShadow,
        padding: style.padding,
        backgroundColor: style.backgroundColor,
      };
    });

    expect(panelStyle.borderTopWidth).toBe('0px');
    expect(panelStyle.boxShadow).toBe('none');
    expect(panelStyle.padding).toBe('0px');
    expect(panelStyle.backgroundColor).toBe('rgb(31, 38, 48)');
    await expect(
      page.getByTestId('workbench-module-panel').getByTestId('module-link-orders-review'),
    ).toHaveAttribute('aria-current', 'page');
  });

  test('closes the module panel when pressing Escape', async ({ page }) => {
    await page.goto('/dashboard');
    await loginAndSelectTenant(page);
    await expect(page.locator('.workbench-shell')).toBeVisible();

    await openModulePanel(page, SALES_MODULE_NAME);
    await page.keyboard.press('Escape');

    await expect(page.getByTestId('workbench-module-panel')).toBeHidden();
  });

  test('closes the function search dropdown when pressing Escape', async ({ page }) => {
    await page.goto('/dashboard');
    await loginAndSelectTenant(page);
    await expect(page.locator('.workbench-shell')).toBeVisible();

    const searchInput = page.getByPlaceholder(SEARCH_PLACEHOLDER);
    await searchInput.fill(ORDER_REVIEW_KEYWORD);
    await expect(page.getByTestId('function-search-option-orders-review')).toBeVisible();

    await page.keyboard.press('Escape');

    await expect(page.getByTestId('function-search-option-orders-review')).toBeHidden();
  });

  test('focuses the function search when pressing Control+K', async ({ page }) => {
    await page.goto('/dashboard');
    await loginAndSelectTenant(page);
    await expect(page.locator('.workbench-shell')).toBeVisible();

    const searchInput = page.getByPlaceholder(SEARCH_PLACEHOLDER);
    await page.keyboard.press('Control+K');

    await expect(searchInput).toBeFocused();
  });

  test('shows an empty-state hint when no function matches the search keyword', async ({
    page,
  }) => {
    await page.goto('/dashboard');
    await loginAndSelectTenant(page);
    await expect(page.locator('.workbench-shell')).toBeVisible();

    await page.getByPlaceholder(SEARCH_PLACEHOLDER).fill(NO_MATCH_KEYWORD);
    await expect(page.getByTestId('function-search-empty')).toBeVisible();
  });

  test('recovers from the first lazy route load without requiring the tab to be reopened', async ({
    page,
  }) => {
    let delayedRequestHandled = false;
    await page.route(/OrderReviewPage\.(tsx|js)(\?.*)?$/, async (route) => {
      if (!delayedRequestHandled) {
        delayedRequestHandled = true;
        await new Promise((resolve) => setTimeout(resolve, 600));
      }
      await route.continue();
    });

    await page.goto('/dashboard');
    await loginAndSelectTenant(page);
    await expect(page.locator('.workbench-shell')).toBeVisible();

    await page.getByTestId('dashboard-role-primary-review').click();

    const routeLoading = page.getByTestId('route-page-loading');
    await expect(routeLoading).toBeVisible();
    await expect(routeLoading).toBeHidden();
    await expect(page).toHaveURL(/\/orders\/review(\?.*)?$/);
    await expect(page.getByTestId('order-review-data-grid')).toBeVisible();
  });

  test('opens key first-visit routes without stalling on the loading fallback', async ({ page }) => {
    const cases = [
      { path: '/channels', readyTestId: 'channels-page' },
      { path: '/system/observability', readyTestId: 'observability-frontend-route-summary' },
      { path: '/after-sales', readyTestId: 'after-sales-title' },
    ];

    for (const testCase of cases) {
      await page.goto(testCase.path);
      await loginAndSelectTenant(page);
      await page.goto(testCase.path);

      await expect(page.getByTestId('route-page-loading')).toBeHidden();
      await expect(page.getByTestId(testCase.readyTestId)).toBeVisible();
    }
  });
});
