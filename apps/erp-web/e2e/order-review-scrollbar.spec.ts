import { expect, test, loginAndSelectTenant } from './fixtures';
import type { Page } from '@playwright/test';

type HorizontalScrollRegion = {
  selector: string;
  count: number;
  visibleCount: number;
  visibleNodes: Array<{
    className: string;
    width: number;
    height: number;
    clientWidth: number;
    clientHeight: number;
    scrollWidth: number;
    scrollHeight: number;
    scrollLeft: number;
    overflowX: string;
    overflowY: string;
    display: string;
    visibility: string;
    scrollbarThicknessX: number;
    rectBottom: number;
  }>;
};

type HorizontalScrollbarDiagnostics = {
  shellWidth: number;
  shellHeight: number;
  regions: HorizontalScrollRegion[];
  bodyHorizontalViewportVisibleCount: number;
  leftSpacerVisibleCount: number;
  rightSpacerVisibleCount: number;
  leftSpacerWidths: number[];
  rightSpacerWidths: number[];
  leftSpacerOverflowX: string[];
  rightSpacerOverflowX: string[];
  centerViewportVisibleCount: number;
  centerViewportOverflowX: string[];
};

type FooterScrollbarPlacementDiagnostics = {
  footerExists: boolean;
  footerBottom: number;
  paginationTop: number;
  paginationBottom: number;
  customScrollbarVisible: boolean;
  customScrollbarBottom: number;
  customScrollbarTop: number;
  nativeScrollbarVisible: boolean;
  nativeScrollbarBottom: number;
  gridBottom: number;
};

const collectHorizontalScrollbarDiagnostics = async (
  page: Page,
): Promise<HorizontalScrollbarDiagnostics> =>
  page.evaluate(() => {
    const gridRoot = document.querySelector<HTMLElement>('[data-testid="order-review-data-grid"]');
    const root = gridRoot?.closest<HTMLElement>('.order-review-table') ?? gridRoot;
    const shell = gridRoot;
    const selectors = [
      '.ag-body-horizontal-scroll',
      '.ag-body-horizontal-scroll-viewport',
      '.ag-body-horizontal-scroll-container',
      '.ag-horizontal-left-spacer',
      '.ag-horizontal-right-spacer',
      '.ag-center-cols-viewport',
      '.order-workbench-scrollbar-row',
      '.order-workbench-scrollbar',
      '.order-review-scrollbar-row',
      '.order-review-scrollbar',
    ];

    const regions = selectors.map((selector) => {
      const nodes = Array.from(root?.querySelectorAll<HTMLElement>(selector) ?? []);
      const visibleNodes = nodes
        .map((node) => {
          const rect = node.getBoundingClientRect();
          const style = window.getComputedStyle(node);
          const isVisible =
            rect.width > 0 &&
            rect.height > 0 &&
            style.display !== 'none' &&
            style.visibility !== 'hidden';
          const hasHorizontalOverflow = node.scrollWidth - node.clientWidth > 1;
          const allowsHorizontalScroll =
            style.overflowX === 'auto' ||
            style.overflowX === 'scroll' ||
            selector.includes('horizontal-scroll');

          if (!isVisible || !hasHorizontalOverflow || !allowsHorizontalScroll) {
            return null;
          }

          return {
            className: node.className,
            width: rect.width,
            height: rect.height,
            clientWidth: node.clientWidth,
            clientHeight: node.clientHeight,
            scrollWidth: node.scrollWidth,
            scrollHeight: node.scrollHeight,
            scrollLeft: node.scrollLeft,
            overflowX: style.overflowX,
            overflowY: style.overflowY,
            display: style.display,
            visibility: style.visibility,
            scrollbarThicknessX: node.offsetHeight - node.clientHeight,
            rectBottom: rect.bottom,
          };
        })
        .filter((item): item is NonNullable<typeof item> => item != null);

      return {
        selector,
        count: nodes.length,
        visibleCount: visibleNodes.length,
        visibleNodes,
      };
    });

    return {
      shellWidth: shell?.getBoundingClientRect().width ?? 0,
      shellHeight: shell?.getBoundingClientRect().height ?? 0,
      regions,
      bodyHorizontalViewportVisibleCount:
        regions.find((region) => region.selector === '.ag-body-horizontal-scroll-viewport')?.visibleCount ?? 0,
      leftSpacerVisibleCount:
        regions.find((region) => region.selector === '.ag-horizontal-left-spacer')?.visibleCount ?? 0,
      rightSpacerVisibleCount:
        regions.find((region) => region.selector === '.ag-horizontal-right-spacer')?.visibleCount ?? 0,
      leftSpacerWidths: Array.from(root?.querySelectorAll<HTMLElement>('.ag-horizontal-left-spacer') ?? []).map(
        (node) => node.getBoundingClientRect().width,
      ),
      rightSpacerWidths: Array.from(root?.querySelectorAll<HTMLElement>('.ag-horizontal-right-spacer') ?? []).map(
        (node) => node.getBoundingClientRect().width,
      ),
      leftSpacerOverflowX:
        Array.from(root?.querySelectorAll<HTMLElement>('.ag-horizontal-left-spacer') ?? []).map(
          (node) => window.getComputedStyle(node).overflowX,
        ),
      rightSpacerOverflowX:
        Array.from(root?.querySelectorAll<HTMLElement>('.ag-horizontal-right-spacer') ?? []).map(
          (node) => window.getComputedStyle(node).overflowX,
        ),
      centerViewportVisibleCount:
        regions.find((region) => region.selector === '.ag-center-cols-viewport')?.visibleCount ?? 0,
      centerViewportOverflowX:
        regions
          .find((region) => region.selector === '.ag-center-cols-viewport')
          ?.visibleNodes.map((node) => node.overflowX) ?? [],
    };
  });

