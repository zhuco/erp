import { describe, expect, it } from 'vitest';

import type { ShopProductSku } from '@/api/shopProducts';
import { Platform } from '@/features/platform';

import {
  resolveActiveShopProductSkuRecordId,
  summarizeShopProductRows,
} from './shopProductWorkbenchModel';

const rows: ShopProductSku[] = [
  {
    sku_record_id: 'shop-sku-1',
    tenant_id: 'TENANT-1',
    shop_id: 'SHOP-1',
    platform: Platform.Taobao,
    platform_product_id: 'product-1',
    platform_sku_id: 'TB-SKU-001',
    product_title: '春季卫衣',
    status: 'ACTIVE',
    mapping_status: 'CONFLICT',
    created_at: '2026-04-18T00:00:00.000Z',
    updated_at: '2026-04-18T00:00:00.000Z',
  },
  {
    sku_record_id: 'shop-sku-2',
    tenant_id: 'TENANT-1',
    shop_id: 'SHOP-1',
    platform: Platform.Taobao,
    platform_product_id: 'product-2',
    platform_sku_id: 'TB-SKU-002',
    product_title: '春季卫衣',
    status: 'ACTIVE',
    mapping_id: 'map-1',
    mapped_sku_id: 'sku-1',
    mapped_sku_code: 'SKU-0001',
    match_source: 'AUTO',
    mapping_status: 'ACTIVE',
    created_at: '2026-04-18T00:00:00.000Z',
    updated_at: '2026-04-18T00:00:00.000Z',
  },
  {
    sku_record_id: 'shop-sku-3',
    tenant_id: 'TENANT-1',
    shop_id: 'SHOP-2',
    platform: Platform.Jd,
    platform_product_id: 'product-3',
    platform_sku_id: 'JD-SKU-003',
    product_title: '轻薄外套',
    status: 'ACTIVE',
    created_at: '2026-04-18T00:00:00.000Z',
    updated_at: '2026-04-18T00:00:00.000Z',
  },
];

describe('shopProductWorkbenchModel', () => {
  it('keeps the current active sku when it still exists on the page', () => {
    expect(resolveActiveShopProductSkuRecordId(rows, 'shop-sku-2')).toBe('shop-sku-2');
  });

  it('falls back to the first row when the current active sku disappears', () => {
    expect(resolveActiveShopProductSkuRecordId(rows.slice(1), 'shop-sku-1')).toBe('shop-sku-2');
  });

  it('clears the active sku when the current page is empty', () => {
    expect(resolveActiveShopProductSkuRecordId([], 'shop-sku-2')).toBeNull();
  });

  it('summarizes matched, unmatched, conflict, and selected counts for the overview panel', () => {
    expect(
      summarizeShopProductRows({
        rows,
        total: 42,
        selectedCount: 2,
      }),
    ).toEqual({
      total: 42,
      matched: 1,
      unmatched: 1,
      conflict: 1,
      selected: 2,
    });
  });
});
