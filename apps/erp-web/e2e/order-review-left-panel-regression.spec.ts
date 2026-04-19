import { expect, loginAndSelectTenant, test } from './fixtures';

test('order review left module shows icon tree, red counters, and bottom stats', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const leftPanel = page.getByTestId('order-review-left-ag-panel');
  const mainGrid = page.getByTestId('order-review-data-grid');
  await expect(leftPanel).toBeVisible();
  await expect(mainGrid).toBeVisible();

  const selectionHeaderCheckbox = mainGrid.getByTestId('erp-selection-header-checkbox');
  await expect(selectionHeaderCheckbox).toBeVisible();
  await expect(mainGrid.getByRole('button', { name: /全选|反选/ })).toHaveCount(0);

  const mainRows = mainGrid.locator('.ag-center-cols-container .ag-row');
  await expect.poll(async () => mainRows.count(), { timeout: 20_000 }).toBeGreaterThan(0);
  const selectedMainRows = mainGrid.locator('.ag-center-cols-container .ag-row.ag-row-selected');
  await expect(selectedMainRows).toHaveCount(0);
  await selectionHeaderCheckbox.click();
  await expect.poll(async () => selectedMainRows.count()).toBeGreaterThan(0);
  await selectionHeaderCheckbox.click();
  await expect(selectedMainRows).toHaveCount(0);

  const firstIcon = leftPanel.locator('.order-review-left-ag-node-icon').first();
  await expect(firstIcon).toBeVisible();

  const counters = leftPanel.getByTestId('order-review-left-node-count');
  await expect.poll(async () => counters.count(), { timeout: 20_000 }).toBeGreaterThan(0);
  await expect(counters.first()).toBeVisible();

  const stats = leftPanel.getByTestId('order-review-left-stats');
  await expect(stats).toBeVisible();
  await expect(stats.getByTestId('order-review-left-stats-pie')).toBeVisible();
  await expect(stats.getByText(/总订单/i)).toBeVisible();

  const totalBadge = stats.getByTestId('order-review-left-stats-total');
  await expect(totalBadge).toContainText(/总订单\s*\d+/);
  const totalBefore = (await totalBadge.innerText()).trim();

  const taobaoFilter = leftPanel.locator('[row-id="platform:taobao"] [role="gridcell"]').first();
  if ((await taobaoFilter.count()) === 0) {
    const platformGroup = leftPanel.locator('[row-id="platform-group"] [role="gridcell"]').first();
    await expect(platformGroup).toBeVisible();
    await platformGroup.click();
  }
  await expect(taobaoFilter).toBeVisible();
  await taobaoFilter.click();
  await expect(leftPanel.locator('[row-id="platform:taobao"].order-review-left-ag-row-checked')).toHaveCount(1);
  await expect(totalBadge).toHaveText(totalBefore);

  const loadTiming = page.getByTestId('order-review-load-timing');
  await expect(loadTiming).toContainText(/加载耗时|加载中/);
});
