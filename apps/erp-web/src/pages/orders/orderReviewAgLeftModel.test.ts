import { describe, expect, it } from 'vitest';

import {
  flattenAgLeftTree,
  parseCheckedKeysToFilters,
  toggleCheckedValue,
  type OrderReviewAgLeftNode,
} from './orderReviewAgLeftModel';

const treeFixture: OrderReviewAgLeftNode[] = [
  {
    key: 'platform-group',
    label: 'Platform',
    nodeType: 'group',
    children: [
      { key: 'platform:TAOBAO', label: 'Taobao', count: 10, nodeType: 'platform' },
      { key: 'platform:JD', label: 'JD', count: 8, nodeType: 'platform' },
    ],
  },
  {
    key: 'shop-group',
    label: 'Shop',
    nodeType: 'group',
    children: [{ key: 'shop:S01', label: 'Shop 1', count: 6, nodeType: 'shop' }],
  },
];

describe('orderReviewAgLeftModel', () => {
  it('flattens tree rows with expanded state', () => {
    const collapsed = flattenAgLeftTree(treeFixture, new Set());
    expect(collapsed.map((row) => row.key)).toEqual(['platform-group', 'shop-group']);

    const expanded = flattenAgLeftTree(treeFixture, new Set(['platform-group']));
    expect(expanded.map((row) => row.key)).toEqual([
      'platform-group',
      'platform:TAOBAO',
      'platform:JD',
      'shop-group',
    ]);
    expect(expanded[1]).toMatchObject({ key: 'platform:TAOBAO', level: 1, hasChildren: false });
  });

  it('parses checked keys to platform/shop filters', () => {
    const result = parseCheckedKeysToFilters(['platform:TAOBAO', 'shop:S01', 'platform:JD']);
    expect(result.platform).toEqual(['TAOBAO', 'JD']);
    expect(result.shopId).toEqual(['S01']);
  });

  it('toggles checked values and removes empty array as undefined', () => {
    expect(toggleCheckedValue(undefined, 'A')).toEqual(['A']);
    expect(toggleCheckedValue(['A'], 'A')).toBeUndefined();
    expect(toggleCheckedValue(['A'], 'B')).toEqual(['A', 'B']);
  });
});
