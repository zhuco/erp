import { expect, test, loginAndSelectTenant } from './fixtures';

test('order review shows vertical scrollbar', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();
  const rows = grid.locator('.erp-data-grid-row');
  await expect(rows.first()).toBeVisible();

  const tableBody = grid.locator('.ag-body-viewport').first();
  await expect(tableBody).toBeVisible();

  const metrics = await tableBody.evaluate((target) => {
    const element = target as HTMLElement;
    const before = element.scrollTop;
    element.scrollTop = element.scrollHeight;
    const canScroll = element.scrollTop > before;
    element.scrollTop = before;
    return {
      scrollHeight: element.scrollHeight,
      clientHeight: element.clientHeight,
      overflowY: getComputedStyle(element).overflowY,
      scrollbarWidth: element.offsetWidth - element.clientWidth,
      canScroll,
    };
  });

  expect(metrics).not.toBeNull();
  expect(metrics?.scrollHeight).toBeGreaterThan(metrics?.clientHeight ?? 0);
  expect(metrics?.canScroll).toBeTruthy();

  const canScrollX = await tableBody.evaluate((element) => {
    const before = element.scrollLeft;
    element.scrollLeft = element.scrollWidth;
    return element.scrollLeft > before;
  });
  expect(canScrollX).toBe(true);
});

