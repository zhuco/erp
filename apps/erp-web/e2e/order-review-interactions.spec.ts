import { expect, test, loginAndSelectTenant } from './fixtures';

test('order review keeps search focus and stable right click behavior', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const searchInput = page.getByTestId('order-review-toolbar-search-input');
  await expect(searchInput).toBeVisible();
  await searchInput.click();
  await expect(searchInput).toBeFocused();

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const rows = grid.locator('.ag-center-cols-container .ag-row.erp-data-grid-row');
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThan(0);

  const firstRowId = (await rows.nth(0).getAttribute('row-id')) ?? '';
  const firstRow = grid.locator(`.ag-center-cols-container .ag-row[row-id="${firstRowId}"]`).first();
  await firstRow.click();
  await expect(firstRow).not.toHaveClass(/erp-data-grid-row-selected/);
  await expect(firstRow).toHaveClass(/erp-data-grid-row-active/);

  const rowCount = await rows.count();
  const rightClickRowId =
    rowCount > 1 ? (await rows.nth(1).getAttribute('row-id')) ?? '' : firstRowId;
  const rightClickRow = grid
    .locator(`.ag-center-cols-container .ag-row[row-id="${rightClickRowId}"]`)
    .first();
  await rightClickRow.click({ button: 'right' });
  await expect
    .poll(
      async () =>
        grid.locator('.ag-center-cols-container .erp-data-grid-row.erp-data-grid-row-active').count(),
      { timeout: 10_000 },
    )
    .toBeLessThanOrEqual(1);
  await expect
    .poll(
      async () => grid.locator('.ag-center-cols-container .ag-row.ag-row-selected').count(),
      { timeout: 10_000 },
    )
    .toBeLessThanOrEqual(1);

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

  const rows = grid.locator('.ag-center-cols-container .ag-row.erp-data-grid-row');
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(2);

  const firstRowId = (await rows.nth(0).getAttribute('row-id')) ?? '';
  const secondRowId = (await rows.nth(1).getAttribute('row-id')) ?? '';
  const firstRow = grid.locator(`.ag-center-cols-container .ag-row[row-id="${firstRowId}"]`).first();
  const secondRow = grid.locator(`.ag-center-cols-container .ag-row[row-id="${secondRowId}"]`).first();

  await firstRow.click();
  await expect(firstRow).toHaveClass(/erp-data-grid-row-active/);

  await secondRow.click({ button: 'right' });
  await secondRow.click();
  await expect(secondRow).toHaveClass(/erp-data-grid-row-active/);
  await expect(firstRow).not.toHaveClass(/erp-data-grid-row-active/);
});

test('order review double click row opens order edit dialog', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const idCell = grid.locator('.ag-center-cols-container .ag-row .ag-cell[col-id="id"]').first();
  await expect(idCell).toBeVisible();

  await idCell.dblclick();

  await expect(page.getByRole('dialog', { name: '订单编辑' })).toBeVisible();
  await expect(page).toHaveURL(/\/orders\/review(\?.*)?$/);
});

test('order review arrow up down changes active row without checkbox selection and reloads detail', async ({
  page,
}) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  const detailLoadedId = page
    .getByTestId('order-review-page')
    .getByTestId('order-review-detail-loaded-id')
    .first();
  await expect(grid).toBeVisible();
  await expect(detailLoadedId).toBeAttached();

  const rows = grid.locator('.ag-center-cols-container .ag-row.erp-data-grid-row');
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(2);

  const firstRowId = (await rows.nth(0).getAttribute('row-id')) ?? '';
  const secondRowId = (await rows.nth(1).getAttribute('row-id')) ?? '';
  const firstRow = grid.locator(`.ag-center-cols-container .ag-row[row-id="${firstRowId}"]`).first();
  const secondRow = grid.locator(`.ag-center-cols-container .ag-row[row-id="${secondRowId}"]`).first();
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
