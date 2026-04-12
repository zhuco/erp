import { describe, expect, it } from 'vitest';

import {
  buildOrderQueryLeftCheckedKeys,
  hasOrderQueryLeftManualFilter,
  reduceOrderQueryLeftFiltersByNode,
  type OrderQueryLeftFilterNode,
  type OrderQueryLeftFilters,
} from './orderQueryLeftInteractions';

describe('orderQueryLeftInteractions', () => {
  it('detects whether manual filter is active', () => {
    expect(hasOrderQueryLeftManualFilter({ selectedFilterId: 'all' })).toBe(false);
    expect(hasOrderQueryLeftManualFilter({ selectedFilterId: 'exception' })).toBe(true);
    expect(hasOrderQueryLeftManualFilter({ selectedFilterId: 'status-REVIEW_PENDING' })).toBe(true);
  });

  it('builds checked keys with all as default', () => {
    const defaultChecked = buildOrderQueryLeftCheckedKeys({});
    expect(defaultChecked.has('all')).toBe(true);

    const filters: OrderQueryLeftFilters = { selectedFilterId: 'channel-TAOBAO' };
    const checked = buildOrderQueryLeftCheckedKeys(filters);
    expect(checked.has('channel-TAOBAO')).toBe(true);
    expect(checked.size).toBe(1);
  });

  it('supports single-select toggle and all reset', () => {
    const statusNode: OrderQueryLeftFilterNode = { key: 'status-REVIEW_PENDING', nodeType: 'status' };
    const next = reduceOrderQueryLeftFiltersByNode({ selectedFilterId: 'all' }, statusNode);
    expect(next.selectedFilterId).toBe('status-REVIEW_PENDING');

    const toggled = reduceOrderQueryLeftFiltersByNode(next, statusNode);
    expect(toggled.selectedFilterId).toBe('all');

    const allNode: OrderQueryLeftFilterNode = { key: 'all', nodeType: 'exception' };
    const reset = reduceOrderQueryLeftFiltersByNode({ selectedFilterId: 'tagged' }, allNode);
    expect(reset.selectedFilterId).toBe('all');
  });
});
