import type { Locator, Page } from '@playwright/test';
import { expect, test, loginAndSelectTenant } from './fixtures';

type SpaceSelectionCase = {
  key: string;
  path: string;
  gridTestId: string;
  cellColId: string;
  beforeScreenshot: string;
  afterScreenshot: string;
  readyPageTestId?: string;
  beforeGridReady?: (page: Page) => Promise<void>;
};

const artifactDir = '../../output/playwright/space-selection';

const getCenterRows = (grid: Locator) => grid.locator('.ag-center-cols-container .ag-row');

const getRowId = async (row: Locator) => {
  const rowId = await row.getAttribute('row-id');
  expect(rowId).toBeTruthy();
  return rowId as string;
};

const activateTagTab = async (page: Page, label: string) => {
  const header = page.locator('.tag-directory-header');
  await expect(header).toBeVisible();

  const segmentedItem = header.locator('.ant-segmented-item').filter({ hasText: label }).first();
  if ((await segmentedItem.count()) > 0) {
    await expect(segmentedItem).toBeVisible();
    await segmentedItem.click();
    return;
  }

  const fallbackTrigger = header.getByText(label, { exact: true }).first();
  await expect(fallbackTrigger).toBeVisible();
  await fallbackTrigger.click();
};

const spaceSelectionCases: SpaceSelectionCase[] = [
  {
    key: 'order-review',
    path: '/orders/review',
    gridTestId: 'order-review-data-grid',
    cellColId: 'id',
    beforeScreenshot: 'order-review-before.png',
    afterScreenshot: 'order-review-after.png',
  },
  {
    key: 'warehouse-directory',
    path: '/warehouses',
    gridTestId: 'warehouse-directory-grid',
    cellColId: 'name',
    beforeScreenshot: 'warehouse-directory-before.png',
    afterScreenshot: 'warehouse-directory-after.png',
    readyPageTestId: 'warehouse-directory-workbench-page',
  },
  {
    key: 'channel-directory',
    path: '/channels',
    gridTestId: 'channel-directory-grid',
    cellColId: 'name',
    beforeScreenshot: 'channel-directory-before.png',
    afterScreenshot: 'channel-directory-after.png',
    readyPageTestId: 'channel-directory-workbench-page',
  },
  {
    key: 'supplier-directory',
    path: '/suppliers',
    gridTestId: 'suppliers-directory-grid',
    cellColId: 'name',
    beforeScreenshot: 'suppliers-directory-before.png',
    afterScreenshot: 'suppliers-directory-after.png',
    readyPageTestId: 'suppliers-workbench-page',
  },
  {
    key: 'logistics-fee-directory',
    path: '/logistics-fees',
    gridTestId: 'logistics-fee-directory-grid',
    cellColId: 'rule_name',
    beforeScreenshot: 'logistics-fee-directory-before.png',
    afterScreenshot: 'logistics-fee-directory-after.png',
    readyPageTestId: 'logistics-fee-directory-workbench-page',
  },
  {
    key: 'couriers-directory',
    path: '/couriers',
    gridTestId: 'couriers-directory-grid',
    cellColId: 'name',
    beforeScreenshot: 'couriers-directory-before.png',
    afterScreenshot: 'couriers-directory-after.png',
    readyPageTestId: 'couriers-workbench-page',
  },
  {
    key: 'customer-categories',
    path: '/customers/categories',
    gridTestId: 'customer-categories-grid',
    cellColId: 'name',
    beforeScreenshot: 'customer-categories-before.png',
    afterScreenshot: 'customer-categories-after.png',
    readyPageTestId: 'customer-categories-workbench-page',
  },
  {
    key: 'customer-tags',
    path: '/customers/tags',
    gridTestId: 'customer-tags-grid',
    cellColId: 'name',
    beforeScreenshot: 'customer-tags-before.png',
    afterScreenshot: 'customer-tags-after.png',
    readyPageTestId: 'customer-tags-workbench-page',
  },
  {
    key: 'member-levels',
    path: '/customers/member-levels',
    gridTestId: 'member-levels-grid',
    cellColId: 'name',
    beforeScreenshot: 'member-levels-before.png',
    afterScreenshot: 'member-levels-after.png',
    readyPageTestId: 'member-levels-workbench-page',
  },
  {
    key: 'tag-directory-orders',
    path: '/tags',
    gridTestId: 'tag-directory-orders-grid',
    cellColId: 'name',
    beforeScreenshot: 'tag-directory-orders-before.png',
    afterScreenshot: 'tag-directory-orders-after.png',
    readyPageTestId: 'tag-directory-workbench-page',
    beforeGridReady: async (page) => {
      await activateTagTab(page, '订单记号');
    },
  },
  {
    key: 'tag-directory-skus',
    path: '/tags',
    gridTestId: 'tag-directory-skus-grid',
    cellColId: 'name',
    beforeScreenshot: 'tag-directory-skus-before.png',
    afterScreenshot: 'tag-directory-skus-after.png',
    readyPageTestId: 'tag-directory-workbench-page',
    beforeGridReady: async (page) => {
      await activateTagTab(page, 'SKU 记号');
    },
  },
  {
    key: 'tag-directory-builtin',
    path: '/tags',
    gridTestId: 'tag-directory-builtin-grid',
    cellColId: 'name',
    beforeScreenshot: 'tag-directory-builtin-before.png',
    afterScreenshot: 'tag-directory-builtin-after.png',
    readyPageTestId: 'tag-directory-workbench-page',
    beforeGridReady: async (page) => {
      await activateTagTab(page, '内置记号');
    },
  },
  {
    key: 'containers-directory',
    path: '/wms/containers',
    gridTestId: 'containers-directory-grid',
    cellColId: 'code',
    beforeScreenshot: 'containers-directory-before.png',
    afterScreenshot: 'containers-directory-after.png',
    readyPageTestId: 'containers-workbench-page',
  },
  {
    key: 'sender-addresses-directory',
    path: '/wms/sender-addresses',
    gridTestId: 'sender-addresses-directory-grid',
    cellColId: 'name',
    beforeScreenshot: 'sender-addresses-directory-before.png',
    afterScreenshot: 'sender-addresses-directory-after.png',
    readyPageTestId: 'sender-addresses-workbench-page',
  },
  {
    key: 'warehouse-areas-directory',
    path: '/wms/warehouse-areas',
    gridTestId: 'warehouse-areas-directory-grid',
    cellColId: 'name',
    beforeScreenshot: 'warehouse-areas-directory-before.png',
    afterScreenshot: 'warehouse-areas-directory-after.png',
    readyPageTestId: 'warehouse-areas-workbench-page',
  },
  {
    key: 'warehouse-locations-directory',
    path: '/wms/warehouse-locations',
    gridTestId: 'warehouse-locations-directory-grid',
    cellColId: 'name',
    beforeScreenshot: 'warehouse-locations-directory-before.png',
    afterScreenshot: 'warehouse-locations-directory-after.png',
    readyPageTestId: 'warehouse-locations-workbench-page',
  },
];

