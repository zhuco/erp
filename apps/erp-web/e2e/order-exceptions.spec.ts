import { expect, loginAndSelectTenant, test } from './fixtures';
import {
  clickSelectionCheckboxCenter,
  clickSelectionCheckboxHotArea,
  clickSelectionColumnBlankArea,
  getBusinessRowById,
  getSelectionCell,
} from './orderWorkbenchGridHelpers';

const orderRecheckRoute = '/orders/recheck';

test('订单复核页面使用统一工作台模板且不再显示异常池文案', async ({ page }) => {
  await page.goto(orderRecheckRoute);
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('order-exceptions-title')).toHaveCount(0);
  await expect(page.getByText('承接订单审核后的复核闭环，直接完成领取、复核与放行')).toHaveCount(0);
  await expect(page.getByText('订单复核异常池')).toHaveCount(0);
  await expect(page.getByTestId('order-exceptions-sla-board')).toHaveCount(0);

  const mainGrid = page.getByTestId('order-exceptions-data-grid');
  const centerRows = mainGrid.locator('.ag-center-cols-container .ag-row');
  await expect(mainGrid.locator('.ag-root-wrapper').first()).toBeVisible();
  await page.waitForTimeout(500);
  const rowCount = await centerRows.count();

  const resolveButton = page.getByTestId('order-exceptions-resolve-action');
  const searchInput = page.getByTestId('order-exceptions-search-input');
  await expect(page.getByTestId('order-exceptions-claim-action')).toHaveCount(0);
  await expect(resolveButton).toContainText(/复\s*核/);
  await expect(resolveButton).toBeEnabled();

  const searchWidth = await searchInput.evaluate((element) => {
    const target = element as HTMLElement;
    return Math.round(target.getBoundingClientRect().width);
  });
  expect(searchWidth).toBeLessThanOrEqual(260);

  if (rowCount > 0) {
    const firstRowId = (await centerRows.first().getAttribute('row-id')) ?? '';
    await clickSelectionCheckboxCenter(page, mainGrid, firstRowId);

    await expect(resolveButton).toBeEnabled();
  } else {
    await expect(mainGrid.getByText('当前没有待处理的复核订单')).toBeVisible();
    await expect(resolveButton).toBeEnabled();
  }
});

test('订单复核后会自动递交WMS发货链路', async ({ page }) => {
  await page.goto(orderRecheckRoute);
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-exceptions-data-grid');
  await expect(grid).toBeVisible();

  const rows = grid.locator('.ag-center-cols-container .ag-row.erp-data-grid-row');
  await page.waitForTimeout(800);
  const beforeCount = await rows.count();
  test.skip(beforeCount === 0, '当前复核数据为空，跳过复核递交WMS断言');

  const firstRowId = (await rows.first().getAttribute('row-id')) ?? '';
  expect(firstRowId).not.toBe('');
  const beforeSnapshot = await page.evaluate<
    { exceptionStatus: string; fulfillmentStatus: string } | null,
    string
  >((targetRowId) => {
    const testApis = (window as Window & { __ERP_GRID_TEST_APIS?: Record<string, unknown> }).__ERP_GRID_TEST_APIS;
    const api = testApis?.['order-exceptions-data-grid'] as
      | {
          forEachNode: (
            callback: (node: {
              id?: string;
              data?: {
                exception_status?: string | null;
                fulfillment_status?: string | null;
              };
            }) => void,
          ) => void;
        }
      | undefined;
    if (!api) return null;

    let snapshot: { exceptionStatus: string; fulfillmentStatus: string } | null = null;
    api.forEachNode((node) => {
      if (snapshot || String(node.id ?? '').trim() !== targetRowId) {
        return;
      }
      snapshot = {
        exceptionStatus: String(node.data?.exception_status ?? '').trim(),
        fulfillmentStatus: String(node.data?.fulfillment_status ?? '').trim(),
      };
    });
    return snapshot;
  }, firstRowId);
  expect(beforeSnapshot).not.toBeNull();

  await clickSelectionCheckboxHotArea(page, grid, firstRowId);

  const resolveButton = page.getByTestId('order-exceptions-resolve-action');
  await expect(resolveButton).toBeEnabled();
  await resolveButton.click();

  await expect(page.locator('.ant-message')).toContainText(/WMS|递交/);
  await expect
    .poll(
      async () =>
        page.evaluate<boolean, string>((targetRowId) => {
          const testApis = (window as Window & { __ERP_GRID_TEST_APIS?: Record<string, unknown> }).__ERP_GRID_TEST_APIS;
          const api = testApis?.['order-exceptions-data-grid'] as
            | {
                forEachNode: (
                  callback: (node: {
                    id?: string;
                    data?: {
                      exception_status?: string | null;
                      fulfillment_status?: string | null;
                    };
                  }) => void,
                ) => void;
              }
            | undefined;
          if (!api) return false;

          let found = false;
          let exceptionStatus = '';
          let fulfillmentStatus = '';
          api.forEachNode((node) => {
            if (found || String(node.id ?? '').trim() !== targetRowId) {
              return;
            }
            found = true;
            exceptionStatus = String(node.data?.exception_status ?? '').trim();
            fulfillmentStatus = String(node.data?.fulfillment_status ?? '').trim();
          });
          return !found || (exceptionStatus === 'RESOLVED' && fulfillmentStatus === 'WAVE_PENDING');
        }, firstRowId),
      { timeout: 15_000 },
    )
    .toBe(true);
});

