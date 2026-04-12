import { expect, test, loginAndSelectTenant } from './fixtures';

test('客户档案右键不抖动', async ({ page }) => {
  await page.goto('/customers');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('customers-data-grid');
  await expect(grid).toBeVisible();
  const rows = grid.locator('.erp-data-grid-row');
  await expect.poll(async () => rows.count(), { timeout: 20000 }).toBeGreaterThan(0);
  await expect(rows.first()).toBeVisible();

  const tableHolder = grid.locator('.ag-body-viewport').first();
  await expect(tableHolder).toBeVisible();

  const { top: baseTop, canScroll } = await tableHolder.evaluate((holder) => {
    const scrollable = holder.scrollHeight > holder.clientHeight;
    if (scrollable) {
      holder.scrollTop = holder.scrollHeight;
      holder.dispatchEvent(new Event('scroll', { bubbles: true }));
    }
    return {
      top: holder.scrollTop,
      canScroll: scrollable
    };
  });

  const targetRow = canScroll ? rows.last() : rows.first();
  await targetRow.click({ button: 'right' });

  const menu = page.locator('.order-workbench-context-menu:visible');
  if (await menu.count()) {
    await expect(menu.first()).toBeVisible();
  }

  await page.keyboard.press('Escape');
  await page.mouse.click(10, 10);

  await expect
    .poll(async () => {
      const value = await tableHolder.evaluate((holder) => holder.scrollTop);
      return Math.abs(value - baseTop);
    })
    .toBeLessThan(6);
});

