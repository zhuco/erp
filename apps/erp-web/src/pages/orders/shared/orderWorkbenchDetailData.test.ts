import { describe, expect, it } from 'vitest';

import {
  resolveOrderWorkbenchDetailItems,
  resolveOrderWorkbenchFeeRows,
  resolveOrderWorkbenchPlatformOrderRows,
  resolveOrderWorkbenchShipmentRows,
} from './orderWorkbenchDetailData';

describe('orderWorkbenchDetailData', () => {
  it('reads order goods from the detail payload items collection', () => {
    const payload = {
      order: {
        id: 'ORDER-1',
        items: [{ id: 'nested-item', title: 'Nested item should not win' }],
      },
      items: [{ id: 'detail-item', title: 'Detail payload item' }],
    };

    expect(resolveOrderWorkbenchDetailItems(payload)).toEqual(payload.items);
  });

  it('builds a stable platform-order tab row from the detail payload', () => {
    const payload = {
      order: {
        id: 'ORDER-1',
        order_source: '抖店',
        order_type: '普通订单',
        platform_order_id: 'PLAT-1',
        platform_status: '待发货',
        total_amount_cent: 129900,
        order_created_at: '2026-04-20T10:11:12.000Z',
      },
    };

    expect(resolveOrderWorkbenchPlatformOrderRows(payload)).toEqual([
      {
        key: 'ORDER-1',
        orderSource: '抖店',
        orderType: '普通订单',
        orderNo: 'PLAT-1',
        orderStatus: '待发货',
        amountCent: 129900,
        time: '2026-04-20T10:11:12.000Z',
      },
    ]);
  });

  it('prefers queried shipment rows and falls back to order-level shipment fields', () => {
    const payload = {
      order: {
        id: 'ORDER-2',
        shipment_no: 'SHIP-FALLBACK',
        oms_status: 'PENDING_SHIPMENT',
        ship_warehouse_id: 'WH-DEFAULT',
        logistics_company: '顺丰速运',
        tracking_no: 'SF0001',
      },
    };

    expect(
      resolveOrderWorkbenchShipmentRows(payload, [
        {
          shipment_id: 'SHIP-1',
          shipment_no: 'SHIP-NO-1',
          status: 'SHIPPED',
          shipped_at: '2026-04-20T08:00:00.000Z',
          warehouse_id: 'WH-EAST',
          carrier_code: 'SF',
          tracking_no: 'SF123456',
        },
      ]),
    ).toEqual([
      {
        key: 'SHIP-1',
        shipmentNo: 'SHIP-NO-1',
        shipmentStatus: 'SHIPPED',
        shippedAt: '2026-04-20T08:00:00.000Z',
        warehouseId: 'WH-EAST',
        logisticsCompany: 'SF',
        trackingNo: 'SF123456',
      },
    ]);

    expect(resolveOrderWorkbenchShipmentRows(payload)).toEqual([
      {
        key: 'SHIP-FALLBACK',
        shipmentNo: 'SHIP-FALLBACK',
        shipmentStatus: 'PENDING_SHIPMENT',
        shippedAt: null,
        warehouseId: 'WH-DEFAULT',
        logisticsCompany: '顺丰速运',
        trackingNo: 'SF0001',
      },
    ]);
  });

  it('keeps a standard fee tab structure even when some amounts are not provided', () => {
    const payload = {
      order: {
        goods_amount_cent: 108000,
        postage_amount_cent: 1200,
        discount_amount_cent: 800,
        total_amount_cent: 108400,
      },
    };

    expect(resolveOrderWorkbenchFeeRows(payload)).toEqual([
      { key: 'platformCommission', feeName: '平台佣金', amountCent: null, remark: '待业务接口补充' },
      { key: 'estimatedPostage', feeName: '预估运费', amountCent: null, remark: '待业务接口补充' },
      { key: 'postage', feeName: '实际运费', amountCent: 1200, remark: null },
      { key: 'goodsAmount', feeName: '商品金额', amountCent: 108000, remark: null },
      { key: 'discountAmount', feeName: '优惠金额', amountCent: 800, remark: null },
      { key: 'totalAmount', feeName: '订单合计', amountCent: 108400, remark: null },
    ]);
  });
});