test('订单复核页面接入统一主表和底部子表模板', async ({ page }) => {
  await page.goto(orderRecheckRoute);
  await loginAndSelectTenant(page);

  const mainGrid = page.getByTestId('order-exceptions-data-grid');
  await expect(mainGrid).toBeVisible();
  await expect(page.locator('.order-exceptions-grid-summary')).toHaveCount(0);
  await expect(page.locator('.order-review-pagination')).toBeVisible();

  await expect(mainGrid.locator('.ag-header-cell[col-id="order_no"]').first()).toBeVisible();

  await expect(page.getByTestId('order-review-goods-tab')).toBeVisible();
  await expect(page.getByRole('tab', { name: '货品', exact: true })).toBeVisible();
  await expect(page.getByRole('tab', { name: '日志', exact: true })).toBeVisible();
});

test('订单复核排序图标使用轻量箭头样式，不显示深色方块', async ({ page }) => {
  await page.goto(orderRecheckRoute);
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-exceptions-data-grid');
  await expect(grid).toBeVisible();
  await expect.poll(async () => grid.locator('.ag-center-cols-container .ag-row.erp-data-grid-row').count()).toBeGreaterThan(0);

  const orderNoHeader = grid.locator('.ag-header-row-column .ag-header-cell[col-id="order_no"]').first();
  await expect(orderNoHeader).toBeVisible();

  await orderNoHeader.click();

  const sortIcon = orderNoHeader.locator('.ag-sort-indicator-container .ag-icon').first();
  await expect(sortIcon).toBeVisible();

  const pseudoStyles = await sortIcon.evaluate((element) => {
    const beforeStyles = getComputedStyle(element, '::before');
    const afterStyles = getComputedStyle(element, '::after');
    return {
      beforeDisplay: beforeStyles.display,
      afterDisplay: afterStyles.display,
      afterBackgroundImage: afterStyles.backgroundImage,
    };
  });

  expect(pseudoStyles.beforeDisplay).toBe('none');
  expect(pseudoStyles.afterDisplay).toBe('block');
  expect(pseudoStyles.afterBackgroundImage).toContain('data:image/svg+xml');
});

test('订单复核左侧过滤到待处理后，主表首行显示与网格数据保持一致', async ({ page }) => {
  await page.goto(orderRecheckRoute);
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-exceptions-data-grid');
  await expect(grid).toBeVisible();

  const leftPanel = page.getByTestId('order-exceptions-left-panel');
  await expect(leftPanel).toBeVisible();
  const pendingNodeCell = leftPanel.locator(
    '.order-review-left-ag-grid-shell .ag-row[row-id="status:OPEN"] .ag-cell',
  );
  test.skip((await pendingNodeCell.count()) === 0, '左侧未渲染待处理节点，跳过待处理首行一致性断言');
  await pendingNodeCell.first().click();

  const visibleRows = grid.locator('.ag-center-cols-container .ag-row.erp-data-grid-row');
  const visibleCount = await visibleRows.count();
  test.skip(visibleCount === 0, '待处理过滤后无数据，跳过首行一致性断言');

  const firstRow = visibleRows.first();
  const firstRowId = (await firstRow.getAttribute('row-id')) ?? '';
  expect(firstRowId.trim()).not.toBe('');

  const firstOrderNoText = (await firstRow.locator('.ag-cell[col-id="order_no"]').first().innerText()).trim();
  const firstRowFromModel = await page.evaluate<
    { orderNo: string; exceptionStatus: string } | null,
    string
  >((targetRowId) => {
    const testApis = (window as Window & { __ERP_GRID_TEST_APIS?: Record<string, unknown> }).__ERP_GRID_TEST_APIS;
    const api = testApis?.['order-exceptions-data-grid'] as
      | {
          forEachNodeAfterFilterAndSort: (callback: (node: { id?: string; data?: { order_no?: string; exception_status?: string } }) => void) => void;
        }
      | undefined;
    if (!api) return null;

    let snapshot: { orderNo: string; exceptionStatus: string } | null = null;
    api.forEachNodeAfterFilterAndSort((node) => {
      if (snapshot || String(node.id ?? '') !== targetRowId) {
        return;
      }
      snapshot = {
        orderNo: String(node.data?.order_no ?? '').trim(),
        exceptionStatus: String(node.data?.exception_status ?? '').trim(),
      };
    });
    return snapshot;
  }, firstRowId);

  expect(firstRowFromModel).not.toBeNull();
  if (!firstRowFromModel) {
    throw new Error('无法读取待处理过滤后的首行网格数据');
  }
  expect(firstRowFromModel.exceptionStatus).toBe('OPEN');
  expect(firstOrderNoText).toContain(firstRowFromModel.orderNo);
});

