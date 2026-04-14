import type { UiModule08OrderReviewLeftNodeType } from '@/components/ui-modules';

export type OrderQueryLeftFilters = {
  selectedFilterId?: string;
};

export type OrderQueryLeftFilterNode = {
  key: string;
  nodeType: Exclude<UiModule08OrderReviewLeftNodeType, 'group'>;
};

const validLeafPrefixes = ['status-', 'channel-'];
const validLeafKeys = new Set(['all', 'exception', 'invalid-goods', 'tagged', 'untagged']);

const normalizeSelected = (value?: string) => {
  const normalized = String(value ?? '').trim();
  if (!normalized) {
    return 'all';
  }
  if (validLeafKeys.has(normalized)) {
    return normalized;
  }
  if (normalized.startsWith('status-')) {
    const suffix = normalized.slice('status-'.length).trim().toUpperCase();
    return suffix ? `status-${suffix}` : 'all';
  }
  if (normalized.startsWith('channel-')) {
    const suffix = normalized.slice('channel-'.length).trim().toLowerCase();
    return suffix ? `channel-${suffix}` : 'all';
  }
  return 'all';
};

const isValidLeafKey = (key: string) => validLeafKeys.has(key) || validLeafPrefixes.some((prefix) => key.startsWith(prefix));

export const hasOrderQueryLeftManualFilter = (filters: OrderQueryLeftFilters) =>
  normalizeSelected(filters.selectedFilterId) !== 'all';

export const buildOrderQueryLeftCheckedKeys = (filters: OrderQueryLeftFilters) =>
  new Set([normalizeSelected(filters.selectedFilterId)]);

export const reduceOrderQueryLeftFiltersByNode = (
  previous: OrderQueryLeftFilters,
  node: OrderQueryLeftFilterNode,
): OrderQueryLeftFilters => {
  const rawKey = String(node.key ?? '').trim();
  const nextKey = normalizeSelected(node.key);
  if (nextKey === 'all' && rawKey !== 'all') {
    return previous;
  }
  if (!isValidLeafKey(nextKey)) {
    return previous;
  }

  if (nextKey === 'all') {
    return { ...previous, selectedFilterId: 'all' };
  }

  const current = normalizeSelected(previous.selectedFilterId);
  if (current === nextKey) {
    return { ...previous, selectedFilterId: 'all' };
  }
  return { ...previous, selectedFilterId: nextKey };
};
