import type { Page } from '@playwright/test';

import { expect, loginAndSelectTenant, test } from './fixtures';

const delayFirstMatchedRequest = async (page: Page, pattern: RegExp, delayMs = 600) => {
  let delayed = false;
  await page.route(pattern, async (route) => {
    if (!delayed) {
      delayed = true;
      await new Promise((resolve) => setTimeout(resolve, delayMs));
    }
    await route.continue();
  });
};

const expectFirstLazyLoadToRecover = async (page: Page, options: {
  path: string;
  modulePattern: RegExp;
  readyLocator: ReturnType<Page['locator']>;
}) => {
  await delayFirstMatchedRequest(page, options.modulePattern);
  await page.goto(options.path);
  await loginAndSelectTenant(page);

  const loading = page.getByTestId('route-page-loading');
  await expect(loading).toBeVisible();
  await expect(loading).toBeHidden();
  await expect(page).toHaveURL(new RegExp(`${options.path.replace(/\//g, '\\/')}(\\?.*)?$`));
  await expect(options.readyLocator).toBeVisible();
  await expect(page.locator('[data-workbench-route-status="ready"]')).toBeVisible();
};

test.describe('frontend route observability', () => {
  test('order review first lazy load recovers without reopening tabs', async ({ page }) => {
    await expectFirstLazyLoadToRecover(page, {
      path: '/orders/review',
      modulePattern: /OrderReviewPage\.(tsx|js)(\?.*)?$/,
      readyLocator: page.getByTestId('order-review-data-grid'),
    });
  });

  test('order exceptions first lazy load recovers without reopening tabs', async ({ page }) => {
    await expectFirstLazyLoadToRecover(page, {
      path: '/orders/exceptions',
      modulePattern: /OrderExceptionsPage\.(tsx|js)(\?.*)?$/,
      readyLocator: page.getByTestId('order-exceptions-page'),
    });
  });

  test('channels first lazy load recovers without reopening tabs', async ({ page }) => {
    await expectFirstLazyLoadToRecover(page, {
      path: '/channels',
      modulePattern: /ChannelsPage\.(tsx|js)(\?.*)?$/,
      readyLocator: page.getByTestId('channels-page'),
    });
  });

  test('orders list first lazy load recovers without reopening tabs', async ({ page }) => {
    await expectFirstLazyLoadToRecover(page, {
      path: '/orders',
      modulePattern: /OrdersPage\.(tsx|js)(\?.*)?$/,
      readyLocator: page.getByTestId('orders-grid'),
    });
  });

  test('after-sales first lazy load recovers without reopening tabs', async ({ page }) => {
    await expectFirstLazyLoadToRecover(page, {
      path: '/after-sales',
      modulePattern: /AfterSalesListPage\.(tsx|js)(\?.*)?$/,
      readyLocator: page.getByTestId('after-sales-title'),
    });
  });

  test('observability first lazy load recovers without reopening tabs', async ({ page }) => {
    await expectFirstLazyLoadToRecover(page, {
      path: '/system/observability',
      modulePattern: /ObservabilityPage\.(tsx|js)(\?.*)?$/,
      readyLocator: page.getByTestId('observability-stats-infra-health'),
    });
  });

  test('observability page shows backend route telemetry after navigating across routes', async ({
    page,
  }) => {
    await delayFirstMatchedRequest(page, /OrderReviewPage\.(tsx|js)(\?.*)?$/);
    await page.goto('/orders/review');
    await loginAndSelectTenant(page);
    await expect(page.getByTestId('route-page-loading')).toBeVisible();
    await expect(page.getByTestId('route-page-loading')).toBeHidden();
    await expect(page.getByTestId('order-review-data-grid')).toBeVisible();

    await page.goto('/system/observability');
    await expect(page.getByTestId('observability-frontend-route-efficiency')).toBeVisible();
    await expect(page.getByTestId('observability-frontend-route-backend-summary')).toBeVisible();
    await expect(page.getByTestId('observability-frontend-route-backend-efficiency')).toBeVisible();
    await expect(page.getByTestId('observability-frontend-route-backend-table')).toContainText(
      '/orders/review',
    );
  });
});
