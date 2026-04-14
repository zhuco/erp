import { expect, test, loginAndSelectTenant } from './fixtures';

const getCenterRows = (grid: any) => grid.locator('.ag-center-cols-container .ag-row.erp-data-grid-row');
const getSelectionCell = (grid: any, rowId: string) =>
  grid.locator(`.ag-center-cols-container .ag-row[row-id="${rowId}"] .ag-cell[col-id="__erpSelection"]`).first();
const getSelectionVisual = (grid: any, rowId: string) =>
  grid
    .locator(
      `.ag-center-cols-container .ag-row[row-id="${rowId}"] .ag-cell[col-id="__erpSelection"] .erp-selection-checkbox-visual`,
    )
    .first();
const getHeaderCell = (grid: any, colId: string) =>
  grid.locator(`.ag-header-row-column .ag-header-cell[col-id="${colId}"]`).first();
const getCellText = async (grid: any, rowIndex: number, colId: string) =>
  (((await getCenterRows(grid).nth(rowIndex).locator(`.ag-cell[col-id="${colId}"]`).first().textContent()) ?? '').trim());

const getRowId = async (row: any) => (await row.getAttribute('row-id')) ?? '';
const clickSelectionCheckboxCenter = async (page: any, grid: any, rowId: string, modifiers: Array<'Shift' | 'Control'> = []) => {
  const cell = getSelectionCell(grid, rowId);
  const box = await cell.boundingBox();
  expect(box).not.toBeNull();
  for (const modifier of modifiers) {
    await page.keyboard.down(modifier);
  }
  try {
    await page.mouse.click(box!.x + box!.width / 2, box!.y + box!.height / 2);
  } finally {
    for (const modifier of modifiers.slice().reverse()) {
      await page.keyboard.up(modifier);
    }
  }
};
const clickSelectionCheckboxHotArea = async (page: any, grid: any, rowId: string, modifiers: Array<'Shift' | 'Control'> = []) => {
  const cell = getSelectionCell(grid, rowId);
  const box = await cell.boundingBox();
  expect(box).not.toBeNull();
  for (const modifier of modifiers) {
    await page.keyboard.down(modifier);
  }
  try {
    await page.mouse.click(box!.x + Math.min(box!.width / 2, 12), box!.y + box!.height / 2);
  } finally {
    for (const modifier of modifiers.slice().reverse()) {
      await page.keyboard.up(modifier);
    }
  }
};
test('order review row click keeps only one active row and does not toggle checkbox selection', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const rows = getCenterRows(grid);
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(3);

  const firstRow = rows.nth(0);
  const secondRow = rows.nth(1);
  const thirdRow = rows.nth(2);
  const firstRowId = await getRowId(firstRow);
  const secondRowId = await getRowId(secondRow);
  const thirdRowId = await getRowId(thirdRow);
  const firstRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${firstRowId}"]`);
  const secondRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${secondRowId}"]`);
  const thirdRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${thirdRowId}"]`);
  const firstIdCell = firstRowById.locator('.ag-cell[col-id="id"]').first();
  const secondIdCell = secondRowById.locator('.ag-cell[col-id="id"]').first();

  await firstIdCell.click();
  await expect(firstRowById).toHaveClass(/erp-data-grid-row-active/);
  await expect(secondRowById).not.toHaveClass(/erp-data-grid-row-active/);
  await expect(thirdRowById).not.toHaveClass(/erp-data-grid-row-active/);
  await expect(firstRowById).not.toHaveClass(/ag-row-selected/);

  await secondIdCell.click();
  await expect(firstRowById).not.toHaveClass(/erp-data-grid-row-active/);
  await expect(secondRowById).toHaveClass(/erp-data-grid-row-active/);
  await expect(thirdRowById).not.toHaveClass(/erp-data-grid-row-active/);
  await expect(secondRowById).not.toHaveClass(/ag-row-selected/);
});

test('order review checkbox multi-select keeps active-row as single-highlight', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const rows = getCenterRows(grid);
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(3);

  const firstRow = rows.nth(0);
  const secondRow = rows.nth(1);
  const thirdRow = rows.nth(2);
  const secondRowId = await getRowId(secondRow);
  const thirdRowId = await getRowId(thirdRow);
  const secondRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${secondRowId}"]`);
  const thirdRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${thirdRowId}"]`);
  await firstRow.click();

  expect(secondRowId).not.toBe('');
  expect(thirdRowId).not.toBe('');

  await clickSelectionCheckboxCenter(page, grid, secondRowId);
  await clickSelectionCheckboxCenter(page, grid, thirdRowId);

  await expect(secondRowById).toHaveClass(/ag-row-selected/);
  await expect(thirdRowById).toHaveClass(/ag-row-selected/);
  await expect(grid.locator('.ag-center-cols-container .erp-data-grid-row.erp-data-grid-row-active')).toHaveCount(1);
});

