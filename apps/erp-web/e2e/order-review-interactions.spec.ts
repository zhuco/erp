import { expect, test, loginAndSelectTenant } from './fixtures';

test('order review keeps search focus and stable right click behavior', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const searchInput = page.getByTestId('order-review-search').locator('input');
  await expect(searchInput).toBeVisible();
  await expect(searchInput).toBeFocused();

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const rows = grid.locator('.erp-data-grid-row');
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThan(0);

  const firstRow = rows.first();
  await firstRow.click();
  await expect(firstRow).not.toHaveClass(/erp-data-grid-row-selected/);
  await expect(firstRow).toHaveClass(/erp-data-grid-row-active/);

  const rowCount = await rows.count();
  await rows.nth(rowCount > 1 ? 1 : 0).click({ button: 'right' });
  await expect(firstRow).toHaveClass(/erp-data-grid-row-active/);
  if (rowCount > 1) {
    await expect(rows.nth(1)).not.toHaveClass(/erp-data-grid-row-active/);
  }

  await page.keyboard.press('Escape');
  await page.mouse.click(10, 10);
});

test('order review right click same row still allows single left click activation', async ({
  page,
}) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const rows = grid.locator('.erp-data-grid-row');
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(2);

  const firstRow = rows.nth(0);
  const secondRow = rows.nth(1);

  await firstRow.click();
  await expect(firstRow).toHaveClass(/erp-data-grid-row-active/);

  await secondRow.click({ button: 'right' });
  await expect(firstRow).toHaveClass(/erp-data-grid-row-active/);
  await expect(secondRow).not.toHaveClass(/erp-data-grid-row-active/);

  await secondRow.click();
  await expect(secondRow).toHaveClass(/erp-data-grid-row-active/);
  await expect(firstRow).not.toHaveClass(/erp-data-grid-row-active/);
});

test('order review double click row opens order edit page', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const idCell = grid.locator('.ag-center-cols-container .ag-row .ag-cell[col-id="id"]').first();
  await expect(idCell).toBeVisible();

  await idCell.dblclick();

  await expect.poll(() => page.url(), { timeout: 20_000 }).toContain('/orders/');
  await expect(page).not.toHaveURL(/\/orders\/review/);
});

test('order review arrow up down changes active row without checkbox selection and reloads detail', async ({
  page,
}) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  const detailLoadedId = page.getByTestId('order-review-detail-loaded-id');
  await expect(grid).toBeVisible();
  await expect(detailLoadedId).toBeAttached();

  const rows = grid.locator('.ag-center-cols-container .ag-row.erp-data-grid-row');
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(2);

  const firstRow = rows.nth(0);
  const secondRow = rows.nth(1);
  const firstIdCell = firstRow.locator('.ag-cell[col-id="id"]').first();
  const firstId = ((await firstIdCell.textContent()) ?? '').trim();
  const secondId = ((await secondRow.locator('.ag-cell[col-id="id"]').first().textContent()) ?? '').trim();

  await firstIdCell.click();
  await expect(firstRow).toHaveClass(/erp-data-grid-row-active/);
  await expect(firstRow).not.toHaveClass(/ag-row-selected/);
  await expect.poll(async () => ((await detailLoadedId.textContent()) ?? '').trim()).toBe(firstId);

  await firstIdCell.press('ArrowDown');

  await expect(secondRow).toHaveClass(/erp-data-grid-row-active/);
  await expect(firstRow).not.toHaveClass(/erp-data-grid-row-active/);
  await expect(secondRow).not.toHaveClass(/ag-row-selected/);
  await expect.poll(async () => ((await detailLoadedId.textContent()) ?? '').trim()).toBe(secondId);
});

