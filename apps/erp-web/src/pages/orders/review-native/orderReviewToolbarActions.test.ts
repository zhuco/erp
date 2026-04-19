import { describe, expect, it, vi } from 'vitest';

import {
  runMergeOrdersAction,
  runReviewOrdersAction,
  runSplitOrderAction,
  type OrderReviewToolbarActionContext,
  type OrderReviewToolbarActionDeps,
} from './orderReviewToolbarActions';

const createDeps = (): OrderReviewToolbarActionDeps => ({
  reviewOrder: vi.fn().mockResolvedValue({}),
  mergeOrders: vi.fn().mockResolvedValue({}),
  splitOrder: vi.fn().mockResolvedValue({}),
  notifySuccess: vi.fn(),
  notifyWarning: vi.fn(),
  notifyError: vi.fn(),
});

const createContext = (
  overrides: Partial<OrderReviewToolbarActionContext> = {},
): OrderReviewToolbarActionContext => ({
  tenantId: 'tenant-1',
  actionOrderIds: ['O-1', 'O-2'],
  canWriteOrders: true,
  ...overrides,
});

describe('orderReviewToolbarActions', () => {
  it('runs review action for all selected orders and refreshes', async () => {
    const deps = createDeps();
    const refresh = vi.fn().mockResolvedValue(undefined);

    const result = await runReviewOrdersAction(createContext(), deps, refresh);

    expect(result).toBe(true);
    expect(deps.reviewOrder).toHaveBeenCalledTimes(2);
    expect(deps.reviewOrder).toHaveBeenNthCalledWith(1, 'tenant-1', 'O-1', {
      action: 'PASS',
      reason: '订单审核页面审核通过',
    });
    expect(deps.reviewOrder).toHaveBeenNthCalledWith(2, 'tenant-1', 'O-2', {
      action: 'PASS',
      reason: '订单审核页面审核通过',
    });
    expect(deps.notifySuccess).toHaveBeenCalledWith('审核完成，共 2 单');
    expect(refresh).toHaveBeenCalledTimes(1);
  });

  it('deduplicates repeated order ids before running review action', async () => {
    const deps = createDeps();
    const refresh = vi.fn().mockResolvedValue(undefined);

    const result = await runReviewOrdersAction(
      createContext({ actionOrderIds: ['O-1', ' O-1 ', 'O-2', 'O-2'] }),
      deps,
      refresh,
    );

    expect(result).toBe(true);
    expect(deps.reviewOrder).toHaveBeenCalledTimes(2);
    expect(deps.reviewOrder).toHaveBeenNthCalledWith(1, 'tenant-1', 'O-1', {
      action: 'PASS',
      reason: '订单审核页面审核通过',
    });
    expect(deps.reviewOrder).toHaveBeenNthCalledWith(2, 'tenant-1', 'O-2', {
      action: 'PASS',
      reason: '订单审核页面审核通过',
    });
    expect(deps.notifySuccess).toHaveBeenCalledWith('审核完成，共 2 单');
    expect(refresh).toHaveBeenCalledTimes(1);
  });

  it('blocks review action when no order is selected', async () => {
    const deps = createDeps();

    const result = await runReviewOrdersAction(
      createContext({ actionOrderIds: [] }),
      deps,
      vi.fn(),
    );

    expect(result).toBe(false);
    expect(deps.reviewOrder).not.toHaveBeenCalled();
    expect(deps.notifyWarning).toHaveBeenCalledWith('请先选中或勾选要审核的订单');
  });

  it('blocks review action when tenantId is missing', async () => {
    const deps = createDeps();

    const result = await runReviewOrdersAction(createContext({ tenantId: '' }), deps, vi.fn());

    expect(result).toBe(false);
    expect(deps.reviewOrder).not.toHaveBeenCalled();
    expect(deps.notifyWarning).toHaveBeenCalledWith('缺少租户上下文，请刷新页面后重试');
  });

  it('blocks review action when tenantId contains only whitespace', async () => {
    const deps = createDeps();

    const result = await runReviewOrdersAction(createContext({ tenantId: '   ' }), deps, vi.fn());

    expect(result).toBe(false);
    expect(deps.reviewOrder).not.toHaveBeenCalled();
    expect(deps.notifyWarning).toHaveBeenCalledWith('缺少租户上下文，请刷新页面后重试');
  });

  it('blocks review action when no write permission', async () => {
    const deps = createDeps();

    const result = await runReviewOrdersAction(
      createContext({ canWriteOrders: false }),
      deps,
      vi.fn(),
    );

    expect(result).toBe(false);
    expect(deps.reviewOrder).not.toHaveBeenCalled();
    expect(deps.notifyWarning).toHaveBeenCalledWith('暂无订单修改权限');
  });

  it('returns false and notifies error when review action fails', async () => {
    const deps = createDeps();
    const refresh = vi.fn().mockResolvedValue(undefined);
    vi.mocked(deps.reviewOrder).mockRejectedValue(new Error('review failed'));

    const result = await runReviewOrdersAction(createContext(), deps, refresh);

    expect(result).toBe(false);
    expect(deps.notifyError).toHaveBeenCalledWith('review failed');
    expect(refresh).not.toHaveBeenCalled();
  });

  it('blocks review action when selected ids are blank strings', async () => {
    const deps = createDeps();

    const result = await runReviewOrdersAction(
      createContext({ actionOrderIds: ['', '   '] }),
      deps,
      vi.fn(),
    );

    expect(result).toBe(false);
    expect(deps.reviewOrder).not.toHaveBeenCalled();
    expect(deps.notifyWarning).toHaveBeenCalledWith('请先选中或勾选要审核的订单');
  });

  it('uses fallback error message when review action throws non-Error', async () => {
    const deps = createDeps();
    vi.mocked(deps.reviewOrder).mockRejectedValue('unknown');

    const result = await runReviewOrdersAction(createContext(), deps, vi.fn());

    expect(result).toBe(false);
    expect(deps.notifyError).toHaveBeenCalledWith('审核失败');
  });

  it('blocks merge action when selected order count is less than 2', async () => {
    const deps = createDeps();

    const result = await runMergeOrdersAction(
      createContext({ actionOrderIds: ['O-1'] }),
      deps,
      vi.fn(),
    );

    expect(result).toBe(false);
    expect(deps.mergeOrders).not.toHaveBeenCalled();
    expect(deps.notifyWarning).toHaveBeenCalledWith('合单至少需要选择 2 单');
  });

  it('blocks merge action when tenantId is missing', async () => {
    const deps = createDeps();

    const result = await runMergeOrdersAction(createContext({ tenantId: '' }), deps, vi.fn());

    expect(result).toBe(false);
    expect(deps.mergeOrders).not.toHaveBeenCalled();
    expect(deps.notifyWarning).toHaveBeenCalledWith('缺少租户上下文，请刷新页面后重试');
  });

  it('blocks merge action when tenantId contains only whitespace', async () => {
    const deps = createDeps();

    const result = await runMergeOrdersAction(createContext({ tenantId: '  ' }), deps, vi.fn());

    expect(result).toBe(false);
    expect(deps.mergeOrders).not.toHaveBeenCalled();
    expect(deps.notifyWarning).toHaveBeenCalledWith('缺少租户上下文，请刷新页面后重试');
  });

  it('blocks merge action when no write permission', async () => {
    const deps = createDeps();

    const result = await runMergeOrdersAction(
      createContext({ canWriteOrders: false }),
      deps,
      vi.fn(),
    );

    expect(result).toBe(false);
    expect(deps.mergeOrders).not.toHaveBeenCalled();
    expect(deps.notifyWarning).toHaveBeenCalledWith('暂无订单修改权限');
  });

  it('returns false and notifies error when merge action fails', async () => {
    const deps = createDeps();
    const refresh = vi.fn().mockResolvedValue(undefined);
    vi.mocked(deps.mergeOrders).mockRejectedValue(new Error('merge failed'));

    const result = await runMergeOrdersAction(createContext(), deps, refresh);

    expect(result).toBe(false);
    expect(deps.notifyError).toHaveBeenCalledWith('merge failed');
    expect(refresh).not.toHaveBeenCalled();
  });

  it('uses fallback error message when merge action throws non-Error', async () => {
    const deps = createDeps();
    vi.mocked(deps.mergeOrders).mockRejectedValue({ message: 'not-error-instance' });

    const result = await runMergeOrdersAction(createContext(), deps, vi.fn());

    expect(result).toBe(false);
    expect(deps.notifyError).toHaveBeenCalledWith('合单失败');
  });

  it('treats blank selected ids as insufficient for merge action', async () => {
    const deps = createDeps();

    const result = await runMergeOrdersAction(
      createContext({ actionOrderIds: [' ', ''] }),
      deps,
      vi.fn(),
    );

    expect(result).toBe(false);
    expect(deps.mergeOrders).not.toHaveBeenCalled();
    expect(deps.notifyWarning).toHaveBeenCalledWith('合单至少需要选择 2 单');
  });

  it('runs split action for first selected order', async () => {
    const deps = createDeps();
    const refresh = vi.fn().mockResolvedValue(undefined);

    const result = await runSplitOrderAction(createContext({ splitCount: 3 }), deps, refresh);

    expect(result).toBe(true);
    expect(deps.splitOrder).toHaveBeenCalledWith('tenant-1', 'O-1', {
      split_count: 3,
    });
    expect(deps.notifySuccess).toHaveBeenCalledWith('拆单完成：O-1');
    expect(refresh).toHaveBeenCalledTimes(1);
  });

  it('blocks split action when no order is selected', async () => {
    const deps = createDeps();

    const result = await runSplitOrderAction(
      createContext({ actionOrderIds: [] }),
      deps,
      vi.fn(),
    );

    expect(result).toBe(false);
    expect(deps.splitOrder).not.toHaveBeenCalled();
    expect(deps.notifyWarning).toHaveBeenCalledWith('请先选择要拆分的订单');
  });

  it('blocks split action when tenantId is missing', async () => {
    const deps = createDeps();

    const result = await runSplitOrderAction(createContext({ tenantId: '' }), deps, vi.fn());

    expect(result).toBe(false);
    expect(deps.splitOrder).not.toHaveBeenCalled();
    expect(deps.notifyWarning).toHaveBeenCalledWith('缺少租户上下文，请刷新页面后重试');
  });

  it('blocks split action when tenantId contains only whitespace', async () => {
    const deps = createDeps();

    const result = await runSplitOrderAction(createContext({ tenantId: ' ' }), deps, vi.fn());

    expect(result).toBe(false);
    expect(deps.splitOrder).not.toHaveBeenCalled();
    expect(deps.notifyWarning).toHaveBeenCalledWith('缺少租户上下文，请刷新页面后重试');
  });

  it('blocks split action when no write permission', async () => {
    const deps = createDeps();

    const result = await runSplitOrderAction(
      createContext({ canWriteOrders: false }),
      deps,
      vi.fn(),
    );

    expect(result).toBe(false);
    expect(deps.splitOrder).not.toHaveBeenCalled();
    expect(deps.notifyWarning).toHaveBeenCalledWith('暂无订单修改权限');
  });

  it.each([
    { input: undefined, expected: 2 },
    { input: Number.NaN, expected: 2 },
    { input: 1, expected: 2 },
    { input: 2.8, expected: 2 },
    { input: -5, expected: 2 },
    { input: 3.9, expected: 3 },
    { input: 6, expected: 6 },
  ])('normalizes split_count from $input to $expected', async ({ input, expected }) => {
    const deps = createDeps();

    const result = await runSplitOrderAction(createContext({ splitCount: input }), deps, vi.fn());

    expect(result).toBe(true);
    expect(deps.splitOrder).toHaveBeenCalledWith('tenant-1', 'O-1', {
      split_count: expected,
    });
  });

  it('returns false and notifies error when split action fails', async () => {
    const deps = createDeps();
    const refresh = vi.fn().mockResolvedValue(undefined);
    vi.mocked(deps.splitOrder).mockRejectedValue(new Error('split failed'));

    const result = await runSplitOrderAction(createContext(), deps, refresh);

    expect(result).toBe(false);
    expect(deps.notifyError).toHaveBeenCalledWith('split failed');
    expect(refresh).not.toHaveBeenCalled();
  });

  it('uses fallback error message when split action throws non-Error', async () => {
    const deps = createDeps();
    vi.mocked(deps.splitOrder).mockRejectedValue(123);

    const result = await runSplitOrderAction(createContext(), deps, vi.fn());

    expect(result).toBe(false);
    expect(deps.notifyError).toHaveBeenCalledWith('拆单失败');
  });

  it('blocks split action when selected ids are blank strings', async () => {
    const deps = createDeps();

    const result = await runSplitOrderAction(
      createContext({ actionOrderIds: ['', '   '] }),
      deps,
      vi.fn(),
    );

    expect(result).toBe(false);
    expect(deps.splitOrder).not.toHaveBeenCalled();
    expect(deps.notifyWarning).toHaveBeenCalledWith('请先选择要拆分的订单');
  });
});
