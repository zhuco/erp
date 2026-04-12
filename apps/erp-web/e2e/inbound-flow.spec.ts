import { expect, test, loginAndSelectTenant } from './fixtures';

test('inbound workflow pages render', async ({ page }) => {
  await page.goto('/wms/inbounds');

  await loginAndSelectTenant(page);

  await expect(page.getByTestId('inbound-list-title')).toBeVisible();

  const detailLinks = page.getByTestId('inbound-detail-link');
  if (await detailLinks.count()) {
    const detailLink = detailLinks.first();
    await expect(detailLink).toBeVisible();
    await detailLink.click();
  } else {
    const emptyState = page.getByText('暂无入库单');
    if (await emptyState.count()) {
      await expect(emptyState).toBeVisible();
    }
    return;
  }

  await expect(page.getByTestId('inbound-detail-title')).toBeVisible();
  await expect(page.getByTestId('inbound-detail-items')).toBeVisible();

  const match = page.url().match(/\/wms\/inbounds\/([^/]+)/);
  expect(match?.[1]).toBeTruthy();
  const docId = match?.[1] ?? '';

  await page.goto(`/wms/inbounds/${docId}/receive`);
  await expect(page.getByTestId('inbound-receive-title')).toBeVisible();
  await expect(page.getByTestId('inbound-receive-items')).toBeVisible();

  await page.goto(`/wms/inbounds/${docId}/inspect`);
  await expect(page.getByTestId('inbound-inspect-title')).toBeVisible();
  await expect(page.getByTestId('inbound-inspect-items')).toBeVisible();

  await page.goto(`/wms/inbounds/${docId}/putaway`);
  await expect(page.getByTestId('inbound-putaway-title')).toBeVisible();
  await expect(page.getByTestId('inbound-putaway-table')).toBeVisible();
});
