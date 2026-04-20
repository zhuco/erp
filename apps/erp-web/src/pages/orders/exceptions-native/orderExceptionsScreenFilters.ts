import type { OrderExceptionSummary } from '@/api/orders';

import { resolveOrderExceptionSlaStatus } from './OrderExceptionsGridFilters';

export type OrderExceptionsScreenFilterState = {
  orderQuery: string;
  platform: string;
  shopId: string;
  auditStatus: string;
  exceptionStatus: string;
  slaStatus: string;
  warehouseId: string;
  assigneeName: string;
  blockingReasonKeyword: string;
};

export type OrderExceptionsScreenFilterOption = {
  label: string;
  value: string;
};

const normalizeFilterValue = (value: unknown) => String(value ?? '').trim();

const normalizeKeyword = (value: unknown) => normalizeFilterValue(value).toLowerCase();

export const createOrderExceptionsScreenFilterState = (): OrderExceptionsScreenFilterState => ({
  orderQuery: '',
  platform: '',
  shopId: '',
  auditStatus: '',
  exceptionStatus: '',
  slaStatus: '',
  warehouseId: '',
  assigneeName: '',
  blockingReasonKeyword: '',
});

export const normalizeOrderExceptionsScreenFilterState = (
  filters: Partial<OrderExceptionsScreenFilterState> | null | undefined,
): OrderExceptionsScreenFilterState => ({
  orderQuery: normalizeFilterValue(filters?.orderQuery),
  platform: normalizeFilterValue(filters?.platform),
  shopId: normalizeFilterValue(filters?.shopId),
  auditStatus: normalizeFilterValue(filters?.auditStatus),
  exceptionStatus: normalizeFilterValue(filters?.exceptionStatus),
  slaStatus: normalizeFilterValue(filters?.slaStatus).toUpperCase(),
  warehouseId: normalizeFilterValue(filters?.warehouseId),
  assigneeName: normalizeFilterValue(filters?.assigneeName),
  blockingReasonKeyword: normalizeFilterValue(filters?.blockingReasonKeyword),
});

export const hasOrderExceptionsScreenFilters = (
  filters: Partial<OrderExceptionsScreenFilterState> | null | undefined,
) => {
  const normalized = normalizeOrderExceptionsScreenFilterState(filters);
  return Object.values(normalized).some((value) => value.length > 0);
};

const matchesKeyword = (row: OrderExceptionSummary, keyword: string) => {
  if (!keyword) {
    return true;
  }

  return [
    row.order_no,
    row.order_id,
    row.receiver_name,
    row.blocking_reason,
    row.assignee_name,
  ].some((value) => normalizeKeyword(value).includes(keyword));
};

export const filterOrderExceptionsByScreenFilters = (
  rows: OrderExceptionSummary[],
  filters: Partial<OrderExceptionsScreenFilterState> | null | undefined,
) => {
  const normalized = normalizeOrderExceptionsScreenFilterState(filters);
  if (!hasOrderExceptionsScreenFilters(normalized)) {
    return rows;
  }

  const orderKeyword = normalizeKeyword(normalized.orderQuery);
  const blockingReasonKeyword = normalizeKeyword(normalized.blockingReasonKeyword);

  return rows.filter((row) => {
    if (!matchesKeyword(row, orderKeyword)) {
      return false;
    }

    if (normalized.platform && normalizeFilterValue(row.platform) !== normalized.platform) {
      return false;
    }

    if (normalized.shopId && normalizeFilterValue(row.shop_id) !== normalized.shopId) {
      return false;
    }

    if (normalized.auditStatus && normalizeFilterValue(row.audit_status) !== normalized.auditStatus) {
      return false;
    }

    if (
      normalized.exceptionStatus &&
      normalizeFilterValue(row.exception_status) !== normalized.exceptionStatus
    ) {
      return false;
    }

    if (normalized.slaStatus && resolveOrderExceptionSlaStatus(row) !== normalized.slaStatus) {
      return false;
    }

    if (normalized.warehouseId && normalizeFilterValue(row.target_warehouse_id) !== normalized.warehouseId) {
      return false;
    }

    if (normalized.assigneeName && normalizeFilterValue(row.assignee_name) !== normalized.assigneeName) {
      return false;
    }

    if (
      blockingReasonKeyword &&
      !normalizeKeyword(row.blocking_reason).includes(blockingReasonKeyword)
    ) {
      return false;
    }

    return true;
  });
};

export const buildOrderExceptionsScreenFilterOptions = (
  values: Array<string | null | undefined>,
  formatLabel?: (value: string) => string,
) => {
  const map = new Map<string, OrderExceptionsScreenFilterOption>();

  values.forEach((item) => {
    const value = normalizeFilterValue(item);
    if (!value) {
      return;
    }

    map.set(value, {
      value,
      label: formatLabel ? formatLabel(value) : value,
    });
  });

  return Array.from(map.values()).sort((left, right) =>
    left.label.localeCompare(right.label, 'zh-CN'),
  );
};
