import type { Key } from 'react';

import type { OrderExceptionSummary } from '@/api/orders';

export const normalizeOrderExceptionKey = (value: unknown) => String(value ?? '').trim();

export const resolveOrderExceptionRowKey = (record: Pick<OrderExceptionSummary, 'order_id'> | null | undefined) =>
  normalizeOrderExceptionKey(record?.order_id);

export const hasOrderExceptionRowKey = (
  record: Pick<OrderExceptionSummary, 'order_id'> | null | undefined,
  key: unknown,
) => {
  const normalizedRecordKey = resolveOrderExceptionRowKey(record);
  const normalizedTargetKey = normalizeOrderExceptionKey(key);
  return Boolean(normalizedRecordKey) && normalizedRecordKey === normalizedTargetKey;
};

export const filterOrderExceptionSelectionKeys = (
  rows: OrderExceptionSummary[],
  keys: Key[],
): Key[] => {
  const existingKeys = new Set(rows.map((row) => resolveOrderExceptionRowKey(row)).filter(Boolean));
  return keys
    .map((key) => normalizeOrderExceptionKey(key))
    .filter((key, index, list) => Boolean(key) && list.indexOf(key) === index && existingKeys.has(key));
};

export const resolveNextOrderExceptionActiveKey = (
  rows: OrderExceptionSummary[],
  currentKey: unknown,
): string | null => {
  const normalizedCurrentKey = normalizeOrderExceptionKey(currentKey);
  if (normalizedCurrentKey && rows.some((row) => hasOrderExceptionRowKey(row, normalizedCurrentKey))) {
    return normalizedCurrentKey;
  }
  return resolveOrderExceptionRowKey(rows[0]) || null;
};

export const findOrderExceptionRowIndex = (
  rows: OrderExceptionSummary[],
  key: unknown,
): number => {
  const normalizedKey = normalizeOrderExceptionKey(key);
  if (!normalizedKey) {
    return -1;
  }
  return rows.findIndex((row) => hasOrderExceptionRowKey(row, normalizedKey));
};
