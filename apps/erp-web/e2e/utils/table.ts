import { expect, type Locator } from '@playwright/test';

export const expectTableReady = async (table: Locator): Promise<void> => {
  await expect(table).toBeVisible();
  const rows = table.locator('.ant-table-tbody tr.ant-table-row');
  if ((await rows.count()) > 0) {
    await expect(rows.first()).toBeVisible();
    return;
  }
  await expect(table.locator('.ant-empty')).toBeVisible();
};
