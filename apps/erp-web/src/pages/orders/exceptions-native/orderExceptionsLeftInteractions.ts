import dayjs from 'dayjs';

import type { OrderExceptionSummary } from '@/api/orders';

export type OrderExceptionsLeftFilters = {
  status?: string[];
  assignee?: string[];
  sla?: Array<'overdue' | 'normal' | 'mine'>;
  warehouseId?: string[];
};

export type OrderExceptionsLeftFilterNode = {
  key: string;
  nodeType: 'group' | 'status' | 'assignee' | 'sla' | 'warehouse';
};

type FilterOptions = {
  currentUserId: string;
  nowTs?: number;
  overdueThresholdMinutes?: number;
};

const normalizeValues = <T extends string>(value?: readonly T[]) => {
  if (!value) {
    return [];
  }
  return Array.from(new Set(value.map((item) => String(item).trim()).filter(Boolean) as T[]));
};

const singleSelectValue = <T extends string>(current: readonly T[] | undefined, value: string): T[] | undefined => {
  const normalized = normalizeValues(current);
  if (normalized.length === 1 && normalized[0] === value) {
    return undefined;
  }
  return [value as T];
};

const clearKeyToFilterField: Record<string, keyof OrderExceptionsLeftFilters> = {
  'status:*': 'status',
  'assignee:*': 'assignee',
  'sla:*': 'sla',
  'warehouse:*': 'warehouseId',
};

const clearAllLeftFilterFields = (
  previous: OrderExceptionsLeftFilters,
): OrderExceptionsLeftFilters => ({
  ...previous,
  status: undefined,
  assignee: undefined,
  sla: undefined,
  warehouseId: undefined,
});

export const isOverdueExceptionRow = (
  row: OrderExceptionSummary,
  nowTs: number,
  overdueThresholdMinutes: number,
) => {
  if (!row.latest_action_at) {
    return false;
  }
  const actionTs = dayjs(row.latest_action_at).valueOf();
  if (!Number.isFinite(actionTs)) {
    return false;
  }
  return nowTs - actionTs >= overdueThresholdMinutes * 60_000;
};

export const hasOrderExceptionsLeftManualFilter = (filters: OrderExceptionsLeftFilters) =>
  normalizeValues(filters.status).length > 0 ||
  normalizeValues(filters.assignee).length > 0 ||
  normalizeValues(filters.sla).length > 0 ||
  normalizeValues(filters.warehouseId).length > 0;

export const buildOrderExceptionsCheckedKeys = (filters: OrderExceptionsLeftFilters) =>
  new Set([
    ...normalizeValues(filters.status).map((value) => `status:${value}`),
    ...normalizeValues(filters.assignee).map((value) => `assignee:${value}`),
    ...normalizeValues(filters.sla).map((value) => `sla:${value}`),
    ...normalizeValues(filters.warehouseId).map((value) => `warehouse:${value}`),
  ]);

export const reduceOrderExceptionsFiltersByNode = (
  previous: OrderExceptionsLeftFilters,
  node: OrderExceptionsLeftFilterNode,
): OrderExceptionsLeftFilters => {
  const cleared = clearAllLeftFilterFields(previous);
  const clearField = clearKeyToFilterField[node.key];
  if (clearField) {
    return { ...cleared, [clearField]: undefined };
  }

  if (node.nodeType === 'status' && node.key.startsWith('status:')) {
    return { ...cleared, status: singleSelectValue(previous.status, node.key.slice('status:'.length)) };
  }
  if (node.nodeType === 'assignee' && node.key.startsWith('assignee:')) {
    return { ...cleared, assignee: singleSelectValue(previous.assignee, node.key.slice('assignee:'.length)) };
  }
  if (node.nodeType === 'sla' && node.key.startsWith('sla:')) {
    return {
      ...cleared,
      sla: singleSelectValue(previous.sla, node.key.slice('sla:'.length)) as Array<'overdue' | 'normal' | 'mine'> | undefined,
    };
  }
  if (node.nodeType === 'warehouse' && node.key.startsWith('warehouse:')) {
    return {
      ...cleared,
      warehouseId: singleSelectValue(previous.warehouseId, node.key.slice('warehouse:'.length)),
    };
  }
  return previous;
};

export const filterOrderExceptionsRows = (
  rows: OrderExceptionSummary[],
  filters: OrderExceptionsLeftFilters,
  options: FilterOptions,
) => {
  const selectedStatus = new Set(normalizeValues(filters.status));
  const selectedAssignee = new Set(normalizeValues(filters.assignee));
  const selectedSla = new Set(normalizeValues(filters.sla));
  const selectedWarehouse = new Set(normalizeValues(filters.warehouseId));
  const nowTs = options.nowTs ?? Date.now();
  const overdueThresholdMinutes = options.overdueThresholdMinutes ?? 15;

  if (
    selectedStatus.size === 0 &&
    selectedAssignee.size === 0 &&
    selectedSla.size === 0 &&
    selectedWarehouse.size === 0
  ) {
    return rows;
  }

  return rows.filter((row) => {
    if (selectedStatus.size > 0) {
      const status = String(row.exception_status ?? '').trim();
      if (!selectedStatus.has(status)) {
        return false;
      }
    }

    if (selectedAssignee.size > 0) {
      const assignee = String(row.assignee_name ?? '').trim() || '__unassigned__';
      if (!selectedAssignee.has(assignee)) {
        return false;
      }
    }

    if (selectedWarehouse.size > 0) {
      const warehouse = String(row.target_warehouse_id ?? '').trim() || '__none__';
      if (!selectedWarehouse.has(warehouse)) {
        return false;
      }
    }

    if (selectedSla.size > 0) {
      const overdue = isOverdueExceptionRow(row, nowTs, overdueThresholdMinutes);
      const isMine = Boolean(options.currentUserId) && String(row.assignee_id ?? '').trim() === options.currentUserId;
      const matched =
        (selectedSla.has('overdue') && overdue) ||
        (selectedSla.has('normal') && !overdue) ||
        (selectedSla.has('mine') && isMine);
      if (!matched) {
        return false;
      }
    }

    return true;
  });
};
