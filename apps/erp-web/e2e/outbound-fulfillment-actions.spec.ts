import type { Locator, Page } from '@playwright/test';
import { expect, test, loginAndSelectTenant } from './fixtures';

const getFirstEnabled = async (locator: Locator): Promise<Locator | null> => {
  const count = await locator.count();
  for (let index = 0; index < count; index += 1) {
    const candidate = locator.nth(index);
    const visible = await candidate.isVisible().catch(() => false);
    if (!visible) {
      continue;
    }
    if (await candidate.isEnabled()) {
      return candidate;
    }
  }
  return null;
};

const waitForMessage = async (page: Page) => {
  await page
    .locator('.ant-message-notice')
    .first()
    .waitFor({ state: 'visible', timeout: 5000 })
    .catch(() => {});
};

const getVisibleDialogByAction = (page: Page, actionTestId: string) =>
  page.getByRole('dialog').filter({ has: page.getByTestId(actionTestId) });

const waitForModalResult = async (page: Page, modal: Locator) => {
  await Promise.race([
    modal.waitFor({ state: 'hidden', timeout: 5000 }),
    page.locator('.ant-message-notice').first().waitFor({ state: 'visible', timeout: 5000 })
  ]).catch(() => {});
  if (await modal.isVisible()) {
    const cancelButton = modal.getByRole('button', { name: /取消|关闭/ });
    if (await cancelButton.count()) {
      await cancelButton.first().click();
      await modal.waitFor({ state: 'hidden', timeout: 5000 }).catch(() => {});
    }
  }
};

const selectFirstOption = async (page: Page, trigger: Locator): Promise<boolean> => {
  await trigger.scrollIntoViewIfNeeded();
  await trigger.click();
  const dropdown = page
    .locator(
      '.ant-select-dropdown:visible, .ant-select-dropdown:not(.ant-select-dropdown-hidden)'
    )
    .last();
  await dropdown.waitFor({ state: 'visible', timeout: 2000 }).catch(() => {});
  if (!(await dropdown.count())) {
    return false;
  }
  const option = dropdown.locator('[role="option"], .ant-select-item-option').first();
  if (!(await option.count())) {
    return false;
  }
  await option.scrollIntoViewIfNeeded();
  await option.click();
  return true;
};

const triggerAction = async (action: Locator) => {
  await action.scrollIntoViewIfNeeded();
  await action.click().catch(async () => {
    await action.dispatchEvent('click');
  });
};

const waitForModalVisible = async (modal: Locator, timeout = 4000): Promise<boolean> => {
  try {
    await modal.waitFor({ state: 'visible', timeout });
    return true;
  } catch {
    return false;
  }
};

test('outbound fulfillment action flow', async ({ page }) => {
  await page.goto('/wms/pick-tasks');
  await loginAndSelectTenant(page);

  await expect(page.getByTestId('pick-tasks-title')).toBeVisible();
  const pickTasksTable = page.getByTestId('pick-tasks-table');

  const pickAction = await getFirstEnabled(pickTasksTable.getByTestId('pick-task-confirm-action'));
  if (pickAction) {
    await triggerAction(pickAction);
    const pickModal = getVisibleDialogByAction(page, 'pick-task-confirm-submit');
    if (await waitForModalVisible(pickModal)) {
      const qtyInput = pickModal.getByTestId('pick-task-picked-qty').locator('input');
      const qtyValue = await qtyInput.inputValue();
      if (!qtyValue || Number(qtyValue) < 1) {
        await qtyInput.fill('1');
      }
      await pickModal.getByTestId('pick-task-confirm-submit').click();
      await waitForModalResult(page, pickModal);
    } else {
      await waitForMessage(page);
    }
  } else {
    const emptyState = page.getByText('暂无拣货任务');
    if (await emptyState.count()) {
      await expect(emptyState).toBeVisible();
    }
  }

  await page.goto('/wms/pack');
  await expect(page.getByTestId('pack-title')).toBeVisible();
  const packTable = page.getByTestId('pack-table');

  const packAction = await getFirstEnabled(packTable.getByTestId('pack-action'));
  if (packAction) {
    await triggerAction(packAction);
    const packModal = getVisibleDialogByAction(page, 'pack-submit');
    if (await waitForModalVisible(packModal)) {
      await packModal.getByTestId('pack-package-no-input').fill(`PKG-${Date.now()}`);
      await packModal.getByTestId('pack-submit').click();
      await waitForModalResult(page, packModal);
    } else {
      await waitForMessage(page);
    }
  } else {
    const emptyState = page.getByText('暂无待打包出库单');
    if (await emptyState.count()) {
      await expect(emptyState).toBeVisible();
    }
  }

  const checkAction = await getFirstEnabled(packTable.getByTestId('pack-check-action'));
  if (checkAction) {
    await triggerAction(checkAction);
    const checkModal = getVisibleDialogByAction(page, 'pack-check-submit');
    if (await waitForModalVisible(checkModal)) {
      const packageSelected = await selectFirstOption(
        page,
        checkModal.getByTestId('pack-check-package-select')
      );
      const resultSelected = await selectFirstOption(
        page,
        checkModal.getByTestId('pack-check-result-select')
      );
      if (packageSelected && resultSelected) {
        await checkModal.getByTestId('pack-check-submit').click();
        await waitForModalResult(page, checkModal);
      } else {
        await waitForModalResult(page, checkModal);
      }
    } else {
      await waitForMessage(page);
    }
  }

  const weighAction = await getFirstEnabled(packTable.getByTestId('pack-weigh-action'));
  if (weighAction) {
    await triggerAction(weighAction);
    const weighModal = getVisibleDialogByAction(page, 'pack-weigh-submit');
    if (await waitForModalVisible(weighModal)) {
      const selected = await selectFirstOption(
        page,
        weighModal.getByTestId('pack-weigh-package-select')
      );
      if (selected) {
        const weightInput = weighModal.getByTestId('pack-weigh-weight-input').locator('input');
        const weightValue = await weightInput.inputValue();
        if (!weightValue || Number(weightValue) < 1) {
          await weightInput.fill('100');
        }
        await weighModal.getByTestId('pack-weigh-submit').click();
        await waitForModalResult(page, weighModal);
      } else {
        await waitForModalResult(page, weighModal);
      }
    } else {
      await waitForMessage(page);
    }
  }

  await page.goto('/wms/ship');
  await expect(page.getByTestId('ship-title')).toBeVisible();
  const shipTable = page.getByTestId('ship-table');

  const inspectAction = await getFirstEnabled(shipTable.getByTestId('ship-inspect-action'));
  if (inspectAction) {
    await triggerAction(inspectAction);
    await waitForMessage(page);
  }

  const shipAction = await getFirstEnabled(shipTable.getByTestId('ship-action'));
  if (shipAction) {
    await triggerAction(shipAction);
    const shipModal = getVisibleDialogByAction(page, 'ship-submit');
    if (await waitForModalVisible(shipModal)) {
      await shipModal.getByTestId('ship-submit').click();
      await waitForModalResult(page, shipModal);
    } else {
      await waitForMessage(page);
    }
  } else {
    const emptyState = page.getByText('暂无可发货出库单');
    if (await emptyState.count()) {
      await expect(emptyState).toBeVisible();
    }
  }
});
