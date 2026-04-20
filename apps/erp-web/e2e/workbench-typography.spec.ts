import { expect, loginAndSelectTenant, test } from './fixtures';

test('workbench body typography comes from the shared 12.8px body scale', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const pageRoot = page.getByTestId('order-review-page');
  const grid = page.getByTestId('order-review-data-grid');

  await expect(pageRoot).toBeVisible();
  await expect(grid).toBeVisible();

  const metrics = await page.evaluate(() => {
    const root = document.documentElement;
    const pageElement = document.querySelector('[data-testid="order-review-page"]');
    const gridElement = document.querySelector('[data-testid="order-review-data-grid"]');
    const gridRoot = gridElement?.querySelector('.ag-root-wrapper') ?? gridElement;
    const leftFilterLabel =
      document.querySelector('.order-review-left-ag-label') ??
      document.querySelector('.order-review-quick-filter-button');

    return {
      bodyVar: getComputedStyle(root).getPropertyValue('--erp-font-size-body').trim(),
      pageFontSize: pageElement ? getComputedStyle(pageElement).fontSize : '',
      gridFontSize: gridRoot ? getComputedStyle(gridRoot).fontSize : '',
      leftFilterFontSize: leftFilterLabel ? getComputedStyle(leftFilterLabel).fontSize : '',
    };
  });

  expect(metrics.bodyVar).toBe('12.8px');
  expect(metrics.pageFontSize).toBe('12.8px');
  expect(metrics.gridFontSize).toBe('12.8px');
  expect(metrics.leftFilterFontSize).toBe('12.8px');
});
