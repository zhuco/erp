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

  test('order list status overview covers core stages', async ({ page }) => {
    await page.goto('/orders');
    await loginAndSelectTenant(page);
    await expect(page.getByTestId('orders-keyword-input')).toBeVisible();

    const grid = page.getByTestId('orders-grid');
    const quickFilterPanel = page.getByTestId('orders-left-panel');
    const statusGroup = quickFilterPanel.locator('.ag-row').filter({ hasText: '订单状态' }).first();

    await expect(grid).toBeVisible();
    await expect(quickFilterPanel).toBeVisible();
    await expect(statusGroup).toBeVisible();
    await expect(statusGroup).toContainText('订单状态');

    await expect(grid.getByText('待发货').first()).toBeVisible();
    await expect(grid.getByText('待复核').first()).toBeVisible();
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

  test('exceptions workbench supports search and detail focus', async ({ page }) => {
    await page.goto('/orders/exceptions');
    await loginAndSelectTenant(page);

    await expect(page.getByTestId('order-exceptions-page')).toBeVisible();
    await expect(page.getByTestId('order-exceptions-search-input')).toBeVisible();
    await page.getByTestId('order-exceptions-search-input').fill('收件人');
    await page.getByTestId('order-exceptions-search-trigger').click();

    const grid = page.getByTestId('order-exceptions-data-grid');
    await expect(grid).toBeVisible();
    const rows = grid.locator('.erp-data-grid-row');
    await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThan(0);

    await rows.first().click();
    await expect(page.getByTestId('order-exceptions-resolve-action')).toBeVisible();
    await expect(page.getByTestId('order-exceptions-detail-panel')).toBeVisible();
  });

  test('shipment detail supports downstream flow', async ({ page }) => {
    await page.goto('/orders/TEST-ORDER-MANY-ITEMS');
    await loginAndSelectTenant(page);

    await expect(page.getByTestId('order-items-card')).toBeVisible();
    const shipTrigger = page.getByTestId('order-detail-ship');
    await expect(shipTrigger).toBeVisible();
    await shipTrigger.click();

    const shipDrawer = page.getByTestId('ship-drawer');
    await expect(shipDrawer).toBeVisible();
    await shipDrawer.getByTestId('ship-carrier-input').fill('SF');
    await shipDrawer.getByTestId('ship-tracking-input').fill(`TRACK-${Date.now()}`);

    const idempotencyInput = shipDrawer.getByTestId('ship-idempotency-input');
    await expect(idempotencyInput).toHaveValue(/.+/);

    await shipDrawer.getByTestId('ship-submit').click();
    await expect(page.getByTestId('ship-status-card')).toBeVisible();
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

