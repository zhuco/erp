import { createElement } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { beforeEach, describe, expect, it, vi } from 'vitest';

let capturedProps: Record<string, unknown> | null = null;

vi.mock('@/components/page-templates', () => ({
  WorkbenchDetailPanelTemplate: (props: Record<string, unknown>) => {
    capturedProps = props;
    return createElement('div', { 'data-testid': 'mock-workbench-detail-panel' });
  },
}));

import { OrderReviewDetailPanel } from './OrderReviewDetailPanel';

describe('OrderReviewDetailPanel', () => {
  beforeEach(() => {
    capturedProps = null;
  });

  it('merges builtin marks into the right-side mark list', () => {
    renderToStaticMarkup(
      createElement(OrderReviewDetailPanel, {
        detailOrder: {
          id: 'ORDER-1',
          platform_order_id: 'PLAT-1',
          platform: 'taobao',
          shop_id: 'SHOP-1',
          receiver_name: 'Alice',
          receiver_phone: '13800138000',
          total_amount_cent: 1230,
          logistics_company: 'SF',
          ship_warehouse_id: 'WH-1',
          mapping_missing: true,
        },
        error: null,
        detailLoadedId: 'ORDER-1',
        warehouseOptions: [{ label: 'WH-1', value: 'WH-1' }],
        courierOptions: [{ label: 'SF', value: 'SF' }],
        detailWarehouseId: 'WH-1',
        detailLogisticsCompany: 'SF',
        warehouseUpdating: false,
        logisticsUpdating: false,
        onWarehouseChange: vi.fn(),
        onLogisticsChange: vi.fn(),
        orderTagDefinitions: [{ tag_id: 'tag-vip', name: 'VIP', color: '#1677ff' }],
        currentOrderTags: ['VIP'],
        orderTagsApplying: false,
        onApplyOrderTags: vi.fn(async () => true),
        onCreateOrderTagDefinition: vi.fn(async () => true),
        onDeleteOrderTagDefinition: vi.fn(async () => true),
        onRefreshOrderTagDefinitions: vi.fn(async () => undefined),
      }),
    );

    expect(capturedProps).not.toBeNull();
    expect(
      ((capturedProps?.markTags as Array<string | { key: string }>) ?? []).map((tag) =>
        typeof tag === 'string' ? tag : tag.key,
      ),
    ).toEqual(['unmatched', 'VIP']);
  });

  it('filters right-side addable marks down to remaining custom marks only', () => {
    renderToStaticMarkup(
      createElement(OrderReviewDetailPanel, {
        detailOrder: {
          id: 'ORDER-2',
          platform_order_id: 'PLAT-2',
          platform: 'taobao',
          shop_id: 'SHOP-2',
          receiver_name: 'Bob',
          receiver_phone: '13900139000',
          total_amount_cent: 2580,
        },
        error: null,
        detailLoadedId: 'ORDER-2',
        warehouseOptions: [{ label: 'WH-1', value: 'WH-1' }],
        courierOptions: [{ label: 'SF', value: 'SF' }],
        detailWarehouseId: 'WH-1',
        detailLogisticsCompany: 'SF',
        warehouseUpdating: false,
        logisticsUpdating: false,
        onWarehouseChange: vi.fn(),
        onLogisticsChange: vi.fn(),
        orderTagDefinitions: [
          { tag_id: 'tag-vip', name: 'VIP', color: '#1677ff' },
          { tag_id: 'tag-shortage', name: 'SHORTAGE', color: '#fa541c' },
          { tag_id: 'tag-urgent', name: 'Urgent', color: '#722ed1' },
        ],
        currentOrderTags: ['Urgent'],
        orderTagsApplying: false,
        onApplyOrderTags: vi.fn(async () => true),
        onCreateOrderTagDefinition: vi.fn(async () => true),
        onDeleteOrderTagDefinition: vi.fn(async () => true),
        onRefreshOrderTagDefinitions: vi.fn(async () => undefined),
      }),
    );

    expect(capturedProps).not.toBeNull();
    expect(capturedProps?.currentMarkValues).toEqual(['Urgent']);
    expect(capturedProps?.markLibraryItems).toEqual([
      { id: 'tag-vip', label: 'VIP', color: '#1677ff' },
      { id: 'tag-urgent', label: 'Urgent', color: '#722ed1' },
    ]);
  });
});
