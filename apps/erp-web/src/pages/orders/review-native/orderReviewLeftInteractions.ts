import { normalizeMulti } from './helpers';
import type { ColumnFilters, OrderReviewAgLeftRenderRow } from './types';

type LeftNodeType = OrderReviewAgLeftRenderRow['nodeType'];

export type OrderReviewLeftFilterNode = {
  key: string;
  nodeType: LeftNodeType;
};

const clearKeyToFilterField: Record<string, keyof ColumnFilters> = {
  'platform:*': 'platform',
  'shop:*': 'shopId',
  'status:*': 'status',
  'time:*': 'timeRange',
  'exception:*': 'exceptionType',
  'warehouse:*': 'warehouseId',
  'logistics:*': 'logisticsCompany',
  'remark:*': 'remarkState',
  'tag:*': 'tagState',
};

const singleSelectValue = <T extends string>(current: readonly T[] | undefined, value: string): T[] | undefined => {
  const normalized = normalizeMulti(current as string[]);
  if (normalized.length === 1 && normalized[0] === value) {
    return undefined;
  }
  return [value as T];
};

export const hasOrderReviewLeftManualFilter = (filters: ColumnFilters) =>
  normalizeMulti(filters.platform).length > 0 ||
  normalizeMulti(filters.shopId).length > 0 ||
  normalizeMulti(filters.status).length > 0 ||
  normalizeMulti(filters.timeRange).length > 0 ||
  normalizeMulti(filters.exceptionType).length > 0 ||
  normalizeMulti(filters.warehouseId).length > 0 ||
  normalizeMulti(filters.logisticsCompany).length > 0 ||
  normalizeMulti(filters.remarkState).length > 0 ||
  normalizeMulti(filters.tagState).length > 0;

export const buildOrderReviewLeftCheckedKeys = (filters: ColumnFilters) =>
  new Set([
    ...normalizeMulti(filters.platform).map((value) => `platform:${value}`),
    ...normalizeMulti(filters.shopId).map((value) => `shop:${value}`),
    ...normalizeMulti(filters.status).map((value) => `status:${value}`),
    ...normalizeMulti(filters.timeRange).map((value) => `time:${value}`),
    ...normalizeMulti(filters.exceptionType).map((value) => `exception:${value}`),
    ...normalizeMulti(filters.warehouseId).map((value) => `warehouse:${value}`),
    ...normalizeMulti(filters.logisticsCompany).map((value) => `logistics:${value}`),
    ...normalizeMulti(filters.remarkState).map((value) => `remark:${value}`),
    ...normalizeMulti(filters.tagState).map((value) => `tag:${value}`),
  ]);

export const reduceOrderReviewFiltersByNode = (
  previous: ColumnFilters,
  node: OrderReviewLeftFilterNode,
): ColumnFilters => {
  const clearField = clearKeyToFilterField[node.key];
  if (clearField) {
    return { ...previous, [clearField]: undefined };
  }

  if (node.nodeType === 'platform' && node.key.startsWith('platform:')) {
    return { ...previous, platform: singleSelectValue(previous.platform, node.key.slice('platform:'.length)) };
  }
  if (node.nodeType === 'shop' && node.key.startsWith('shop:')) {
    return { ...previous, shopId: singleSelectValue(previous.shopId, node.key.slice('shop:'.length)) };
  }
  if (node.nodeType === 'status' && node.key.startsWith('status:')) {
    return {
      ...previous,
      status: singleSelectValue(previous.status, node.key.slice('status:'.length)) as ('normal' | 'exception')[] | undefined,
    };
  }
  if (node.nodeType === 'time' && node.key.startsWith('time:')) {
    return {
      ...previous,
      timeRange: singleSelectValue(previous.timeRange, node.key.slice('time:'.length)) as
        | ('today' | 'yesterday' | 'within3d' | 'within7d' | 'within30d' | 'gt30d')[]
        | undefined,
    };
  }
  if (node.nodeType === 'exception' && node.key.startsWith('exception:')) {
    return {
      ...previous,
      exceptionType: singleSelectValue(previous.exceptionType, node.key.slice('exception:'.length)) as
        | ('frozen' | 'noLogistics' | 'stopShipping' | 'inventoryShortage' | 'invalidGoods' | 'refundReturn')[]
        | undefined,
    };
  }
  if (node.nodeType === 'warehouse' && node.key.startsWith('warehouse:')) {
    return {
      ...previous,
      warehouseId: singleSelectValue(previous.warehouseId, node.key.slice('warehouse:'.length)),
    };
  }
  if (node.nodeType === 'logistics' && node.key.startsWith('logistics:')) {
    return {
      ...previous,
      logisticsCompany: singleSelectValue(previous.logisticsCompany, node.key.slice('logistics:'.length)),
    };
  }
  if (node.nodeType === 'remark' && node.key.startsWith('remark:')) {
    return {
      ...previous,
      remarkState: singleSelectValue(previous.remarkState, node.key.slice('remark:'.length)) as ('has' | 'none')[] | undefined,
    };
  }
  if (node.nodeType === 'tag' && node.key.startsWith('tag:')) {
    return {
      ...previous,
      tagState: singleSelectValue(previous.tagState, node.key.slice('tag:'.length)) as ('has' | 'none')[] | undefined,
    };
  }

  return previous;
};
