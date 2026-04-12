export type OrderReviewAgNodeType =
  | 'group'
  | 'platform'
  | 'shop'
  | 'time'
  | 'status'
  | 'exception'
  | 'assignee'
  | 'sla'
  | 'warehouse'
  | 'logistics'
  | 'remark'
  | 'tag';

export type OrderReviewAgLeftNode = {
  key: string;
  label: string;
  count?: number;
  nodeType: OrderReviewAgNodeType;
  children?: OrderReviewAgLeftNode[];
};

export type OrderReviewAgLeftRow = {
  key: string;
  label: string;
  count?: number;
  nodeType: OrderReviewAgNodeType;
  level: number;
  hasChildren: boolean;
  expanded: boolean;
};

export type OrderReviewCheckedFilters = {
  platform?: string[];
  shopId?: string[];
};

const normalizeValues = (value?: string[]) => {
  if (!value || !value.length) {
    return undefined;
  }
  const cleaned = Array.from(new Set(value.map((item) => item.trim()).filter(Boolean)));
  return cleaned.length ? cleaned : undefined;
};

export const flattenAgLeftTree = (
  nodes: OrderReviewAgLeftNode[],
  expandedKeys: Set<string>,
): OrderReviewAgLeftRow[] => {
  const output: OrderReviewAgLeftRow[] = [];

  const walk = (items: OrderReviewAgLeftNode[], level: number) => {
    items.forEach((node) => {
      const hasChildren = Boolean(node.children?.length);
      const expanded = hasChildren && expandedKeys.has(node.key);
      output.push({
        key: node.key,
        label: node.label,
        count: node.count,
        nodeType: node.nodeType,
        level,
        hasChildren,
        expanded,
      });
      if (expanded && node.children?.length) {
        walk(node.children, level + 1);
      }
    });
  };

  walk(nodes, 0);
  return output;
};

export const parseCheckedKeysToFilters = (checkedKeys: string[]): OrderReviewCheckedFilters => {
  const platform = normalizeValues(
    checkedKeys
      .filter((key) => key.startsWith('platform:'))
      .map((key) => key.slice('platform:'.length)),
  );
  const shopId = normalizeValues(
    checkedKeys
      .filter((key) => key.startsWith('shop:'))
      .map((key) => key.slice('shop:'.length)),
  );
  return { platform, shopId };
};

export const toggleCheckedValue = (values: string[] | undefined, value: string) => {
  const base = values ? [...values] : [];
  const index = base.indexOf(value);
  if (index >= 0) {
    base.splice(index, 1);
  } else {
    base.push(value);
  }
  const normalized = normalizeValues(base);
  return normalized;
};
