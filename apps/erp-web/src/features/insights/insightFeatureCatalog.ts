import {
  insightDimensionCatalog,
  type InsightDimensionDefinition,
} from './insightDimensionCatalog';

export type InsightAvailabilityPhase = 'phase-1' | 'phase-1.5' | 'phase-2';
export type InsightOwnerDomain =
  | 'revenue'
  | 'fulfillment'
  | 'catalog'
  | 'inventory'
  | 'service'
  | 'workbench';

export type InsightFeatureDefinition = {
  key: string;
  title: string;
  group:
    | '\u603b\u89c8\u5feb\u62a5'
    | '\u6536\u5165\u4e0e\u8ba2\u5355\u8d28\u91cf'
    | '\u5c65\u7ea6\u4e0e\u4ed3\u914d\u6548\u7387'
    | '\u5546\u54c1\u7ed3\u6784\u4e0e\u751f\u547d\u5468\u671f'
    | '\u5e93\u5b58\u5065\u5eb7\u4e0e\u8c03\u62e8'
    | '\u552e\u540e\u4e0e\u8865\u507f\u6cbb\u7406'
    | '\u62a5\u8868\u4e0e\u5bfc\u51fa\u5de5\u574a';
  canonicalPath: string;
  legacyPaths: string[];
  ownerDomain: InsightOwnerDomain;
  viewKind: 'metric' | 'ledger' | 'placeholder' | 'workbench';
  defaultDimensions: string[];
  availabilityPhase: InsightAvailabilityPhase;
};

