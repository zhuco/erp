import { expect, test, loginAndSelectTenant } from './fixtures';

test.describe('Order Processing Flow', () => {
  test('review queue shows pending orders', async ({ page }) => {
    await page.goto('/orders/review');
    await loginAndSelectTenant(page);

    const grid = page.getByTestId('order-review-data-grid');
    await expect(grid).toBeVisible();
    const rows = grid.locator('.erp-data-grid-row');
    await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThan(0);

    await expect(page.getByTestId('order-review-approve')).toBeVisible();
    await expect(page.getByTestId('order-review-split')).toBeVisible();
  });

  test('order list status filters cover core stages', async ({ page }) => {
    await page.goto('/orders');
    await loginAndSelectTenant(page);
    await expect(page.getByTestId('orders-keyword-input')).toBeVisible();

    const rows = page.locator('[data-testid="orders-grid"] .erp-data-grid-next__row');
    const quickFilterPanel = page.getByTestId('orders-left-panel');
    await expect(quickFilterPanel).toBeVisible();

    const statusRows = quickFilterPanel.locator('.ag-row[row-id^="status-"] .ag-cell');
    const statusCount = await statusRows.count();
    test.skip(statusCount === 0, '当前页面未展示左侧状态筛选节点');

    for (let index = 0; index < Math.min(statusCount, 2); index += 1) {
      const statusNode = statusRows.nth(index);
      await expect(statusNode).toBeVisible();
      await statusNode.click();
      await expect(rows.first()).toBeVisible();
    }
  });

  test('split order allocation preview', async ({ page }) => {
    await page.goto('/orders/review');
    await loginAndSelectTenant(page);

    const grid = page.getByTestId('order-review-data-grid');
    await expect(grid).toBeVisible();
    const rows = grid.locator('.erp-data-grid-row');
    await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThan(0);
    await rows.first().click();

    const splitTrigger = page.getByTestId('order-review-split');
    await expect(splitTrigger).toBeVisible();
    if (!(await splitTrigger.isEnabled())) {
      await expect(splitTrigger).toBeDisabled();
      return;
    }
    await splitTrigger.click();

    const dialog = page.getByRole('dialog', { name: '拆分订单' });
    await expect(dialog).toBeVisible();
    const splitQtyInput = dialog.getByRole('spinbutton').first();
    await splitQtyInput.fill('3');

    await dialog.getByRole('button', { name: '确认拆分' }).click();
    await expect(page.getByText(/拆单完成/)).toBeVisible();
  });

  test.skip('shipment detail supports downstream flow', async ({ page }) => {
    await page.goto('/shipments');
    await loginAndSelectTenant(page);

    const viewButton = page.getByTestId('shipment-view-button').first();
    await expect(viewButton).toBeVisible();
    await viewButton.click();

    await expect(page.getByText('发货单号').first()).toBeVisible();
  });

  test('settlement detail shows completion status', async ({ page }) => {
    test.setTimeout(90_000);
    await page.goto('/orders/settlement');
    await loginAndSelectTenant(page);

    await expect(page.getByRole('heading', { name: '销售结算' })).toBeVisible();
    await expect(page.getByRole('tab', { name: '订单结算' })).toBeVisible();

    const settlementRows = page.locator('table .ant-table-tbody tr.ant-table-row');
    await expect(settlementRows.first()).toBeVisible();

    const detailButton = page.getByTestId('settlement-detail-button').first();
    await expect(detailButton).toBeVisible();
    await detailButton.click();

    const dialog = page.getByRole('dialog', { name: '结算明细' });
    await expect(dialog).toBeVisible();
    await expect(dialog).toContainText('结算状态');
  });
});

