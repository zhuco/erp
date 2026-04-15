import { mkdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { expect, loginAndSelectTenant, test } from './fixtures';
import type { Page, TestInfo } from '@playwright/test';

type FooterVisibilityDiagnostics = {
  viewportWidth: number;
  viewportHeight: number;
  footerVisible: boolean;
  footerRect: { left: number; right: number; top: number; bottom: number; width: number; height: number } | null;
  controlsVisible: boolean;
  controlsRect: { left: number; right: number; top: number; bottom: number; width: number; height: number } | null;
  sizeChangerVisible: boolean;
  sizeChangerRect: { left: number; right: number; top: number; bottom: number; width: number; height: number } | null;
  fullyInsideViewport: boolean;
  ancestorChain: Array<{
    tag: string;
    className: string;
    width: number;
    left: number;
    right: number;
    overflowX: string;
  }>;
  layoutChain: Array<{
    selector: string;
    found: boolean;
    width: number;
    left: number;
    right: number;
    overflowX: string;
  }>;
};

const collectFooterDiagnostics = async (
  page: Page,
  options: { footerSelector: string; gridTestId: string },
): Promise<FooterVisibilityDiagnostics> =>
  page.evaluate(({ selector, gridTestId }) => {
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;
    const grid = document.querySelector<HTMLElement>(`[data-testid="${gridTestId}"]`);
    const pageRoot = grid?.closest<HTMLElement>('.order-workbench-page');
    const footer =
      pageRoot?.querySelector<HTMLElement>('.order-workbench-table-footer') ??
      document.querySelector<HTMLElement>(selector);
    const controls = footer?.querySelector<HTMLElement>('.order-workbench-pagination-controls, .order-review-pagination-controls') ?? null;
    const sizeChanger = footer?.querySelector<HTMLElement>('.ant-pagination-options-size-changer') ?? null;

    const toRect = (node: HTMLElement | null) => {
      if (!node) {
        return null;
      }
      const rect = node.getBoundingClientRect();
      const style = window.getComputedStyle(node);
      const visible =
        rect.width > 0 &&
        rect.height > 0 &&
        style.display !== 'none' &&
        style.visibility !== 'hidden' &&
        style.opacity !== '0';
      if (!visible) {
        return null;
      }
      return {
        left: rect.left,
        right: rect.right,
        top: rect.top,
        bottom: rect.bottom,
        width: rect.width,
        height: rect.height,
      };
    };

    const footerRect = toRect(footer ?? null);
    const controlsRect = toRect(controls);
    const sizeChangerRect = toRect(sizeChanger);
    const fullyInsideViewport = Boolean(
      controlsRect &&
        sizeChangerRect &&
        controlsRect.left >= 0 &&
        controlsRect.right <= viewportWidth &&
        controlsRect.top >= 0 &&
        controlsRect.bottom <= viewportHeight &&
        sizeChangerRect.left >= 0 &&
        sizeChangerRect.right <= viewportWidth &&
        sizeChangerRect.top >= 0 &&
        sizeChangerRect.bottom <= viewportHeight,
    );

    const layoutChainSelectors = [
      '.workbench-shell',
      '.workbench-body',
      '.workbench-main',
      '.workbench-main-content',
      '.workbench-main-split',
      '.workbench-main-top',
      '.order-workbench-page.order-review-page',
      '.order-workbench-body',
      '.order-workbench-main',
      '.order-review-table',
      '.order-workbench-table-footer',
      '.order-workbench-pagination-controls',
    ];
    const layoutChain = layoutChainSelectors.map((layoutSelector) => {
      const node = document.querySelector<HTMLElement>(layoutSelector);
      if (!node) {
        return {
          selector: layoutSelector,
          found: false,
          width: 0,
          left: 0,
          right: 0,
          overflowX: '',
        };
      }
      const rect = node.getBoundingClientRect();
      const style = window.getComputedStyle(node);
      return {
        selector: layoutSelector,
        found: true,
        width: rect.width,
        left: rect.left,
        right: rect.right,
        overflowX: style.overflowX,
      };
    });

    const ancestorChain: FooterVisibilityDiagnostics['ancestorChain'] = [];
    let current: HTMLElement | null = pageRoot ?? null;
    let depth = 0;
    while (current && depth < 12) {
      const rect = current.getBoundingClientRect();
      const style = window.getComputedStyle(current);
      ancestorChain.push({
        tag: current.tagName.toLowerCase(),
        className: current.className,
        width: rect.width,
        left: rect.left,
        right: rect.right,
        overflowX: style.overflowX,
      });
      current = current.parentElement;
      depth += 1;
    }

    return {
      viewportWidth,
      viewportHeight,
      footerVisible: Boolean(footerRect),
      footerRect,
      controlsVisible: Boolean(controlsRect),
      controlsRect,
      sizeChangerVisible: Boolean(sizeChangerRect),
      sizeChangerRect,
      fullyInsideViewport,
      ancestorChain,
      layoutChain,
    };
  }, { selector: options.footerSelector, gridTestId: options.gridTestId });

const verifyFooterVisibleAndCapture = async (
  page: Page,
  testInfo: TestInfo,
  options: {
    route: string;
    gridTestId: string;
    footerSelector: string;
    screenshotName: string;
  },
) => {
  const artifactDir = resolve(process.cwd(), 'e2e', 'artifacts', 'pagination-visibility');
  mkdirSync(artifactDir, { recursive: true });

  await page.goto(options.route);
  await loginAndSelectTenant(page);

  const grid = page.getByTestId(options.gridTestId);
  await expect(grid).toBeVisible();
  await expect
    .poll(
      async () => grid.locator('.erp-data-grid-row, .erp-data-grid-next__row, .ag-center-cols-container .ag-row').count(),
      { timeout: 20_000 },
    )
    .toBeGreaterThan(0);

  const footer = page.locator(options.footerSelector).first();
  await expect(footer).toBeVisible();
  await expect(footer.locator('.ant-pagination')).toBeVisible();
  await expect(footer.locator('.ant-pagination-options-size-changer')).toBeVisible();

  await page.screenshot({
    path: testInfo.outputPath(options.screenshotName),
    fullPage: true,
  });
  await page.screenshot({
    path: resolve(artifactDir, options.screenshotName),
    fullPage: true,
  });

  const diagnostics = await collectFooterDiagnostics(page, {
    footerSelector: options.footerSelector,
    gridTestId: options.gridTestId,
  });
  console.log(`[footer-visibility:${options.route}] ${JSON.stringify(diagnostics, null, 2)}`);

  expect(diagnostics.footerVisible).toBe(true);
  expect(diagnostics.controlsVisible).toBe(true);
  expect(diagnostics.sizeChangerVisible).toBe(true);
  expect(diagnostics.fullyInsideViewport).toBe(true);
};

test('order review footer pagination and size changer are visible in viewport', async ({
  page,
}, testInfo) => {
  await verifyFooterVisibleAndCapture(page, testInfo, {
    route: '/orders/review',
    gridTestId: 'order-review-data-grid',
    footerSelector: '.order-review-page .order-workbench-table-footer',
    screenshotName: '01-order-review-pagination-visible.png',
  });
});

test('orders query footer pagination and size changer are visible in viewport', async ({
  page,
}, testInfo) => {
  await verifyFooterVisibleAndCapture(page, testInfo, {
    route: '/orders',
    gridTestId: 'orders-grid',
    footerSelector: '.order-query-page .order-workbench-table-footer',
    screenshotName: '02-order-query-pagination-visible.png',
  });
});
