import { createElement } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';

import type { ShopProductMatchFailure, ShopProductSku, ShopProductTask } from '@/api/shopProducts';
import { Platform } from '@/features/platform';

import { ShopProductWorkbenchBottomPanel } from './ShopProductWorkbenchBottomPanel';
import { ShopProductWorkbenchDetailPanel } from './ShopProductWorkbenchDetailPanel';
import { ShopProductWorkbenchLeftPanel } from './ShopProductWorkbenchLeftPanel';

const activeSku: ShopProductSku = {
  sku_record_id: 'shop-sku-2',
  tenant_id: 'TENANT-1',
  shop_id: 'SHOP-1',
  platform: Platform.Taobao,
  platform_product_id: 'product-2',
  platform_sku_id: 'TB-SKU-002',
  merchant_sku_code: 'TB-001-BLUE',
  product_title: 'Spring hoodie',
  sku_title: 'Blue M',
  spec_text: 'Blue M',
  price_cent: 19900,
  stock_qty: 30,
  status: 'ACTIVE',
  mapping_id: 'map-1',
  mapped_sku_id: 'sku-1',
  mapped_sku_code: 'SKU-0001',
  mapped_sku_name: 'Hoodie Blue M',
  match_source: 'AUTO',
  mapping_status: 'ACTIVE',
  platform_updated_at: '2026-04-18T00:00:00.000Z',
  created_at: '2026-04-18T00:00:00.000Z',
  updated_at: '2026-04-18T00:00:00.000Z',
};

const tasks: ShopProductTask[] = [
  {
    task_id: 'task-1',
    tenant_id: 'TENANT-1',
    task_type: 'AUTO_MATCH',
    status: 'SUCCESS',
    total_count: 2,
    success_count: 2,
    failure_count: 0,
    error: '',
    payload_json: {},
    result_json: {},
    started_at: '2026-04-18T00:00:00.000Z',
    finished_at: '2026-04-18T00:01:00.000Z',
    created_at: '2026-04-18T00:00:00.000Z',
    updated_at: '2026-04-18T00:01:00.000Z',
  },
];

const failures: ShopProductMatchFailure[] = [
  {
    sku_record_id: 'shop-sku-1',
    platform_sku_id: 'TB-SKU-001',
    merchant_sku_code: 'TB-001-RED',
    reason: 'COLOR_DICT_MISS',
  },
];

describe('ShopProductWorkbenchPanels', () => {
  it('renders the left panel with the order review template structure', () => {
    const markup = renderToStaticMarkup(
      createElement(ShopProductWorkbenchLeftPanel, {
        filterContent: createElement('div', { 'data-testid': 'shop-products-filter-content' }, 'filter form'),
        summary: {
          total: 42,
          matched: 16,
          unmatched: 20,
          conflict: 6,
          selected: 3,
        },
        apparelTemplateName: 'Apparel template',
      }),
    );

    expect(markup).toContain('data-testid="shop-products-left-panel"');
    expect(markup).toContain('order-review-left-ag-panel');
    expect(markup).toContain('order-review-left-ag-grid-shell');
    expect(markup).toContain('order-review-left-ag-stats');
    expect(markup).toContain('data-testid="order-review-left-stats"');
    expect(markup).toContain('Apparel template');
    expect(markup).toContain('42');
    expect(markup).toContain('3');
    expect(markup).toContain('过滤');
    expect(markup).toContain('筛选');
  });

  it('renders the detail panel with the order review right-panel template shell', () => {
    const markup = renderToStaticMarkup(
      createElement(ShopProductWorkbenchDetailPanel, {
        activeSku,
        shopName: 'Taobao flagship shop',
      }),
    );

    expect(markup).toContain('data-testid="shop-products-detail-panel"');
    expect(markup).toContain('data-testid="shop-products-detail-template"');
    expect(markup).toContain('ui-module-05-order-review-right-panel');
    expect(markup).toContain('ui-module-05-order-mark-inline');
    expect(markup).toContain('TB-SKU-002');
    expect(markup).toContain('SKU-0001');
    expect(markup).toContain('Taobao flagship shop');
    expect(markup).toContain('AUTO');
  });

  it('renders the bottom panel with the order review bottom shell and product tabs', () => {
    const markup = renderToStaticMarkup(
      createElement(ShopProductWorkbenchBottomPanel, {
        tasks,
        failures,
        selectedCount: 2,
        createScope: 'filtered',
      }),
    );

    expect(markup).toContain('data-testid="shop-products-bottom-panel"');
    expect(markup).toContain('order-review-bottom-table');
    expect(markup).toContain('order-workbench-bottom-panel');
    expect(markup).toContain('data-testid="shop-products-task-center"');
    expect(markup).toContain('data-testid="shop-products-failure-center"');
    expect(markup).toContain('data-testid="shop-products-batch-context"');
    expect(markup).toContain('AUTO_MATCH');
    expect(markup).toContain('COLOR_DICT_MISS');
    expect(markup).toContain('2');
  });
});
