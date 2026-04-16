import { expect, test, loginAndSelectTenant } from './fixtures';
import type { Locator, Page } from '@playwright/test';

const rowLocator = (grid: Locator) =>
  grid.locator('.erp-data-grid-next__row, .erp-data-grid-row');

const orderRowLocator = (grid: Locator, orderNo: string) =>
  rowLocator(grid).filter({ hasText: orderNo }).first();

const prepareOrdersList = async (page: Page) => {
  await page.goto('/orders');
  await loginAndSelectTenant(page);
  await expect(page.getByTestId('orders-keyword-input')).toBeVisible();

  await page.getByTestId('orders-keyword-input').fill('PLAT-TEST-');
  const searchButton = page.getByTestId('orders-search-submit');
  await searchButton.scrollIntoViewIfNeeded();
  await searchButton.click({ trial: true });
  await searchButton.click();

  const grid = page.getByTestId('orders-grid');
  await expect(grid).toBeVisible();
  await expect(orderRowLocator(grid, 'TEST-ORDER-MANY-ITEMS')).toBeVisible();
  return grid;
};

test('orders list filter', async ({ page }) => {
  await page.goto('/orders');

  await loginAndSelectTenant(page);

  await expect(page.getByTestId('orders-keyword-input')).toBeVisible();

  await page.getByTestId('orders-keyword-input').fill('PLAT-TEST-');
  const searchButton = page.getByTestId('orders-search-submit');
  await searchButton.scrollIntoViewIfNeeded();
  await searchButton.click({ trial: true });
  await searchButton.click();

  const grid = page.getByTestId('orders-grid');
  await expect(grid).toBeVisible();
  await expect(orderRowLocator(grid, 'TEST-ORDER-MANY-ITEMS')).toBeVisible();
});

test('orders column filter only applies on Enter while typing keeps focus', async ({ page }) => {
  const grid = await prepareOrdersList(page);

  const platformOrderInput = grid.getByPlaceholder('网店订单号').first();
  await expect(platformOrderInput).toBeVisible();
  await expect(orderRowLocator(grid, 'TEST-ORDER-NO-LOGISTICS')).toBeVisible();

  await platformOrderInput.click();
  await platformOrderInput.fill('M');
  await page.waitForTimeout(700);

  await expect(platformOrderInput).toBeFocused();
  await expect(orderRowLocator(grid, 'TEST-ORDER-NO-LOGISTICS')).toBeVisible();

  await platformOrderInput.press('Enter');
  await expect(orderRowLocator(grid, 'TEST-ORDER-NO-LOGISTICS')).toHaveCount(0);
  await expect(orderRowLocator(grid, 'TEST-ORDER-MISSING-SKU')).toBeVisible();
});

test('orders column filter paste auto applies without Enter', async ({ page }) => {
  const grid = await prepareOrdersList(page);

  const platformOrderInput = grid.getByPlaceholder('网店订单号').first();
  await expect(platformOrderInput).toBeVisible();

  await platformOrderInput.evaluate((element, value) => {
    const input = element as HTMLInputElement;
    input.focus();
    const clipboardData = new DataTransfer();
    clipboardData.setData('text/plain', value);
    input.dispatchEvent(new ClipboardEvent('paste', { bubbles: true, clipboardData }));
  }, 'PLAT-TEST-MISSING-SKU');

  await expect(orderRowLocator(grid, 'TEST-ORDER-MANY-ITEMS')).toHaveCount(0);
  await expect(orderRowLocator(grid, 'TEST-ORDER-MISSING-SKU')).toBeVisible();
});

test('orders column filter state persists after tab switching', async ({ page }) => {
  const grid = await prepareOrdersList(page);

  const platformOrderInput = grid.getByPlaceholder('网店订单号').first();
  await expect(platformOrderInput).toBeVisible();
  await platformOrderInput.fill('PLAT-TEST-MISSING-SKU');
  await platformOrderInput.press('Enter');

  await expect(orderRowLocator(grid, 'TEST-ORDER-MANY-ITEMS')).toHaveCount(0);
  await expect(orderRowLocator(grid, 'TEST-ORDER-MISSING-SKU')).toBeVisible();

  await page.goto('/orders/review');
  await expect(page).toHaveURL(/\/orders\/review(\?.*)?$/);
  await expect(page.getByTestId('order-review-data-grid')).toBeVisible();

  const ordersTab = page.locator('.workbench-tabs [role="tab"]').filter({ hasText: '订单查询' }).first();
  if ((await ordersTab.count()) > 0) {
    await ordersTab.click();
  } else {
    await page.goto('/orders');
  }

  await expect(page).toHaveURL(/\/orders(\?.*)?$/);
  const revisitGrid = page.getByTestId('orders-grid');
  await expect(revisitGrid).toBeVisible();

  const revisitPlatformOrderInput = revisitGrid.getByPlaceholder('网店订单号').first();
  await expect(revisitPlatformOrderInput).toHaveValue('PLAT-TEST-MISSING-SKU');
  await expect(orderRowLocator(revisitGrid, 'TEST-ORDER-MANY-ITEMS')).toHaveCount(0);
  await expect(orderRowLocator(revisitGrid, 'TEST-ORDER-MISSING-SKU')).toBeVisible();
});

