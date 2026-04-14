import type { OrderSummary } from '@/api/orders';

const readStringField = (record: OrderSummary, keys: string[]): string => {
  const rawRecord = record as Record<string, unknown>;
  for (const key of keys) {
    const value = String(rawRecord[key] ?? '').trim();
    if (value) {
      return value;
    }
  }
  return '';
};

export const resolveOrderLogisticsText = (record: OrderSummary): string =>
  readStringField(record, ['logistics_company', 'carrier_name', 'courier_name']);

export const hasOrdersWithEmptyLogistics = (
  actionOrderIds: string[],
  rows: OrderSummary[],
  resolveOrderActionId: (record: OrderSummary) => string,
): boolean => {
  if (!actionOrderIds.length) {
    return false;
  }
  const actionOrderSet = new Set(actionOrderIds);
  return rows.some((record) => {
    const actionOrderId = resolveOrderActionId(record);
    if (!actionOrderId || !actionOrderSet.has(actionOrderId)) {
      return false;
    }
    return !resolveOrderLogisticsText(record);
  });
};