const collectFooterScrollbarPlacementDiagnostics = async (
  page: Page,
): Promise<FooterScrollbarPlacementDiagnostics> =>
  page.evaluate(() => {
    const table = document.querySelector<HTMLElement>('.order-review-table');
    const footer = table?.querySelector<HTMLElement>('.order-workbench-table-footer') ?? null;
    const pagination = footer?.querySelector<HTMLElement>('.order-review-pagination') ?? null;
    const customScrollbar = footer?.querySelector<HTMLElement>('.order-review-scrollbar-row') ?? null;
    const nativeScrollbar = table?.querySelector<HTMLElement>('.ag-body-horizontal-scroll-viewport') ?? null;

    const footerRect = footer?.getBoundingClientRect();
    const paginationRect = pagination?.getBoundingClientRect();
    const customScrollbarRect = customScrollbar?.getBoundingClientRect();
    const nativeScrollbarRect = nativeScrollbar?.getBoundingClientRect();
    const tableRect = table?.getBoundingClientRect();

    const isVisible = (element: HTMLElement | null, rect?: DOMRect) => {
      if (!element || !rect) {
        return false;
      }
      const style = window.getComputedStyle(element);
      return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
    };

    return {
      footerExists: Boolean(footerRect),
      footerBottom: footerRect?.bottom ?? 0,
      paginationTop: paginationRect?.top ?? 0,
      paginationBottom: paginationRect?.bottom ?? 0,
      customScrollbarVisible: isVisible(customScrollbar, customScrollbarRect),
      customScrollbarBottom: customScrollbarRect?.bottom ?? 0,
      customScrollbarTop: customScrollbarRect?.top ?? 0,
      nativeScrollbarVisible: isVisible(nativeScrollbar, nativeScrollbarRect),
      nativeScrollbarBottom: nativeScrollbarRect?.bottom ?? 0,
      gridBottom: tableRect?.bottom ?? 0,
    };
  });

test('order review shows vertical scrollbar and keeps a dedicated footer scrollbar shell', async ({ page }) => {
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

  const footerScrollbar = page.locator('.order-workbench-scrollbar-row .order-workbench-scrollbar').first();
  await expect(footerScrollbar).toBeVisible();
});

test('order review main grid bottom shows only one visible horizontal scrollbar region', async ({
  page,
}) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();
  const rows = grid.locator('.erp-data-grid-row');
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThan(0);

  const diagnostics = await collectHorizontalScrollbarDiagnostics(page);
  const placement = await collectFooterScrollbarPlacementDiagnostics(page);
  console.log(`[order-review-scrollbar-diagnostics] ${JSON.stringify(diagnostics, null, 2)}`);
  console.log(`[order-review-footer-scrollbar-placement] ${JSON.stringify(placement, null, 2)}`);

  expect(diagnostics.bodyHorizontalViewportVisibleCount).toBe(0);
  expect(diagnostics.leftSpacerVisibleCount).toBe(0);
  expect(diagnostics.rightSpacerVisibleCount).toBe(0);
  expect(diagnostics.leftSpacerWidths.some((width) => width > 0)).toBe(true);
  expect(diagnostics.rightSpacerWidths.every((width) => width <= 1)).toBe(true);
  expect(diagnostics.leftSpacerOverflowX).toEqual(['hidden']);
  expect(diagnostics.rightSpacerOverflowX).toEqual(['hidden']);
  expect(diagnostics.centerViewportVisibleCount).toBe(0);
  expect(diagnostics.centerViewportOverflowX).toEqual([]);
  expect(placement.customScrollbarVisible).toBe(true);
  expect(placement.nativeScrollbarVisible).toBe(false);
});

