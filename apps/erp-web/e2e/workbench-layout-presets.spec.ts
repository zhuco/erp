import { expect, loginAndSelectTenant, test } from './fixtures';

const openWorkbenchRoute = async (
  page: import('@playwright/test').Page,
  route: '/orders/review' | '/sales/order-query',
) => {
  await page.goto(route);
  await expect(page.getByTestId('order-review-data-grid')).toBeVisible();
};

const expectWorkbenchFontScale = async (
  page: import('@playwright/test').Page,
  fontScale: 'small' | 'medium' | 'large',
  fontScaleLabel: '小' | '中' | '大',
) => {
  await expect(page.locator('.workbench-shell')).toHaveAttribute(
    'data-workbench-font-scale',
    fontScale,
  );
  await expect(page.getByTestId('workbench-font-scale-status')).toContainText(
    `工作台字号：${fontScaleLabel}`,
  );
};

const readWorkbenchFontSize = async (page: import('@playwright/test').Page) =>
  page.evaluate(() => {
    const shell = document.querySelector<HTMLElement>('.workbench-shell');
    if (!shell) {
      return '';
    }

    return window.getComputedStyle(shell).getPropertyValue('--erp-font-size-body').trim();
  });

test.describe('workbench font scale controls', () => {
  test('switches font scale by buttons and updates status', async ({ page }) => {
    await page.goto('/dashboard');
    await loginAndSelectTenant(page);
    await openWorkbenchRoute(page, '/orders/review');

    await page.getByTestId('workbench-font-scale-large').click();
    await expectWorkbenchFontScale(page, 'large', '大');
    await expect.poll(() => readWorkbenchFontSize(page)).toBe('13.8px');

    await page.getByTestId('workbench-font-scale-small').click();
    await expectWorkbenchFontScale(page, 'small', '小');
    await expect.poll(() => readWorkbenchFontSize(page)).toBe('11.8px');

    await page.getByTestId('workbench-font-scale-medium').click();
    await expectWorkbenchFontScale(page, 'medium', '中');
    await expect.poll(() => readWorkbenchFontSize(page)).toBe('12.8px');
  });

  test('supports Alt+Shift+1/2/3 hotkeys', async ({ page }) => {
    await page.goto('/dashboard');
    await loginAndSelectTenant(page);
    await openWorkbenchRoute(page, '/orders/review');

    await page.locator('.workbench-main').click({ position: { x: 20, y: 20 } });

    await page.keyboard.press('Alt+Shift+1');
    await expectWorkbenchFontScale(page, 'small', '小');
    await expect.poll(() => readWorkbenchFontSize(page)).toBe('11.8px');

    await page.keyboard.press('Alt+Shift+2');
    await expectWorkbenchFontScale(page, 'medium', '中');
    await expect.poll(() => readWorkbenchFontSize(page)).toBe('12.8px');

    await page.keyboard.press('Alt+Shift+3');
    await expectWorkbenchFontScale(page, 'large', '大');
    await expect.poll(() => readWorkbenchFontSize(page)).toBe('13.8px');
  });

  test('keeps font scale across integrated workbench routes and reload', async ({ page }) => {
    await page.goto('/dashboard');
    await loginAndSelectTenant(page);

    await openWorkbenchRoute(page, '/orders/review');
    await page.getByTestId('workbench-font-scale-large').click();
    await expectWorkbenchFontScale(page, 'large', '大');
    await expect.poll(() => readWorkbenchFontSize(page)).toBe('13.8px');

    await openWorkbenchRoute(page, '/sales/order-query');
    await expectWorkbenchFontScale(page, 'large', '大');
    await expect.poll(() => readWorkbenchFontSize(page)).toBe('13.8px');

    await openWorkbenchRoute(page, '/orders/review');
    await expectWorkbenchFontScale(page, 'large', '大');

    await page.reload();
    await expect(page.getByTestId('order-review-data-grid')).toBeVisible();
    await expectWorkbenchFontScale(page, 'large', '大');
    await expect.poll(() => readWorkbenchFontSize(page)).toBe('13.8px');
  });

  test('keeps extended panel state in URL and restores it after reload', async ({ page }) => {
    await page.goto('/dashboard');
    await loginAndSelectTenant(page);
    await openWorkbenchRoute(page, '/orders/review');

    await page.getByTestId('workbench-font-scale-large').click();
    await expectWorkbenchFontScale(page, 'large', '大');

    const shell = page.locator('.workbench-shell');
    const beforeReloadState = await shell.evaluate((node) => ({
      leftCollapsed: node.getAttribute('data-workbench-left-collapsed'),
      rightCollapsed: node.getAttribute('data-workbench-right-collapsed'),
      bottomCollapsed: node.getAttribute('data-workbench-bottom-collapsed'),
      leftWidth: node.getAttribute('data-workbench-left-width'),
      rightWidth: node.getAttribute('data-workbench-right-width'),
      bottomHeight: node.getAttribute('data-workbench-bottom-height'),
    }));

    const resolveCollapsedQueryValue = (value: string | null) => (value === 'true' ? '1' : '0');

    await expect(page).toHaveURL(
      new RegExp(`leftCollapsed=${resolveCollapsedQueryValue(beforeReloadState.leftCollapsed)}`),
    );
    await expect(page).toHaveURL(
      new RegExp(`rightCollapsed=${resolveCollapsedQueryValue(beforeReloadState.rightCollapsed)}`),
    );
    await expect(page).toHaveURL(
      new RegExp(
        `bottomCollapsed=${resolveCollapsedQueryValue(beforeReloadState.bottomCollapsed)}`,
      ),
    );
    await expect(page).toHaveURL(/leftWidth=\d+/);
    await expect(page).toHaveURL(/rightWidth=\d+/);
    await expect(page).toHaveURL(/bottomHeight=\d+/);

    const sharedUrl = page.url();
    await page.goto(sharedUrl);
    await expect(page.getByTestId('order-review-data-grid')).toBeVisible();
    await expectWorkbenchFontScale(page, 'large', '大');

    await expect(page).toHaveURL(/leftCollapsed=[01]/);
    await expect(page).toHaveURL(/rightCollapsed=[01]/);
    await expect(page).toHaveURL(/bottomCollapsed=[01]/);
    await expect(shell).toHaveAttribute('data-workbench-left-width', beforeReloadState.leftWidth ?? '');
    await expect(shell).toHaveAttribute(
      'data-workbench-right-width',
      beforeReloadState.rightWidth ?? '',
    );
    await expect(shell).toHaveAttribute(
      'data-workbench-bottom-height',
      beforeReloadState.bottomHeight ?? '',
    );
  });
});
