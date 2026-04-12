import { isValidElement } from 'react';
import type { ReactElement, ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';

import { UiModule05DetailPanel } from './UiModule05DetailPanel';
import { UiModule05OrderReviewRightPanelTemplate } from './UiModule05OrderReviewRightPanelTemplate';

const findElement = (
  node: ReactNode,
  predicate: (element: ReactElement) => boolean,
): ReactElement | null => {
  if (Array.isArray(node)) {
    for (const child of node) {
      const match = findElement(child, predicate);
      if (match) {
        return match;
      }
    }
    return null;
  }

  if (!isValidElement(node)) {
    return null;
  }

  if (predicate(node)) {
    return node;
  }

  return findElement(node.props.children, predicate);
};

const collectText = (node: ReactNode): string[] => {
  if (Array.isArray(node)) {
    return node.flatMap((child) => collectText(child));
  }
  if (typeof node === 'string') {
    return [node];
  }
  if (!isValidElement(node)) {
    return [];
  }
  return collectText(node.props.children);
};

describe('UiModule05OrderReviewRightPanelTemplate', () => {
  it('renders expected Chinese labels and placeholders', () => {
    const element = UiModule05OrderReviewRightPanelTemplate({
      detailError: null,
      detailLoadedId: 'ORDER-1',
      orderId: 'ORDER-1',
      platformOrderId: 'PLAT-1',
      platformText: '淘宝',
      shopId: 'SHOP-1',
      receiverName: '张三',
      receiverPhone: '13800138000',
      logisticsCompany: '顺丰速运',
      warehouseId: 'WH-DEFAULT',
      totalAmountText: '¥12.30',
      courierOptions: [{ label: '顺丰速运 (SF)', value: '顺丰速运' }],
      warehouseOptions: [{ label: '默认仓', value: 'WH-DEFAULT' }],
      logisticsUpdating: false,
      warehouseUpdating: false,
      detailSelectDisabled: false,
      orderTagsApplying: false,
      markTags: ['紧急'],
      markMenuItems: [{ key: '紧急', label: '紧急' }],
      onMarkSelect: vi.fn(),
      onLogisticsChange: vi.fn(),
      onWarehouseChange: vi.fn(),
    });

    const detailPanel = findElement(
      element,
      (child) => child.type === UiModule05DetailPanel && Array.isArray(child.props.panels),
    );
    expect(detailPanel).not.toBeNull();

    const panels = (detailPanel?.props.panels ?? []) as Array<{
      key: string;
      label: string;
      children: ReactNode;
    }>;

    const marksPanel = panels.find((panel) => panel.key === 'marks');
    const detailInfoPanel = panels.find((panel) => panel.key === 'detail');

    expect(marksPanel?.label).toBe('订单记号');
    expect(detailInfoPanel?.label).toBe('订单详情');

    const marksText = collectText(marksPanel?.children ?? null);
    expect(marksText).toContain('添加');

    const logisticsSelect = findElement(
      detailInfoPanel?.children ?? null,
      (child) => child.props.placeholder === '选择物流公司',
    );
    const warehouseSelect = findElement(
      detailInfoPanel?.children ?? null,
      (child) => child.props.placeholder === '选择发货仓库',
    );

    expect(logisticsSelect).not.toBeNull();
    expect(warehouseSelect).not.toBeNull();
  });
});