test('order review selection cell blank area toggles checkbox in one click', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const rows = getCenterRows(grid);
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(2);

  const firstRow = rows.nth(0);
  const secondRow = rows.nth(1);
  const secondRowId = await getRowId(secondRow);
  expect(secondRowId).not.toBe('');

  await firstRow.locator('.ag-cell[col-id="id"]').first().click();

  const secondSelectionCell = grid
    .locator(`.ag-center-cols-container .ag-row[row-id="${secondRowId}"] .ag-cell[col-id="__erpSelection"]`)
    .first();
  const secondRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${secondRowId}"]`);
  await expect(secondSelectionCell).toBeVisible();

  const box = await secondSelectionCell.boundingBox();
  expect(box).not.toBeNull();
  await page.mouse.click(box!.x + box!.width - 3, box!.y + box!.height / 2);

  await expect(secondRowById).toHaveClass(/ag-row-selected/);
});

test('order review checkbox hot area toggles selection in one click', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const rows = getCenterRows(grid);
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(2);

  const secondRowId = await getRowId(rows.nth(1));
  expect(secondRowId).not.toBe('');

  const secondRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${secondRowId}"]`);
  await clickSelectionCheckboxHotArea(page, grid, secondRowId);

  await expect(secondRowById).toHaveClass(/ag-row-selected/);
});

test('order review renders visible unchecked checkbox indicator on first paint', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const rows = getCenterRows(grid);
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(1);
  const firstRowId = await getRowId(rows.nth(0));
  expect(firstRowId).not.toBe('');

  const checkboxVisual = getSelectionVisual(grid, firstRowId);
  await expect(checkboxVisual).toBeVisible();

  const visualStyle = await checkboxVisual.evaluate((element: Element) => {
    const marker = element as HTMLElement;
    const style = window.getComputedStyle(marker);
    return {
      borderStyle: style.borderStyle,
      borderColor: style.borderColor,
      width: style.width,
      height: style.height,
    };
  });

  expect(visualStyle.borderStyle).toContain('solid');
  expect(visualStyle.borderColor).toBe('rgb(71, 85, 105)');
  expect(visualStyle.width).toBe('14px');
  expect(visualStyle.height).toBe('14px');
});

test('order review keeps selection column visible after switching away and back', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const reviewGrid = page.getByTestId('order-review-data-grid');
  await expect(reviewGrid).toBeVisible();

  const reviewRows = getCenterRows(reviewGrid);
  await expect.poll(async () => reviewRows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(1);
  const firstRowId = await getRowId(reviewRows.nth(0));
  expect(firstRowId).not.toBe('');

  const functionSearchInput = page.locator('.workbench-function-search input').first();
  await expect(functionSearchInput).toBeVisible();
  await functionSearchInput.fill('订单查');
  await page.getByTestId('function-search-option-orders').click();

  await expect(page).toHaveURL(/\/orders(\?.*)?$/);
  await expect(page.getByTestId('orders-grid')).toBeVisible();

  await page.locator('.workbench-tabs [role="tab"]').filter({ hasText: '订单审核' }).first().click();
  await expect(page).toHaveURL(/\/orders\/review(\?.*)?$/);

  const revisitedGrid = page.getByTestId('order-review-data-grid');
  await expect(revisitedGrid).toBeVisible();
  const revisitedRows = getCenterRows(revisitedGrid);
  await expect.poll(async () => revisitedRows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(1);

  const selectionHeaderCheckbox = revisitedGrid.getByTestId('erp-selection-header-checkbox');
  await expect(selectionHeaderCheckbox).toBeVisible();

  const selectionCell = revisitedGrid.locator(
    `.ag-center-cols-container .ag-row .ag-cell[col-id="__erpSelection"] .erp-selection-checkbox-visual`,
  ).first();
  await expect(selectionCell).toBeVisible();

  const checkboxVisual = await selectionCell.evaluate((element: Element) => {
    const marker = element as HTMLElement;
    return {
      borderStyle: window.getComputedStyle(marker).borderStyle,
      width: window.getComputedStyle(marker).width,
      height: window.getComputedStyle(marker).height,
    };
  });

  expect(checkboxVisual.borderStyle).toContain('solid');
  expect(checkboxVisual.width).toBe('14px');
  expect(checkboxVisual.height).toBe('14px');
});

test('order review keeps selection column visible after browser tab blur and focus', async ({ page, context }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const rows = getCenterRows(grid);
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(1);

  const otherTab = await context.newPage();
  await otherTab.goto('about:blank');
  await otherTab.bringToFront();
  await page.waitForTimeout(800);
  await page.bringToFront();

  await expect(grid).toBeVisible();
  const selectionCell = grid
    .locator(`.ag-center-cols-container .ag-row .ag-cell[col-id="__erpSelection"] .erp-selection-checkbox-visual`)
    .first();
  await expect(selectionCell).toBeVisible();
});

test('order review keeps selection column visible after page reload', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();
  await expect.poll(async () => getCenterRows(grid).count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(1);

  await page.reload();
  await loginAndSelectTenant(page);

  const reloadedGrid = page.getByTestId('order-review-data-grid');
  await expect(reloadedGrid).toBeVisible();
  await expect.poll(async () => getCenterRows(reloadedGrid).count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(1);
  await expect(reloadedGrid.getByTestId('erp-selection-header-checkbox')).toBeVisible();
  await expect(
    reloadedGrid
      .locator(`.ag-center-cols-container .ag-row .ag-cell[col-id="__erpSelection"] .erp-selection-checkbox-visual`)
      .first(),
  ).toBeVisible();
});

test('order review shows selection column when opened from another workbench page with warm cache', async ({ page }) => {
  await page.goto('/orders');
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('orders-grid')).toBeVisible();

  const functionSearchInput = page.locator('.workbench-function-search input').first();
  await expect(functionSearchInput).toBeVisible();
  await functionSearchInput.fill('订单审');
  await page.getByTestId('function-search-option-orders-review').click();

  await expect(page).toHaveURL(/\/orders\/review(\?.*)?$/);
  const reviewGrid = page.getByTestId('order-review-data-grid');
  await expect(reviewGrid).toBeVisible();
  await expect.poll(async () => getCenterRows(reviewGrid).count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(1);
  await expect(reviewGrid.getByTestId('erp-selection-header-checkbox')).toBeVisible();
  await expect(
    reviewGrid
      .locator(`.ag-center-cols-container .ag-row .ag-cell[col-id="__erpSelection"] .erp-selection-checkbox-visual`)
      .first(),
  ).toBeVisible();
});

test('order review preserves active row and checked rows across workbench tab switches', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const rows = getCenterRows(grid);
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(3);

  const secondRowId = await getRowId(rows.nth(1));
  const thirdRowId = await getRowId(rows.nth(2));
  expect(secondRowId).not.toBe('');
  expect(thirdRowId).not.toBe('');

  const secondRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${secondRowId}"]`);
  const thirdRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${thirdRowId}"]`);

  await clickSelectionCheckboxHotArea(page, grid, secondRowId);
  await expect(secondRowById).toHaveClass(/ag-row-selected/);

  await thirdRowById.locator('.ag-cell[col-id="id"]').first().click();
  await expect(thirdRowById).toHaveClass(/erp-data-grid-row-active/);

  const functionSearchInput = page.locator('.workbench-function-search input').first();
  await expect(functionSearchInput).toBeVisible();
  await functionSearchInput.fill('订单查');
  await page.getByTestId('function-search-option-orders').click();

  await expect(page).toHaveURL(/\/orders(\?.*)?$/);
  await expect(page.getByTestId('orders-grid')).toBeVisible();

  await page.locator('.workbench-tabs [role="tab"]').filter({ hasText: '订单审核' }).first().click();
  await expect(page).toHaveURL(/\/orders\/review(\?.*)?$/);

  const revisitedGrid = page.getByTestId('order-review-data-grid');
  await expect(revisitedGrid).toBeVisible();

  const revisitedSecondRow = revisitedGrid.locator(`.ag-center-cols-container .ag-row[row-id="${secondRowId}"]`);
  const revisitedThirdRow = revisitedGrid.locator(`.ag-center-cols-container .ag-row[row-id="${thirdRowId}"]`);

  await expect(revisitedSecondRow).toHaveClass(/ag-row-selected/);
  await expect(revisitedThirdRow).toHaveClass(/erp-data-grid-row-active/);
});

test('order review first selection-column click toggles without prior row activation click', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const diagnostic = grid.getByTestId('order-review-selection-diagnostic');
  await expect(diagnostic).toBeVisible();

  const rows = getCenterRows(grid);
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(1);
  const firstRowId = await getRowId(rows.nth(0));
  expect(firstRowId).not.toBe('');

  const firstRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${firstRowId}"]`);
  await clickSelectionCheckboxHotArea(page, grid, firstRowId);

  await expect(firstRowById).toHaveClass(/ag-row-selected/);
  await expect(diagnostic).toHaveAttribute('data-action', 'toggle-selection');
  await expect(diagnostic).toHaveAttribute('data-hit-zone', /selection-(column|checkbox)/);
  await expect(diagnostic).toHaveAttribute('data-row-key', firstRowId);
});