const runSpaceSelectionCase = async ({
  page,
  scenario,
}: {
  page: Page;
  scenario: SpaceSelectionCase;
}) => {
  await page.goto(scenario.path);
  await loginAndSelectTenant(page);

  if (scenario.readyPageTestId) {
    await expect(page.getByTestId(scenario.readyPageTestId)).toBeVisible();
  }

  await scenario.beforeGridReady?.(page);

  const grid = page.getByTestId(scenario.gridTestId);
  await expect(grid).toBeVisible();

  const rows = getCenterRows(grid);
  await expect.poll(async () => rows.count(), { timeout: 20_000 }).toBeGreaterThanOrEqual(2);

  const secondRow = rows.nth(1);
  const secondRowId = await getRowId(secondRow);
  const secondRowById = grid.locator(`.ag-center-cols-container .ag-row[row-id="${secondRowId}"]`);
  const targetCell = secondRowById.locator(`.ag-cell[col-id="${scenario.cellColId}"]`).first();

  await expect(targetCell).toBeVisible();
  await grid.screenshot({ path: `${artifactDir}/${scenario.beforeScreenshot}` });

  await targetCell.click();
  await targetCell.press('Space');

  await expect(secondRowById).toHaveClass(/ag-row-selected/);
  await grid.screenshot({ path: `${artifactDir}/${scenario.afterScreenshot}` });
};

for (const scenario of spaceSelectionCases) {
  test(`${scenario.key} supports single click then Space selection`, async ({ page }) => {
    await runSpaceSelectionCase({ page, scenario });
  });
}
