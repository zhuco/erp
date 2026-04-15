import { expect, loginAndSelectTenant, test } from './fixtures';

test('multi-entity data scopes workflow exposes merchant and organization scopes', async ({ page }) => {
  await page.goto('/system/data-scopes');
  await loginAndSelectTenant(page);

  const userSelect = page.locator('.ant-select').first();
  await expect(userSelect).toBeVisible();
  await userSelect.click();
  await page.locator('.ant-select-item-option:visible').first().click();

  await expect(page.getByText('可访问店铺')).toBeVisible();
  await expect(page.getByText('可访问仓库')).toBeVisible();
  await expect(page.getByText('可访问商家')).toBeVisible();
  await expect(page.getByText('可访问组织')).toBeVisible();

  await page.goto('/system/users');
  await expect(page.getByText('用户管理')).toBeVisible();
});
