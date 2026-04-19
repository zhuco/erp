export type BuiltinOrderTagKey =
  | 'unmatched'
  | 'shortage'
  | 'frozen'
  | 'presale'
  | 'timeout'
  | 'refund'
  | 'merged'
  | 'split';

export type BuiltinOrderTag = {
  key: BuiltinOrderTagKey;
  code: string;
  name: string;
  description: string;
  backgroundColor: string;
  textColor: string;
  codeClassName?: string;
  aliases?: string[];
};

export type OrderDisplayTag = string | BuiltinOrderTag;

const normalizeLookupValue = (value?: string | null) => String(value ?? '').trim().toUpperCase();

const readStringField = (record: Record<string, unknown>, keys: string[]) => {
  for (const key of keys) {
    const rawValue = record[key];
    if (rawValue == null) {
      continue;
    }
    const value = String(rawValue).trim();
    if (value) {
      return value;
    }
  }
  return '';
};

const readStringListField = (record: Record<string, unknown>, keys: string[]) => {
  for (const key of keys) {
    const rawValue = record[key];
    if (Array.isArray(rawValue)) {
      return rawValue.map((item) => String(item ?? '').trim()).filter(Boolean);
    }
    if (typeof rawValue === 'string') {
      return rawValue
        .split(',')
        .map((item) => item.trim())
        .filter(Boolean);
    }
  }
  return [];
};

const readBooleanField = (record: Record<string, unknown>, keys: string[]) => {
  for (const key of keys) {
    if (typeof record[key] === 'boolean') {
      return record[key] as boolean;
    }
  }
  return false;
};

const readNumberField = (record: Record<string, unknown>, keys: string[]) => {
  for (const key of keys) {
    const rawValue = record[key];
    if (typeof rawValue === 'number' && Number.isFinite(rawValue)) {
      return rawValue;
    }
    if (typeof rawValue === 'string') {
      const parsedValue = Number(rawValue);
      if (Number.isFinite(parsedValue)) {
        return parsedValue;
      }
    }
  }
  return null;
};

const hasNormalizedKeyword = (values: string[], keywords: string[]) => {
  const normalizedKeywords = keywords.map((keyword) => normalizeLookupValue(keyword));
  return values.some((value) => {
    const normalizedValue = normalizeLookupValue(value);
    return normalizedKeywords.some((keyword) => normalizedValue.includes(keyword));
  });
};

const isPastDateField = (record: Record<string, unknown>, keys: string[], now: Date) => {
  const rawValue = readStringField(record, keys);
  if (!rawValue) {
    return false;
  }
  const timestamp = Date.parse(rawValue);
  if (!Number.isFinite(timestamp)) {
    return false;
  }
  return timestamp < now.getTime();
};

export const builtinOrderTags: BuiltinOrderTag[] = [
  {
    key: 'unmatched',
    code: '匹',
    name: '商品未匹配',
    description: '代表订单货品仍未完成系统匹配，需要人工处理。',
    backgroundColor: '#1677ff',
    textColor: '#ffffff',
    aliases: [
      '匹',
      '商品未匹配',
      '商品映射缺失',
      '未匹配',
      'MAPPING_MISSING',
      'INVALID_GOODS',
      'INVALID_ITEM',
      'MISSING_SKU',
    ],
  },
  {
    key: 'shortage',
    code: '缺',
    name: '缺货',
    description: '代表订单全部缺货或部分缺货。',
    backgroundColor: '#fa541c',
    textColor: '#ffffff',
    aliases: ['缺', '缺货', '部分缺货', '库存不足', 'INVENTORY_SHORTAGE', 'SHORTAGE', 'OUT_OF_STOCK'],
  },
  {
    key: 'frozen',
    code: '冻',
    name: '冻结',
    description: '代表订单被自动策略或人工冻结。',
    backgroundColor: '#722ed1',
    textColor: '#ffffff',
    aliases: ['冻', '冻结', 'ORDER_FROZEN', 'FROZEN', 'FREEZE'],
  },
  {
    key: 'presale',
    code: '预',
    name: '预售',
    description: '代表人工标记预售货品，或该单为预售订单。',
    backgroundColor: '#d46b08',
    textColor: '#ffffff',
    aliases: ['预', '预售', '预售订单', 'PRESALE', 'PRE_SALE', 'PREORDER', 'PRE_ORDER'],
  },
  {
    key: 'timeout',
    code: '超时',
    name: '超时',
    description: '代表已超过平台要求的发货时限。',
    backgroundColor: '#cf1322',
    textColor: '#ffffff',
    aliases: ['超时', '已超时', 'OVERDUE', 'TIMEOUT'],
  },
  {
    key: 'refund',
    code: '退',
    name: '退款',
    description: '代表平台存在退款或部分退款。',
    backgroundColor: '#13a8a8',
    textColor: '#ffffff',
    aliases: ['退', '退款', '部分退款', '退货', 'REFUND', 'RETURN', 'AFTERSALE', 'AFTER_SALE'],
  },
  {
    key: 'merged',
    code: '合',
    name: '合并',
    description: '代表该单由多个订单合并而成。',
    backgroundColor: '#0958d9',
    textColor: '#ffffff',
    aliases: ['合', '合并', '订单合并', 'ORDER_MERGED', 'MERGED', 'MERGE'],
  },
  {
    key: 'split',
    code: '拆',
    name: '拆分',
    description: '代表该单已被拆分，或本身是拆分后子单。',
    backgroundColor: '#389e0d',
    textColor: '#ffffff',
    aliases: ['拆', '拆分', '订单拆分', 'ORDER_SPLIT', 'SPLIT'],
  },
];