test('order review exposes selection diagnostics for single-click selection-column toggle', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const diagnostic = grid.getByTestId('order-review-selection-diagnostic');
  await expect(diagnostic).toBeVisible();

  const rows = getCenterRows(grid);
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(2);

  const firstRowId = await getRowId(rows.nth(0));
  const secondRowId = await getRowId(rows.nth(1));
  const firstRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${firstRowId}"]`);
  const secondRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${secondRowId}"]`);
  const secondSelectionCell = secondRowById.locator('.ag-cell[col-id="__erpSelection"]').first();

  await firstRowById.locator('.ag-cell[col-id="id"]').first().click();
  const secondSelectionBox = await secondSelectionCell.boundingBox();
  expect(secondSelectionBox).not.toBeNull();
  await page.mouse.click(secondSelectionBox!.x + secondSelectionBox!.width / 2, secondSelectionBox!.y + secondSelectionBox!.height / 2);
  await expect(secondRowById).toHaveClass(/ag-row-selected/);

  await expect(diagnostic).toHaveAttribute('data-hit-zone', /selection-(column|checkbox)/);
  await expect(diagnostic).toHaveAttribute('data-action', 'toggle-selection');
  await expect(diagnostic).toHaveAttribute('data-col-id', '__erpSelection');
  await expect(diagnostic).toHaveAttribute('data-row-key', secondRowId);
  await expect(diagnostic).toContainText('选择列');
  await expect(diagnostic).toContainText('勾选');
});