test('order review keeps the only visible horizontal scrollbar above summary and pagination', async ({
  page,
}) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const grid = page.getByTestId('order-review-data-grid');
  await expect(grid).toBeVisible();
  await expect.poll(async () => grid.locator('.erp-data-grid-row').count(), { timeout: 20_000 }).toBeGreaterThan(0);

  const diagnostics = await collectFooterScrollbarPlacementDiagnostics(page);
  console.log(`[order-review-footer-scrollbar-placement] ${JSON.stringify(diagnostics, null, 2)}`);

  expect(diagnostics.footerExists).toBe(true);
  expect(diagnostics.customScrollbarVisible).toBe(true);
  expect(diagnostics.nativeScrollbarVisible).toBe(false);
  expect(diagnostics.customScrollbarBottom).toBeLessThanOrEqual(diagnostics.paginationTop);
  expect(Math.abs(diagnostics.customScrollbarBottom - diagnostics.paginationTop)).toBeLessThanOrEqual(1);
});

type VerticalScrollbarOwnershipDiagnostics = {
  outerContainer: {
    clientHeight: number;
    scrollHeight: number;
    canScroll: boolean;
    overflowY: string;
  } | null;
  gridViewport: {
    clientHeight: number;
    scrollHeight: number;
    canScroll: boolean;
    overflowY: string;
  } | null;
};

const collectVerticalScrollbarOwnershipDiagnostics = async (
  page: Page,
  gridTestId: string,
): Promise<VerticalScrollbarOwnershipDiagnostics> =>
  page.evaluate((targetGridTestId) => {
    const outerContainer = document.querySelector<HTMLElement>('.workbench-main-top');
    const gridRoot = document.querySelector<HTMLElement>(`[data-testid="${targetGridTestId}"]`);
    const gridViewport = gridRoot?.querySelector<HTMLElement>('.ag-body-viewport') ?? null;

    const toScrollableMetrics = (element: HTMLElement | null) => {
      if (!element) {
        return null;
      }

      return {
        clientHeight: element.clientHeight,
        scrollHeight: element.scrollHeight,
        canScroll: element.scrollHeight > element.clientHeight + 1,
        overflowY: window.getComputedStyle(element).overflowY,
      };
    };

    return {
      outerContainer: toScrollableMetrics(outerContainer),
      gridViewport: toScrollableMetrics(gridViewport),
    };
  }, gridTestId);

const verticalScrollbarOwnershipCases = [
  {
    name: '订单审核',
    route: '/orders/review',
    gridTestId: 'order-review-data-grid',
  },
  {
    name: '订单复核',
    route: '/orders/recheck',
    gridTestId: 'order-exceptions-data-grid',
  },
  {
    name: '订单查询',
    route: '/sales/order-query',
    gridTestId: 'order-review-data-grid',
  },
] as const;

for (const scenario of verticalScrollbarOwnershipCases) {
  test(`${scenario.name}主数据区只保留主表自身的纵向滚动`, async ({ page }) => {
    await page.goto(scenario.route);
    await loginAndSelectTenant(page);

    const grid = page.getByTestId(scenario.gridTestId);
    await expect(grid).toBeVisible();
    await expect
      .poll(async () => grid.locator('.erp-data-grid-row, .ag-center-cols-container .ag-row').count(), {
        timeout: 20_000,
      })
      .toBeGreaterThan(0);

    const diagnostics = await collectVerticalScrollbarOwnershipDiagnostics(page, scenario.gridTestId);
    console.log(
      `[order-workbench-vertical-scrollbar:${scenario.route}] ${JSON.stringify(diagnostics, null, 2)}`,
    );

    expect(diagnostics.gridViewport?.canScroll).toBe(true);
    expect(diagnostics.outerContainer?.canScroll).toBe(false);
  });
}
