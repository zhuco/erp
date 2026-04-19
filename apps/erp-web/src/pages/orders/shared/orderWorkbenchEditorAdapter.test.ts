import { describe, expect, it } from 'vitest';

import {
  buildWorkbenchOrderRecordDraft,
  buildWorkbenchOrderUpdatePayload,
  validateWorkbenchOrderRecordDraft,
} from './orderWorkbenchEditorAdapter';

describe('orderWorkbenchEditorAdapter', () => {
  it('builds an editable draft from detail data', () => {
    const draft = buildWorkbenchOrderRecordDraft(
      {
        order: {
          id: 'HC202604190001',
          platform_order_id: 'TB-001',
          platform: 'taobao',
          shop_id: 'SHOP-taobao-1',
          receiver_name: '张三',
          receiver_phone: '13800001111',
          ship_warehouse_id: 'WH-001',
          logistics_company: '顺丰',
          buyer_message: '尽快发货',
          seller_remark: '优先处理',
        },
        items: [
          {
            id: 'LINE-1',
            platform_sku_id: 'SKU-1',
            outer_sku_id: 'OUTER-1',
            title: '工业胶带',
            quantity: 3,
            price_cent: 1200,
          },
        ],
      },
      'HC202604190001',
    );

    expect(draft).toMatchObject({
      id: 'HC202604190001',
      orderNo: 'HC202604190001',
      platformOrderId: 'TB-001',
      receiverName: '张三',
      receiverPhone: '13800001111',
      shipWarehouseId: 'WH-001',
      logisticsCompany: '顺丰',
    });
    expect(draft.items).toHaveLength(1);
    expect(draft.items[0]).toMatchObject({
      id: 'LINE-1',
      platformSkuId: 'SKU-1',
      outerSkuId: 'OUTER-1',
      title: '工业胶带',
      quantity: 3,
      priceCent: 1200,
    });
  });

  it('validates required record fields and line items', () => {
    const messages = validateWorkbenchOrderRecordDraft({
      id: 'HC202604190002',
      orderNo: 'HC202604190002',
      receiverName: '  ',
      receiverPhone: '',
      items: [
        {
          draftKey: 'line-1',
          quantity: 0,
          priceCent: -1,
          title: '  ',
        },
      ],
    });

    expect(messages).toEqual([
      '收件人不能为空',
      '手机号不能为空',
      '第 1 行货品标题不能为空',
      '第 1 行货品数量必须大于 0',
      '第 1 行货品单价不能小于 0',
    ]);
  });

  it('requires at least one goods line before save', () => {
    const messages = validateWorkbenchOrderRecordDraft({
      id: 'HC202604190003',
      orderNo: 'HC202604190003',
      receiverName: '李四',
      receiverPhone: '13900002222',
      items: [],
    });

    expect(messages).toEqual(['至少保留 1 行货品明细']);
  });

  it('builds a normalized update payload from the draft', () => {
    const payload = buildWorkbenchOrderUpdatePayload({
      id: 'HC202604190004',
      orderNo: 'HC202604190004',
      receiverName: ' 王五 ',
      receiverPhone: ' 13700003333 ',
      shipWarehouseId: ' WH-002 ',
      logisticsCompany: ' 中通 ',
      buyerMessage: ' 轻拿轻放 ',
      sellerRemark: ' 加急 ',
      items: [
        {
          draftKey: 'line-1',
          id: 'LINE-1',
          platformSkuId: ' SKU-9 ',
          outerSkuId: ' OUT-9 ',
          title: ' 纸箱 ',
          quantity: 2,
          priceCent: 560,
        },
      ],
    });

    expect(payload).toEqual({
      receiver_name: '王五',
      receiver_phone: '13700003333',
      ship_warehouse_id: 'WH-002',
      logistics_company: '中通',
      buyer_message: '轻拿轻放',
      seller_remark: '加急',
      items: [
        {
          line_id: 'LINE-1',
          sku_id: 'SKU-9',
          title: '纸箱',
          qty: 2,
          price_cent: 560,
        },
      ],
    });
  });
});