test('order review supports Shift/Ctrl row-click with previous-click anchor behavior', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const rows = getCenterRows(grid);
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(4);

  const secondRow = rows.nth(1);
  const thirdRow = rows.nth(2);
  const fourthRow = rows.nth(3);

  const secondRowId = await getRowId(secondRow);
  const thirdRowId = await getRowId(thirdRow);
  const fourthRowId = await getRowId(fourthRow);

  const secondRowIdCell = grid.locator(
    `.ag-center-cols-container .ag-row[row-id="${secondRowId}"] .ag-cell[col-id="id"]`
  );
  const fourthRowIdCell = grid.locator(
    `.ag-center-cols-container .ag-row[row-id="${fourthRowId}"] .ag-cell[col-id="id"]`
  );
  const secondRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${secondRowId}"]`);
  const thirdRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${thirdRowId}"]`);
  const fourthRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${fourthRowId}"]`);

  await secondRowIdCell.click({ modifiers: ['Control'] });
  await expect(secondRowById).toHaveClass(/ag-row-selected/);
  await expect(thirdRowById).not.toHaveClass(/ag-row-selected/);
  await expect(fourthRowById).not.toHaveClass(/ag-row-selected/);

  await fourthRowIdCell.click({ modifiers: ['Shift'] });
  await expect(secondRowById).toHaveClass(/ag-row-selected/);
  await expect(thirdRowById).toHaveClass(/ag-row-selected/);
  await expect(fourthRowById).toHaveClass(/ag-row-selected/);

  await thirdRowById.locator('.ag-cell[col-id="id"]').first().click({ modifiers: ['Shift'] });
  await expect(secondRowById).toHaveClass(/ag-row-selected/);
  await expect(thirdRowById).not.toHaveClass(/ag-row-selected/);
  await expect(fourthRowById).not.toHaveClass(/ag-row-selected/);
});

test('order review selection interactions regression chain stays stable', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const rows = getCenterRows(grid);
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(4);

  const firstRowId = await getRowId(rows.nth(0));
  const secondRowId = await getRowId(rows.nth(1));
  const thirdRowId = await getRowId(rows.nth(2));
  const fourthRowId = await getRowId(rows.nth(3));

  const firstRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${firstRowId}"]`);
  const secondRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${secondRowId}"]`);
  const thirdRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${thirdRowId}"]`);
  const fourthRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${fourthRowId}"]`);

  await firstRowById.locator('.ag-cell[col-id="id"]').first().click();
  await expect(firstRowById).toHaveClass(/erp-data-grid-row-active/);

  const secondSelectionCell = secondRowById.locator('.ag-cell[col-id="__erpSelection"]').first();
  const secondSelectionBox = await secondSelectionCell.boundingBox();
  expect(secondSelectionBox).not.toBeNull();
  await page.mouse.click(secondSelectionBox!.x + secondSelectionBox!.width - 3, secondSelectionBox!.y + secondSelectionBox!.height / 2);
  await expect(secondRowById).toHaveClass(/ag-row-selected/);

  await thirdRowById.locator('.ag-cell[col-id="id"]').first().click({ modifiers: ['Control'] });
  await expect(secondRowById).toHaveClass(/ag-row-selected/);
  await expect(thirdRowById).toHaveClass(/ag-row-selected/);
  await expect(fourthRowById).not.toHaveClass(/ag-row-selected/);

  await fourthRowById.locator('.ag-cell[col-id="id"]').first().click({ modifiers: ['Shift'] });
  await expect(secondRowById).toHaveClass(/ag-row-selected/);
  await expect(thirdRowById).toHaveClass(/ag-row-selected/);
  await expect(fourthRowById).toHaveClass(/ag-row-selected/);

  await thirdRowById.locator('.ag-cell[col-id="id"]').first().click({ modifiers: ['Shift'] });
  await expect(secondRowById).toHaveClass(/ag-row-selected/);
  await expect(thirdRowById).not.toHaveClass(/ag-row-selected/);
  await expect(fourthRowById).not.toHaveClass(/ag-row-selected/);

  const secondIdCell = secondRowById.locator('.ag-cell[col-id="id"]').first();
  await secondIdCell.click();
  await secondIdCell.press('Space');
  await expect(secondRowById).not.toHaveClass(/ag-row-selected/);
  await secondIdCell.press('Space');
  await expect(secondRowById).toHaveClass(/ag-row-selected/);

  const selectionHeaderCheckbox = grid.getByTestId('erp-selection-header-checkbox');
  await selectionHeaderCheckbox.click();
  await expect.poll(async () => grid.locator('.ag-center-cols-container .ag-row.ag-row-selected').count()).toBeGreaterThanOrEqual(4);

  await selectionHeaderCheckbox.click();
  await expect(grid.locator('.ag-center-cols-container .ag-row.ag-row-selected')).toHaveCount(0);
});

