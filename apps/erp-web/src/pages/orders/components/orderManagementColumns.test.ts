import { isValidElement } from 'react';
import { describe, expect, it } from 'vitest';

import type { OrderSummary } from '@/api/orders';
import { OrderTagBadges } from '@/components/OrderTagBadges';

import { buildOrderManagementColumns } from './orderManagementColumns';

describe('buildOrderManagementColumns', () => {
  it('renders standardized builtin mark badges in the exceptions column', () => {
    const columns = buildOrderManagementColumns({
      rowOffset: 0,
      orderStatusMeta: {},
      formatCurrency: () => '楼0.00',
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