test('orders left stats stay on full query scope when switching left filters', async ({ page }) => {
  await page.goto('/orders');

  await loginAndSelectTenant(page);

  await expect(page.getByTestId('orders-keyword-input')).toBeVisible();

  await page.getByTestId('orders-keyword-input').fill('PLAT-TEST-');
  const searchButton = page.getByTestId('orders-search-submit');
  await searchButton.scrollIntoViewIfNeeded();
  await searchButton.click({ trial: true });
  await searchButton.click();

  const grid = page.getByTestId('orders-grid');
  const leftPanel = page.getByTestId('orders-left-panel');
  const leftStatsTotal = leftPanel.getByTestId('order-review-left-stats-total');
  const exceptionNode = leftPanel.locator('.ag-row').filter({ hasText: '异常订单' }).first();

  await expect(grid).toBeVisible();
  await expect(leftPanel).toBeVisible();
  await expect.poll(async () => (await leftStatsTotal.textContent())?.replace(/\s+/g, ' ').trim() ?? '').toContain('15');

  await exceptionNode.click();

  await expect(orderRowLocator(grid, 'TEST-ORDER-MISSING-SKU')).toBeVisible();
  await expect.poll(async () => (await leftStatsTotal.textContent())?.replace(/\s+/g, ' ').trim() ?? '').toContain('15');
});

test('orders list keeps selected row after local sort', async ({ page }) => {
  await page.goto('/orders');

  await loginAndSelectTenant(page);

  await expect(page.getByTestId('orders-keyword-input')).toBeVisible();

  await page.getByTestId('orders-keyword-input').fill('PLAT-TEST-');
  const searchButton = page.getByTestId('orders-search-submit');
  await searchButton.scrollIntoViewIfNeeded();
  await searchButton.click({ trial: true });
  await searchButton.click();

  const grid = page.getByTestId('orders-grid');
  await expect(grid).toBeVisible();

  const selectedRow = orderRowLocator(grid, 'TEST-ORDER-MANY-ITEMS');
  await selectedRow.click({ modifiers: ['Control'] });
  await expect(selectedRow).toHaveClass(/ag-row-selected/);

  await grid.locator('.ag-header-cell[col-id="id"]').first().click();

  await expect(grid.locator('.ag-row.ag-row-selected')).toHaveCount(1);
});

test('orders list right click does not change active row or selected scope', async ({ page }) => {
  await page.goto('/orders');

  await loginAndSelectTenant(page);

  await expect(page.getByTestId('orders-keyword-input')).toBeVisible();

  await page.getByTestId('orders-keyword-input').fill('PLAT-TEST-');
  const searchButton = page.getByTestId('orders-search-submit');
  await searchButton.scrollIntoViewIfNeeded();
  await searchButton.click({ trial: true });
  await searchButton.click();

  const grid = page.getByTestId('orders-grid');
  await expect(grid).toBeVisible();

  const firstRow = orderRowLocator(grid, 'TEST-ORDER-NO-LOGISTICS');
  const secondRow = orderRowLocator(grid, 'TEST-ORDER-MISSING-SKU');
  const lastRow = orderRowLocator(grid, 'TEST-ORDER-NO-ITEMS');

  await firstRow.click({ modifiers: ['Control'] });
  await expect(firstRow).toHaveClass(/ag-row-selected/);

  await secondRow.click();
  await expect(secondRow).toHaveClass(/erp-data-grid-next__row--active|erp-data-grid-row-active/);

  await lastRow.click({ button: 'right' });

  await expect(grid.locator('.ag-row.ag-row-selected')).toHaveCount(1);
  await expect(firstRow).toHaveClass(/ag-row-selected/);
  await expect(lastRow).not.toHaveClass(/ag-row-selected/);
  await expect(secondRow).toHaveClass(/erp-data-grid-next__row--active|erp-data-grid-row-active/);
});

