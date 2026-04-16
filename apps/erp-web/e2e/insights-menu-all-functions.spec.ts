import { expect, loginAndSelectTenant, test } from './fixtures';

const INSIGHTS_MODULE_NAME = '\u6d1e\u5bdf';

const insightEntries = [
  {
    linkTestId: 'module-link-insights-revenue-overview',
    path: '/insights/revenue/overview',
    pageTestId: 'insights-revenue-overview-page',
  },
  {
    linkTestId: 'module-link-insights-revenue-daily-brief',
    path: '/insights/revenue/daily-brief',
    pageTestId: 'insights-revenue-daily-brief-page',
  },
  {
    linkTestId: 'module-link-insights-revenue-trend-radar',
    path: '/insights/revenue/trend-radar',
    pageTestId: 'insights-revenue-trend-radar-page',
  },
  {
    linkTestId: 'module-link-insights-revenue-order-mix',
    path: '/insights/revenue/order-mix',
    pageTestId: 'insights-revenue-order-mix-page',
  },
  {
    linkTestId: 'module-link-insights-revenue-discount-insight',
    path: '/insights/revenue/discount-insight',
    pageTestId: 'insights-revenue-discount-insight-page',
  },
  {
    linkTestId: 'module-link-insights-fulfillment-lead-time',
    path: '/insights/fulfillment/lead-time',
    pageTestId: 'insights-fulfillment-lead-time-page',
  },
  {
    linkTestId: 'module-link-insights-fulfillment-order-cycle',
    path: '/insights/fulfillment/order-cycle',
    pageTestId: 'insights-fulfillment-order-cycle-page',
  },
  {
    linkTestId: 'module-link-insights-fulfillment-operation-ledger',
    path: '/insights/fulfillment/operation-ledger',
    pageTestId: 'insights-fulfillment-operation-ledger-page',
  },
  {
    linkTestId: 'module-link-insights-fulfillment-packaging-usage',
    path: '/insights/fulfillment/packaging-usage',
    pageTestId: 'insights-fulfillment-packaging-usage-page',
  },
  {
    linkTestId: 'module-link-insights-fulfillment-stocktake-variance',
    path: '/insights/fulfillment/stocktake-variance',
    pageTestId: 'insights-fulfillment-stocktake-variance-page',
  },
  {
    linkTestId: 'module-link-insights-fulfillment-operation-accuracy',
    path: '/insights/fulfillment/operation-accuracy',
    pageTestId: 'insights-fulfillment-operation-accuracy-page',
  },
  {
    linkTestId: 'module-link-insights-fulfillment-warehouse-network-efficiency',
    path: '/insights/fulfillment/warehouse-network-efficiency',
    pageTestId: 'insights-fulfillment-warehouse-network-efficiency-page',
  },
  {
    linkTestId: 'module-link-insights-catalog-hot-sku',
    path: '/insights/catalog/hot-sku',
    pageTestId: 'insights-catalog-hot-sku-page',
  },
  {
    linkTestId: 'module-link-insights-catalog-slow-sku',
    path: '/insights/catalog/slow-sku',
    pageTestId: 'insights-catalog-slow-sku-page',
  },
  {
    linkTestId: 'module-link-insights-catalog-bundle-combo',
    path: '/insights/catalog/bundle-combo',
    pageTestId: 'insights-catalog-bundle-combo-page',
  },
  {
    linkTestId: 'module-link-insights-inventory-stock-balance',
    path: '/insights/inventory/stock-balance',
    pageTestId: 'insights-inventory-stock-balance-page',
  },
  {
    linkTestId: 'module-link-insights-inventory-stock-aging',
    path: '/insights/inventory/stock-aging',
    pageTestId: 'insights-inventory-stock-aging-page',
  },
  {
    linkTestId: 'module-link-insights-inventory-turnover-velocity',
    path: '/insights/inventory/turnover-velocity',
    pageTestId: 'insights-inventory-turnover-velocity-page',
  },
  {
    linkTestId: 'module-link-insights-inventory-transfer-benefit',
    path: '/insights/inventory/transfer-benefit',
    pageTestId: 'insights-inventory-transfer-benefit-page',
  },
  {
    linkTestId: 'module-link-insights-inventory-negative-stock-alert',
    path: '/insights/inventory/negative-stock-alert',
    pageTestId: 'insights-inventory-negative-stock-alert-page',
  },
  {
    linkTestId: 'module-link-insights-service-return-loop',
    path: '/insights/service/return-loop',
    pageTestId: 'insights-service-return-loop-page',
  },
  {
    linkTestId: 'module-link-insights-service-compensation',
    path: '/insights/service/compensation',
    pageTestId: 'insights-service-compensation-page',
  },
  {
    linkTestId: 'module-link-insights-service-reship-quality',
    path: '/insights/service/reship-quality',
    pageTestId: 'insights-service-reship-quality-page',
  },
  {
    linkTestId: 'module-link-insights-service-ticket-efficiency',
    path: '/insights/service/ticket-efficiency',
    pageTestId: 'insights-service-ticket-efficiency-page',
  },
  {
    linkTestId: 'module-link-insights-workbench-report-center',
    path: '/insights/workbench/report-center',
    pageTestId: 'insights-workbench-report-center-page',
  },
  {
    linkTestId: 'module-link-insights-workbench-export-queue',
    path: '/insights/workbench/export-queue',
    pageTestId: 'insights-workbench-export-queue-page',
  },
] as const;

