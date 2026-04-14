import { expect, test, loginAndSelectTenant } from './fixtures';

test('order review filters quickly by order id', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const rows = grid.locator('.ag-center-cols-container .ag-row.erp-data-grid-row');
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThan(0);
  await expect(rows.first()).toBeVisible();
  const initialCount = await rows.count();

  const orderIdCell = rows.first().locator('.ag-cell[col-id="id"]');
  await expect(orderIdCell).toBeVisible();
  const orderId = (await orderIdCell.textContent())?.trim() ?? '';
  expect(orderId).not.toBe('');

  const orderInput = grid.getByTestId('order-review-floating-filter-id');
  await expect(orderInput).toBeVisible();
  await orderInput.fill(orderId);
  await expect(orderInput).toHaveValue(orderId);

  const visibleOrderIds = grid.locator(
    '.ag-center-cols-container .ag-row.erp-data-grid-row .ag-cell[col-id="id"]',
  );
  await expect
    .poll(async () => {
      const values = (await visibleOrderIds.allTextContents())
        .map((text) => text.trim())
        .filter(Boolean);
      return values.length > 0 && values.every((text) => text.includes(orderId));
    })
    .toBe(true);

  await expect.poll(async () => rows.count()).toBeLessThanOrEqual(initialCount);
});

test('order review clears active target after filtering active row out of view', async ({
  page,
}) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const rows = grid.locator('.ag-center-cols-container .ag-row.erp-data-grid-row');
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(2);

  const firstRow = rows.nth(0);
  const secondRow = rows.nth(1);

  const secondRowOrderIdCell = secondRow.locator('.ag-cell[col-id="id"]');
  await expect(secondRowOrderIdCell).toBeVisible();
  const secondRowOrderId = (await secondRowOrderIdCell.textContent())?.trim() ?? '';
  expect(secondRowOrderId).not.toBe('');

  await firstRow.click();
  await expect(firstRow).toHaveClass(/erp-data-grid-row-active/);
  await expect(page.getByTestId('order-review-approve')).toBeEnabled();

  const orderInput = grid.getByTestId('order-review-floating-filter-id');
  await expect(orderInput).toBeVisible();
  await orderInput.fill(secondRowOrderId);
  await expect(orderInput).toHaveValue(secondRowOrderId);

  const visibleOrderIds = grid.locator(
    '.ag-center-cols-container .ag-row.erp-data-grid-row .ag-cell[col-id="id"]',
  );
  await expect
    .poll(async () => {
      const values = (await visibleOrderIds.allTextContents())
        .map((text) => text.trim())
        .filter(Boolean);
      return values.length > 0 && values.every((text) => text.includes(secondRowOrderId));
    })
    .toBe(true);

  await expect(page.getByTestId('order-review-approve')).toBeDisabled();
});

