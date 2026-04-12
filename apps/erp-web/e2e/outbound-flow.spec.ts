import { expect, test, loginAndSelectTenant } from './fixtures';

test('outbound list and confirm pages render', async ({ page }) => {
  await page.goto('/wms/outbounds');

  await loginAndSelectTenant(page);

  await expect(page.getByTestId('outbound-list-title')).toBeVisible();

  const detailButtons = page.getByTestId('outbound-detail-link');
  if (await detailButtons.count()) {
    const detailButton = detailButtons.first();
    await expect(detailButton).toBeVisible();
    await detailButton.click();

    await expect(page.getByTestId('outbound-detail-drawer')).toBeVisible();
    await expect(page.getByTestId('outbound-detail-doc-no')).toBeVisible();
  } else {
    const emptyState = page.getByText('暂无出库单');
    if (await emptyState.count()) {
      await expect(emptyState).toBeVisible();
    }
  }

  await page.goto('/wms/outbound-confirm');

  await expect(page.getByTestId('outbound-confirm-input')).toBeVisible();
  await expect(page.getByTestId('outbound-confirm-recent')).toBeVisible();
});
