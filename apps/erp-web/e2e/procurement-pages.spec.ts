import { expect, test, loginAndSelectTenant } from './fixtures';
import { expectTableReady } from './utils/table';

const openPage = async (
  page: Parameters<typeof loginAndSelectTenant>[0],
  path: string
) => {
  await page.goto(path);
  await loginAndSelectTenant(page);
};

test('purchase orders list renders', async ({ page }) => {
  await openPage(page, '/procurement/orders');

  await expect(page.getByRole('heading', { name: /Purchase Orders/ })).toBeVisible();
  await expect(page.getByRole('button', { name: /New Purchase Order/ })).toBeVisible();
  await expectTableReady(page.locator('.ant-table').first());
});

test('purchase receipts list renders', async ({ page }) => {
  await openPage(page, '/procurement/receipts');

  await expect(page.getByRole('heading', { name: /Purchase Receipts/ })).toBeVisible();
  await expectTableReady(page.locator('.ant-table').first());
});

test('purchase returns list renders', async ({ page }) => {
  await openPage(page, '/procurement/returns');

  await expect(page.getByRole('heading', { name: /Purchase Returns/ })).toBeVisible();
  await expectTableReady(page.locator('.ant-table').first());
});

test('purchase suggestions list renders', async ({ page }) => {
  await openPage(page, '/procurement/suggestions');

  await expect(page.getByRole('heading', { name: /Purchase Suggestions/ })).toBeVisible();
  await expect(page.getByPlaceholder(/Enter SKU ID/)).toBeVisible();
  await expectTableReady(page.locator('.ant-table').first());
});

test('procurement requests list renders', async ({ page }) => {
  await openPage(page, '/procurement/requests');

  await expect(page.getByRole('heading', { name: /采购申请/ })).toBeVisible();
  await expect(page.getByRole('button', { name: /新建申请/ })).toBeVisible();
  await expectTableReady(page.locator('.ant-table').first());
});

test('procurement plans list renders', async ({ page }) => {
  await openPage(page, '/procurement/plans');

  await expect(page.getByRole('heading', { name: /采购计划/ })).toBeVisible();
  await expect(page.getByRole('button', { name: /新建计划/ })).toBeVisible();
  await expectTableReady(page.locator('.ant-table').first());
});

test('procurement sourcing list renders', async ({ page }) => {
  await openPage(page, '/procurement/sourcing');

  await expect(page.getByRole('heading', { name: /寻源与选品/ })).toBeVisible();
  await expect(page.getByRole('button', { name: /新建需求/ })).toBeVisible();
  await expectTableReady(page.locator('.ant-table').first());
});

test('procurement inspections list renders', async ({ page }) => {
  await openPage(page, '/procurement/inspections');

  await expect(page.getByRole('heading', { name: /质检单/ })).toBeVisible();
  await expect(page.getByRole('button', { name: /新建质检单/ })).toBeVisible();
  await expectTableReady(page.locator('.ant-table').first());
});

test('procurement reconciliations list renders', async ({ page }) => {
  await openPage(page, '/procurement/reconciliations');

  await expect(page.getByRole('heading', { name: /供应商对账/ })).toBeVisible();
  await expect(page.getByRole('button', { name: /新建对账/ })).toBeVisible();
  await expectTableReady(page.locator('.ant-table').first());
});

test('supplier performance list renders', async ({ page }) => {
  await openPage(page, '/procurement/performance');

  await expect(page.getByRole('heading', { name: /供应商绩效/ })).toBeVisible();
  await expect(page.getByRole('button', { name: /新建绩效/ })).toBeVisible();
  await expectTableReady(page.locator('.ant-table').first());
});

test('planning purchase plans list renders', async ({ page }) => {
  await openPage(page, '/planning/purchase-plans');

  await expect(page.getByRole('heading', { name: /采购补货计划/ })).toBeVisible();
  await expect(page.getByRole('button', { name: /生成采购计划/ })).toBeVisible();
  await expectTableReady(page.locator('.ant-table').first());
});
