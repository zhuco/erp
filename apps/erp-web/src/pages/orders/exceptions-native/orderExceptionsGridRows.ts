import type { OrderExceptionSummary } from '@/api/orders';

import { resolveOrderExceptionRowKey } from './orderExceptionsRowKey';

export type OrderExceptionsGridRow = OrderExceptionSummary & {
  __erpRowId: string;
};

export const toOrderExceptionsGridRows = (rows: OrderExceptionSummary[]): OrderExceptionsGridRow[] => {
  const keyCounter = new Map<string, number>();

  return rows.map((row, index) => {
    const normalizedOrderId = resolveOrderExceptionRowKey(row);
    const baseKey = normalizedOrderId || '__missing_order_id__';
    const occurrence = keyCounter.get(baseKey) ?? 0;

    keyCounter.set(baseKey, occurrence + 1);

    const hasValidOrderId = Boolean(normalizedOrderId);
    const stableRowId = hasValidOrderId
      ? occurrence === 0
        ? normalizedOrderId
        : `${normalizedOrderId}__dup_${occurrence}`
      : `${baseKey}__dup_${occurrence}__idx_${index}`;

    return {
      ...row,
      __erpRowId: stableRowId,
    };
  });
};