test('order review supports Shift checkbox selection with previous-click anchor behavior', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const rows = getCenterRows(grid);
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(4);

  const secondRow = rows.nth(1);
  const thirdRow = rows.nth(2);
  const fourthRow = rows.nth(3);

  const secondRowId = await getRowId(secondRow);
  const thirdRowId = await getRowId(thirdRow);
  const fourthRowId = await getRowId(fourthRow);

  const secondRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${secondRowId}"]`);
  const thirdRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${thirdRowId}"]`);
  const fourthRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${fourthRowId}"]`);

  await clickSelectionCheckboxCenter(page, grid, secondRowId);
  await expect(secondRowById).toHaveClass(/ag-row-selected/);
  await expect(thirdRowById).not.toHaveClass(/ag-row-selected/);
  await expect(fourthRowById).not.toHaveClass(/ag-row-selected/);

  await clickSelectionCheckboxCenter(page, grid, fourthRowId, ['Shift']);
  const selectedAfterShiftCheckbox = await page.evaluate(() => {
    const gridTestWindow = window as Window & {
      __ERP_GRID_TEST_APIS?: Record<string, { getSelectedNodes?: () => Array<{ id?: string; data?: { id?: string } }> }>;
    };
    const api = gridTestWindow.__ERP_GRID_TEST_APIS?.['order-review-data-grid'];
    return (
      api?.getSelectedNodes?.().map((node) => String(node.data?.id ?? node.id ?? '').trim()).filter(Boolean) ?? []
    );
  });
  expect(selectedAfterShiftCheckbox).toEqual([secondRowId, thirdRowId, fourthRowId]);
  await expect(secondRowById).toHaveClass(/ag-row-selected/);
  await expect(thirdRowById).toHaveClass(/ag-row-selected/);
  await expect(fourthRowById).toHaveClass(/ag-row-selected/);

  await clickSelectionCheckboxCenter(page, grid, thirdRowId, ['Shift']);
  await expect(secondRowById).toHaveClass(/ag-row-selected/);
  await expect(thirdRowById).not.toHaveClass(/ag-row-selected/);
  await expect(fourthRowById).not.toHaveClass(/ag-row-selected/);
});

test('order review supports Space to check/uncheck without changing pagination', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const rows = getCenterRows(grid);
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(2);

  const firstRow = rows.nth(0);
  const firstRowId = await getRowId(firstRow);
  const firstRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${firstRowId}"]`);
  const firstRowIdCell = firstRowById.locator('.ag-cell[col-id="id"]').first();
  const activePage = page.locator('.order-review-pagination .ant-pagination-item-active');
  const pageBefore = ((await activePage.first().textContent()) ?? '').trim();

  await firstRowIdCell.click();
  await firstRowIdCell.press('Space');
  await expect(firstRowById).toHaveClass(/ag-row-selected/);

  await firstRowIdCell.press('Space');
  await expect(firstRowById).not.toHaveClass(/ag-row-selected/);

  const pageAfter = ((await activePage.first().textContent()) ?? '').trim();
  expect(pageAfter).toBe(pageBefore);
});

test('order review uses dropdown filters for fixed-value columns and text filters for keyword columns', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  await expect.poll(async () => grid.locator('.ag-floating-filter').count()).toBeGreaterThan(0);
  await expect(grid.getByTestId('order-review-floating-filter-platform')).toBeVisible();
  await expect(grid.getByTestId('order-review-floating-filter-shop')).toBeVisible();
  await expect(grid.getByTestId('order-review-floating-filter-logistics')).toBeVisible();

  const orderIdFilterInput = grid.getByTestId('order-review-floating-filter-id');
  const platformOrderIdFilterInput = grid.getByTestId('order-review-floating-filter-platform-order-id');
  await expect(orderIdFilterInput).toBeVisible();
  await expect(platformOrderIdFilterInput).toBeVisible();
});

test('order review shows business time column and supports date-range filtering', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const rows = getCenterRows(grid);
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThan(0);

  const createdAtHeader = grid.locator('.ag-header-row-column .ag-header-cell[col-id="created_at"]').first();
  await expect(createdAtHeader).toBeVisible();

  const createdAtTrigger = grid.getByTestId('order-review-floating-filter-created-at-trigger');
  await expect(createdAtTrigger).toBeVisible();

  await createdAtTrigger.click();

  const dropdown = page.locator('.ant-picker-dropdown:visible').last();
  await expect(dropdown).toBeVisible();

  const leftPanel = dropdown.locator('.ant-picker-panel').first();
  await leftPanel.getByText('20', { exact: true }).first().click();
  await expect(dropdown).toBeVisible();
  await expect(createdAtTrigger).toContainText('日期范围');

  await leftPanel.getByText('10', { exact: true }).first().click();
  await expect(dropdown).toBeHidden();

  const triggerText = ((await createdAtTrigger.textContent()) ?? '').trim();
  const selectedDates = triggerText.match(/\d{4}-\d{2}-\d{2}/g) ?? [];
  expect(selectedDates).toHaveLength(0);
  expect(triggerText).toContain('10');
  expect(triggerText).toContain('20');
  expect(triggerText.includes(String(new Date().getFullYear()))).toBeFalsy();

  const createdAtClear = grid.getByRole('button', { name: '清空日期范围' });
  await expect(createdAtClear).toBeVisible();

  await createdAtClear.click();
  await expect(createdAtTrigger).toContainText('日期范围');
});

test('order review hides unused header menu buttons and shows resize handles for main and goods grids', async ({
  page,
}) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  await expect(grid.locator('.ag-header-cell-menu-button')).toHaveCount(0);
  await expect.poll(async () => grid.locator('.ag-header-cell-resize').count()).toBeGreaterThan(0);

  const firstResizeHandle = grid.locator('.ag-header-cell-resize').first();
  await expect(firstResizeHandle).toBeVisible();
  const resizeCursor = await firstResizeHandle.evaluate((el: Element) => getComputedStyle(el).cursor);
  expect(resizeCursor).toBe('col-resize');

  const goodsGrid = page.getByTestId('order-review-goods-grid');
  await expect(goodsGrid).toBeVisible();
  await expect(goodsGrid.locator('.ag-header-cell-menu-button')).toHaveCount(0);
  await expect.poll(async () => goodsGrid.locator('.ag-header-cell-resize').count()).toBeGreaterThan(0);
});

