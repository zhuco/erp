import type {
  OrderMergeRequest,
  OrderReviewRequest,
  OrderSplitRequest,
} from '@/api/orders';

export type OrderReviewToolbarActionContext = {
  tenantId?: string;
  actionOrderIds: string[];
  canWriteOrders: boolean;
  splitCount?: number;
};

export type OrderReviewToolbarActionDeps = {
  reviewOrder: (tenantId: string, orderId: string, payload: OrderReviewRequest) => Promise<unknown>;
  mergeOrders: (tenantId: string, payload: OrderMergeRequest) => Promise<unknown>;
  splitOrder: (tenantId: string, orderId: string, payload: OrderSplitRequest) => Promise<unknown>;
  notifySuccess: (message: string) => void;
  notifyWarning: (message: string) => void;
  notifyError: (message: string) => void;
};

const MIN_SPLIT_COUNT = 2;

const normalizeTenantId = (tenantId?: string): string | undefined => {
  const normalized = tenantId?.trim();
  if (!normalized) {
    return undefined;
  }
  return normalized;
};

const normalizeActionOrderIds = (orderIds: string[]): string[] =>
  Array.from(
    new Set(
      orderIds
        .map((orderId) => orderId.trim())
        .filter((orderId) => orderId.length > 0),
    ),
  );

const notifyActionError = (
  error: unknown,
  fallbackMessage: string,
  notifyError: (message: string) => void,
): void => {
  notifyError(error instanceof Error ? error.message : fallbackMessage);
};

const normalizeSplitCount = (rawCount: number | undefined): number => {
  if (rawCount === undefined) {
    return MIN_SPLIT_COUNT;
  }
  const parsed = Number(rawCount);
  if (!Number.isFinite(parsed)) {
    return MIN_SPLIT_COUNT;
  }
  if (parsed < MIN_SPLIT_COUNT) {
    return MIN_SPLIT_COUNT;
  }
  return Math.floor(parsed);
};

export const resolveToolbarActionOrderIds = (
  selectedOrderIds: string[],
  activeVisibleOrderId: string,
): string[] => {
  const normalizedSelectedOrderIds = normalizeActionOrderIds(selectedOrderIds);
  if (normalizedSelectedOrderIds.length > 0) {
    return normalizedSelectedOrderIds;
  }

  const normalizedActiveVisibleOrderId = activeVisibleOrderId.trim();
  if (normalizedActiveVisibleOrderId) {
    return [normalizedActiveVisibleOrderId];
  }

  return [];
};

export const runReviewOrdersAction = async (
  context: OrderReviewToolbarActionContext,
  deps: OrderReviewToolbarActionDeps,
  onRefresh: () => Promise<void>,
): Promise<boolean> => {
  const actionOrderIds = normalizeActionOrderIds(context.actionOrderIds);
  if (!actionOrderIds.length) {
    deps.notifyWarning('请先选中或勾选要审核的订单');
    return false;
  }

  const tenantId = normalizeTenantId(context.tenantId);
  if (!tenantId) {
    deps.notifyWarning('缺少租户上下文，请刷新页面后重试');
    return false;
  }

  if (!context.canWriteOrders) {
    deps.notifyWarning('暂无订单修改权限');
    return false;
  }

  try {
    await Promise.all(
      actionOrderIds.map((orderId) =>
        deps.reviewOrder(tenantId, orderId, {
          action: 'PASS',
          reason: '订单审核页面审核通过',
        }),
      ),
    );
    deps.notifySuccess(`审核完成，共 ${actionOrderIds.length} 单`);
    await onRefresh();
    return true;
  } catch (error) {
    notifyActionError(error, '审核失败', deps.notifyError);
    return false;
  }
};

export const runMergeOrdersAction = async (
  context: OrderReviewToolbarActionContext,
  deps: OrderReviewToolbarActionDeps,
  onRefresh: () => Promise<void>,
): Promise<boolean> => {
  const actionOrderIds = normalizeActionOrderIds(context.actionOrderIds);
  if (actionOrderIds.length < 2) {
    deps.notifyWarning('合单至少需要选择 2 单');
    return false;
  }

  const tenantId = normalizeTenantId(context.tenantId);
  if (!tenantId) {
    deps.notifyWarning('缺少租户上下文，请刷新页面后重试');
    return false;
  }

  if (!context.canWriteOrders) {
    deps.notifyWarning('暂无订单修改权限');
    return false;
  }

  try {
    await deps.mergeOrders(tenantId, {
      order_ids: actionOrderIds,
    });
    deps.notifySuccess(`合单完成，共处理 ${actionOrderIds.length} 单`);
    await onRefresh();
    return true;
  } catch (error) {
    notifyActionError(error, '合单失败', deps.notifyError);
    return false;
  }
};

export const runSplitOrderAction = async (
  context: OrderReviewToolbarActionContext,
  deps: OrderReviewToolbarActionDeps,
  onRefresh: () => Promise<void>,
): Promise<boolean> => {
  const actionOrderIds = normalizeActionOrderIds(context.actionOrderIds);
  if (!actionOrderIds.length) {
    deps.notifyWarning('请先选择要拆分的订单');
    return false;
  }

  const tenantId = normalizeTenantId(context.tenantId);
  if (!tenantId) {
    deps.notifyWarning('缺少租户上下文，请刷新页面后重试');
    return false;
  }

  if (!context.canWriteOrders) {
    deps.notifyWarning('暂无订单修改权限');
    return false;
  }

  const sourceOrderId = actionOrderIds[0];
  try {
    await deps.splitOrder(tenantId, sourceOrderId, {
      split_count: normalizeSplitCount(context.splitCount),
    });
    deps.notifySuccess(`拆单完成：${sourceOrderId}`);
    await onRefresh();
    return true;
  } catch (error) {
    notifyActionError(error, '拆单失败', deps.notifyError);
    return false;
  }
};
