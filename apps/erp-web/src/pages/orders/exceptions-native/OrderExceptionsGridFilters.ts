import type { FilterModel } from 'ag-grid-community';
import dayjs from 'dayjs';

import type { OrderExceptionSummary } from '@/api/orders';

type SelectOption = {
  label: string;
  value: string;
};

export type OrderExceptionsColumnFilters = {
  orderNoKeyword?: string;
  platform?: string[];
  shopId?: string[];
  receiverNameKeyword?: string;
  auditStatus?: string[];
  exceptionStatus?: string[];
  slaStatus?: string[];
  warehouseId?: string[];
  assigneeName?: string[];
  blockingReasonKeyword?: string;
};

type OrderExceptionSummaryWithSla = OrderExceptionSummary & {
  sla_status?: string | null;
  sla_remaining_minutes?: number | null;
};

type ColumnFilterFieldKey = keyof OrderExceptionsColumnFilters;

const columnFilterFieldKeys: ColumnFilterFieldKey[] = [
  'orderNoKeyword',
  'platform',
  'shopId',
  'receiverNameKeyword',
  'auditStatus',
  'exceptionStatus',
  'slaStatus',
  'warehouseId',
  'assigneeName',
  'blockingReasonKeyword',
];

const normalizeKeyword = (value: string | undefined) => value?.trim() ?? '';

const normalizeSingleValue = (value?: string[]) => {
  const normalized = String(value?.[0] ?? '').trim();
  return normalized || '';
};

const readSetValue = (model: unknown) => {
  if (!model || typeof model !== 'object' || !('values' in model)) {
    return '';
  }
  const values = (model as { values?: unknown }).values;
  if (!Array.isArray(values) || values.length === 0) {
    return '';
  }
  return String(values[0] ?? '').trim();
};

const readTextValue = (model: unknown) => {
  if (!model || typeof model !== 'object' || !('filter' in model)) {
    return '';
  }
  return normalizeKeyword(String((model as { filter?: unknown }).filter ?? ''));
};

const normalizeSetModel = (value: string) =>
  value
    ? {
        filterType: 'set' as const,
        values: [value],
      }
    : null;

const normalizeTextModel = (value: string) =>
  value
    ? {
        filterType: 'text' as const,
        type: 'contains' as const,
        filter: value,
      }
    : null;

const normalizeColumnFilters = (filters: OrderExceptionsColumnFilters): OrderExceptionsColumnFilters => ({
  orderNoKeyword: normalizeKeyword(filters.orderNoKeyword) || undefined,
  platform: normalizeSingleValue(filters.platform) ? [normalizeSingleValue(filters.platform)] : undefined,
  shopId: normalizeSingleValue(filters.shopId) ? [normalizeSingleValue(filters.shopId)] : undefined,
  receiverNameKeyword: normalizeKeyword(filters.receiverNameKeyword) || undefined,
  auditStatus: normalizeSingleValue(filters.auditStatus) ? [normalizeSingleValue(filters.auditStatus)] : undefined,
  exceptionStatus: normalizeSingleValue(filters.exceptionStatus)
    ? [normalizeSingleValue(filters.exceptionStatus)]
    : undefined,
  slaStatus: normalizeSingleValue(filters.slaStatus) ? [normalizeSingleValue(filters.slaStatus)] : undefined,
  warehouseId: normalizeSingleValue(filters.warehouseId) ? [normalizeSingleValue(filters.warehouseId)] : undefined,
  assigneeName: normalizeSingleValue(filters.assigneeName) ? [normalizeSingleValue(filters.assigneeName)] : undefined,
  blockingReasonKeyword: normalizeKeyword(filters.blockingReasonKeyword) || undefined,
});

const isColumnFilterValueActive = (
  key: ColumnFilterFieldKey,
  value: OrderExceptionsColumnFilters[ColumnFilterFieldKey] | undefined,
) => {
  if (key === 'orderNoKeyword' || key === 'receiverNameKeyword' || key === 'blockingReasonKeyword') {
    return normalizeKeyword(value as string | undefined).length > 0;
  }
  return normalizeSingleValue(value as string[] | undefined).length > 0;
};

const isSameColumnFilterValue = (
  key: ColumnFilterFieldKey,
  left: OrderExceptionsColumnFilters[ColumnFilterFieldKey] | undefined,
  right: OrderExceptionsColumnFilters[ColumnFilterFieldKey] | undefined,
) => {
  if (key === 'orderNoKeyword' || key === 'receiverNameKeyword' || key === 'blockingReasonKeyword') {
    return normalizeKeyword(left as string | undefined) === normalizeKeyword(right as string | undefined);
  }
  return normalizeSingleValue(left as string[] | undefined) === normalizeSingleValue(right as string[] | undefined);
};

