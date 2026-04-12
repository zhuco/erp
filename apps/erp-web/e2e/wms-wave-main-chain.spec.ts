import { expect, loginAndSelectTenant, test } from './fixtures';

test('workflow config enables wave execution on waves page', async ({ page }) => {
  await page.goto('/wms/workflow-configs');
  await loginAndSelectTenant(page);

  const waveSwitch = page.getByTestId('workflow-config-enable-wave-switch');

  await expect(waveSwitch).toBeVisible();
  if ((await waveSwitch.getAttribute('aria-checked')) !== 'true') {
    await waveSwitch.click();
  }

  await page.getByTestId('workflow-config-save').click();
  await expect(page.locator('.ant-message-notice').first()).toBeVisible();

  await page.goto('/wms/waves');

  const runWaveButton = page.getByTestId('wms-wave-run-button');
  await expect(runWaveButton).toBeEnabled();
});