test('order review supports horizontal scrolling inside AG grid viewport', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const viewport = grid.locator('.ag-center-cols-viewport').first();
  await expect(viewport).toBeVisible();

  const before = await viewport.evaluate((el) => el.scrollLeft);
  await viewport.evaluate((el) => {
    el.scrollLeft = 9999;
    el.dispatchEvent(new Event('scroll', { bubbles: true }));
  });
  const after = await viewport.evaluate((el) => el.scrollLeft);

  expect(after).toBeGreaterThanOrEqual(before);
  expect(after).toBeGreaterThan(0);
});

test('order review main grid shows full frame lines', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();
  await expect.poll(async () => getCenterRows(grid).count(), { timeout: 20_000 }).toBeGreaterThan(0);

  const rootWrapper = grid.locator('.ag-root-wrapper').first();
  const headerCell = getHeaderCell(grid, 'id');
  const bodyCell = getCenterRows(grid).first().locator('.ag-cell[col-id="id"]').first();

  const rootBorderWidth = await rootWrapper.evaluate((el: Element) => Number.parseFloat(getComputedStyle(el).borderWidth || '0'));
  const headerBorderWidth = await headerCell.evaluate((el: Element) =>
    Number.parseFloat(getComputedStyle(el).borderRightWidth || '0'),
  );
  const bodyRightBorderWidth = await bodyCell.evaluate((el: Element) =>
    Number.parseFloat(getComputedStyle(el).borderRightWidth || '0'),
  );
  const bodyBottomBorderWidth = await bodyCell.evaluate((el: Element) =>
    Number.parseFloat(getComputedStyle(el).borderBottomWidth || '0'),
  );

  expect(rootBorderWidth).toBeGreaterThan(0);
  expect(headerBorderWidth).toBeGreaterThan(0);
  expect(bodyRightBorderWidth).toBeGreaterThan(0);
  expect(bodyBottomBorderWidth).toBeGreaterThan(0);
});

test('order review supports header click sorting on the main grid', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();
  await expect.poll(async () => getCenterRows(grid).count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(3);

  const idHeader = getHeaderCell(grid, 'id');
  await expect(idHeader).toBeVisible();

  await idHeader.click();
  await idHeader.click();

  const sortIndicator = idHeader.locator('.ag-sort-indicator-order, .ag-sort-descending-icon');
  await expect(sortIndicator.first()).toBeVisible();

  const firstId = await getCellText(grid, 0, 'id');
  const secondId = await getCellText(grid, 1, 'id');
  expect(firstId.localeCompare(secondId)).toBeGreaterThan(0);
});

test('order review opens native column menu on header right click and can hide or show columns', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();
  await expect.poll(async () => getCenterRows(grid).count(), { timeout: 20_000 }).toBeGreaterThan(0);

  const receiverHeader = getHeaderCell(grid, 'receiver_name');
  await expect(receiverHeader).toBeVisible();
  await receiverHeader.click({ button: 'right' });

  const menu = page.locator('.ag-menu:visible').last();
  await expect(menu).toBeVisible();
  await expect(menu.locator('.ag-menu-option', { hasText: /Pin|固定|锁定/ }).first()).toBeVisible();
  await expect(menu.locator('.ag-menu-option', { hasText: /Choose Columns|选择列|列/ }).first()).toBeVisible();

  const toggleReceiverColumn = async (hide: boolean) => {
    return page.evaluate(
      async ({ hidden }) => {
        const gridTestWindow = window as Window & {
          __ERP_GRID_TEST_APIS?: Record<
            string,
            {
              applyColumnState: (params: {
                state: Array<{ colId: string; hide?: boolean }>;
              }) => void;
              getColumnState: () => Array<{ colId?: string | null; hide?: boolean | null }>;
            }
          >;
        };

        const api = gridTestWindow.__ERP_GRID_TEST_APIS?.['order-review-data-grid'];
        if (!api) {
          return false;
        }

        api.applyColumnState({
          state: [{ colId: 'receiver_name', hide: hidden }],
        });

        await new Promise((resolve) => window.setTimeout(resolve, 300));

        return api
          .getColumnState()
          .some(
            (column: { colId?: string | null; hide?: boolean | null }) =>
              column.colId === 'receiver_name' && Boolean(column.hide) === hidden,
          );
      },
      { hidden: hide },
    );
  };

  expect(await toggleReceiverColumn(true)).toBe(true);
  await expect(receiverHeader).toHaveCount(0);

  expect(await toggleReceiverColumn(false)).toBe(true);
  await expect(getHeaderCell(grid, 'receiver_name')).toBeVisible();
});