export const resolveSingleOrderExceptionsColumnFilters = (
  previous: OrderExceptionsColumnFilters,
  next: OrderExceptionsColumnFilters,
): OrderExceptionsColumnFilters => {
  const normalizedNext = normalizeColumnFilters(next);
  const activeKeys = columnFilterFieldKeys.filter((key) =>
    isColumnFilterValueActive(key, normalizedNext[key]),
  );
  if (activeKeys.length <= 1) {
    return normalizedNext;
  }

  const changedActiveKey =
    activeKeys.find(
      (key) =>
        !isSameColumnFilterValue(key, previous[key], normalizedNext[key]) &&
        isColumnFilterValueActive(key, normalizedNext[key]),
    ) ?? activeKeys[activeKeys.length - 1];

  const resolved: OrderExceptionsColumnFilters = {};
  if (changedActiveKey) {
    resolved[changedActiveKey] = normalizedNext[changedActiveKey];
  }
  return resolved;
};

export const hasOrderExceptionsMainGridColumnFilters = (filters: OrderExceptionsColumnFilters) =>
  columnFilterFieldKeys.some((key) => isColumnFilterValueActive(key, filters[key]));

export const clearOrderExceptionsMainGridColumnFilters = (): OrderExceptionsColumnFilters => ({});

export const buildOrderExceptionsMainGridFilterModel = (
  filters: OrderExceptionsColumnFilters,
): FilterModel => {
  const nextModel: FilterModel = {};
  const normalizedFilters = normalizeColumnFilters(filters);

  const orderNoKeyword = normalizeKeyword(normalizedFilters.orderNoKeyword);
  if (orderNoKeyword) {
    nextModel.order_no = normalizeTextModel(orderNoKeyword);
  }

  const platformValue = normalizeSingleValue(normalizedFilters.platform);
  if (platformValue) {
    nextModel.platform = normalizeSetModel(platformValue);
  }

  const shopValue = normalizeSingleValue(normalizedFilters.shopId);
  if (shopValue) {
    nextModel.shop_id = normalizeSetModel(shopValue);
  }

  const receiverNameKeyword = normalizeKeyword(normalizedFilters.receiverNameKeyword);
  if (receiverNameKeyword) {
    nextModel.receiver_name = normalizeTextModel(receiverNameKeyword);
  }

  const auditStatusValue = normalizeSingleValue(normalizedFilters.auditStatus);
  if (auditStatusValue) {
    nextModel.audit_status = normalizeSetModel(auditStatusValue);
  }

  const exceptionStatusValue = normalizeSingleValue(normalizedFilters.exceptionStatus);
  if (exceptionStatusValue) {
    nextModel.exception_status = normalizeSetModel(exceptionStatusValue);
  }

  const slaStatusValue = normalizeSingleValue(normalizedFilters.slaStatus);
  if (slaStatusValue) {
    nextModel.sla_status = normalizeSetModel(slaStatusValue);
  }

  const warehouseValue = normalizeSingleValue(normalizedFilters.warehouseId);
  if (warehouseValue) {
    nextModel.target_warehouse_id = normalizeSetModel(warehouseValue);
  }

  const assigneeValue = normalizeSingleValue(normalizedFilters.assigneeName);
  if (assigneeValue) {
    nextModel.assignee_name = normalizeSetModel(assigneeValue);
  }

  const blockingReasonKeyword = normalizeKeyword(normalizedFilters.blockingReasonKeyword);
  if (blockingReasonKeyword) {
    nextModel.blocking_reason = normalizeTextModel(blockingReasonKeyword);
  }

  return nextModel;
};

export const extractOrderExceptionsMainGridColumnFilters = (
  model: FilterModel | null | undefined,
): OrderExceptionsColumnFilters => ({
  orderNoKeyword: readTextValue(model?.order_no) || undefined,
  platform: (() => {
    const value = readSetValue(model?.platform);
    return value ? [value] : undefined;
  })(),
  shopId: (() => {
    const value = readSetValue(model?.shop_id);
    return value ? [value] : undefined;
  })(),
  receiverNameKeyword: readTextValue(model?.receiver_name) || undefined,
  auditStatus: (() => {
    const value = readSetValue(model?.audit_status);
    return value ? [value] : undefined;
  })(),
  exceptionStatus: (() => {
    const value = readSetValue(model?.exception_status);
    return value ? [value] : undefined;
  })(),
  slaStatus: (() => {
    const value = readSetValue(model?.sla_status);
    return value ? [value] : undefined;
  })(),
  warehouseId: (() => {
    const value = readSetValue(model?.target_warehouse_id);
    return value ? [value] : undefined;
  })(),
  assigneeName: (() => {
    const value = readSetValue(model?.assignee_name);
    return value ? [value] : undefined;
  })(),
  blockingReasonKeyword: readTextValue(model?.blocking_reason) || undefined,
});

export const hasSameOrderExceptionsMainGridColumnFilters = (
  left: OrderExceptionsColumnFilters,
  right: OrderExceptionsColumnFilters,
) => {
  const normalizedLeft = normalizeColumnFilters(left);
  const normalizedRight = normalizeColumnFilters(right);

  return columnFilterFieldKeys.every((key) =>
    isSameColumnFilterValue(key, normalizedLeft[key], normalizedRight[key]),
  );
};