export const insightFeatureCatalog: InsightFeatureDefinition[] = [
  {
    key: 'revenue-overview',
    title: '\u4ea4\u6613\u603b\u89c8',
    group: '\u603b\u89c8\u5feb\u62a5',
    canonicalPath: '/insights/revenue/overview',
    legacyPaths: ['/stats/sales'],
    ownerDomain: 'revenue',
    viewKind: 'metric',
    defaultDimensions: ['platform', 'shop'],
    availabilityPhase: 'phase-1',
  },
  {
    key: 'revenue-daily-brief',
    title: '\u8425\u8fd0\u65e5\u62a5',
    group: '\u603b\u89c8\u5feb\u62a5',
    canonicalPath: '/insights/revenue/daily-brief',
    legacyPaths: ['/stats/sales-daily'],
    ownerDomain: 'revenue',
    viewKind: 'metric',
    defaultDimensions: ['shop', 'warehouse'],
    availabilityPhase: 'phase-1',
  },
  {
    key: 'revenue-trend-radar',
    title: '\u589e\u957f\u96f7\u8fbe',
    group: '\u6536\u5165\u4e0e\u8ba2\u5355\u8d28\u91cf',
    canonicalPath: '/insights/revenue/trend-radar',
    legacyPaths: ['/stats/sales-trends'],
    ownerDomain: 'revenue',
    viewKind: 'metric',
    defaultDimensions: ['platform', 'shop', 'category'],
    availabilityPhase: 'phase-1',
  },
  {
    key: 'revenue-discount-insight',
    title: '\u8ba9\u5229\u8bca\u65ad',
    group: '\u6536\u5165\u4e0e\u8ba2\u5355\u8d28\u91cf',
    canonicalPath: '/insights/revenue/discount-insight',
    legacyPaths: [],
    ownerDomain: 'revenue',
    viewKind: 'placeholder',
    defaultDimensions: ['platform', 'shop', 'sku', 'category'],
    availabilityPhase: 'phase-2',
  },
  {
    key: 'revenue-order-mix',
    title: '\u8ba2\u5355\u7ed3\u6784',
    group: '\u6536\u5165\u4e0e\u8ba2\u5355\u8d28\u91cf',
    canonicalPath: '/insights/revenue/order-mix',
    legacyPaths: [],
    ownerDomain: 'revenue',
    viewKind: 'placeholder',
    defaultDimensions: ['platform', 'shop', 'category', 'brand', 'sku'],
    availabilityPhase: 'phase-2',
  },
  {
    key: 'fulfillment-lead-time',
    title: '\u5c65\u7ea6\u65f6\u6548',
    group: '\u5c65\u7ea6\u4e0e\u4ed3\u914d\u6548\u7387',
    canonicalPath: '/insights/fulfillment/lead-time',
    legacyPaths: ['/stats/shipping'],
    ownerDomain: 'fulfillment',
    viewKind: 'metric',
    defaultDimensions: ['warehouse', 'courier', 'platform', 'shop'],
    availabilityPhase: 'phase-1',
  },
  {
    key: 'fulfillment-order-cycle',
    title: '\u8ba2\u5355\u5c65\u7ea6\u5468\u671f',
    group: '\u5c65\u7ea6\u4e0e\u4ed3\u914d\u6548\u7387',
    canonicalPath: '/insights/fulfillment/order-cycle',
    legacyPaths: ['/stats/sales-cycle'],
    ownerDomain: 'fulfillment',
    viewKind: 'metric',
    defaultDimensions: ['platform', 'shop', 'warehouse', 'courier'],
    availabilityPhase: 'phase-1',
  },
  {
    key: 'fulfillment-operation-ledger',
    title: '\u6536\u53d1\u4f5c\u4e1a\u6d41\u6c34',
    group: '\u5c65\u7ea6\u4e0e\u4ed3\u914d\u6548\u7387',
    canonicalPath: '/insights/fulfillment/operation-ledger',
    legacyPaths: [],
    ownerDomain: 'fulfillment',
    viewKind: 'placeholder',
    defaultDimensions: ['warehouse', 'sku', 'operator'],
    availabilityPhase: 'phase-2',
  },
  {
    key: 'catalog-hot-sku',
    title: '\u7206\u54c1\u70ed\u5ea6',
    group: '\u5546\u54c1\u7ed3\u6784\u4e0e\u751f\u547d\u5468\u671f',
    canonicalPath: '/insights/catalog/hot-sku',
    legacyPaths: ['/stats/best-sellers'],
    ownerDomain: 'catalog',
    viewKind: 'metric',
    defaultDimensions: ['sku', 'spu', 'category', 'brand', 'shop', 'platform'],
    availabilityPhase: 'phase-1',
  },
  {
    key: 'catalog-slow-sku',
    title: '\u957f\u5c3e\u6c89\u79ef',
    group: '\u5546\u54c1\u7ed3\u6784\u4e0e\u751f\u547d\u5468\u671f',
    canonicalPath: '/insights/catalog/slow-sku',
    legacyPaths: ['/stats/slow-moving'],
    ownerDomain: 'catalog',
    viewKind: 'metric',
    defaultDimensions: ['sku', 'category', 'brand', 'warehouse'],
    availabilityPhase: 'phase-1',
  },
  {
    key: 'inventory-stock-balance',
    title: '\u6536\u53d1\u5b58\u603b\u89c8',
    group: '\u5e93\u5b58\u5065\u5eb7\u4e0e\u8c03\u62e8',
    canonicalPath: '/insights/inventory/stock-balance',
    legacyPaths: [],
    ownerDomain: 'inventory',
    viewKind: 'placeholder',
    defaultDimensions: ['warehouse', 'sku', 'category'],
    availabilityPhase: 'phase-1.5',
  },
  {
    key: 'service-compensation',
    title: '\u8865\u507f\u770b\u677f',
    group: '\u552e\u540e\u4e0e\u8865\u507f\u6cbb\u7406',
    canonicalPath: '/insights/service/compensation',
    legacyPaths: [],
    ownerDomain: 'service',
    viewKind: 'placeholder',
    defaultDimensions: ['reason_code', 'shop', 'platform'],
    availabilityPhase: 'phase-2',
  },
  {
    key: 'workbench-report-center',
    title: '\u62a5\u8868\u4e2d\u5fc3',
    group: '\u62a5\u8868\u4e0e\u5bfc\u51fa\u5de5\u574a',
    canonicalPath: '/insights/workbench/report-center',
    legacyPaths: ['/reports'],
    ownerDomain: 'workbench',
    viewKind: 'workbench',
    defaultDimensions: ['platform', 'shop', 'warehouse'],
    availabilityPhase: 'phase-1',
  },
  {
    key: 'workbench-export-queue',
    title: '\u5bfc\u51fa\u961f\u5217',
    group: '\u62a5\u8868\u4e0e\u5bfc\u51fa\u5de5\u574a',
    canonicalPath: '/insights/workbench/export-queue',
    legacyPaths: ['/exports'],
    ownerDomain: 'workbench',
    viewKind: 'workbench',
    defaultDimensions: ['platform', 'shop', 'warehouse'],
    availabilityPhase: 'phase-1',
  },
];

export const insightPhaseOneRoutes = insightFeatureCatalog
  .filter((feature) => feature.availabilityPhase === 'phase-1')
  .map((feature) => feature.canonicalPath);

export { insightDimensionCatalog };
export type { InsightDimensionDefinition };