test('order review restores selection and row index columns even when persisted state omits them', async ({
  page,
}) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();
  await expect.poll(async () => getCenterRows(grid).count(), { timeout: 20_000 }).toBeGreaterThan(0);

  const injectedState = await page.evaluate(async () => {
    const gridTestWindow = window as Window & {
      __ERP_GRID_TEST_APIS?: Record<
        string,
        {
          applyColumnState: (params: {
            state: Array<{ colId: string; hide?: boolean }>;
          }) => void;
        }
      >;
    };

    const api = gridTestWindow.__ERP_GRID_TEST_APIS?.['order-review-data-grid'];
    if (!api) {
      return { storageKey: null, storedColumnIds: [] as string[] };
    }

    api.applyColumnState({
      state: [{ colId: 'receiver_name', hide: true }],
    });

    await new Promise((resolve) => window.setTimeout(resolve, 300));

    const storageKey =
      Object.keys(window.localStorage).find((key) => key.includes('orders.review.main-grid')) ?? null;
    if (!storageKey) {
      return { storageKey: null, storedColumnIds: [] as string[] };
    }

    const raw = window.localStorage.getItem(storageKey);
    if (!raw) {
      return { storageKey, storedColumnIds: [] as string[] };
    }

    const parsed = JSON.parse(raw) as { version?: number; columns?: Array<{ colId?: string | null }> };
    const businessColumns = (parsed.columns ?? []).filter((column) => {
      const colId = String(column?.colId ?? '').trim();
      return colId !== '__erpSelection' && colId !== '__erpRowIndex';
    });

    window.localStorage.setItem(
      storageKey,
      JSON.stringify({
        version: 1,
        columns: businessColumns,
      }),
    );

    return {
      storageKey,
      storedColumnIds: businessColumns.map((column) => String(column.colId ?? '')),
    };
  });

  expect(injectedState.storageKey).toBeTruthy();
  expect(injectedState.storedColumnIds).not.toContain('__erpSelection');
  expect(injectedState.storedColumnIds).not.toContain('__erpRowIndex');

  await page.reload();
  await loginAndSelectTenant(page);
  await expect(grid).toBeVisible();

  await expect
    .poll(
      async () =>
        page.evaluate(() => {
          const gridTestWindow = window as Window & {
            __ERP_GRID_TEST_APIS?: Record<
              string,
              {
                getAllDisplayedColumns?: () => Array<{ getColId: () => string }>;
                getDisplayedLeftColumns?: () => Array<{ getColId: () => string }>;
              }
            >;
          };

          const api = gridTestWindow.__ERP_GRID_TEST_APIS?.['order-review-data-grid'];
          if (!api) {
            return { left: [] as string[], all: [] as string[] };
          }

          return {
            left: (api.getDisplayedLeftColumns?.() ?? []).map((column) => column.getColId()),
            all: (api.getAllDisplayedColumns?.() ?? []).map((column) => column.getColId()),
          };
        }),
      { timeout: 20_000 },
    )
    .toEqual({
      left: [],
      all: expect.arrayContaining(['__erpSelection', '__erpRowIndex']),
    });
});

test('order review supports Shift/Ctrl checkbox toggle to check and uncheck', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const rows = getCenterRows(grid);
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(4);

  const secondRow = rows.nth(1);
  const thirdRow = rows.nth(2);
  const fourthRow = rows.nth(3);

  const secondRowId = await getRowId(secondRow);
  const thirdRowId = await getRowId(thirdRow);
  const fourthRowId = await getRowId(fourthRow);

  const secondRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${secondRowId}"]`);
  const thirdRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${thirdRowId}"]`);
  const fourthRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${fourthRowId}"]`);

  await clickSelectionCheckboxCenter(page, grid, secondRowId);
  await clickSelectionCheckboxCenter(page, grid, fourthRowId, ['Shift']);

  await expect(secondRowById).toHaveClass(/ag-row-selected/);
  await expect(thirdRowById).toHaveClass(/ag-row-selected/);
  await expect(fourthRowById).toHaveClass(/ag-row-selected/);

  await clickSelectionCheckboxCenter(page, grid, secondRowId, ['Control']);
  await expect(secondRowById).not.toHaveClass(/ag-row-selected/);
  await expect(thirdRowById).toHaveClass(/ag-row-selected/);
  await expect(fourthRowById).toHaveClass(/ag-row-selected/);

  await clickSelectionCheckboxCenter(page, grid, thirdRowId, ['Control']);
  await expect(thirdRowById).not.toHaveClass(/ag-row-selected/);

  await clickSelectionCheckboxCenter(page, grid, fourthRowId, ['Control']);
  await expect(fourthRowById).not.toHaveClass(/ag-row-selected/);
});

test('order review supports free drag cell selection starting from an unselected row', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const rows = getCenterRows(grid);
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(3);

  const secondRow = rows.nth(1);
  const thirdRow = rows.nth(2);
  await expect(secondRow).not.toHaveClass(/ag-row-selected/);

  const startCell = secondRow.locator('.ag-cell[col-id="id"]').first();
  const endCell = thirdRow.locator('.ag-cell[col-id="receiver_name"]').first();
  await expect(startCell).toBeVisible();
  await expect(endCell).toBeVisible();

  const startBox = await startCell.boundingBox();
  const endBox = await endCell.boundingBox();
  expect(startBox).not.toBeNull();
  expect(endBox).not.toBeNull();

  await page.mouse.move(startBox!.x + startBox!.width / 2, startBox!.y + startBox!.height / 2);
  await page.mouse.down();
  await page.mouse.move(endBox!.x + endBox!.width / 2, endBox!.y + endBox!.height / 2, { steps: 8 });
  await page.mouse.up();

  const rangeCount = await page.evaluate(() => {
    const gridTestWindow = window as Window & {
      __ERP_GRID_TEST_APIS?: Record<string, { getCellRanges?: () => Array<unknown> | null | undefined }>;
    };
    const api = gridTestWindow.__ERP_GRID_TEST_APIS?.['order-review-data-grid'];
    return api?.getCellRanges?.()?.length ?? 0;
  });

  expect(rangeCount).toBeGreaterThan(0);
  await expect.poll(async () => grid.locator('.ag-cell-range-selected').count()).toBeGreaterThan(0);
});