test('订单复核连续切换左侧过滤后首行渲染保持稳定', async ({ page }) => {
  await page.goto(orderRecheckRoute);
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-exceptions-data-grid');
  await expect(grid).toBeVisible();

  const leftPanel = page.getByTestId('order-exceptions-left-panel');
  await expect(leftPanel).toBeVisible();

  const allStatusNode = leftPanel.locator('.order-review-left-ag-grid-shell .ag-row[row-id="status:*"] .ag-cell').first();
  const pendingNode = leftPanel.locator('.order-review-left-ag-grid-shell .ag-row[row-id="status:OPEN"] .ag-cell').first();

  test.skip((await allStatusNode.count()) === 0 || (await pendingNode.count()) === 0, '左侧状态节点未渲染，跳过首行稳定性断言');

  for (let i = 0; i < 3; i += 1) {
    await allStatusNode.click();
    await pendingNode.click();
  }

  const visibleRows = grid.locator('.ag-center-cols-container .ag-row.erp-data-grid-row');
  const rowCount = await visibleRows.count();
  test.skip(rowCount === 0, '待处理过滤后无数据，跳过首行稳定性断言');

  const firstRow = visibleRows.first();
  const firstOrderNoText = (await firstRow.locator('.ag-cell[col-id="order_no"]').first().innerText()).trim();
  expect(firstOrderNoText).not.toBe('');
  expect(firstOrderNoText).not.toBe('-');

  const firstRowBox = await firstRow.boundingBox();
  const firstCellBox = await firstRow.locator('.ag-cell[col-id="order_no"]').first().boundingBox();
  expect(firstRowBox).not.toBeNull();
  expect(firstCellBox).not.toBeNull();
  if (firstRowBox && firstCellBox) {
    expect(firstCellBox.y).toBeGreaterThanOrEqual(firstRowBox.y - 1);
  }
});

test('订单复核支持表头全选和取消全选', async ({ page }) => {
  await page.goto(orderRecheckRoute);
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-exceptions-data-grid');
  await expect(grid).toBeVisible();

  const selectionHeaderCheckbox = grid.getByTestId('erp-selection-header-checkbox');
  await expect(selectionHeaderCheckbox).toBeVisible();

  const rows = grid.locator('.ag-center-cols-container .ag-row.erp-data-grid-row');
  await page.waitForTimeout(800);
  const rowCount = await rows.count();
  test.skip(rowCount === 0, '当前复核数据为空，跳过表头全选断言');

  const selectedRows = grid.locator('.ag-center-cols-container .ag-row.ag-row-selected');

  await selectionHeaderCheckbox.click();
  await expect.poll(async () => selectedRows.count()).toBe(rowCount);

  await selectionHeaderCheckbox.click();
  await expect(selectedRows).toHaveCount(0);
});

test('订单复核复选框列空白区域支持单击直接勾选', async ({ page }) => {
  await page.goto(orderRecheckRoute);
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-exceptions-data-grid');
  await expect(grid).toBeVisible();

  const rows = grid.locator('.ag-center-cols-container .ag-row.erp-data-grid-row');
  await page.waitForTimeout(800);
  const rowCount = await rows.count();
  test.skip(rowCount < 2, '当前复核数据不足 2 行，跳过复选框列空白区域勾选断言');

  const firstRow = rows.nth(0);
  const secondRow = rows.nth(1);
  const secondRowId = (await secondRow.getAttribute('row-id')) ?? '';

  await firstRow.locator('.ag-cell[col-id="order_no"]').first().click();

  const secondRowById = getBusinessRowById(grid, secondRowId);
  await expect(getSelectionCell(grid, secondRowId)).toBeVisible();
  await clickSelectionColumnBlankArea(page, grid, secondRowId);

  await expect(secondRowById).toHaveClass(/ag-row-selected/);
});

