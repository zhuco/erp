import { describe, expect, it } from 'vitest';

import {
  buildOrderEditableMarkMenuItems,
  builtinOrderTags,
  resolveOrderDisplayTags,
  resolveBuiltinOrderTag,
  resolveOrderBuiltinTags,
} from './orderTags';

describe('orderTags builtin standard', () => {
  it('defines the standardized builtin order marks with short labels and white-text backgrounds', () => {
    expect(
      builtinOrderTags.map((tag) => ({
        key: tag.key,
        code: tag.code,
        name: tag.name,
      })),
    ).toEqual([
      { key: 'unmatched', code: '匹', name: '商品未匹配' },
      { key: 'shortage', code: '缺', name: '缺货' },
      { key: 'frozen', code: '冻', name: '冻结' },
      { key: 'presale', code: '预', name: '预售' },
      { key: 'timeout', code: '超时', name: '超时' },
      { key: 'refund', code: '退', name: '退款' },
      { key: 'merged', code: '合', name: '合并' },
      { key: 'split', code: '拆', name: '拆分' },
    ]);

    expect(builtinOrderTags.every((tag) => tag.textColor === '#ffffff')).toBe(true);
  });

  it('resolves builtin aliases and extracts standardized marks from order records', () => {
    expect(resolveBuiltinOrderTag('匹')?.key).toBe('unmatched');
    expect(resolveBuiltinOrderTag('缺货')?.key).toBe('shortage');
    expect(resolveBuiltinOrderTag('ORDER_SPLIT')?.key).toBe('split');
    expect(resolveBuiltinOrderTag('订单合并')?.key).toBe('merged');

    const tags = resolveOrderBuiltinTags({
      mapping_missing: true,
      inventory_shortage: true,
      freeze_reason: '高风险自动冻结',
      tags: ['预售'],
      exception_flags: ['REFUND', 'ORDER_MERGED', 'ORDER_SPLIT'],
      platform_ship_deadline_at: '2026-04-18T00:00:00.000Z',
    });

    expect(tags.map((tag) => tag.key)).toEqual([
      'unmatched',
      'shortage',
      'frozen',
      'presale',
      'timeout',
      'refund',
      'merged',
      'split',
    ]);
  });

  it('derives standardized marks from exception summaries and merges custom tags without builtin duplicates', () => {
    const tags = resolveOrderDisplayTags({
      blocking_reason: '商品映射缺失，订单已冻结',
      latest_action: '退款申请待确认',
      tags: ['预售', '人工加急'],
    });

    expect(
      tags.map((tag) => (typeof tag === 'string' ? tag : tag.key)),
    ).toEqual(['unmatched', 'frozen', 'presale', 'refund', '人工加急']);
  });
});

describe('buildOrderEditableMarkMenuItems', () => {
  it('keeps only custom marks that are not already attached to the current order', () => {
    const items = buildOrderEditableMarkMenuItems(
      [
        { label: 'VIP', value: 'VIP' },
        { label: 'Shortage', value: 'SHORTAGE' },
        { label: 'Urgent', value: 'Urgent' },
        { label: 'Urgent', value: 'Urgent' },
      ],
      ['Urgent'],
    );

    expect(items).toEqual([{ key: 'VIP', label: 'VIP' }]);
  });

  it('returns an empty-state menu item when there are no editable custom marks left', () => {
    const items = buildOrderEditableMarkMenuItems([{ label: 'Split', value: 'ORDER_SPLIT' }], ['VIP']);

    expect(items).toEqual([{ key: '__empty__', label: '暂无可选记号', disabled: true }]);
  });
});