test('order review has checkbox + serial columns with centered header and separated boundary', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const rows = getCenterRows(grid);
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(1);

  const firstCenterRow = rows.first();
  const selectionCells = firstCenterRow.locator('.ag-cell[col-id="__erpSelection"]');
  const rowIndexCells = firstCenterRow.locator('.ag-cell[col-id="__erpRowIndex"]');
  const idColumnCheckbox = firstCenterRow.locator('.ag-cell[col-id="id"] .ag-selection-checkbox');

  await expect(selectionCells).toHaveCount(1);
  await expect(rowIndexCells).toHaveCount(1);
  await expect(idColumnCheckbox).toHaveCount(0);

  const rowIndexHeader = grid.locator('.ag-header-row-column .ag-header-cell[col-id="__erpRowIndex"]').first();
  await expect(rowIndexHeader).toBeVisible();

  await expect(rowIndexHeader).toHaveClass(/order-review-main-row-index-header/);

  const rowIndexTextAlign = await rowIndexCells.first().evaluate((el: Element) => getComputedStyle(el).textAlign);
  expect(rowIndexTextAlign).toBe('center');

  const selectionBorder = await selectionCells
    .first()
    .evaluate((el: Element) => getComputedStyle(el).borderRightWidth);
  const rowIndexBorder = await rowIndexCells.first().evaluate((el: Element) => getComputedStyle(el).borderRightWidth);
  expect(Number.parseFloat(selectionBorder)).toBeGreaterThan(0);
  expect(Number.parseFloat(rowIndexBorder)).toBeGreaterThan(0);

  const selectionHeaderCheckbox = grid.getByTestId('erp-selection-header-checkbox');
  await expect(selectionHeaderCheckbox).toBeVisible();

  await expect(selectionCells.first().locator('.ag-selection-checkbox')).toBeVisible();

  const selectedRows = grid.locator('.ag-center-cols-container .ag-row.ag-row-selected');
  await selectionHeaderCheckbox.click();
  await expect.poll(async () => selectedRows.count()).toBeGreaterThan(0);

  await selectionHeaderCheckbox.click();
  await expect(selectedRows).toHaveCount(0);
});

test('order review footer spacing is compact', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const pageRoot = page.getByTestId('order-review-page');
  await expect(pageRoot).toBeVisible();

  const paddingTop = await pageRoot.locator('.order-review-pagination').evaluate((el) => {
    const computed = window.getComputedStyle(el);
    return Number.parseFloat(computed.paddingTop || '0');
  });
  const paddingBottom = await pageRoot.locator('.order-review-pagination').evaluate((el) => {
    const computed = window.getComputedStyle(el);
    return Number.parseFloat(computed.paddingBottom || '0');
  });

  expect(paddingTop).toBeLessThanOrEqual(5);
  expect(paddingBottom).toBeLessThanOrEqual(5);
});

test('order review pagination size changer defaults to 100 and opens upward with expected options', async ({
  page,
}) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const pagination = page.locator('.order-review-pagination');
  await expect(pagination).toBeVisible();

  const sizeChanger = pagination.locator('.ant-pagination-options-size-changer').first();
  await expect(sizeChanger).toBeVisible();
  await expect(sizeChanger).toContainText('100');

  await sizeChanger.click();

  const dropdown = page.locator('.ant-select-dropdown:visible').last();
  await expect(dropdown).toBeVisible();
  await expect(dropdown).toHaveClass(/ant-select-dropdown-placement-top/);

  const expectedOptions = ['50', '100', '200', '300', '500', '800', '1000', '3000', '5000'];
  for (const option of expectedOptions) {
    await expect(dropdown.getByRole('option', { name: `${option} 条/页`, exact: true })).toBeVisible();
  }
});

test('order review template smoke', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const pageRoot = page.getByTestId('order-review-page');
  await expect(pageRoot).toBeVisible();

  // Page skeleton from new template
  await expect(pageRoot.locator('.order-workbench-body')).toHaveCount(1);
  await expect(pageRoot.locator('.order-workbench-main')).toHaveCount(1);

  // Toolbar assembly: stable page-level class + business actions + search input inside toolbar root.
  const toolbarRoot = pageRoot.locator('.order-review-toolbar');
  await expect(toolbarRoot).toBeVisible();
  await expect(toolbarRoot.getByTestId('order-review-approve')).toBeVisible();
  await expect(toolbarRoot.getByTestId('order-review-split')).toBeVisible();
  await expect(toolbarRoot.getByTestId('order-review-merge')).toBeVisible();

  await expect(toolbarRoot.getByTestId('order-review-toolbar-search-input')).toBeVisible();

  // Main grid still visible
  await expect(page.getByTestId('order-review-data-grid')).toBeVisible();
});
