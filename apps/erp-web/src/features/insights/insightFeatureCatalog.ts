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
    | '总览快报'
    | '收入与订单质量'
    | '履约与仓配效率'
    | '商品结构与生命周期'
    | '库存健康与调拨'
    | '售后与补偿治理'
    | '报表与导出工坊';
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
    title: '交易总览',
    group: '总览快报',
    canonicalPath: '/insights/revenue/overview',
    legacyPaths: ['/stats/sales'],
    ownerDomain: 'revenue',
    viewKind: 'metric',
    defaultDimensions: ['platform', 'shop', 'order_source', 'order_status'],
    availabilityPhase: 'phase-1',
  },
  {
    key: 'revenue-daily-brief',
    title: '营运日报',
    group: '总览快报',
    canonicalPath: '/insights/revenue/daily-brief',
    legacyPaths: ['/stats/sales-daily'],
    ownerDomain: 'revenue',
    viewKind: 'metric',
    defaultDimensions: ['shop', 'warehouse', 'team'],
    availabilityPhase: 'phase-1',
  },
  {
    key: 'revenue-trend-radar',
    title: '增长雷达',
    group: '收入与订单质量',
    canonicalPath: '/insights/revenue/trend-radar',
    legacyPaths: ['/stats/sales-trends'],
    ownerDomain: 'revenue',
    viewKind: 'metric',
    defaultDimensions: ['platform', 'shop', 'category'],
    availabilityPhase: 'phase-1',
  },
  {
    key: 'revenue-discount-insight',
    title: '让利诊断',
    group: '收入与订单质量',
    canonicalPath: '/insights/revenue/discount-insight',
    legacyPaths: [],
    ownerDomain: 'revenue',
    viewKind: 'placeholder',
    defaultDimensions: ['platform', 'shop', 'activity', 'sku', 'category'],
    availabilityPhase: 'phase-2',
  },
  {
    key: 'revenue-order-mix',
    title: '订单结构',
    group: '收入与订单质量',
    canonicalPath: '/insights/revenue/order-mix',
    legacyPaths: [],
    ownerDomain: 'revenue',
    viewKind: 'placeholder',
    defaultDimensions: ['platform', 'shop', 'category', 'brand', 'sku', 'order_source'],
    availabilityPhase: 'phase-2',
  },
  {
    key: 'fulfillment-lead-time',
    title: '履约时效',
    group: '履约与仓配效率',
    canonicalPath: '/insights/fulfillment/lead-time',
    legacyPaths: ['/stats/shipping'],
    ownerDomain: 'fulfillment',
    viewKind: 'metric',
    defaultDimensions: ['warehouse', 'courier', 'platform', 'shop'],
    availabilityPhase: 'phase-1',
  },
  {
    key: 'fulfillment-order-cycle',
    title: '订单履约周期',
    group: '履约与仓配效率',
    canonicalPath: '/insights/fulfillment/order-cycle',
    legacyPaths: ['/stats/sales-cycle'],
    ownerDomain: 'fulfillment',
    viewKind: 'metric',
    defaultDimensions: ['platform', 'shop', 'warehouse', 'courier'],
    availabilityPhase: 'phase-1',
  },
  {
    key: 'fulfillment-operation-ledger',
    title: '收发作业流水',
    group: '履约与仓配效率',
    canonicalPath: '/insights/fulfillment/operation-ledger',
    legacyPaths: [],
    ownerDomain: 'fulfillment',
    viewKind: 'placeholder',
    defaultDimensions: ['document_type', 'warehouse', 'sku', 'operator'],
    availabilityPhase: 'phase-2',
  },
  {
    key: 'fulfillment-packaging-usage',
    title: '包装耗用',
    group: '履约与仓配效率',
    canonicalPath: '/insights/fulfillment/packaging-usage',
    legacyPaths: [],
    ownerDomain: 'fulfillment',
    viewKind: 'placeholder',
    defaultDimensions: ['warehouse', 'packaging_type', 'operator', 'channel'],
    availabilityPhase: 'phase-2',
  },
  {
    key: 'fulfillment-stocktake-variance',
    title: '盘点偏差',
    group: '履约与仓配效率',
    canonicalPath: '/insights/fulfillment/stocktake-variance',
    legacyPaths: [],
    ownerDomain: 'fulfillment',
    viewKind: 'placeholder',
    defaultDimensions: ['warehouse', 'stocktake_batch', 'sku', 'operator'],
    availabilityPhase: 'phase-2',
  },
  {
    key: 'fulfillment-operation-accuracy',
    title: '作业准确度',
    group: '履约与仓配效率',
    canonicalPath: '/insights/fulfillment/operation-accuracy',
    legacyPaths: [],
    ownerDomain: 'fulfillment',
    viewKind: 'placeholder',
    defaultDimensions: ['warehouse', 'operator', 'shift', 'error_type'],
    availabilityPhase: 'phase-2',
  },
  {
    key: 'fulfillment-warehouse-network-efficiency',
    title: '仓网效能',
    group: '履约与仓配效率',
    canonicalPath: '/insights/fulfillment/warehouse-network-efficiency',
    legacyPaths: [],
    ownerDomain: 'fulfillment',
    viewKind: 'placeholder',
    defaultDimensions: ['warehouse', 'team', 'shift', 'date'],
    availabilityPhase: 'phase-2',
  },
  {
    key: 'catalog-hot-sku',
    title: '爆品热度',
    group: '商品结构与生命周期',
    canonicalPath: '/insights/catalog/hot-sku',
    legacyPaths: ['/stats/best-sellers'],
    ownerDomain: 'catalog',
    viewKind: 'metric',
    defaultDimensions: ['sku', 'spu', 'category', 'brand', 'shop', 'platform'],
    availabilityPhase: 'phase-1',
  },
  {
    key: 'catalog-slow-sku',
    title: '长尾沉积',
    group: '商品结构与生命周期',
    canonicalPath: '/insights/catalog/slow-sku',
    legacyPaths: ['/stats/slow-moving'],
    ownerDomain: 'catalog',
    viewKind: 'metric',
    defaultDimensions: ['sku', 'category', 'brand', 'warehouse', 'lifecycle_stage'],
    availabilityPhase: 'phase-1',
  },
  {
    key: 'catalog-bundle-combo',
    title: '连带组合',
    group: '商品结构与生命周期',
    canonicalPath: '/insights/catalog/bundle-combo',
    legacyPaths: [],
    ownerDomain: 'catalog',
    viewKind: 'placeholder',
    defaultDimensions: ['sku', 'related_sku', 'category', 'shop', 'platform'],
    availabilityPhase: 'phase-2',
  },
  {
    key: 'inventory-stock-balance',
    title: '收发存总览',
    group: '库存健康与调拨',
    canonicalPath: '/insights/inventory/stock-balance',
    legacyPaths: [],
    ownerDomain: 'inventory',
    viewKind: 'placeholder',
    defaultDimensions: ['warehouse', 'sku', 'category', 'transaction_type'],
    availabilityPhase: 'phase-1.5',
  },
  {
    key: 'inventory-stock-aging',
    title: '库龄健康',
    group: '库存健康与调拨',
    canonicalPath: '/insights/inventory/stock-aging',
    legacyPaths: [],
    ownerDomain: 'inventory',
    viewKind: 'placeholder',
    defaultDimensions: ['warehouse', 'sku', 'category', 'age_bucket', 'batch'],
    availabilityPhase: 'phase-1.5',
  },
  {
    key: 'inventory-turnover-velocity',
    title: '周转流速',
    group: '库存健康与调拨',
    canonicalPath: '/insights/inventory/turnover-velocity',
    legacyPaths: [],
    ownerDomain: 'inventory',
    viewKind: 'placeholder',
    defaultDimensions: ['warehouse', 'sku', 'category', 'stock_status'],
    availabilityPhase: 'phase-1.5',
  },
  {
    key: 'inventory-transfer-benefit',
    title: '调拨效益',
    group: '库存健康与调拨',
    canonicalPath: '/insights/inventory/transfer-benefit',
    legacyPaths: [],
    ownerDomain: 'inventory',
    viewKind: 'placeholder',
    defaultDimensions: ['warehouse', 'target_warehouse', 'sku', 'transfer_type', 'in_transit_status'],
    availabilityPhase: 'phase-2',
  },
  {
    key: 'inventory-negative-stock-alert',
    title: '负库存预警',
    group: '库存健康与调拨',
    canonicalPath: '/insights/inventory/negative-stock-alert',
    legacyPaths: [],
    ownerDomain: 'inventory',
    viewKind: 'placeholder',
    defaultDimensions: ['warehouse', 'sku', 'transaction_source', 'date'],
    availabilityPhase: 'phase-2',
  },
  {
    key: 'service-return-loop',
    title: '退回闭环',
    group: '售后与补偿治理',
    canonicalPath: '/insights/service/return-loop',
    legacyPaths: [],
    ownerDomain: 'service',
    viewKind: 'placeholder',
    defaultDimensions: ['reason_code', 'warehouse', 'shop', 'platform', 'sku'],
    availabilityPhase: 'phase-2',
  },
  {
    key: 'service-compensation',
    title: '补偿看板',
    group: '售后与补偿治理',
    canonicalPath: '/insights/service/compensation',
    legacyPaths: [],
    ownerDomain: 'service',
    viewKind: 'placeholder',
    defaultDimensions: ['reason_code', 'shop', 'platform', 'reviewer'],
    availabilityPhase: 'phase-2',
  },
  {
    key: 'service-reship-quality',
    title: '补发质量',
    group: '售后与补偿治理',
    canonicalPath: '/insights/service/reship-quality',
    legacyPaths: [],
    ownerDomain: 'service',
    viewKind: 'placeholder',
    defaultDimensions: ['reason_code', 'warehouse', 'operator', 'shop', 'platform', 'sku'],
    availabilityPhase: 'phase-2',
  },
  {
    key: 'service-ticket-efficiency',
    title: '工单效率',
    group: '售后与补偿治理',
    canonicalPath: '/insights/service/ticket-efficiency',
    legacyPaths: [],
    ownerDomain: 'service',
    viewKind: 'placeholder',
    defaultDimensions: ['ticket_type', 'service_team', 'agent', 'priority', 'channel'],
    availabilityPhase: 'phase-2',
  },
  {
    key: 'workbench-report-center',
    title: '报表中心',
    group: '报表与导出工坊',
    canonicalPath: '/insights/workbench/report-center',
    legacyPaths: ['/reports'],
    ownerDomain: 'workbench',
    viewKind: 'workbench',
    defaultDimensions: ['platform', 'shop', 'warehouse'],
    availabilityPhase: 'phase-1',
  },
  {
    key: 'workbench-export-queue',
    title: '导出队列',
    group: '报表与导出工坊',
    canonicalPath: '/insights/workbench/export-queue',
    legacyPaths: ['/exports'],
    ownerDomain: 'workbench',
    viewKind: 'workbench',
    defaultDimensions: ['task_type', 'platform', 'shop', 'warehouse'],
    availabilityPhase: 'phase-1',
  },
];

export const insightPhaseOneRoutes = insightFeatureCatalog
  .filter((feature) => feature.availabilityPhase === 'phase-1')
  .map((feature) => feature.canonicalPath);

export { insightDimensionCatalog };
export type { InsightDimensionDefinition };
