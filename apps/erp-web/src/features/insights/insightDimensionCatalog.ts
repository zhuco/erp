export type InsightDimensionDefinition = {
  key: string;
  label: string;
  category:
    | 'time'
    | 'trade'
    | 'fulfillment'
    | 'catalog'
    | 'inventory'
    | 'service';
};

export const insightDimensionCatalog: Record<string, InsightDimensionDefinition> = {
  platform: { key: 'platform', label: '平台', category: 'trade' },
  shop: { key: 'shop', label: '店铺', category: 'trade' },
  warehouse: { key: 'warehouse', label: '仓库', category: 'fulfillment' },
  category: { key: 'category', label: '品类', category: 'catalog' },
  brand: { key: 'brand', label: '品牌', category: 'catalog' },
  sku: { key: 'sku', label: 'SKU', category: 'catalog' },
  spu: { key: 'spu', label: 'SPU', category: 'catalog' },
  operator: { key: 'operator', label: '作业员', category: 'fulfillment' },
  courier: { key: 'courier', label: '承运商', category: 'fulfillment' },
  age_bucket: { key: 'age_bucket', label: '库龄区间', category: 'inventory' },
  reason_code: { key: 'reason_code', label: '原因', category: 'service' },
  agent: { key: 'agent', label: '坐席', category: 'service' },
};
