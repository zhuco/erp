import { describe, expect, it } from 'vitest';

import {
  canCreateWave,
  canHandoverOutbound,
  canRetryShippingPush,
  canReviewAndWeigh,
  canStartPicking,
  canSubmitForFulfillment
} from './guards';

describe('canSubmitForFulfillment', () => {
  it('AUTO_PASSED + NONE + reserve_precheck_passed=true 时允许提交发货', () => {
    const result = canSubmitForFulfillment({
      audit_status: 'AUTO_PASSED',
      exception_status: 'NONE',
      reserve_precheck_passed: true
    });

    expect(result.allowed).toBe(true);
    expect(result.reasons).toEqual([]);
  });

  it('失败分支返回中文阻断原因', () => {
    const result = canSubmitForFulfillment({
      audit_status: 'UNDER_REVIEW',
      exception_status: 'PROCESSING',
      reserve_precheck_passed: false
    });

    expect(result.allowed).toBe(false);
    expect(result.reasons.length).toBeGreaterThan(0);
    expect(result.reasons.every((reason) => /[\u4e00-\u9fa5]/.test(reason))).toBe(true);
  });
});

describe('canCreateWave', () => {
  it('非 RESERVED 时禁止建波次', () => {
    const result = canCreateWave({
      reserve_status: 'RELEASED'
    });

    expect(result.allowed).toBe(false);
    expect(result.reasons).toContain('仅已占库存订单允许建波次');
  });
});

describe('一期新增动作 guard（新状态语义）', () => {
  it('canStartPicking 在 RELEASED 时放行', () => {
    const result = canStartPicking({ wave_status: 'RELEASED' });
    expect(result.allowed).toBe(true);
    expect(result.reasons).toEqual([]);
  });

  it('canStartPicking 在非 RELEASED 时阻断并返回中文原因', () => {
    const result = canStartPicking({ wave_status: 'CREATED' });
    expect(result.allowed).toBe(false);
    expect(result.reasons).toContain('波次未释放，禁止开始拣货');
  });

  it('canReviewAndWeigh 在 PICKED 时放行', () => {
    const result = canReviewAndWeigh({ pick_status: 'PICKED' });
    expect(result.allowed).toBe(true);
    expect(result.reasons).toEqual([]);
  });

  it('canReviewAndWeigh 在非 PICKED 时阻断并返回中文原因', () => {
    const result = canReviewAndWeigh({ pick_status: 'PICKING' });
    expect(result.allowed).toBe(false);
    expect(result.reasons).toContain('拣货未完成，禁止复核称重');
  });

  it('canHandoverOutbound 在 PACKED 时放行', () => {
    const result = canHandoverOutbound({ pack_status: 'PACKED' });
    expect(result.allowed).toBe(true);
    expect(result.reasons).toEqual([]);
  });

  it('canHandoverOutbound 在非 PACKED 时阻断并返回中文原因', () => {
    const result = canHandoverOutbound({ pack_status: 'WEIGHED' });
    expect(result.allowed).toBe(false);
    expect(result.reasons).toContain('打包未完成，禁止出库交接');
  });

  it('canRetryShippingPush 在物流回传失败时放行', () => {
    const result = canRetryShippingPush({ fulfillment_status: 'SHIPPING_PUSH_FAILED' });
    expect(result.allowed).toBe(true);
    expect(result.reasons).toEqual([]);
  });

  it('canRetryShippingPush 在非失败状态时阻断并返回中文原因', () => {
    const result = canRetryShippingPush({ fulfillment_status: 'SHIPPED' });
    expect(result.allowed).toBe(false);
    expect(result.reasons).toContain('仅物流回传失败订单允许重试');
  });
});