const legacyRedirectCases = [
  {
    legacyPath: '/stats/sales',
    canonicalPath: '/insights/revenue/overview',
    pageTestId: 'insights-revenue-overview-page',
  },
  {
    legacyPath: '/stats/sales-daily',
    canonicalPath: '/insights/revenue/daily-brief',
    pageTestId: 'insights-revenue-daily-brief-page',
  },
  {
    legacyPath: '/stats/sales-trends',
    canonicalPath: '/insights/revenue/trend-radar',
    pageTestId: 'insights-revenue-trend-radar-page',
  },
  {
    legacyPath: '/stats/shipping',
    canonicalPath: '/insights/fulfillment/lead-time',
    pageTestId: 'insights-fulfillment-lead-time-page',
  },
  {
    legacyPath: '/stats/sales-cycle',
    canonicalPath: '/insights/fulfillment/order-cycle',
    pageTestId: 'insights-fulfillment-order-cycle-page',
  },
  {
    legacyPath: '/stats/best-sellers',
    canonicalPath: '/insights/catalog/hot-sku',
    pageTestId: 'insights-catalog-hot-sku-page',
  },
  {
    legacyPath: '/stats/slow-moving',
    canonicalPath: '/insights/catalog/slow-sku',
    pageTestId: 'insights-catalog-slow-sku-page',
  },
  {
    legacyPath: '/reports',
    canonicalPath: '/insights/workbench/report-center',
    pageTestId: 'insights-workbench-report-center-page',
  },
  {
    legacyPath: '/exports',
    canonicalPath: '/insights/workbench/export-queue',
    pageTestId: 'insights-workbench-export-queue-page',
  },
] as const;

const escapeForRegex = (value: string) => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

const openInsightsPanel = async (page: import('@playwright/test').Page) => {
  await page.getByRole('button', { name: INSIGHTS_MODULE_NAME, exact: true }).click();
  await expect(page.getByTestId('workbench-module-panel')).toBeVisible();
};

test.describe('insights menu full coverage', () => {
  test('opens every insight menu function and lands on the expected page', async ({ page }) => {
    await page.goto('/dashboard');
    await loginAndSelectTenant(page);
    await expect(page.locator('.workbench-shell')).toBeVisible();

    for (const entry of insightEntries) {
      await openInsightsPanel(page);
      await page.getByTestId('workbench-module-panel').getByTestId(entry.linkTestId).click();

      await expect(page).toHaveURL(new RegExp(`${escapeForRegex(entry.path)}(\\?.*)?$`));
      await expect(page.getByTestId(entry.pageTestId)).toBeVisible();
    }
  });

  test('redirects every legacy stats path to the canonical insight route', async ({ page }) => {
    await page.goto('/dashboard');
    await loginAndSelectTenant(page);

    for (const redirectCase of legacyRedirectCases) {
      await page.goto(`${redirectCase.legacyPath}?legacy=1#trace`);
      await expect(page).toHaveURL(
        new RegExp(`${escapeForRegex(redirectCase.canonicalPath)}\\?legacy=1#trace$`),
      );
      await expect(page.getByTestId(redirectCase.pageTestId)).toBeVisible();
    }
  });
});
