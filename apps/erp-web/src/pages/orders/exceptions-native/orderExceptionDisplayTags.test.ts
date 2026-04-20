import { describe, expect, it } from 'vitest';

import type { OrderExceptionDetail, OrderExceptionSummary } from '@/api/orders';

import {
  extractOrderExceptionExplicitTags,
  resolveOrderExceptionDetailPackage,
  resolveOrderExceptionDisplayTags,
} from './orderExceptionDisplayTags';

const buildSummary = (
  overrides: Partial<OrderExceptionSummary> = {},
): OrderExceptionSummary => ({
  order_id: 'EX-1001',
  order_no: 'HC202604190001',
  platform: 'taobao',
  shop_id: 'SHOP-taobao-1',
  receiver_name: '收件人1',
  total_amount_cent: 1999,
  audit_status: 'UNDER_REVIEW',
  exception_status: 'OPEN',
  fulfillment_status: 'NOT_SUBMITTED',
  reserve_status: null,
  target_warehouse_id: 'WH-DEFAULT',
  blocking_reason: '商品映射缺失',
  assignee_id: null,
  assignee_name: null,
  latest_action: '系统命中商品映射缺失规则，进入异常池',
  latest_action_at: '2026-04-19T12:00:00.000Z',
  ...overrides,
});

const buildDetail = (
  overrides: Partial<OrderExceptionDetail> = {},
): OrderExceptionDetail =>
  ({
    ...buildSummary(),
    blocking_reasons: ['商品映射缺失'],
    action_logs: [],
    order: {
      order: {
        id: 'EX-1001',
        presale: true,
        freeze_reason: '风控自动冻结',
        refund_state: 'PARTIAL_REFUND',
        merge_group_id: 'MERGE-1001',
        split_source_order_id: 'PARENT-1001',
        platform_ship_deadline_at: '2026-04-18T12:00:00.000Z',
      },
      items: [],
      tags: [{ tag: '加急' }, { tag: '人工复核' }, { tag: '加急' }],
    },
    ...overrides,
  }) as OrderExceptionDetail;

describe('orderExceptionDisplayTags', () => {
  it('supports nested detail packages returned by the exception detail API', () => {
    const detailPackage = resolveOrderExceptionDetailPackage(buildDetail());

    expect(detailPackage?.order).toMatchObject({
      id: 'EX-1001',
      presale: true,
      freeze_reason: '风控自动冻结',
    });
    expect(detailPackage?.items).toEqual([]);
    expect(detailPackage?.tags).toEqual([
      { tag: '加急' },
      { tag: '人工复核' },
      { tag: '加急' },
    ]);
  });

  it('extracts explicit order tags with de-duplication', () => {
    expect(extractOrderExceptionExplicitTags(buildDetail())).toEqual(['加急', '人工复核']);
  });

  it('resolves the same rich tag set for exception rows once detail data is available', () => {
    const summary = buildSummary();
    const detail = buildDetail();

    const resolved = resolveOrderExceptionDisplayTags(summary, detail, {
      now: new Date('2026-04-19T12:00:00.000Z'),
    }).map((tag) => (typeof tag === 'string' ? tag : tag.key));

    expect(resolved).toEqual([
      'unmatched',
      'frozen',
      'presale',
      'timeout',
      'refund',
      'merged',
      'split',
      '加急',
      '人工复核',
    ]);
  });
});