test('订单复核复选框热区支持单击直接勾选', async ({ page }) => {
  await page.goto(orderRecheckRoute);
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-exceptions-data-grid');
  await expect(grid).toBeVisible();

  const rows = grid.locator('.ag-center-cols-container .ag-row.erp-data-grid-row');
  await page.waitForTimeout(800);
  const rowCount = await rows.count();
  test.skip(rowCount < 2, '当前复核数据不足 2 行，跳过复选框本体单击勾选断言');

  const secondRowId = (await rows.nth(1).getAttribute('row-id')) ?? '';
  expect(secondRowId).not.toBe('');

  const secondRowById = getBusinessRowById(grid, secondRowId);
  await clickSelectionCheckboxHotArea(page, grid, secondRowId);

  await expect(secondRowById).toHaveClass(/ag-row-selected/);
});

test('订单复核首击选择列即可勾选，无需先激活行', async ({ page }) => {
  await page.goto(orderRecheckRoute);
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-exceptions-data-grid');
  await expect(grid).toBeVisible();

  const diagnostic = grid.getByTestId('order-exceptions-selection-diagnostic');
  await expect(diagnostic).toBeHidden();

  const rows = grid.locator('.ag-center-cols-container .ag-row.erp-data-grid-row');
  await page.waitForTimeout(800);
  const rowCount = await rows.count();
  test.skip(rowCount < 1, '当前复核数据为空，跳过首击选择列勾选断言');

  const firstRowId = (await rows.nth(0).getAttribute('row-id')) ?? '';
  expect(firstRowId).not.toBe('');
  const firstRowById = getBusinessRowById(grid, firstRowId);

  await clickSelectionCheckboxHotArea(page, grid, firstRowId);

  await expect(firstRowById).toHaveClass(/ag-row-selected/);
  await expect(diagnostic).toHaveAttribute('data-action', 'toggle-selection');
  await expect(diagnostic).toHaveAttribute('data-hit-zone', /selection-(column|checkbox)/);
  await expect(diagnostic).toHaveAttribute('data-row-key', firstRowId);
});

