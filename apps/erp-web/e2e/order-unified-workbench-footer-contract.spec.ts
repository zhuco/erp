import { expect, loginAndSelectTenant, test } from './fixtures';
import type { Page } from '@playwright/test';

type FooterContractDiagnostics = {
  footerVisible: boolean;
  footerHeight: number;
  sizeChangerVisible: boolean;
  customScrollbarVisible: boolean;
  nativeHorizontalVisible: boolean;
  nativeHorizontalVisibility: string;
  nativeHorizontalPointerEvents: string;
};

const collectFooterContractDiagnostics = async (
  page: Page,
  options: { pageSelector: string },
): Promise<FooterContractDiagnostics> =>
  page.evaluate(({ pageSelector }) => {
    const pageRoot = document.querySelector<HTMLElement>(pageSelector);
    const footer = pageRoot?.querySelector<HTMLElement>('.order-workbench-table-footer') ?? null;
    const sizeChanger =
      footer?.querySelector<HTMLElement>('.ant-pagination-options-size-changer') ?? null;
    const customScrollbar =
      footer?.querySelector<HTMLElement>('.order-workbench-scrollbar') ?? null;
    const nativeHorizontal =
      pageRoot?.querySelector<HTMLElement>('.ag-body-horizontal-scroll-viewport') ?? null;

    const isVisible = (node: HTMLElement | null) => {
      if (!node) {
        return false;
      }
      const rect = node.getBoundingClientRect();
      const style = window.getComputedStyle(node);
      return (
        rect.width > 0 &&
        rect.height > 0 &&
        style.display !== 'none' &&
        style.visibility !== 'hidden' &&
        style.opacity !== '0'
      );
    };

    const footerRect = footer?.getBoundingClientRect();
    const nativeStyle = nativeHorizontal ? window.getComputedStyle(nativeHorizontal) : null;

    return {
      footerVisible: isVisible(footer),
      footerHeight: footerRect?.height ?? 0,
      sizeChangerVisible: isVisible(sizeChanger),
      customScrollbarVisible: isVisible(customScrollbar),
      nativeHorizontalVisible: isVisible(nativeHorizontal),
      nativeHorizontalVisibility: nativeStyle?.visibility ?? '',
      nativeHorizontalPointerEvents: nativeStyle?.pointerEvents ?? '',
    };
  }, options);

const verifyUnifiedFooterContract = async (
  page: Page,
  options: {
    route: string;
    gridTestId: string;
    pageSelector: string;
  },
) => {
  await page.goto(options.route);
  await loginAndSelectTenant(page);

  const grid = page.getByTestId(options.gridTestId);
  await expect(grid).toBeVisible();
  await expect
    .poll(
      async () => grid.locator('.erp-data-grid-row, .erp-data-grid-next__row, .ag-row').count(),
      { timeout: 20_000 },
    )
    .toBeGreaterThan(0);

  const diagnostics = await collectFooterContractDiagnostics(page, {
    pageSelector: options.pageSelector,
  });

  console.log(
    `[workbench-footer-contract:${options.route}] ${JSON.stringify(diagnostics, null, 2)}`,
  );

  expect(diagnostics.footerVisible).toBe(true);
  expect(diagnostics.footerHeight).toBeGreaterThanOrEqual(40);
  expect(diagnostics.sizeChangerVisible).toBe(true);
  expect(diagnostics.customScrollbarVisible).toBe(true);
  expect(diagnostics.nativeHorizontalVisible).toBe(false);
  expect(diagnostics.nativeHorizontalVisibility).toBe('hidden');
  expect(diagnostics.nativeHorizontalPointerEvents).toBe('none');
};

test('order recheck follows the unified footer contract', async ({ page }) => {
  await verifyUnifiedFooterContract(page, {
    route: '/orders/recheck',
    gridTestId: 'order-exceptions-data-grid',
    pageSelector: '.order-exceptions-page',
  });
});

test('sales order query follows the unified footer contract', async ({ page }) => {
  await verifyUnifiedFooterContract(page, {
    route: '/sales/order-query',
    gridTestId: 'order-review-data-grid',
    pageSelector: '.order-query-page',
  });
});
