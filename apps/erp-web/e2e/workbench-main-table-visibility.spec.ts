import { expect, loginAndSelectTenant, test } from './fixtures';
import type { Page, TestInfo } from '@playwright/test';

type GridDiagnostics = {
  gridApiKeys: string[];
  rootExists: boolean;
  rootRect: { width: number; height: number } | null;
  shellRect: { width: number; height: number } | null;
  rowCount: number;
  selectionColumnExists: boolean;
  selectionColumnWidth: number;
  columnIds: string[];
  diagnosticsText: string | null;
};

const collectGridDiagnostics = async (
  page: Page,
  dataTestId: string,
  selectionDiagnosticTestId?: string,
): Promise<GridDiagnostics> =>
  page.evaluate(
    ({ targetDataTestId, targetSelectionDiagnosticTestId }) => {
      const root = document.querySelector<HTMLElement>(`[data-testid="${targetDataTestId}"]`);
      const shell = root?.querySelector<HTMLElement>('.ag-root-wrapper') ?? null;
      const gridApi = window.__ERP_GRID_TEST_APIS?.[targetDataTestId];
      const columnDefs = gridApi?.getColumnDefs?.() ?? [];
      const rootRect = root?.getBoundingClientRect();
      const shellRect = shell?.getBoundingClientRect();
      const rows = root?.querySelectorAll('.erp-data-grid-row, .erp-data-grid-next__row, .ag-center-cols-container .ag-row') ?? [];
      const selectionDiagnostic = targetSelectionDiagnosticTestId
        ? document.querySelector<HTMLElement>(`[data-testid="${targetSelectionDiagnosticTestId}"]`)
        : null;

      return {
        gridApiKeys: Object.keys(window.__ERP_GRID_TEST_APIS ?? {}),
        rootExists: Boolean(root),
        rootRect: rootRect ? { width: rootRect.width, height: rootRect.height } : null,
        shellRect: shellRect ? { width: shellRect.width, height: shellRect.height } : null,
        rowCount: rows.length,
        selectionColumnExists: Boolean(gridApi?.getColumn?.('__erpSelection')),
        selectionColumnWidth: gridApi?.getColumn?.('__erpSelection')?.getActualWidth?.() ?? 0,
        columnIds: columnDefs.map((column) =>
          String(column.colId ?? column.field ?? column.headerName ?? 'unknown'),
        ),
        diagnosticsText: selectionDiagnostic?.textContent?.trim() ?? null,
      };
    },
    {
      targetDataTestId: dataTestId,
      targetSelectionDiagnosticTestId: selectionDiagnosticTestId ?? null,
    },
  );

const captureGridVisibleSnapshot = async (
  page: Page,
  testInfo: TestInfo,
  options: {
    route: string;
    readyGridTestId: string;
    screenshotName: string;
    selectionDiagnosticTestId?: string;
    requireSelectionColumn?: boolean;
    searchSetup?: () => Promise<void>;
    expectNoMaximumUpdateDepth?: boolean;
  },
) => {
  const consoleLogs: string[] = [];
  const pageErrors: string[] = [];

  page.on('console', (message) => {
    consoleLogs.push(`[${message.type()}] ${message.text()}`);
  });
  page.on('pageerror', (error) => {
    pageErrors.push(error instanceof Error ? error.stack ?? error.message : String(error));
  });

  await page.goto(options.route);
  await loginAndSelectTenant(page);

  if (options.searchSetup) {
    await options.searchSetup();
  }

  const grid = page.getByTestId(options.readyGridTestId);
  await expect(grid).toBeVisible();
  await expect
    .poll(async () => grid.locator('.erp-data-grid-row, .erp-data-grid-next__row, .ag-center-cols-container .ag-row').count(), {
      timeout: 20_000,
    })
    .toBeGreaterThan(0);

  await page.screenshot({
    path: testInfo.outputPath(options.screenshotName),
    fullPage: true,
  });

  const diagnostics = await collectGridDiagnostics(
    page,
    options.readyGridTestId,
    options.selectionDiagnosticTestId,
  );

  console.log(`[grid-diagnostics:${options.readyGridTestId}] ${JSON.stringify(diagnostics, null, 2)}`);
  if (consoleLogs.length > 0) {
    console.log(`[page-console:${options.readyGridTestId}]\n${consoleLogs.join('\n')}`);
  }
  if (pageErrors.length > 0) {
    console.log(`[page-errors:${options.readyGridTestId}]\n${pageErrors.join('\n')}`);
  }

  await expect(diagnostics.rootExists, `${options.readyGridTestId} 根节点必须存在`).toBe(true);
  await expect(diagnostics.rootRect?.height ?? 0, `${options.readyGridTestId} 高度不能为 0`).toBeGreaterThan(0);
  await expect(diagnostics.shellRect?.height ?? 0, `${options.readyGridTestId} AG Grid 根容器高度不能为 0`).toBeGreaterThan(0);
  await expect(diagnostics.rowCount, `${options.readyGridTestId} 首屏必须至少渲染一行`).toBeGreaterThan(0);

  if (options.requireSelectionColumn) {
    await expect(diagnostics.selectionColumnExists, `${options.readyGridTestId} 选择列必须存在`).toBe(true);
    await expect(diagnostics.selectionColumnWidth, `${options.readyGridTestId} 选择列宽度必须大于 0`).toBeGreaterThan(0);
  }

  if (options.expectNoMaximumUpdateDepth) {
    expect(consoleLogs.some((entry) => entry.includes('Maximum update depth exceeded'))).toBe(false);
  }
  expect(
    consoleLogs.some((entry) => entry.includes('Theming API and CSS File Themes are both used in the same page')),
  ).toBe(false);
};

test('订单审核首开后主表格可见并完成截图回归', async ({ page }, testInfo) => {
  await captureGridVisibleSnapshot(page, testInfo, {
    route: '/orders/review',
    readyGridTestId: 'order-review-data-grid',
    selectionDiagnosticTestId: 'order-review-selection-diagnostic',
    screenshotName: '01-order-review-main-grid.png',
    requireSelectionColumn: true,
    expectNoMaximumUpdateDepth: true,
  });
});

test('订单查询首开后主表格可见并完成截图回归', async ({ page }, testInfo) => {
  await captureGridVisibleSnapshot(page, testInfo, {
    route: '/orders',
    readyGridTestId: 'orders-grid',
    screenshotName: '02-orders-query-main-grid.png',
    searchSetup: async () => {
      await expect(page.getByTestId('orders-keyword-input')).toBeVisible();
      await page.getByTestId('orders-keyword-input').fill('PLAT-TEST-');
      const searchButton = page.getByTestId('orders-search-submit');
      await searchButton.scrollIntoViewIfNeeded();
      await searchButton.click({ trial: true });
      await searchButton.click();
    },
  });
});

test('订单异常首开后主表格可见并完成截图回归', async ({ page }, testInfo) => {
  await captureGridVisibleSnapshot(page, testInfo, {
    route: '/orders/exceptions',
    readyGridTestId: 'order-exceptions-data-grid',
    selectionDiagnosticTestId: 'order-exceptions-selection-diagnostic',
    screenshotName: '03-order-exceptions-main-grid.png',
    requireSelectionColumn: true,
  });
});