export const mergeOrderExceptionsFilterOptionsWithCurrentValue = (
  options: SelectOption[],
  currentValue: string | undefined,
  fallbackLabel?: (value: string) => string,
) => {
  const normalizedCurrentValue = String(currentValue ?? '').trim();
  const map = new Map<string, SelectOption>();

  options.forEach((option) => {
    const value = String(option.value ?? '').trim();
    if (!value) {
      return;
    }
    map.set(value, {
      value,
      label: String(option.label ?? '').trim() || value,
    });
  });

  if (normalizedCurrentValue && !map.has(normalizedCurrentValue)) {
    map.set(normalizedCurrentValue, {
      value: normalizedCurrentValue,
      label: fallbackLabel ? fallbackLabel(normalizedCurrentValue) : normalizedCurrentValue,
    });
  }

  return Array.from(map.values());
};

export const resolveOrderExceptionSlaStatus = (record: OrderExceptionSummary): string => {
  const raw = String((record as OrderExceptionSummaryWithSla).sla_status ?? '')
    .trim()
    .toUpperCase();
  if (raw === 'OVERDUE') return 'OVERDUE';
  if (raw === 'DUE_SOON') return 'DUE_SOON';
  if (raw === 'NORMAL') return 'NORMAL';

  const remainingMinutes = (record as OrderExceptionSummaryWithSla).sla_remaining_minutes;
  if (Number.isFinite(remainingMinutes)) {
    if ((remainingMinutes as number) < 0) {
      return 'OVERDUE';
    }
    if ((remainingMinutes as number) <= 15) {
      return 'DUE_SOON';
    }
  }

  if (record.latest_action_at) {
    const latestActionAt = dayjs(record.latest_action_at);
    if (latestActionAt.isValid() && dayjs().diff(latestActionAt, 'minute') >= 15) {
      return 'OVERDUE';
    }
  }

  return 'NORMAL';
};

export const resolveOrderExceptionSlaStatusLabel = (status: string): string => {
  switch (status) {
    case 'OVERDUE':
      return '已超时';
    case 'DUE_SOON':
      return '即将超时';
    case 'NORMAL':
    default:
      return '正常';
  }
};

export const filterOrderExceptionsByColumnFilters = (
  rows: OrderExceptionSummary[],
  filters: OrderExceptionsColumnFilters,
) => {
  const normalizedFilters = normalizeColumnFilters(filters);
  const orderNoKeyword = normalizeKeyword(normalizedFilters.orderNoKeyword).toLowerCase();
  const platformValue = normalizeSingleValue(normalizedFilters.platform);
  const shopValue = normalizeSingleValue(normalizedFilters.shopId);
  const receiverNameKeyword = normalizeKeyword(normalizedFilters.receiverNameKeyword).toLowerCase();
  const auditStatusValue = normalizeSingleValue(normalizedFilters.auditStatus);
  const exceptionStatusValue = normalizeSingleValue(normalizedFilters.exceptionStatus);
  const slaStatusValue = normalizeSingleValue(normalizedFilters.slaStatus);
  const warehouseValue = normalizeSingleValue(normalizedFilters.warehouseId);
  const assigneeValue = normalizeSingleValue(normalizedFilters.assigneeName);
  const blockingReasonKeyword = normalizeKeyword(normalizedFilters.blockingReasonKeyword).toLowerCase();

  if (!hasOrderExceptionsMainGridColumnFilters(normalizedFilters)) {
    return rows;
  }

  return rows.filter((row) => {
    if (orderNoKeyword && !String(row.order_no ?? '').trim().toLowerCase().includes(orderNoKeyword)) {
      return false;
    }

    if (platformValue && String(row.platform ?? '').trim() !== platformValue) {
      return false;
    }

    if (shopValue && String(row.shop_id ?? '').trim() !== shopValue) {
      return false;
    }

    if (
      receiverNameKeyword &&
      !String(row.receiver_name ?? '').trim().toLowerCase().includes(receiverNameKeyword)
    ) {
      return false;
    }

    if (auditStatusValue && String(row.audit_status ?? '').trim() !== auditStatusValue) {
      return false;
    }

    if (exceptionStatusValue && String(row.exception_status ?? '').trim() !== exceptionStatusValue) {
      return false;
    }

    if (slaStatusValue && resolveOrderExceptionSlaStatus(row) !== slaStatusValue) {
      return false;
    }

    if (warehouseValue && String(row.target_warehouse_id ?? '').trim() !== warehouseValue) {
      return false;
    }

    if (assigneeValue && String(row.assignee_name ?? '').trim() !== assigneeValue) {
      return false;
    }

    if (
      blockingReasonKeyword &&
      !String(row.blocking_reason ?? '').trim().toLowerCase().includes(blockingReasonKeyword)
    ) {
      return false;
    }

    return true;
  });
};
