import { expect, loginAndSelectTenant, test } from './fixtures';

test('系统集成页展示平台店铺接入总览并支持刷新 Token 入口', async ({ page }) => {
  await page.goto('/integrations/center');
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('system-integrations-title')).toBeVisible();
  await expect(page.getByTestId('system-integrations-overview-section')).toBeVisible();
  await expect(page.getByTestId('system-integrations-overview-table')).toContainText(
    '天猫旗舰店'
  );
  await expect(page.getByTestId('system-integrations-overview-table')).toContainText(
    '京东自营店'
  );
  await expect(page.getByTestId('system-integrations-overview-table')).toContainText('授权异常');

  await expect(page.getByTestId('system-integrations-overview-table')).toContainText(
    'PDD-WEBHOOK-DEAD'
  );
  await expect(page.getByTestId('system-integrations-overview-table')).toContainText(
    'Webhook 自动重试中'
  );
  await expect(page.getByTestId('system-integrations-overview-table')).toContainText(
    'Webhook 死信待处理'
  );

  const refreshButton = page.getByTestId('system-integrations-shop-refresh-SHOP-TB-001');
  await expect(refreshButton).toBeEnabled();
  await refreshButton.click();

  await expect(page.locator('.ant-message')).toContainText('已提交 Token 刷新任务');
  await expect(page.getByTestId('system-integrations-catalog-section')).toBeVisible();
});