test('orders list supports Ctrl toggle and Shift range inversion', async ({ page }) => {
  await page.goto('/orders');

  await loginAndSelectTenant(page);

  await expect(page.getByTestId('orders-keyword-input')).toBeVisible();

  await page.getByTestId('orders-keyword-input').fill('PLAT-TEST-');
  const searchButton = page.getByTestId('orders-search-submit');
  await searchButton.scrollIntoViewIfNeeded();
  await searchButton.click({ trial: true });
  await searchButton.click();

  const grid = page.getByTestId('orders-grid');
  await expect(grid).toBeVisible();

  const firstRow = orderRowLocator(grid, 'TEST-ORDER-NO-LOGISTICS');
  const secondRow = orderRowLocator(grid, 'TEST-ORDER-MISSING-SKU');
  const thirdRow = orderRowLocator(grid, 'TEST-ORDER-SHORTAGE');
  const fourthRow = orderRowLocator(grid, 'TEST-ORDER-FROZEN');

  await secondRow.click({ modifiers: ['Control'] });
  await expect(secondRow).toHaveClass(/ag-row-selected/);

  await fourthRow.click({ modifiers: ['Shift'] });
  await expect(secondRow).toHaveClass(/ag-row-selected|erp-data-grid-next__row--selected|erp-data-grid-row-selected/);
  await expect(thirdRow).toHaveClass(/ag-row-selected|erp-data-grid-next__row--selected|erp-data-grid-row-selected/);
  await expect(fourthRow).toHaveClass(/ag-row-selected|erp-data-grid-next__row--selected|erp-data-grid-row-selected/);
  await expect(firstRow).not.toHaveClass(/ag-row-selected|erp-data-grid-next__row--selected|erp-data-grid-row-selected/);

  await fourthRow.click({ modifiers: ['Shift'] });
  await expect(secondRow).not.toHaveClass(/ag-row-selected|erp-data-grid-next__row--selected|erp-data-grid-row-selected/);
  await expect(thirdRow).not.toHaveClass(/ag-row-selected|erp-data-grid-next__row--selected|erp-data-grid-row-selected/);
  await expect(fourthRow).not.toHaveClass(/ag-row-selected|erp-data-grid-next__row--selected|erp-data-grid-row-selected/);

  await firstRow.click({ modifiers: ['Control'] });
  await expect(firstRow).toHaveClass(/ag-row-selected|erp-data-grid-next__row--selected|erp-data-grid-row-selected/);

  await firstRow.click({ modifiers: ['Control'] });
  await expect(firstRow).not.toHaveClass(/ag-row-selected|erp-data-grid-next__row--selected|erp-data-grid-row-selected/);
});

test('orders detail shows items', async ({ page }) => {
  await page.goto('/orders');

  await loginAndSelectTenant(page);

  await expect(page.getByTestId('orders-keyword-input')).toBeVisible();

  await page.getByTestId('orders-keyword-input').fill('PLAT-TEST-');
  const searchButton = page.getByTestId('orders-search-submit');
  await searchButton.scrollIntoViewIfNeeded();
  await searchButton.click({ trial: true });
  await searchButton.click();

  await expect(orderRowLocator(page.getByTestId('orders-grid'), 'TEST-ORDER-MANY-ITEMS')).toBeVisible();

  await page.goto('/orders/TEST-ORDER-MANY-ITEMS');

  await expect(page).toHaveURL(/\/orders\/TEST-ORDER-MANY-ITEMS$/);
  await expect(page.getByTestId('order-items-card')).toBeVisible();
  await expect(page.getByTestId('order-items-card').locator('tbody tr').first()).toBeVisible();
});

test('orders list keeps native column settings after reload', async ({ page }) => {
  await page.goto('/orders');

  await loginAndSelectTenant(page);

  await page.evaluate(() => {
    const keys = Object.keys(window.localStorage).filter((key) =>
      key.includes('orders.query.main-grid'),
    );
    keys.forEach((key) => window.localStorage.removeItem(key));
  });

  await expect(page.getByTestId('orders-keyword-input')).toBeVisible();

  await page.getByTestId('orders-keyword-input').fill('PLAT-TEST-');
  const searchButton = page.getByTestId('orders-search-submit');
  await searchButton.scrollIntoViewIfNeeded();
  await searchButton.click({ trial: true });
  await searchButton.click();

  const grid = page.getByTestId('orders-grid');
  await expect(grid).toBeVisible();

  const orderHeader = grid.locator('.ag-header-cell[col-id="id"]').first();
  await expect(orderHeader).toBeVisible();
  const hideColumnApplied = await page.evaluate(async () => {
    const gridTestWindow = window as Window & {
      __ERP_GRID_TEST_APIS?: Record<
        string,
        {
          applyColumnState: (params: { state: Array<{ colId: string; hide?: boolean }> }) => void;
          getColumnState: () => Array<{ colId?: string | null; hide?: boolean | null }>;
        }
      >;
    };
    const api = gridTestWindow.__ERP_GRID_TEST_APIS?.['orders-grid'];
    if (!api) {
      return false;
    }

    api.applyColumnState({
      state: [{ colId: 'shop_id', hide: true }],
    });

    await new Promise((resolve) => window.setTimeout(resolve, 300));

    return api
      .getColumnState()
      .some(
        (column: { colId?: string | null; hide?: boolean | null }) =>
          column.colId === 'shop_id' && column.hide === true,
      );
  });
  expect(hideColumnApplied).toBe(true);

  await expect(grid.locator('.ag-header-cell[col-id="shop_id"]')).toHaveCount(0);

  await page.reload();
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('orders-keyword-input')).toBeVisible();
  await page.getByTestId('orders-keyword-input').fill('PLAT-TEST-');
  const searchButtonAfterReload = page.getByTestId('orders-search-submit');
  await searchButtonAfterReload.scrollIntoViewIfNeeded();
  await searchButtonAfterReload.click({ trial: true });
  await searchButtonAfterReload.click();

  await expect(page.getByTestId('orders-grid')).toBeVisible();
  await expect(page.getByTestId('orders-grid').locator('.ag-header-cell[col-id="shop_id"]')).toHaveCount(0);
});
