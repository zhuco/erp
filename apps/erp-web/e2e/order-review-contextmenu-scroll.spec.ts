import { expect, test, loginAndSelectTenant } from './fixtures';

test('order review context menu does not change scroll, active row, or selected scope', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();

  const rows = grid.locator('.erp-data-grid-row');
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThan(0);
  await expect(rows.first()).toBeVisible();

  // Establish selected set = 1 so context-menu scope should keep this count.
  const firstRowCheckbox = rows
    .first()
    .locator('.ag-cell[col-id="__erpSelection"] .ag-checkbox-input');
  await firstRowCheckbox.click();
  await expect(firstRowCheckbox).toBeChecked();

  const tableHolder = grid.locator('.ag-body-viewport').first();
  await expect(tableHolder).toBeVisible();
  await tableHolder.evaluate((holder) => {
    holder.scrollTop = holder.scrollHeight;
    holder.dispatchEvent(new Event('scroll', { bubbles: true }));
  });
  await expect.poll(async () => tableHolder.evaluate((holder) => holder.scrollTop)).toBeGreaterThanOrEqual(0);

  const mainBaseMetrics = await tableHolder.evaluate((holder) => ({
    top: holder.scrollTop,
    height: holder.scrollHeight,
    client: holder.clientHeight
  }));
  const mainBaseTop = mainBaseMetrics.top;
  const mainTolerance = 6;

  await expect(rows.first()).toBeVisible();
  await expect(rows.last()).toBeVisible();

  const rowCount = await rows.count();
  if (rowCount > 1) {
    await rows.first().click();
  }
  await rows.last().click({ button: 'right' });
  const mainMenu = page.locator('.order-workbench-context-menu.order-review-main-context-menu:visible').first();
  await expect(mainMenu).toBeVisible();
  await expect(page.getByTestId('order-review-context-selected-count')).toContainText('1');
  await expect(rows.first()).toHaveClass(/erp-data-grid-row-active/);

  await expect
    .poll(async () => {
      const value = await tableHolder.evaluate((holder) => holder.scrollTop);
      return Math.abs(value - mainBaseTop);
    })
    .toBeLessThan(mainTolerance);

  await page.mouse.click(10, 10);

  const tabPane = page.locator('.order-workbench-bottom-panel .ant-tabs-tabpane-active').first();
  await expect(tabPane).toBeVisible();

  const goodsRows = tabPane.locator('.ant-table-row');
  await expect.poll(async () => goodsRows.count(), { timeout: 20_000 }).toBeGreaterThan(0);

  const scrollMetrics = await tabPane.evaluate((pane) => {
    const before = pane.scrollTop;
    pane.scrollTop = pane.scrollHeight;
    return {
      before,
      after: pane.scrollTop,
      scrollHeight: pane.scrollHeight,
      clientHeight: pane.clientHeight
    };
  });

  const canScroll = scrollMetrics.scrollHeight > scrollMetrics.clientHeight;
  if (canScroll) {
    expect(scrollMetrics.after).toBeGreaterThanOrEqual(scrollMetrics.before);
  } else {
    expect(scrollMetrics.after).toBe(scrollMetrics.before);
  }

  const baseTop = await tabPane.evaluate((pane) => pane.scrollTop);

  const lastGoodsRow = goodsRows.last();
  await expect(lastGoodsRow).toBeVisible();
  await lastGoodsRow.click({ button: 'right' });
  const menu = page.locator('.order-workbench-context-menu.order-review-goods-context-menu:visible').first();
  await expect(menu).toBeVisible();

  await expect
    .poll(async () => {
      const value = await tabPane.evaluate((pane) => pane.scrollTop);
      return Math.abs(value - baseTop);
    })
    .toBeLessThan(2);

  await expect
    .poll(async () => {
      const value = await tableHolder.evaluate((holder) => holder.scrollTop);
      return Math.abs(value - mainBaseTop);
    })
    .toBeLessThan(mainTolerance);
});

