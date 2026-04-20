import { expect, test, loginAndSelectTenant } from './fixtures';
import type { Locator, Page } from '@playwright/test';

const reviewRowLocator = (grid: Locator) =>
  grid.locator('.ag-center-cols-container .ag-row.erp-data-grid-row');

const clearOrderWorkbenchSavedViews = async (page: Page) => {
  await page.evaluate(() => {
    const keys = Object.keys(window.localStorage).filter(
      (key) =>
        key.includes('orders.review.savedView.v1') ||
        key.includes('orders.exceptions.savedView.v1') ||
        key.includes('orders.list.savedView'),
    );
    keys.forEach((key) => window.localStorage.removeItem(key));
  });
};

const openOrderReviewPage = async (page: Page, resetSavedView = false) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);
  if (resetSavedView) {
    await clearOrderWorkbenchSavedViews(page);
    await page.reload();
    await loginAndSelectTenant(page);
  }

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();
  await expect
    .poll(async () => reviewRowLocator(grid).count(), { timeout: 20_000 })
    .toBeGreaterThan(0);
  return grid;
};

const readGridFilterModel = async (page: Page, gridKey: string) =>
  page.evaluate((targetKey) => {
    const testWindow = window as Window & {
      __ERP_GRID_TEST_APIS?: Record<string, { getFilterModel: () => Record<string, unknown> }>;
    };
    return testWindow.__ERP_GRID_TEST_APIS?.[targetKey]?.getFilterModel?.() ?? null;
  }, gridKey);

test('订单审核列筛选仅在回车后执行且输入时保持焦点（截图）', async ({ page }, testInfo) => {
  const grid = await openOrderReviewPage(page, true);

  const platformOrderInput = grid.getByTestId('order-review-floating-filter-platform-order-id');
  await expect(platformOrderInput).toBeVisible();

  const targetPlatformOrderId =
    ((await grid
      .locator('.ag-center-cols-container .ag-row.erp-data-grid-row .ag-cell[col-id="platform_order_id"]')
      .first()
      .textContent()) ?? '')
      .trim();
  expect(targetPlatformOrderId).not.toBe('');

  await platformOrderInput.click();
  await platformOrderInput.fill(targetPlatformOrderId);
  await grid.screenshot({
    path: testInfo.outputPath('order-review-column-filter-before-enter.png'),
  });

  await page.waitForTimeout(600);
  await expect(platformOrderInput).toBeFocused();
  await expect
    .poll(async () => {
      const model = await readGridFilterModel(page, 'order-review-data-grid');
      return Boolean(model && Object.prototype.hasOwnProperty.call(model, 'platform_order_id'));
    })
    .toBe(false);

  await platformOrderInput.press('Enter');
  await expect
    .poll(async () => {
      const model = await readGridFilterModel(page, 'order-review-data-grid');
      return Boolean(model && Object.prototype.hasOwnProperty.call(model, 'platform_order_id'));
    })
    .toBe(true);

  await grid.screenshot({
    path: testInfo.outputPath('order-review-column-filter-after-enter.png'),
  });
});

test('订单审核列筛选粘贴后自动执行（截图）', async ({ page }, testInfo) => {
  const grid = await openOrderReviewPage(page, true);

  const idInput = grid.getByTestId('order-review-floating-filter-id');
  await expect(idInput).toBeVisible();

  const targetOrderId =
    ((await grid
      .locator('.ag-center-cols-container .ag-row.erp-data-grid-row .ag-cell[col-id="id"]')
      .first()
      .textContent()) ?? '')
      .trim();
  expect(targetOrderId).not.toBe('');

  await idInput.evaluate((element, value) => {
    const input = element as HTMLInputElement;
    input.focus();
    const clipboardData = new DataTransfer();
    clipboardData.setData('text/plain', value);
    input.dispatchEvent(new ClipboardEvent('paste', { bubbles: true, clipboardData }));
  }, targetOrderId);

  await expect
    .poll(async () => {
      const model = await readGridFilterModel(page, 'order-review-data-grid');
      return Boolean(model && Object.prototype.hasOwnProperty.call(model, 'id'));
    })
    .toBe(true);

  await grid.screenshot({
    path: testInfo.outputPath('order-review-column-filter-after-paste.png'),
  });
});

