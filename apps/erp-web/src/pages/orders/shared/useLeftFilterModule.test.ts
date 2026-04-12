import { describe, expect, it } from 'vitest';

import type { OrderReviewAgLeftNode } from '@/pages/orders/orderReviewAgLeftModel';

import { buildLeftRowsForRender, toggleExpandedGroupKeys } from './useLeftFilterModule';

describe('useLeftFilterModule core', () => {
  it('会根据左侧树和勾选键构建渲染行', () => {
    const nodes: OrderReviewAgLeftNode[] = [
      {
        key: 'status-group',
        label: '状态',
        nodeType: 'group',
        children: [
          { key: 'status:*', label: '全部状态', nodeType: 'status', count: 10 },
          { key: 'status:normal', label: '正常订单', nodeType: 'status', count: 8 },
        ],
      },
    ];
    const expanded = new Set<string>(['status-group']);
    const checked = new Set<string>(['status:normal']);

    const rows = buildLeftRowsForRender(nodes, expanded, checked);
    expect(rows).toHaveLength(3);
    expect(rows[0]).toMatchObject({ key: 'status-group', checked: false, level: 0 });
    expect(rows[2]).toMatchObject({ key: 'status:normal', checked: true, level: 1 });
  });

  it('没有数据时会回退为空状态行', () => {
    const rows = buildLeftRowsForRender([], new Set(), new Set(), '暂无数据');
    expect(rows).toEqual([
      {
        key: '__left-empty__',
        label: '暂无数据',
        count: 0,
        nodeType: 'group',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: false,
      },
    ]);
  });

  it('支持切换分组展开状态', () => {
    const base = new Set<string>(['status-group']);
    const collapsed = toggleExpandedGroupKeys(base, 'status-group');
    expect(collapsed.has('status-group')).toBe(false);

    const expanded = toggleExpandedGroupKeys(collapsed, 'shop-group');
    expect(expanded.has('shop-group')).toBe(true);
  });
});
