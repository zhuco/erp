import { describe, expect, it } from 'vitest';

import type { Product } from '@/api/products';
import {
  buildApparelSpecs,
  getApparelPolicySummary,
  getDigitalPolicySummary,
  getFoodPolicySummary,
  type ProductSpecForm,
} from './productIndustry';

const buildProduct = (overrides: Partial<Product>): Product =>
  ({
    product_id: 'PROD-TEST',
    tenant_id: 'TENANT-TEST',
    name: '测试商品',
    status: 'ACTIVE',
    warning_stock: {},
    product_tags: [],
    batch_enabled: false,
    shelf_life_days: null,
    forbid_sale_days: null,
    serial_enabled: false,
    serial_push_platform: false,
    attrs: {},
    created_at: '2026-04-15T00:00:00.000Z',
    updated_at: '2026-04-15T00:00:00.000Z',
    ...overrides,
  }) as Product;

describe('productIndustry', () => {
  it('合并鞋服规格时会保留已有扩展字段并跳过重复颜色尺码组合', () => {
    const existingSpecs: ProductSpecForm[] = [
      {
        spec_code: 'SKU-BLK-41',
        spec_name: '经典黑 / 41',
        color: '黑色',
        size: '41',
        barcode: '690000000041',
        cost: 199,
        unit: '双',
        remark: '保留原规格扩展字段',
      },
    ];

    const nextSpecs = buildApparelSpecs('黑色, 白色', '41, 42', existingSpecs);

    expect(nextSpecs).toHaveLength(4);
    expect(nextSpecs[0]).toMatchObject({
      spec_code: 'SKU-BLK-41',
      spec_name: '经典黑 / 41',
      color: '黑色',
      size: '41',
      barcode: '690000000041',
      cost: 199,
      unit: '双',
      remark: '保留原规格扩展字段',
    });
    expect(nextSpecs).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ spec_name: '黑色 / 42', color: '黑色', size: '42' }),
        expect.objectContaining({ spec_name: '白色 / 41', color: '白色', size: '41' }),
        expect.objectContaining({ spec_name: '白色 / 42', color: '白色', size: '42' }),
      ]),
    );
  });

  it('生成食品策略摘要时会突出生产日期、临期预警与禁售提前', () => {
    const summary = getFoodPolicySummary(
      buildProduct({
        batch_enabled: true,
        shelf_life_days: 180,
        forbid_sale_days: 7,
        attrs: {
          industry_profile: 'FOOD',
          production_date_enabled: true,
          shelf_life_unit: 'DAY',
          near_expiry_warning_days: 15,
        },
      }),
    );

    expect(summary).toContain('批次管理已启用');
    expect(summary).toContain('生产日期必填');
    expect(summary).toContain('保质期 180天');
    expect(summary).toContain('临期预警 15 天');
    expect(summary).toContain('禁售提前 7 天');
  });

  it('生成鞋服策略摘要时会突出颜色数、尺码数与矩阵规格数', () => {
    const summary = getApparelPolicySummary(
      buildProduct({
        attrs: {
          industry_profile: 'APPAREL',
          apparel_size_system: 'EU',
          specs: [
            { spec_name: '黑色 / 41', color: '黑色', size: '41' },
            { spec_name: '黑色 / 42', color: '黑色', size: '42' },
            { spec_name: '白色 / 41', color: '白色', size: '41' },
            { spec_name: '白色 / 42', color: '白色', size: '42' },
          ],
        },
      }),
    );

    expect(summary).toContain('尺码体系 欧码');
    expect(summary).toContain('颜色 2 个');
    expect(summary).toContain('尺码 2 个');
    expect(summary).toContain('规格 4 条');
  });

  it('生成数码策略摘要时会突出追踪模式与出入库校验规则', () => {
    const summary = getDigitalPolicySummary(
      buildProduct({
        serial_enabled: true,
        serial_push_platform: true,
        attrs: {
          industry_profile: 'DIGITAL',
          serial_tracking_mode: 'DEVICE',
          serial_outbound_required: true,
          serial_inbound_unique_required: true,
        },
      }),
    );

    expect(summary).toContain('唯一码管理已启用');
    expect(summary).toContain('追踪模式 设备追踪');
    expect(summary).toContain('出库需绑定序列号');
    expect(summary).toContain('入库唯一校验');
    expect(summary).toContain('平台回传已启用');
  });
});
