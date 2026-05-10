import { isValidElement } from 'react';
import { describe, expect, it } from 'vitest';

import type { OrderSummary } from '@/api/orders';
import { OrderTagBadges } from '@/components/OrderTagBadges';

import { buildOrderManagementColumns } from './orderManagementColumns';

describe('buildOrderManagementColumns', () => {
  it('uses readable Chinese titles for the main order query columns', () => {
    const columns = buildOrderManagementColumns({
      rowOffset: 0,
      orderStatusMeta: {},
      formatCurrency: () => '¥0.00',
      formatDateTime: () => '-',
      resolveOrderNo: () => '-',
      resolveShopName: () => '-',
      resolveWarehouseName: () => '-',
      resolveItemSummary: () => '-',
      resolveItemQuantity: () => '-',
      resolveReceiverAddress: () => '-',
      resolveRemark: () => '-',
      resolveOrderSource: () => '-',
      resolveLogisticsCompany: () => '-',
      resolveTags: () => '-',
    });

    const titleByKey = new Map(
      columns.map((column) => [
        String(column.key ?? (Array.isArray(column.dataIndex) ? column.dataIndex.join('.') : column.dataIndex ?? '')),
        column.title,
      ]),
    );

    expect(titleByKey.get('tags')).toBe('记号');
    expect(titleByKey.get('id')).toBe('订单编号');
    expect(titleByKey.get('shop_id')).toBe('销售渠道');
    expect(titleByKey.get('platform')).toBe('销售平台');
    expect(titleByKey.get('created_at')).toBe('付款时间');
    expect(titleByKey.get('receiver_address')).toBe('收货地址');
    expect(titleByKey.get('logistics_company')).toBe('物流公司');
    expect(titleByKey.get('platform_order_id')).toBe('网店订单号');
  });

  it('renders standardized builtin mark badges in the exceptions column', () => {
    const columns = buildOrderManagementColumns({
      rowOffset: 0,
      orderStatusMeta: {},
      formatCurrency: () => '¥0.00',
      formatDateTime: () => '-',
      resolveOrderNo: () => '-',
      resolveShopName: () => '-',
      resolveWarehouseName: () => '-',
      resolveItemSummary: () => '-',
      resolveItemQuantity: () => '-',
      resolveReceiverAddress: () => '-',
      resolveRemark: () => '-',
      resolveOrderSource: () => '-',
      resolveLogisticsCompany: () => '-',
      resolveTags: () => '-',
      includeExceptions: true,
    });

    const exceptionColumn = columns.find((column) => column.key === 'exceptions');
    expect(exceptionColumn).toBeDefined();

    const element = exceptionColumn?.render?.(
      undefined,
      {
        id: 'ORDER-1',
        platform: 'taobao',
        shop_id: 'SHOP-1',
        oms_status: 'REVIEW_PENDING',
        review_status: 'PENDING',
        total_amount_cent: 1000,
        created_at: '2026-04-19T00:00:00.000Z',
        updated_at: '2026-04-19T00:00:00.000Z',
        exception_flags: ['ORDER_FROZEN', 'ORDER_MERGED'],
        mapping_missing: true,
      } as unknown as OrderSummary,
      0,
    );

    expect(isValidElement(element)).toBe(true);
    expect(element?.type).toBe(OrderTagBadges);
    expect(
      (element?.props.tags as Array<{ key: string }>).map((tag) => tag.key),
    ).toEqual(['unmatched', 'frozen', 'merged']);
  });
});
