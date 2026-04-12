import type { Page } from '@playwright/test';

import { expect, loginAndSelectTenant, test } from './fixtures';

const gotoPreview = async (page: Page) => {
  await page.goto('/system/strategy-layout-preview');
  await loginAndSelectTenant(page);
  await page.goto('/system/strategy-layout-preview');
  await expect(page.getByTestId('strategy-layout-preview-page')).toBeVisible();
};

test.describe('strategy layout preview interactions', () => {
  test('left tree +/- toggle expands and collapses correctly', async ({ page }) => {
    await gotoPreview(page);

    const tree = page.getByTestId('strategy-layout-preview-tree');
    const treeRows = tree.locator('.strategy-tree-list-row');
    const beforeCount = await treeRows.count();
    expect(beforeCount).toBeGreaterThan(0);

    const exceptionRow = treeRows
      .filter({
        has: page.locator('.strategy-tree-title', { hasText: '异常' })
      })
      .first();
    const exceptionToggle = exceptionRow.locator('.strategy-tree-toggle[data-tree-toggle="1"]');

    await expect(exceptionToggle).toHaveText('+');
    await exceptionToggle.click();
    await expect(exceptionToggle).toHaveText('-');
    await expect.poll(async () => treeRows.count()).toBeGreaterThan(beforeCount);

    await exceptionToggle.click();
    await expect(exceptionToggle).toHaveText('+');
    await expect.poll(async () => treeRows.count()).toBe(beforeCount);
  });

  test('main grid row click does not recreate table and does not reset outer scroll', async ({ page }) => {
    await gotoPreview(page);

    const grid = page.getByTestId('strategy-layout-preview-main-grid');
    const rows = grid.locator('.erp-data-grid-row');
    await expect.poll(async () => rows.count()).toBeGreaterThan(5);

    const pageRoot = page.getByTestId('strategy-layout-preview-page');
    const getPageMountId = async () =>
      pageRoot.evaluate((element) => {
        const host = element as HTMLElement;
        if (!host.dataset.e2eMountId) {
          host.dataset.e2eMountId = `${Date.now()}-${Math.random().toString(16).slice(2)}`;
        }
        return host.dataset.e2eMountId;
      });
    const pageMountBefore = await getPageMountId();

    const tableHost = grid.locator('.erp-data-grid-table').first();
    const getInstanceId = async () =>
      tableHost.evaluate((element) => {
        const host = element as HTMLElement;
        return (
          host.dataset.erpTableInstanceId ??
          host.getAttribute('data-erp-table-instance-id') ??
          host
            .querySelector<HTMLElement>('[data-erp-table-instance-id]')
            ?.getAttribute('data-erp-table-instance-id') ??
          ''
        );
      });

    const instanceBefore = await getInstanceId();
    expect(instanceBefore).not.toBe('');
    await page.waitForTimeout(400);
    expect(await getInstanceId()).toBe(instanceBefore);

    const scrollContainer = page.locator('.workbench-main-top').first();
    const topBefore = await scrollContainer.evaluate((element) => {
      const host = element as HTMLElement;
      host.scrollTop = 240;
      return host.scrollTop;
    });
    expect(topBefore).toBeGreaterThan(100);

    await rows
      .nth(1)
      .locator('.ag-cell[col-id]:not([col-id="__erpSelection"])')
      .first()
      .click({ force: true });

    const pageMountAfter = await getPageMountId();
    const topAfter = await scrollContainer.evaluate((element) => (element as HTMLElement).scrollTop);

    await expect.poll(async () => getInstanceId()).toBe(instanceBefore);
    expect(pageMountAfter).toBe(pageMountBefore);
    expect(Math.abs(topAfter - topBefore)).toBeLessThanOrEqual(24);
  });

  test('bottom tab switch does not recreate main grid', async ({ page }) => {
    await gotoPreview(page);

    const grid = page.getByTestId('strategy-layout-preview-main-grid');
    const rows = grid.locator('.erp-data-grid-row');
    await expect.poll(async () => rows.count()).toBeGreaterThan(5);

    const tableHost = grid.locator('.erp-data-grid-table').first();
    const getInstanceId = async () =>
      tableHost.evaluate((element) => {
        const host = element as HTMLElement;
        return (
          host.dataset.erpTableInstanceId ??
          host.getAttribute('data-erp-table-instance-id') ??
          host
            .querySelector<HTMLElement>('[data-erp-table-instance-id]')
            ?.getAttribute('data-erp-table-instance-id') ??
          ''
        );
      });
    const instanceBefore = await getInstanceId();
    expect(instanceBefore).not.toBe('');

    const scrollContainer = page.locator('.workbench-main-top').first();
    const topBefore = await scrollContainer.evaluate((element) => {
      const host = element as HTMLElement;
      host.scrollTop = 240;
      return host.scrollTop;
    });
    expect(topBefore).toBeGreaterThan(100);

    const bottomTabs = page.locator('.strategy-layout-preview-bottom .ant-tabs-tab');
    await expect.poll(async () => bottomTabs.count()).toBeGreaterThanOrEqual(4);
    await bottomTabs.nth(2).click();
    await bottomTabs.nth(3).click();
    await bottomTabs.nth(0).click();

    await expect.poll(async () => getInstanceId()).toBe(instanceBefore);
    const topAfter = await scrollContainer.evaluate((element) => (element as HTMLElement).scrollTop);
    expect(Math.abs(topAfter - topBefore)).toBeLessThanOrEqual(24);
  });

  test('selection counter stays stable after row focus click', async ({ page }) => {
    await gotoPreview(page);

    const grid = page.getByTestId('strategy-layout-preview-main-grid');
    const rows = grid.locator('.erp-data-grid-row');
    await expect.poll(async () => rows.count()).toBeGreaterThan(1);

    const firstCheckbox = rows
      .first()
      .locator('.ag-cell[col-id="__erpSelection"] .ag-checkbox-input');
    await firstCheckbox.click();
    await expect(page.locator('.ant-descriptions-item-content').first()).toHaveText('1');

    await rows
      .nth(1)
      .locator('.ag-cell[col-id]:not([col-id="__erpSelection"])')
      .first()
      .click({ force: true });
    await expect(page.locator('.ant-descriptions-item-content').first()).toHaveText('1');
  });

  test('can switch to order data mode and expose order-review entry', async ({ page }) => {
    await gotoPreview(page);

    await page
      .locator('.ant-segmented-item')
      .filter({ hasText: '订单审核数据' })
      .first()
      .click();

    const grid = page.getByTestId('strategy-layout-preview-main-grid');
    const rows = grid.locator('.erp-data-grid-row');
    await expect.poll(async () => rows.count()).toBeGreaterThan(0);

    const headerCells = grid.locator('.ag-header-cell-text');
    await expect(headerCells.filter({ hasText: '订单号' }).first()).toBeVisible();
    await expect(headerCells.filter({ hasText: '平台订单号' }).first()).toBeVisible();

    await expect(
      page.getByRole('button', { name: '打开订单审核页' }).first()
    ).toBeVisible();
  });
});

