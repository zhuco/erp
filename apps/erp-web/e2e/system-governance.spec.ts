import { expect, test, loginAndSelectTenant } from './fixtures';

test('system governance workflow metadata can be edited and observed across system pages', async ({
  page
}) => {
  await page.goto('/system/workflows');
  await loginAndSelectTenant(page);

  const governanceTable = page.getByTestId('system-workflows-governance-table');
  await expect(governanceTable).toBeVisible();
  await expect(governanceTable).toContainText('CAMUNDA8');

  await page.getByRole('button', { name: '新增流程' }).click();
  await page.getByLabel('流程名称').fill('系统治理审批流程');
  await page.getByLabel('流程键（BPMN）').fill('system_governance_approval');
  await page.getByLabel('规则键（DMN）').fill('system_governance_route');
  await page.getByLabel('最近部署结果').fill('SUCCESS');
  const workflowModal = page.locator('.ant-modal-wrap:visible').last();
  await expect(workflowModal).toBeVisible();
  await workflowModal.locator('.ant-btn-primary').click();

  await expect(governanceTable).toContainText('system_governance_approval');

  await page.goto('/system/custom-fields');
  await expect(page.getByTestId('system-custom-fields-governance-summary')).toBeVisible();

  await page.goto('/system/integrations');
  await expect(page.getByTestId('system-integrations-governance-summary')).toBeVisible();

  await page.goto('/system/audit-reports');
  await expect(page.getByTestId('system-audit-reports-governance-summary')).toBeVisible();
});
