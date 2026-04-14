import { test, expect, loginAndSelectTenant } from './fixtures';

test('order review left panel keeps only the latest time filter highlighted', async ({ page }, testInfo) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const leftPanel = page.getByTestId('order-review-left-ag-panel');
  await expect(leftPanel).toBeVisible();

  const yesterdayRow = leftPanel.locator('.order-review-left-ag-grid-shell .ag-row[row-id="time:yesterday"]').first();
  const todayRow = leftPanel.locator('.order-review-left-ag-grid-shell .ag-row[row-id="time:today"]').first();
  const yesterdayCell = yesterdayRow.locator('.ag-cell').first();
  const todayCell = todayRow.locator('.ag-cell').first();

  await expect(yesterdayCell).toBeVisible();
  await expect(todayCell).toBeVisible();

  await yesterdayCell.click();
  await todayCell.click();
  await page.waitForTimeout(200);

  const yesterdayClass = (await yesterdayRow.getAttribute('class')) ?? '';
  const todayClass = (await todayRow.getAttribute('class')) ?? '';
  const yesterdayChecked = yesterdayClass.includes('order-review-left-ag-row-checked');
  const todayChecked = todayClass.includes('order-review-left-ag-row-checked');
  console.log(`[left-filter][review] yesterdayChecked=${yesterdayChecked} todayChecked=${todayChecked}`);

  const shot = testInfo.outputPath('order-review-time-filter-repro.png');
  await leftPanel.screenshot({ path: shot });
  console.log(`[left-filter][review] screenshot=${shot}`);

  expect(yesterdayChecked).toBe(false);
  expect(todayChecked).toBe(true);
});

test('order recheck left panel keeps only the latest status filter highlighted', async ({ page }, testInfo) => {
  await page.goto('/orders/recheck');
  await loginAndSelectTenant(page);

  const leftPanel = page.getByTestId('order-exceptions-left-panel');
  await expect(leftPanel).toBeVisible();

  const openRow = leftPanel.locator('.order-review-left-ag-grid-shell .ag-row[row-id="status:OPEN"]').first();
  const assignedRow = leftPanel.locator('.order-review-left-ag-grid-shell .ag-row[row-id="status:ASSIGNED"]').first();
  const openCell = openRow.locator('.ag-cell').first();
  const assignedCell = assignedRow.locator('.ag-cell').first();

  await expect(openCell).toBeVisible();
  await expect(assignedCell).toBeVisible();

  await openCell.click();
  await assignedCell.click();
  await page.waitForTimeout(200);

  const openClass = (await openRow.getAttribute('class')) ?? '';
  const assignedClass = (await assignedRow.getAttribute('class')) ?? '';
  const openChecked = openClass.includes('order-review-left-ag-row-checked');
  const assignedChecked = assignedClass.includes('order-review-left-ag-row-checked');
  console.log(`[left-filter][recheck] openChecked=${openChecked} assignedChecked=${assignedChecked}`);

  const shot = testInfo.outputPath('order-recheck-left-filter-repro.png');
  await leftPanel.screenshot({ path: shot });
  console.log(`[left-filter][recheck] screenshot=${shot}`);

  expect(openChecked).toBe(false);
  expect(assignedChecked).toBe(true);
});