test('订单复核暴露单击选择列勾选的点击诊断信息', async ({ page }) => {
  await page.goto(orderRecheckRoute);
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-exceptions-data-grid');
  await expect(grid).toBeVisible();

  const diagnostic = grid.getByTestId('order-exceptions-selection-diagnostic');
  await expect(diagnostic).toBeHidden();

  const rows = grid.locator('.ag-center-cols-container .ag-row.erp-data-grid-row');
  await page.waitForTimeout(800);
  const rowCount = await rows.count();
  test.skip(rowCount < 2, '当前复核数据不足 2 行，跳过点击诊断断言');

  const firstRowId = (await rows.nth(0).getAttribute('row-id')) ?? '';
  const secondRowId = (await rows.nth(1).getAttribute('row-id')) ?? '';
  const firstRowById = getBusinessRowById(grid, firstRowId);
  const secondRowById = getBusinessRowById(grid, secondRowId);
  const secondSelectionCell = getSelectionCell(grid, secondRowId);

  await firstRowById.locator('.ag-cell[col-id="order_no"]').first().click();
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

test('订单复核支持 Shift 连续勾选且以上一次点击行为锚点', async ({ page }) => {
  await page.goto(orderRecheckRoute);
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-exceptions-data-grid');
  await expect(grid).toBeVisible();

  const rows = grid.locator('.ag-center-cols-container .ag-row.erp-data-grid-row');
  await page.waitForTimeout(800);
  const rowCount = await rows.count();
  test.skip(rowCount < 4, '当前复核数据不足 4 行，跳过 Shift 范围勾选断言');

  const secondRow = rows.nth(1);
  const thirdRow = rows.nth(2);
  const fourthRow = rows.nth(3);

  const secondRowId = (await secondRow.getAttribute('row-id')) ?? '';
  const thirdRowId = (await thirdRow.getAttribute('row-id')) ?? '';
  const fourthRowId = (await fourthRow.getAttribute('row-id')) ?? '';

  const secondIdCell = grid.locator(
    `.ag-center-cols-container .ag-row[row-id="${secondRowId}"] .ag-cell[col-id="order_no"]`,
  );
  const fourthIdCell = grid.locator(
    `.ag-center-cols-container .ag-row[row-id="${fourthRowId}"] .ag-cell[col-id="order_no"]`,
  );
  const secondRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${secondRowId}"]`);
  const thirdRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${thirdRowId}"]`);
  const fourthRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${fourthRowId}"]`);

  await secondIdCell.click({ modifiers: ['Control'] });
  await expect(secondRowById).toHaveClass(/ag-row-selected/);
  await expect(thirdRowById).not.toHaveClass(/ag-row-selected/);
  await expect(fourthRowById).not.toHaveClass(/ag-row-selected/);

  await fourthIdCell.click({ modifiers: ['Shift'] });
  await expect(secondRowById).toHaveClass(/ag-row-selected/);
  await expect(thirdRowById).toHaveClass(/ag-row-selected/);
  await expect(fourthRowById).toHaveClass(/ag-row-selected/);

  await thirdRowById.locator('.ag-cell[col-id="order_no"]').first().click({ modifiers: ['Shift'] });
  await expect(secondRowById).toHaveClass(/ag-row-selected/);
  await expect(thirdRowById).not.toHaveClass(/ag-row-selected/);
  await expect(fourthRowById).not.toHaveClass(/ag-row-selected/);
});

test('订单复核支持 Shift 点击复选框按上次点击行为锚点进行连续勾选和取消', async ({ page }) => {
  await page.goto(orderRecheckRoute);
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-exceptions-data-grid');
  await expect(grid).toBeVisible();

  const rows = grid.locator('.ag-center-cols-container .ag-row.erp-data-grid-row');
  await page.waitForTimeout(800);
  const rowCount = await rows.count();
  test.skip(rowCount < 4, '当前复核数据不足 4 行，跳过 Shift 复选框范围勾选断言');

  const secondRow = rows.nth(1);
  const thirdRow = rows.nth(2);
  const fourthRow = rows.nth(3);

  const secondRowId = (await secondRow.getAttribute('row-id')) ?? '';
  const thirdRowId = (await thirdRow.getAttribute('row-id')) ?? '';
  const fourthRowId = (await fourthRow.getAttribute('row-id')) ?? '';

  const secondRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${secondRowId}"]`);
  const thirdRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${thirdRowId}"]`);
  const fourthRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${fourthRowId}"]`);

  await clickSelectionCheckboxCenter(page, grid, secondRowId);
  await expect(secondRowById).toHaveClass(/ag-row-selected/);
  await expect(thirdRowById).not.toHaveClass(/ag-row-selected/);
  await expect(fourthRowById).not.toHaveClass(/ag-row-selected/);

  await clickSelectionCheckboxCenter(page, grid, fourthRowId, ['Shift']);
  await expect(secondRowById).toHaveClass(/ag-row-selected/);
  await expect(thirdRowById).toHaveClass(/ag-row-selected/);
  await expect(fourthRowById).toHaveClass(/ag-row-selected/);

  await clickSelectionCheckboxCenter(page, grid, thirdRowId, ['Shift']);
  await expect(secondRowById).toHaveClass(/ag-row-selected/);
  await expect(thirdRowById).not.toHaveClass(/ag-row-selected/);
  await expect(fourthRowById).not.toHaveClass(/ag-row-selected/);
});

test('订单复核选择交互整链路回归保持稳定', async ({ page }) => {
  await page.goto(orderRecheckRoute);
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-exceptions-data-grid');
  await expect(grid).toBeVisible();

  const rows = grid.locator('.ag-center-cols-container .ag-row.erp-data-grid-row');
  await page.waitForTimeout(800);
  const rowCount = await rows.count();
  test.skip(rowCount < 4, '当前复核数据不足 4 行，跳过整链路选择回归');

  const firstRowId = (await rows.nth(0).getAttribute('row-id')) ?? '';
  const secondRowId = (await rows.nth(1).getAttribute('row-id')) ?? '';
  const thirdRowId = (await rows.nth(2).getAttribute('row-id')) ?? '';
  const fourthRowId = (await rows.nth(3).getAttribute('row-id')) ?? '';

  const firstRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${firstRowId}"]`);
  const secondRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${secondRowId}"]`);
  const thirdRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${thirdRowId}"]`);
  const fourthRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${fourthRowId}"]`);

  await firstRowById.locator('.ag-cell[col-id="order_no"]').first().click();
  await expect(firstRowById).toHaveClass(/erp-data-grid-row-active/);

  await clickSelectionColumnBlankArea(page, grid, secondRowId);
  await expect(secondRowById).toHaveClass(/ag-row-selected/);

  await thirdRowById.locator('.ag-cell[col-id="order_no"]').first().click({ modifiers: ['Control'] });
  await expect(secondRowById).toHaveClass(/ag-row-selected/);
  await expect(thirdRowById).toHaveClass(/ag-row-selected/);
  await expect(fourthRowById).not.toHaveClass(/ag-row-selected/);

  await fourthRowById.locator('.ag-cell[col-id="order_no"]').first().click({ modifiers: ['Shift'] });
  await expect(secondRowById).toHaveClass(/ag-row-selected/);
  await expect(thirdRowById).toHaveClass(/ag-row-selected/);
  await expect(fourthRowById).toHaveClass(/ag-row-selected/);

  await thirdRowById.locator('.ag-cell[col-id="order_no"]').first().click({ modifiers: ['Shift'] });
  await expect(secondRowById).toHaveClass(/ag-row-selected/);
  await expect(thirdRowById).not.toHaveClass(/ag-row-selected/);
  await expect(fourthRowById).not.toHaveClass(/ag-row-selected/);

  const secondOrderCell = secondRowById.locator('.ag-cell[col-id="order_no"]').first();
  await secondOrderCell.click();
  await secondOrderCell.press('Space');
  await expect(secondRowById).not.toHaveClass(/ag-row-selected/);
  await secondOrderCell.press('Space');
  await expect(secondRowById).toHaveClass(/ag-row-selected/);

  const selectionHeaderCheckbox = grid.getByTestId('erp-selection-header-checkbox');
  await selectionHeaderCheckbox.click();
  await expect.poll(async () => grid.locator('.ag-center-cols-container .ag-row.ag-row-selected').count()).toBe(rowCount);

  await selectionHeaderCheckbox.click();
  await expect(grid.locator('.ag-center-cols-container .ag-row.ag-row-selected')).toHaveCount(0);
});

test('订单复核左侧状态切换时首行 DOM 与 Grid 模型严格一致', async ({ page }) => {
  await page.goto(orderRecheckRoute);
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-exceptions-data-grid');
  await expect(grid).toBeVisible();

  const leftPanel = page.getByTestId('order-exceptions-left-panel');
  await expect(leftPanel).toBeVisible();

  const scenarios = [
    { nodeId: 'status:*', expectedStatus: null },
    { nodeId: 'status:OPEN', expectedStatus: 'OPEN' },
    { nodeId: 'status:PROCESSING', expectedStatus: 'PROCESSING' },
    { nodeId: 'status:ASSIGNED', expectedStatus: 'ASSIGNED' },
  ] as const;

  let executed = 0;
  for (const scenario of scenarios) {
    const nodeCell = leftPanel.locator(
      `.order-review-left-ag-grid-shell .ag-row[row-id="${scenario.nodeId}"] .ag-cell`,
    ).first();
    if ((await nodeCell.count()) === 0) {
      continue;
    }

    executed += 1;
    await nodeCell.click();
    await page.waitForTimeout(120);

    const visibleRows = grid.locator('.ag-center-cols-container .ag-row.erp-data-grid-row');
    const rowCount = await visibleRows.count();
    if (rowCount === 0) {
      continue;
    }

    const visualFirstRow = await page.evaluate<{
      rowId: string;
    } | null>(() => {
      const rows = Array.from(
        document.querySelectorAll<HTMLElement>(
          '[data-testid="order-exceptions-data-grid"] .ag-center-cols-container .ag-row.erp-data-grid-row',
        ),
      );
      if (!rows.length) {
        return null;
      }

      let visualTopRow: HTMLElement | null = null;
      let visualTop = Number.POSITIVE_INFINITY;
      for (const row of rows) {
        const top = Number.parseFloat(String(row.style.top ?? '0')) || 0;
        if (top < visualTop) {
          visualTop = top;
          visualTopRow = row;
        }
      }
      if (!visualTopRow) {
        return null;
      }

      const rowId = String(visualTopRow.getAttribute('row-id') ?? '').trim();
      return {
        rowId,
      };
    });

    expect(visualFirstRow).not.toBeNull();
    if (!visualFirstRow) {
      throw new Error(`无法读取 ${scenario.nodeId} 过滤后的视觉首行`);
    }
    const firstRowId = visualFirstRow.rowId;
    expect(firstRowId).not.toBe('');

    const snapshot = await page.evaluate<
      { rowId: string; orderNo: string; exceptionStatus: string } | null
    >(() => {
      const testApis = (window as Window & { __ERP_GRID_TEST_APIS?: Record<string, unknown> }).__ERP_GRID_TEST_APIS;
      const api = testApis?.['order-exceptions-data-grid'] as
        | {
            getDisplayedRowAtIndex: (
              index: number,
            ) => { id?: string; data?: { order_no?: string; exception_status?: string } } | null;
          }
        | undefined;
      if (!api) return null;

      const firstNode = api.getDisplayedRowAtIndex(0);
      if (!firstNode) return null;

      return {
        rowId: String(firstNode.id ?? '').trim(),
        orderNo: String(firstNode.data?.order_no ?? '').trim(),
        exceptionStatus: String(firstNode.data?.exception_status ?? '').trim(),
      };
    });

    expect(snapshot).not.toBeNull();
    if (!snapshot) {
      throw new Error(`无法读取 ${scenario.nodeId} 过滤后的首行 Grid 模型`);
    }

    expect(snapshot.rowId).toBe(firstRowId);
    expect(snapshot.orderNo).not.toBe('');
    if (scenario.expectedStatus) {
      expect(snapshot.exceptionStatus).toBe(scenario.expectedStatus);
    }
  }

  test.skip(executed === 0, '左侧状态节点未渲染，跳过首行一致性断言');
});

test('订单复核单行筛选结果在延迟后仍保持可见', async ({ page }) => {
  await page.goto(orderRecheckRoute);
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-exceptions-data-grid');
  await expect(grid).toBeVisible();

  const leftPanel = page.getByTestId('order-exceptions-left-panel');
  await expect(leftPanel).toBeVisible();

  const singleRowScenarios = ['status:PROCESSING', 'status:ASSIGNED'] as const;

  let executed = 0;
  for (const nodeId of singleRowScenarios) {
    const nodeCell = leftPanel.locator(
      `.order-review-left-ag-grid-shell .ag-row[row-id="${nodeId}"] .ag-cell`,
    ).first();
    if ((await nodeCell.count()) === 0) {
      continue;
    }

    executed += 1;
    await nodeCell.click();
    await page.waitForTimeout(2200);

    const snapshot = await page.evaluate<
      { displayedRowCount: number; orderNo: string; rowId: string } | null
    >(() => {
      const testApis = (window as Window & { __ERP_GRID_TEST_APIS?: Record<string, unknown> }).__ERP_GRID_TEST_APIS;
      const api = testApis?.['order-exceptions-data-grid'] as
        | {
            getDisplayedRowAtIndex: (
              index: number,
            ) => { id?: string; data?: { order_no?: string | null } } | null;
            getDisplayedRowCount: () => number;
          }
        | undefined;
      if (!api) {
        return null;
      }

      const firstNode = api.getDisplayedRowAtIndex(0);
      if (!firstNode) {
        return null;
      }

      return {
        displayedRowCount: api.getDisplayedRowCount(),
        orderNo: String(firstNode.data?.order_no ?? '').trim(),
        rowId: String(firstNode.id ?? '').trim(),
      };
    });

    expect(snapshot).not.toBeNull();
    if (!snapshot) {
      throw new Error(`鏃犳硶璇诲彇 ${nodeId} 鐨勫綋鍓嶉琛屽揩鐓?`);
    }

    const visualFirstRowText = await page.evaluate(() => {
      const rows = Array.from(
        document.querySelectorAll<HTMLElement>(
          '[data-testid="order-exceptions-data-grid"] .ag-center-cols-container .ag-row.erp-data-grid-row',
        ),
      );
      if (!rows.length) {
        return null;
      }

      let visualTopRow: HTMLElement | null = null;
      let visualTop = Number.POSITIVE_INFINITY;
      for (const row of rows) {
        const top = Number.parseFloat(String(row.style.top ?? '0')) || 0;
        if (top < visualTop) {
          visualTop = top;
          visualTopRow = row;
        }
      }

      return String(visualTopRow?.textContent ?? '').trim();
    });

    expect(snapshot.displayedRowCount).toBeGreaterThan(0);
    expect(snapshot.orderNo).not.toBe('');
    expect(visualFirstRowText).toContain(snapshot.orderNo);
  }

  test.skip(executed === 0, '左侧单行筛选节点未渲染，跳过延迟可见性断言');
});

test('订单复核左侧过滤会清空列筛选且不留下空白选择框', async ({ page }, testInfo) => {
  await page.goto(orderRecheckRoute);
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-exceptions-data-grid');
  await expect(grid).toBeVisible();

  const platformFilter = page.getByTestId('order-exceptions-floating-filter-platform');
  await expect(platformFilter).toBeVisible();

  const candidate = await page.evaluate<
    | {
        platform: string;
        leftNodeId: string;
      }
    | null
  >(() => {
    const gridTestWindow = window as Window & {
      __ERP_GRID_TEST_APIS?: Record<string, unknown>;
    };
    const api = gridTestWindow.__ERP_GRID_TEST_APIS?.['order-exceptions-data-grid'] as
      | {
          forEachNode: (
            callback: (node: {
              data?: {
                platform?: string | null;
                exception_status?: string | null;
                target_warehouse_id?: string | null;
                assignee_name?: string | null;
              };
            }) => void,
          ) => void;
        }
      | undefined;
    if (!api) {
      return null;
    }

    const rows: Array<{
      platform: string;
      status: string;
      warehouse: string;
      assignee: string;
    }> = [];

    api.forEachNode((node) => {
      const platform = String(node.data?.platform ?? '').trim();
      const status = String(node.data?.exception_status ?? '').trim();
      if (!platform || !status) {
        return;
      }
      rows.push({
        platform,
        status,
        warehouse: String(node.data?.target_warehouse_id ?? '').trim() || '__none__',
        assignee: String(node.data?.assignee_name ?? '').trim() || '__unassigned__',
      });
    });

    if (rows.length === 0) {
      return null;
    }

    const buildCandidate = (
      getNodeId: (row: (typeof rows)[number]) => string,
      getGroupValue: (row: (typeof rows)[number]) => string,
    ) => {
      const platforms = Array.from(new Set(rows.map((row) => row.platform)));
      const groups = Array.from(new Set(rows.map((row) => getGroupValue(row))));

      for (const platform of platforms) {
        for (const group of groups) {
          const groupRows = rows.filter((row) => getGroupValue(row) === group);
          if (groupRows.length === 0) {
            continue;
          }
          const hasPlatformInsideGroup = groupRows.some((row) => row.platform === platform);
          const hasPlatformOutsideGroup = rows.some(
            (row) => row.platform === platform && getGroupValue(row) !== group,
          );
          if (!hasPlatformInsideGroup && hasPlatformOutsideGroup) {
            return {
              platform,
              leftNodeId: getNodeId(groupRows[0]!),
            };
          }
        }
      }

      return null;
    };

    return (
      buildCandidate((row) => `status:${row.status}`, (row) => row.status) ??
      buildCandidate((row) => `warehouse:${row.warehouse}`, (row) => row.warehouse) ??
      buildCandidate((row) => `assignee:${row.assignee}`, (row) => row.assignee)
    );
  });

  test.skip(candidate == null, '当前模拟数据未找到“左侧过滤后应清空列筛选”的样本');
  if (candidate == null) {
    return;
  }

  await platformFilter.click();
  const dropdown = page.locator('.order-review-floating-filter-dropdown:visible').last();
  await expect(dropdown).toBeVisible();
  await dropdown
    .locator(`input[type="checkbox"][value="${candidate.platform}"]`)
    .first()
    .evaluate((input) => {
      (input.closest('.ant-checkbox-wrapper') as HTMLElement | null)?.click();
    });
  await expect(platformFilter).not.toContainText('全部平台');
  await grid.screenshot({
    path: testInfo.outputPath('order-exceptions-column-filter-active.png'),
  });

  const leftNode = page
    .locator(`.order-review-left-ag-grid-shell .ag-row[row-id="${candidate.leftNodeId}"] .ag-cell`)
    .first();
  await expect(leftNode).toBeVisible();
  await leftNode.click();
  await page.waitForTimeout(300);
  await grid.screenshot({
    path: testInfo.outputPath('order-exceptions-column-filter-cleared-after-left-filter.png'),
  });

  await expect
    .poll(async () => {
      return platformFilter.textContent();
    })
    .toContain('全部平台');

  await expect
    .poll(async () => grid.locator('.ag-center-cols-container .ag-row.erp-data-grid-row').count())
    .toBeGreaterThan(0);
  await expect
    .poll(async () => grid.locator('.ag-cell[col-id="__erpSelection"] .ag-selection-checkbox').count())
    .toBeGreaterThan(0);
});