test('订单审核列筛选在标签切换后保持原有钩选（截图）', async ({ page }, testInfo) => {
  const reviewGrid = await openOrderReviewPage(page, true);

  const platformSelect = reviewGrid.getByTestId('order-review-floating-filter-platform');
  await expect(platformSelect).toBeVisible();
  const selectedPlatform = await page.evaluate(() => {
    const testWindow = window as Window & {
      __ERP_GRID_TEST_APIS?: Record<string, { getDisplayedRowAtIndex?: (index: number) => { data?: { platform?: string } } | null }>;
    };
    return String(
      testWindow.__ERP_GRID_TEST_APIS?.['order-review-data-grid']?.getDisplayedRowAtIndex?.(0)?.data?.platform ?? '',
    ).trim();
  });
  expect(selectedPlatform).not.toBe('');
  await platformSelect.click();
  const dropdown = page.locator('.order-review-floating-filter-dropdown:visible').last();
  await expect(dropdown).toBeVisible();
  await dropdown
    .locator(`input[type="checkbox"][value="${selectedPlatform}"]`)
    .first()
    .evaluate((input) => {
      (input.closest('.ant-checkbox-wrapper') as HTMLElement | null)?.click();
    });
  await expect(platformSelect).not.toContainText('鍏ㄩ儴');
  await expect
    .poll(async () => {
      const model = await readGridFilterModel(page, 'order-review-data-grid');
      const platformFilter = (model as Record<string, { values?: string[] }> | null)?.platform;
      return String(platformFilter?.values?.[0] ?? '');
    })
    .toBe(selectedPlatform);

  const platformOrderInput = reviewGrid.getByTestId('order-review-floating-filter-platform-order-id');
  await expect(platformOrderInput).toBeVisible();
  const targetPlatformOrderId =
    ((await reviewGrid
      .locator('.ag-center-cols-container .ag-row.erp-data-grid-row .ag-cell[col-id="platform_order_id"]')
      .first()
      .textContent()) ?? '')
      .trim();
  expect(targetPlatformOrderId).not.toBe('');
  await platformOrderInput.fill(targetPlatformOrderId);
  await platformOrderInput.press('Enter');

  await expect
    .poll(async () =>
      page.evaluate(() => {
        const storageKey = Object.keys(window.localStorage).find((key) =>
          key.includes('orders.review.savedView.v1'),
        );
        if (!storageKey) {
          return '';
        }
        try {
          const parsed = JSON.parse(window.localStorage.getItem(storageKey) ?? '{}') as {
            columnFilters?: { platform?: string[] };
          };
          return String(parsed.columnFilters?.platform?.[0] ?? '');
        } catch {
          return '';
        }
      }),
    )
    .toBe(selectedPlatform);

  await reviewGrid.screenshot({
    path: testInfo.outputPath('order-review-column-filter-before-tab-switch.png'),
  });

  await page.goto('/orders/recheck');
  await loginAndSelectTenant(page);
  await expect(page.getByTestId('order-exceptions-data-grid')).toBeVisible();

  const reviewTab = page.locator('.workbench-tabs [role="tab"]').filter({ hasText: '订单审核' }).first();
  if ((await reviewTab.count()) > 0) {
    await reviewTab.click();
  } else {
    await page.goto('/orders/review');
  }
  await loginAndSelectTenant(page);

  const revisitedGrid = page.getByTestId('order-review-data-grid');
  await expect(revisitedGrid).toBeVisible();

  const revisitedPlatformSelect = revisitedGrid.getByTestId('order-review-floating-filter-platform');
  const revisitedPlatformOrderInput = revisitedGrid.getByTestId(
    'order-review-floating-filter-platform-order-id',
  );
  await expect(revisitedPlatformSelect).not.toContainText('鍏ㄩ儴');
  await expect(revisitedPlatformOrderInput).toHaveValue(targetPlatformOrderId);
  await expect
    .poll(async () => {
      const model = await readGridFilterModel(page, 'order-review-data-grid');
      const platformFilter = (model as Record<string, { values?: string[] }> | null)?.platform;
      return String(platformFilter?.values?.[0] ?? '');
    })
    .toBe(selectedPlatform);

  await revisitedGrid.screenshot({
    path: testInfo.outputPath('order-review-column-filter-after-tab-switch.png'),
  });
});
