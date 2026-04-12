import { expect, test, loginAndSelectTenant } from './fixtures';
import { expectTableReady } from './utils/table';

test('marketing promotions supports create and update flow', async ({ page }) => {
  await page.goto('/marketing/promotions');
  await loginAndSelectTenant(page);

  await expect(page.getByRole('heading', { name: '促销活动' })).toBeVisible();
  await expectTableReady(page.locator('.ant-table').first());

  await page.getByRole('button', { name: '新建活动' }).click();
  const drawer = page.getByRole('dialog', { name: '新建促销活动' });
  await expect(drawer).toBeVisible();
  await drawer.getByLabel('活动名称').fill('E2E-促销活动');
  await drawer.getByLabel('活动类型').click();
  await page.getByRole('option', { name: '折扣' }).click();
  await drawer.getByLabel('范围JSON').fill('{"channel_ids":["CH-1"]}');
  await drawer.getByLabel('规则JSON').fill('{"discount":0.85}');
  await drawer.getByRole('button', { name: '保存' }).click();

  await expect(page.getByText('促销活动已创建')).toBeVisible();

  const row = page.locator('.ant-table-tbody tr.ant-table-row', { hasText: 'E2E-促销活动' }).first();
  await expect(row).toBeVisible();
  await row.getByRole('button', { name: '编辑' }).click();

  const editDrawer = page.getByRole('dialog', { name: '编辑促销活动' });
  await expect(editDrawer).toBeVisible();
  await editDrawer.getByLabel('活动名称').fill('E2E-促销活动-已更新');
  await editDrawer.getByRole('button', { name: '保存' }).click();

  await expect(page.getByText('促销活动已更新')).toBeVisible();
  await expect(
    page.locator('.ant-table-tbody tr.ant-table-row', { hasText: 'E2E-促销活动-已更新' }).first()
  ).toBeVisible();
});

test('marketing price list supports detail and item add flow', async ({ page }) => {
  await page.goto('/marketing/prices');
  await loginAndSelectTenant(page);

  await expect(page.getByRole('heading', { name: '渠道价表' })).toBeVisible();
  await expectTableReady(page.locator('.ant-table').first());

  await page.getByRole('button', { name: '新建价表' }).click();
  const drawer = page.getByRole('dialog', { name: '新建价表' });
  await expect(drawer).toBeVisible();
  await drawer.getByLabel('价表名称').fill('E2E-价表');
  await drawer.getByLabel('币种').fill('CNY');
  await drawer.getByRole('button', { name: '保存' }).click();

  await expect(page.getByText('渠道价表已创建')).toBeVisible();

  const row = page.locator('.ant-table-tbody tr.ant-table-row', { hasText: 'E2E-价表' }).first();
  await expect(row).toBeVisible();
  await row.getByRole('button', { name: '明细' }).click();

  const detailDrawer = page.getByRole('dialog', { name: '价表明细' });
  await expect(detailDrawer).toBeVisible();
  await detailDrawer.getByLabel('SKU ID').fill('SKU-E2E-001');
  await detailDrawer.getByLabel('基础价(分)').fill('3999');
  await detailDrawer.getByRole('button', { name: '保存' }).click();

  await expect(page.getByText('价表明细已保存')).toBeVisible();
  await expect(
    detailDrawer.locator('.ant-table-tbody tr.ant-table-row', { hasText: 'SKU-E2E-001' }).first()
  ).toBeVisible();
});

test('marketing gift policy supports create and remove flow', async ({ page }) => {
  await page.goto('/marketing/gifts');
  await loginAndSelectTenant(page);

  await expect(page.getByRole('heading', { name: '赠品策略' })).toBeVisible();
  await expectTableReady(page.locator('.ant-table').first());

  await page.getByRole('button', { name: '新建策略' }).click();
  const drawer = page.getByRole('dialog', { name: '新建赠品策略' });
  await expect(drawer).toBeVisible();
  await drawer.getByLabel('策略名称').fill('E2E-赠品策略');
  await drawer.getByLabel('范围JSON').fill('{"channels":["online"]}');
  await drawer.getByLabel('赠品池JSON').fill('[{"sku_id":"SKU-GIFT-E2E","qty":1}]');
  await drawer.getByRole('button', { name: '保存' }).click();

  await expect(page.getByText('赠品策略已创建')).toBeVisible();
  const row = page.locator('.ant-table-tbody tr.ant-table-row', { hasText: 'E2E-赠品策略' }).first();
  await expect(row).toBeVisible();
  await row.getByRole('button', { name: '删除' }).click();
  await page.locator('.ant-popconfirm-buttons').getByRole('button', { name: '删除' }).click();
  await expect(page.getByText('赠品策略已删除')).toBeVisible();
});

