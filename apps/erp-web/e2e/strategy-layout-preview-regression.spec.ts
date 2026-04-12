import { expect, loginAndSelectTenant, test } from './fixtures';

test.describe('strategy layout preview regressions', () => {
  test('keeps Chinese module labels and preserves selection after row focus changes', async ({
    page
  }) => {
    await page.goto('/system/strategy-layout-preview');
    await loginAndSelectTenant(page);
    await page.goto('/system/strategy-layout-preview');
    await expect(page.getByTestId('strategy-layout-preview-page')).toBeVisible();

    const moduleButtons = page.locator('.workbench-module-item');
    await expect.poll(async () => moduleButtons.count()).toBeGreaterThan(6);
    const moduleTexts = (await moduleButtons.allTextContents()).map((item) => item.trim());
    expect(moduleTexts).toContain('系统');
    expect(moduleTexts).toContain('销售');

    const grid = page.getByTestId('strategy-layout-preview-main-grid');
    const gridBox = await grid.boundingBox();
    expect(gridBox?.height ?? 0).toBeGreaterThan(120);
    const rows = grid.locator('.erp-data-grid-row');
    await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThan(1);

    const firstRow = rows.first();
    const secondRow = rows.nth(1);
    const firstRowCheckbox = firstRow.locator(
      '.ag-cell[col-id="__erpSelection"] .ag-checkbox-input'
    );
    await firstRowCheckbox.click();
    await expect(page.locator('.ant-descriptions-item-content').first()).toHaveText('1');

    await secondRow
      .locator('.ag-cell[col-id]:not([col-id="__erpSelection"])')
      .first()
      .click({ force: true });

    await expect(page.locator('.ant-descriptions-item-content').first()).toHaveText('1');
  });
});

