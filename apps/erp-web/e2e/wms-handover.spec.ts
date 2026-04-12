import { expect, loginAndSelectTenant, test } from './fixtures';

const defaultWaveId = '00000000-0000-4000-8000-000000000001';

test('outbound confirm supports wave batch handover', async ({ page }) => {
  await page.goto('/wms/outbound-confirm');
  await loginAndSelectTenant(page);

  const input = page.getByTestId('outbound-confirm-input');
  await expect(input).toBeVisible();

  await input.fill(defaultWaveId);
  await input.press('Enter');

  const waveModal = page.getByRole('dialog', { name: '波次批量出库确认' });
  await expect(waveModal).toBeVisible();
  await expect(waveModal).toContainText(defaultWaveId);

  await waveModal.getByRole('button', { name: '确认出库' }).click();
  await expect(waveModal).toBeHidden();

  await expect(page.getByTestId('outbound-confirm-recent')).toContainText(`波次 ${defaultWaveId}`);
});