test('marketing coupon supports create update and remove flow', async ({ page }) => {
  await page.goto('/marketing/coupons');
  await loginAndSelectTenant(page);

  await expect(page.getByRole('heading', { name: '优惠券' })).toBeVisible();
  await expectTableReady(page.locator('.ant-table').first());

  await page.getByRole('button', { name: '新建优惠券' }).click();
  const drawer = page.getByRole('dialog', { name: '新建优惠券' });
  await expect(drawer).toBeVisible();
  await drawer.getByLabel('优惠券名称').fill('E2E-优惠券');
  await drawer.getByLabel('优惠类型').click();
  await page.getByRole('option', { name: '折扣' }).click();
  await drawer.getByLabel('优惠值').fill('8.5');
  await drawer.getByLabel('总使用次数').fill('100');
  await drawer.getByLabel('单人次数').fill('2');
  await drawer.getByLabel('规则JSON').fill('{"min_amount":100}');
  await drawer.getByRole('button', { name: '保存' }).click();

  await expect(page.getByText('优惠券已创建')).toBeVisible();
  const createdRow = page.locator('.ant-table-tbody tr.ant-table-row', { hasText: 'E2E-优惠券' }).first();
  await expect(createdRow).toBeVisible();
  await createdRow.getByRole('button', { name: '编辑' }).click();

  const editDrawer = page.getByRole('dialog', { name: '编辑优惠券' });
  await expect(editDrawer).toBeVisible();
  await editDrawer.getByLabel('优惠券名称').fill('E2E-优惠券-已更新');
  await editDrawer.getByRole('button', { name: '保存' }).click();

  await expect(page.getByText('优惠券已更新')).toBeVisible();
  const updatedRow = page
    .locator('.ant-table-tbody tr.ant-table-row', { hasText: 'E2E-优惠券-已更新' })
    .first();
  await expect(updatedRow).toBeVisible();
  await updatedRow.getByRole('button', { name: '删除' }).click();
  await page.locator('.ant-popconfirm-buttons').getByRole('button', { name: '删除' }).click();
  await expect(page.getByText('优惠券已删除')).toBeVisible();
});

test('marketing cost supports create update and remove flow', async ({ page }) => {
  await page.goto('/marketing/costs');
  await loginAndSelectTenant(page);

  await expect(page.getByRole('heading', { name: '促销费用' })).toBeVisible();
  await expectTableReady(page.locator('.ant-table').first());

  await page.getByRole('button', { name: '新建费用' }).click();
  const drawer = page.getByRole('dialog', { name: '新建促销费用' });
  await expect(drawer).toBeVisible();
  await drawer.getByLabel('费用名称').fill('E2E-促销费用');
  await drawer.getByLabel('金额(分)').fill('5200');
  await drawer.getByLabel('分摊规则JSON').fill('{"method":"by_order"}');
  await drawer.getByRole('button', { name: '保存' }).click();

  await expect(page.getByText('促销费用已创建')).toBeVisible();
  const createdRow = page
    .locator('.ant-table-tbody tr.ant-table-row', { hasText: 'E2E-促销费用' })
    .first();
  await expect(createdRow).toBeVisible();
  await createdRow.getByRole('button', { name: '编辑' }).click();

  const editDrawer = page.getByRole('dialog', { name: '编辑促销费用' });
  await expect(editDrawer).toBeVisible();
  await editDrawer.getByLabel('费用名称').fill('E2E-促销费用-已更新');
  await editDrawer.getByLabel('金额(分)').fill('6800');
  await editDrawer.getByRole('button', { name: '保存' }).click();

  await expect(page.getByText('促销费用已更新')).toBeVisible();
  const updatedRow = page
    .locator('.ant-table-tbody tr.ant-table-row', { hasText: 'E2E-促销费用-已更新' })
    .first();
  await expect(updatedRow).toBeVisible();
  await updatedRow.getByRole('button', { name: '删除' }).click();
  await page.locator('.ant-popconfirm-buttons').getByRole('button', { name: '删除' }).click();
  await expect(page.getByText('促销费用已删除')).toBeVisible();
});

test('marketing postmortem supports create update and remove flow', async ({ page }) => {
  await page.goto('/marketing/postmortems');
  await loginAndSelectTenant(page);

  await expect(page.getByRole('heading', { name: '活动复盘' })).toBeVisible();
  await expectTableReady(page.locator('.ant-table').first());

  await page.getByRole('button', { name: '新建复盘' }).click();
  const drawer = page.getByRole('dialog', { name: '新建活动复盘' });
  await expect(drawer).toBeVisible();
  await drawer.getByLabel('复盘名称').fill('E2E-活动复盘');
  await drawer.getByLabel('摘要').fill('E2E 复盘摘要');
  await drawer.getByLabel('指标JSON').fill('{"roi":1.23}');
  await drawer.getByRole('button', { name: '保存' }).click();

  await expect(page.getByText('活动复盘已创建')).toBeVisible();
  const createdRow = page
    .locator('.ant-table-tbody tr.ant-table-row', { hasText: 'E2E-活动复盘' })
    .first();
  await expect(createdRow).toBeVisible();
  await createdRow.getByRole('button', { name: '编辑' }).click();

  const editDrawer = page.getByRole('dialog', { name: '编辑活动复盘' });
  await expect(editDrawer).toBeVisible();
  await editDrawer.getByLabel('复盘名称').fill('E2E-活动复盘-已更新');
  await editDrawer.getByRole('button', { name: '保存' }).click();

  await expect(page.getByText('活动复盘已更新')).toBeVisible();
  const updatedRow = page
    .locator('.ant-table-tbody tr.ant-table-row', { hasText: 'E2E-活动复盘-已更新' })
    .first();
  await expect(updatedRow).toBeVisible();
  await updatedRow.getByRole('button', { name: '删除' }).click();
  await page.locator('.ant-popconfirm-buttons').getByRole('button', { name: '删除' }).click();
  await expect(page.getByText('活动复盘已删除')).toBeVisible();
});
