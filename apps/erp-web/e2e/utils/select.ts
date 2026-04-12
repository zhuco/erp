import { expect, type Locator, type Page } from '@playwright/test';

export const selectAntdOptionByText = async (
  page: Page,
  selectTrigger: Locator,
  textOrRegex: string | RegExp
): Promise<void> => {
  await selectTrigger.scrollIntoViewIfNeeded();
  await selectTrigger.click();

  const dropdown = page
    .locator(
      '.ant-select-dropdown:visible, .ant-select-dropdown:not(.ant-select-dropdown-hidden)'
    )
    .last();
  await expect(dropdown).toBeVisible();

  const option = dropdown
    .locator('[role="option"], .ant-select-item-option')
    .filter({ hasText: textOrRegex })
    .first();
  await expect(option).toBeVisible();
  await option.scrollIntoViewIfNeeded();
  try {
    await option.click();
  } catch {
    await option.click({ force: true });
  }
};
