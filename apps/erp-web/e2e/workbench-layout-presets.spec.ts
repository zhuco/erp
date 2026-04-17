import { expect, loginAndSelectTenant, test } from './fixtures';

const openWorkbenchRoute = async (
  page: import('@playwright/test').Page,
  route: '/orders/review' | '/orders',
) => {
  await page.goto(route);
  if (route === '/orders/review') {
    await expect(page.getByTestId('order-review-data-grid')).toBeVisible();
    return;
  }
  await expect(page.getByTestId('orders-grid')).toBeVisible();
};

const expectLayoutPreset = async (
  page: import('@playwright/test').Page,
  preset: 'compact' | 'default' | 'analysis',
  presetLabel: '紧凑' | '标准' | '分析',
) => {
  await expect(page.locator('.workbench-shell')).toHaveAttribute(
    'data-workbench-layout-preset',
    preset,
  );
  await expect(page.getByTestId('workbench-layout-preset-status')).toContainText(
    `布局预设：${presetLabel}`,
  );
};

test.describe('workbench layout presets', () => {
  test('switches presets by buttons and updates status', async ({ page }) => {
    await page.goto('/dashboard');
    await loginAndSelectTenant(page);
    await openWorkbenchRoute(page, '/orders/review');

    await page.getByTestId('workbench-layout-preset-analysis').click();
    await expectLayoutPreset(page, 'analysis', '分析');

    await page.getByTestId('workbench-layout-preset-compact').click();
    await expectLayoutPreset(page, 'compact', '紧凑');

    await page.getByTestId('workbench-layout-preset-default').click();
    await expectLayoutPreset(page, 'default', '标准');
  });

  test('supports Alt+Shift+1/2/3 hotkeys', async ({ page }) => {
    await page.goto('/dashboard');
    await loginAndSelectTenant(page);
    await openWorkbenchRoute(page, '/orders/review');

    await page.locator('.workbench-main').click({ position: { x: 20, y: 20 } });

    await page.keyboard.press('Alt+Shift+1');
    await expectLayoutPreset(page, 'compact', '紧凑');

    await page.keyboard.press('Alt+Shift+2');
    await expectLayoutPreset(page, 'default', '标准');

    await page.keyboard.press('Alt+Shift+3');
    await expectLayoutPreset(page, 'analysis', '分析');
  });

  test('remembers preset per route key', async ({ page }) => {
    await page.goto('/dashboard');
    await loginAndSelectTenant(page);

    await openWorkbenchRoute(page, '/orders/review');
    await page.getByTestId('workbench-layout-preset-compact').click();
    await expectLayoutPreset(page, 'compact', '紧凑');

    await openWorkbenchRoute(page, '/orders');
    await expectLayoutPreset(page, 'default', '标准');
    await page.getByTestId('workbench-layout-preset-analysis').click();
    await expectLayoutPreset(page, 'analysis', '分析');

    await openWorkbenchRoute(page, '/orders/review');
    await expectLayoutPreset(page, 'compact', '紧凑');

    await openWorkbenchRoute(page, '/orders');
    await expectLayoutPreset(page, 'analysis', '分析');
  });

  test('keeps preset in URL for shareable workbench state', async ({ page }) => {
    await page.goto('/dashboard');
    await loginAndSelectTenant(page);
    await openWorkbenchRoute(page, '/orders/review');

    await page.getByTestId('workbench-layout-preset-analysis').click();
    await expectLayoutPreset(page, 'analysis', '分析');
    await expect(page).toHaveURL(/layoutPreset=analysis/);

    const sharedUrl = page.url();
    await page.goto(sharedUrl);
    await expect(page.getByTestId('order-review-data-grid')).toBeVisible();
    await expectLayoutPreset(page, 'analysis', '分析');
  });

  test('keeps extended panel state in URL and restores it after reload', async ({ page }) => {
    await page.goto('/dashboard');
    await loginAndSelectTenant(page);
    await openWorkbenchRoute(page, '/orders/review');

    await page.getByTestId('workbench-layout-preset-analysis').click();
    await expectLayoutPreset(page, 'analysis', '分析');

    const shell = page.locator('.workbench-shell');
    const beforeReloadState = await shell.evaluate((node) => ({
      leftCollapsed: node.getAttribute('data-workbench-left-collapsed'),
      rightCollapsed: node.getAttribute('data-workbench-right-collapsed'),
      bottomCollapsed: node.getAttribute('data-workbench-bottom-collapsed'),
      leftWidth: node.getAttribute('data-workbench-left-width'),
      rightWidth: node.getAttribute('data-workbench-right-width'),
      bottomHeight: node.getAttribute('data-workbench-bottom-height'),
    }));

    await expect(page).toHaveURL(/layoutPreset=analysis/);
    await expect(page).toHaveURL(/leftCollapsed=1/);
    await expect(page).toHaveURL(/rightCollapsed=0/);
    await expect(page).toHaveURL(/bottomCollapsed=0/);
    await expect(page).toHaveURL(/leftWidth=\d+/);
    await expect(page).toHaveURL(/rightWidth=\d+/);
    await expect(page).toHaveURL(/bottomHeight=\d+/);

    const sharedUrl = page.url();
    await page.goto(sharedUrl);
    await expect(page.getByTestId('order-review-data-grid')).toBeVisible();
    await expectLayoutPreset(page, 'analysis', '分析');

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