const builtinOrderTagLookup = new Map<string, BuiltinOrderTag>();

builtinOrderTags.forEach((tag) => {
  [tag.key, tag.code, tag.name, ...(tag.aliases ?? [])].forEach((value) => {
    const normalizedValue = normalizeLookupValue(value);
    if (!normalizedValue || builtinOrderTagLookup.has(normalizedValue)) {
      return;
    }
    builtinOrderTagLookup.set(normalizedValue, tag);
  });
});

export const resolveBuiltinOrderTag = (value?: string | null) => {
  const normalizedValue = normalizeLookupValue(value);
  if (!normalizedValue) {
    return undefined;
  }
  return builtinOrderTagLookup.get(normalizedValue);
};

export const resolveOrderBuiltinTags = (
  rawRecord: Record<string, unknown>,
  options?: {
    now?: Date;
  },
) => {
  const record = rawRecord ?? {};
  const now = options?.now ?? new Date();
  const tags = readStringListField(record, ['tags', 'tag_list', 'marks']);
  const exceptionFlags = readStringListField(record, ['exception_flags']);
  const textSignals = [
    readStringField(record, ['blocking_reason', 'blockingReason']),
    readStringField(record, ['latest_action', 'latestAction']),
    readStringField(record, ['exception_reason', 'exceptionReason']),
    readStringField(record, ['freeze_reason', 'freezeReason']),
    readStringField(record, ['refund_state', 'after_sale_type', 'afterSaleType']),
  ].filter(Boolean);
  const combinedLabels = [...tags, ...exceptionFlags, ...textSignals];

  const tagState = {
    unmatched:
      readBooleanField(record, ['mapping_missing']) ||
      hasNormalizedKeyword(combinedLabels, [
        'MAPPING_MISSING',
        'INVALID_GOODS',
        'INVALID_ITEM',
        'MISSING_SKU',
        '商品未匹配',
        '商品映射缺失',
        '未匹配',
      ]),
    shortage:
      readBooleanField(record, ['inventory_shortage', 'partial_shortage']) ||
      (readNumberField(record, ['shortage_qty', 'out_of_stock_qty']) ?? 0) > 0 ||
      hasNormalizedKeyword(combinedLabels, ['INVENTORY_SHORTAGE', 'SHORTAGE', 'OUT_OF_STOCK', '缺货', '部分缺货']),
    frozen:
      readBooleanField(record, ['frozen']) ||
      Boolean(readStringField(record, ['freeze_reason'])) ||
      hasNormalizedKeyword(combinedLabels, ['ORDER_FROZEN', 'FROZEN', 'FREEZE', '冻结']),
    presale:
      readBooleanField(record, ['presale', 'pre_sale']) ||
      hasNormalizedKeyword(combinedLabels, ['预售', 'PRE_SALE', 'PRESALE', 'PREORDER', 'PRE_ORDER']),
    timeout:
      readBooleanField(record, ['is_timeout', 'is_overdue', 'ship_timeout']) ||
      hasNormalizedKeyword(combinedLabels, ['超时', '已超时', 'OVERDUE', 'TIMEOUT']) ||
      isPastDateField(
        record,
        ['platform_ship_deadline_at', 'ship_deadline_at', 'delivery_deadline_at', 'promise_ship_by', 'sla_deadline_at'],
        now,
      ),
    refund:
      (readNumberField(record, ['refund_amount_cent']) ?? 0) > 0 ||
      Boolean(readStringField(record, ['refund_state', 'after_sale_type'])) ||
      hasNormalizedKeyword(combinedLabels, ['REFUND', 'RETURN', 'AFTERSALE', 'AFTER_SALE', '退款', '退货']),
    merged:
      Boolean(readStringField(record, ['merge_source_order_id', 'merge_group_id'])) ||
      readStringListField(record, ['merged_order_ids']).length > 0 ||
      hasNormalizedKeyword(combinedLabels, ['ORDER_MERGED', 'MERGED', 'MERGE', '合并']),
    split:
      Boolean(readStringField(record, ['split_source_order_id', 'parent_order_id'])) ||
      readStringListField(record, ['split_child_order_ids']).length > 0 ||
      hasNormalizedKeyword(combinedLabels, ['ORDER_SPLIT', 'SPLIT', '拆分']),
  } satisfies Record<BuiltinOrderTagKey, boolean>;

  return builtinOrderTags.filter((tag) => tagState[tag.key]);
};

export const resolveOrderDisplayTags = (
  rawRecord: Record<string, unknown>,
  options?: {
    now?: Date;
  },
): OrderDisplayTag[] => {
  const record = rawRecord ?? {};
  const builtinTags = resolveOrderBuiltinTags(record, options);
  const explicitTags = readStringListField(record, ['tags', 'tag_list', 'marks']);
  const customTags: string[] = [];
  const seenCustomTags = new Set<string>();

  explicitTags.forEach((tag) => {
    const value = String(tag ?? '').trim();
    if (!value) {
      return;
    }
    if (resolveBuiltinOrderTag(value)) {
      return;
    }

    const normalizedValue = normalizeLookupValue(value);
    if (!normalizedValue || seenCustomTags.has(normalizedValue)) {
      return;
    }

    seenCustomTags.add(normalizedValue);
    customTags.push(value);
  });

  return [...builtinTags, ...customTags];
};
