import { describe, expect, it } from 'vitest';

import type { KitDetail } from '@/api/kits';
import type { Product } from '@/api/products';
import type { SkuDetailResponse } from '@/api/skus';

import {
  defaultGoodsKitFormValues,
  defaultGoodsProductFormValues,
} from './goodsFields';
import {
  mapKitDetailToFormValues,
  mapKitFormToBomPayload,
  mapKitFormToProductPayload,
  mapKitFormToSkuPayload,
  mapProductFormToCreatePayload,
  mapProductFormToUpdatePayload,
  mapProductToFormValues,
  type GoodsKitFormValues,
} from './goodsFormUtils';

describe('goods domain mappings', () => {
  const createWarningStock = (upper: number, lower: number) =>
    ({
      upper: { mode: 'FIXED', value: upper },
      lower: { mode: 'FIXED', value: lower },
    }) as unknown as Product['warning_stock'];

  it('maps product form values to create payload with normalized warning stock and text fields', () => {
    const payload = mapProductFormToCreatePayload({
      ...defaultGoodsProductFormValues,
      product_code: '  P-001  ',
      name: '  工业测试货品  ',
      alias: '  主推款  ',
      brand: '  品牌A  ',
      category: '  类目A  ',
      base_unit: '  个  ',
      second_unit: '  箱  ',
      default_supplier_id: '  SUP-1  ',
      default_warehouse_id: '  WH-1  ',
      primary_barcode: '  690000000001  ',
      product_tags: [' 常温 ', ' 爆品 ', ''],
      remark: '  支持混批  ',
      warning_upper: 180,
      warning_lower: 20,
      batch_enabled: true,
      shelf_life_days: 365,
      forbid_sale_days: 30,
      serial_enabled: true,
      serial_push_platform: true,
      allow_negative_stock: true,
    });

    expect(payload).toEqual({
      product_code: 'P-001',
      name: '工业测试货品',
      alias: '主推款',
      brand: '品牌A',
      category: '类目A',
      base_unit: '个',
      second_unit: '箱',
      default_supplier_id: 'SUP-1',
      default_warehouse_id: 'WH-1',
      primary_barcode: '690000000001',
      product_tags: ['常温', '爆品'],
      remark: '支持混批',
      warning_stock: {
        upper: { mode: 'FIXED', value: 180 },
        lower: { mode: 'FIXED', value: 20 },
      },
      batch_enabled: true,
      shelf_life_days: 365,
      forbid_sale_days: 30,
      serial_enabled: true,
      serial_push_platform: true,
      allow_negative_stock: true,
    });
  });

  it('maps product form values to update payload and carries status for edit mode', () => {
    const payload = mapProductFormToUpdatePayload({
      ...defaultGoodsProductFormValues,
      name: '货品升级版',
      warning_upper: undefined,
      warning_lower: 8,
      status: 'INACTIVE',
      remark: '',
    });

    expect(payload).toEqual({
      name: '货品升级版',
      alias: null,
      brand: null,
      category: null,
      base_unit: null,
      second_unit: null,
      default_supplier_id: null,
      default_warehouse_id: null,
      primary_barcode: null,
      product_tags: [],
      remark: null,
      warning_stock: {
        upper: { mode: 'UNLIMITED' },
        lower: { mode: 'FIXED', value: 8 },
      },
      batch_enabled: false,
      shelf_life_days: null,
      forbid_sale_days: null,
      serial_enabled: false,
      serial_push_platform: false,
      allow_negative_stock: false,
      status: 'INACTIVE',
    });
  });

  it('maps product detail back to form values for editor initialization', () => {
    const product: Product = {
      product_id: 'product-1',
      tenant_id: 'tenant-1',
      product_code: 'PRD-1',
      name: '货品一号',
      alias: '简名',
      brand: '品牌A',
      category: '类目A',
      base_unit: '件',
      second_unit: '箱',
      default_supplier_id: 'supplier-1',
      default_warehouse_id: 'warehouse-1',
      primary_barcode: '690001',
      product_tags: ['常温', '新品'],
      remark: '重点维护',
      warning_stock: createWarningStock(120, 16),
      batch_enabled: true,
      shelf_life_days: 180,
      forbid_sale_days: 15,
      serial_enabled: false,
      serial_push_platform: false,
      attrs: {},
      allow_negative_stock: true,
      status: 'ACTIVE',
      created_at: '2026-05-10T00:00:00.000Z',
      updated_at: '2026-05-10T00:00:00.000Z',
    };

    expect(mapProductToFormValues(product)).toEqual({
      ...defaultGoodsProductFormValues,
      product_code: 'PRD-1',
      name: '货品一号',
      alias: '简名',
      brand: '品牌A',
      category: '类目A',
      base_unit: '件',
      second_unit: '箱',
      default_supplier_id: 'supplier-1',
      default_warehouse_id: 'warehouse-1',
      primary_barcode: '690001',
      product_tags: ['常温', '新品'],
      remark: '重点维护',
      warning_upper: 120,
      warning_lower: 16,
      batch_enabled: true,
      shelf_life_days: 180,
      forbid_sale_days: 15,
      serial_enabled: false,
      serial_push_platform: false,
      allow_negative_stock: true,
      status: 'ACTIVE',
    });
  });

  it('maps goods kit form values to product payload, sku payload, and bom payload', () => {
    const formValues: GoodsKitFormValues = {
      ...defaultGoodsKitFormValues,
      product_code: 'KIT-PROD-01',
      name: '豪华组合装',
      alias: '组合A',
      brand: '品牌B',
      category: '套装',
      base_unit: '套',
      second_unit: '箱',
      default_supplier_id: 'SUP-KIT',
      default_warehouse_id: 'WH-KIT',
      primary_barcode: '690010',
      product_tags: ['组合', '活动'],
      remark: '主推节日礼盒',
      warning_upper: 50,
      warning_lower: 5,
      batch_enabled: true,
      shelf_life_days: 90,
      forbid_sale_days: 7,
      serial_enabled: false,
      serial_push_platform: false,
      allow_negative_stock: false,
      sku_code: 'KIT-SKU-01',
      sku_name: '豪华组合装-SKU',
      sku_barcode: '691010',
      sku_note: '用于活动投放',
      sku_tags: ['组合', '礼盒'],
      cost_cent: 29900,
      sku_status: 'INACTIVE',
      components: [
        {
          component_sku_id: 'sku-a',
          qty: 2,
          expire_at: '2026-12-31T00:00:00.000Z',
        },
        {
          component_sku_id: 'sku-b',
          qty: 1,
          expire_at: null,
        },
      ],
    };

    expect(mapKitFormToProductPayload(formValues)).toEqual({
      product_code: 'KIT-PROD-01',
      name: '豪华组合装',
      alias: '组合A',
      brand: '品牌B',
      category: '套装',
      base_unit: '套',
      second_unit: '箱',
      default_supplier_id: 'SUP-KIT',
      default_warehouse_id: 'WH-KIT',
      primary_barcode: '690010',
      product_tags: ['组合', '活动'],
      remark: '主推节日礼盒',
      warning_stock: {
        upper: { mode: 'FIXED', value: 50 },
        lower: { mode: 'FIXED', value: 5 },
      },
      batch_enabled: true,
      shelf_life_days: 90,
      forbid_sale_days: 7,
      serial_enabled: false,
      serial_push_platform: false,
      allow_negative_stock: false,
    });

    expect(
      mapKitFormToSkuPayload('product-1', formValues),
    ).toEqual({
      product_id: 'product-1',
      sku_code: 'KIT-SKU-01',
      barcode: '691010',
      name: '豪华组合装-SKU',
      note: '用于活动投放',
      tags: ['组合', '礼盒'],
      sku_kind: 'KIT',
      cost_cent: 29900,
      status: 'INACTIVE',
      batch_enabled: true,
      serial_enabled: false,
    });

    expect(mapKitFormToBomPayload('sku-1', formValues)).toEqual({
      kit_sku_id: 'sku-1',
      components: [
        {
          component_sku_id: 'sku-a',
          qty: 2,
          expire_at: '2026-12-31T00:00:00.000Z',
        },
        {
          component_sku_id: 'sku-b',
          qty: 1,
          expire_at: null,
        },
      ],
    });
  });

  it('maps kit detail and sku detail back to form values for edit mode', () => {
    const detail: KitDetail = {
      kit_sku: {
        sku_id: 'sku-kit-1',
        sku_code: 'KIT-01',
        name: '组合装一号',
        barcode: '900001',
        sku_kind: 'KIT',
      },
      components: [
        {
          component_sku_id: 'sku-comp-1',
          component_sku_code: 'COMP-1',
          component_name: '配件一',
          component_barcode: '800001',
          qty: 3,
          expire_at: '2026-06-01T00:00:00.000Z',
        },
      ],
      updated_at: '2026-05-10T00:00:00.000Z',
    };
    const skuDetail: SkuDetailResponse = {
      sku: {
        sku_id: 'sku-kit-1',
        tenant_id: 'tenant-1',
        product_id: 'product-1',
        sku_code: 'KIT-01',
        barcode: '900001',
        name: '组合装一号',
        note: '组合备注',
        spec: {},
        tags: ['组合', '大促'],
        weight_gram: null,
        length_mm: null,
        width_mm: null,
        height_mm: null,
        volume_cm3: null,
        sku_kind: 'KIT',
        cost_cent: 12800,
        is_active: true,
        status: 'ACTIVE',
        batch_enabled: true,
        serial_enabled: false,
        created_at: '2026-05-10T00:00:00.000Z',
        updated_at: '2026-05-10T00:00:00.000Z',
      },
      product: {
        product_id: 'product-1',
        tenant_id: 'tenant-1',
        product_code: 'PROD-KIT-01',
        name: '组合装主货品',
        alias: '礼盒装',
        brand: '品牌K',
        category: '组合装',
        base_unit: '套',
        second_unit: '箱',
        default_supplier_id: 'supplier-kit',
        default_warehouse_id: 'warehouse-kit',
        primary_barcode: '700001',
        product_tags: ['组合'],
        remark: '用于测试',
        warning_stock: createWarningStock(22, 6),
        batch_enabled: true,
        shelf_life_days: 120,
        forbid_sale_days: 12,
        serial_enabled: false,
        serial_push_platform: false,
        attrs: {},
        allow_negative_stock: false,
        status: 'ACTIVE',
        created_at: '2026-05-10T00:00:00.000Z',
        updated_at: '2026-05-10T00:00:00.000Z',
      },
      suppliers: [],
    };

    expect(mapKitDetailToFormValues(detail, skuDetail)).toEqual({
      ...defaultGoodsKitFormValues,
      product_code: 'PROD-KIT-01',
      name: '组合装主货品',
      alias: '礼盒装',
      brand: '品牌K',
      category: '组合装',
      base_unit: '套',
      second_unit: '箱',
      default_supplier_id: 'supplier-kit',
      default_warehouse_id: 'warehouse-kit',
      primary_barcode: '700001',
      product_tags: ['组合'],
      remark: '用于测试',
      warning_upper: 22,
      warning_lower: 6,
      batch_enabled: true,
      shelf_life_days: 120,
      forbid_sale_days: 12,
      serial_enabled: false,
      serial_push_platform: false,
      allow_negative_stock: false,
      status: 'ACTIVE',
      sku_code: 'KIT-01',
      sku_name: '组合装一号',
      sku_barcode: '900001',
      sku_note: '组合备注',
      sku_tags: ['组合', '大促'],
      cost_cent: 12800,
      sku_status: 'ACTIVE',
      components: [
        {
          component_sku_id: 'sku-comp-1',
          qty: 3,
          expire_at: '2026-06-01T00:00:00.000Z',
        },
      ],
    });
  });
});
