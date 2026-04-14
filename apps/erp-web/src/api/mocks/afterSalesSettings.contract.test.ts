import { afterEach, describe, expect, it, vi } from 'vitest';

const loadAfterSalesSettingsMocks = async () => {
  vi.resetModules();
  return import('./afterSalesSettings');
};

afterEach(() => {
  vi.resetModules();
});

describe('售后设置 mock 回退对象', () => {
  it('更新不存在的售后原因时返回完整对象', async () => {
    const { updateMockAfterSalesReason } = await loadAfterSalesSettingsMocks();
    const reason = updateMockAfterSalesReason('REASON-AUTO', {
      label: '自动生成原因',
    });

    expect(reason).toMatchObject({
      reason_code: 'REASON-AUTO',
      label: '自动生成原因',
      category: null,
      status: 'ACTIVE',
    });
    expect(reason.sort_order).toBeGreaterThan(0);
    expect(reason.created_at).toBeTruthy();
    expect(reason.updated_at).toBeTruthy();
  });

  it('删除不存在的售后原因时返回禁用回退对象', async () => {
    const { deleteMockAfterSalesReason } = await loadAfterSalesSettingsMocks();
    const reason = deleteMockAfterSalesReason('REASON-MISSING');

    expect(reason).toMatchObject({
      reason_code: 'REASON-MISSING',
      label: 'REASON-MISSING',
      category: null,
      status: 'DISABLED',
      sort_order: 0,
    });
    expect(reason.created_at).toBeTruthy();
    expect(reason.updated_at).toBeTruthy();
  });

  it('更新不存在的质检规则时返回完整对象', async () => {
    const { updateMockAfterSalesQualityRule } = await loadAfterSalesSettingsMocks();
    const rule = updateMockAfterSalesQualityRule('QC-AUTO', {
      after_sales_type: 'RETURN_REFUND',
      result_code: 'RESALE',
    });

    expect(rule).toMatchObject({
      rule_id: 'QC-AUTO',
      after_sales_type: 'RETURN_REFUND',
      result_code: 'RESALE',
      status: 'ACTIVE',
    });
    expect(rule.sort_order).toBeGreaterThan(0);
    expect(rule.created_at).toBeTruthy();
    expect(rule.updated_at).toBeTruthy();
  });

  it('删除不存在的质检规则时返回禁用回退对象', async () => {
    const { deleteMockAfterSalesQualityRule } = await loadAfterSalesSettingsMocks();
    const rule = deleteMockAfterSalesQualityRule('QC-MISSING');

    expect(rule).toMatchObject({
      rule_id: 'QC-MISSING',
      after_sales_type: 'RETURN_REFUND',
      result_code: 'UNKNOWN',
      status: 'DISABLED',
      sort_order: 0,
    });
    expect(rule.created_at).toBeTruthy();
    expect(rule.updated_at).toBeTruthy();
  });
});
