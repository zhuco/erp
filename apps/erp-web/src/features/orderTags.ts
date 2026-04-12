export type BuiltinOrderTag = {
  key: string;
  code: string;
  name: string;
  description: string;
  color: string;
  codeClassName?: string;
  aliases?: string[];
};

export const builtinOrderTags: BuiltinOrderTag[] = [
  {
    key: 'presale',
    code: '预',
    name: '预售',
    description: '平台预售订单',
    color: 'orange'
  },
  {
    key: 'shortage',
    code: '缺',
    name: '缺货',
    description: '缺货订单',
    color: 'red'
  },
  {
    key: 'same-name',
    code: '同',
    name: '同名未合并',
    description: '有同名订单未合并',
    color: 'geekblue'
  },
  {
    key: 'refund',
    code: '退',
    name: '退款',
    description: '订单有退款',
    color: 'blue'
  },
  {
    key: 'frozen',
    code: '冻',
    name: '冻结',
    description: '订单被冻结',
    color: 'volcano'
  },
  {
    key: 'merged',
    code: '合',
    name: '合并',
    description: '多个订单合并',
    color: 'cyan'
  },
  {
    key: 'partial-shortage',
    code: '缺',
    codeClassName: 'order-tag-code-half',
    name: '部分缺货',
    description: '部分缺货订单',
    color: 'gold'
  },
  {
    key: 'timeout',
    code: '超',
    name: '超时',
    description: '超过平台发货时间要求的订单',
    color: 'magenta'
  },
  {
    key: 'invalid-goods',
    code: '效',
    name: '无效货品',
    description: '包含无效货品',
    color: 'purple'
  },
  {
    key: 'priority',
    code: '优',
    name: '优先发货',
    description: '优先发货订单',
    color: 'green'
  }
];

const builtinOrderTagLookup = new Map<string, BuiltinOrderTag>();
builtinOrderTags.forEach((tag) => {
  builtinOrderTagLookup.set(tag.name, tag);
  if (tag.code && !builtinOrderTagLookup.has(tag.code)) {
    builtinOrderTagLookup.set(tag.code, tag);
  }
  (tag.aliases ?? []).forEach((alias) => {
    builtinOrderTagLookup.set(alias, tag);
  });
});

export const resolveBuiltinOrderTag = (value?: string | null) => {
  if (!value) {
    return undefined;
  }
  return builtinOrderTagLookup.get(value);
};
