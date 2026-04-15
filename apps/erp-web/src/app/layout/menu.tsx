import {
  AccountBookOutlined,
  AppstoreOutlined,
  BarChartOutlined,
  CloudDownloadOutlined,
  DashboardOutlined,
  DatabaseOutlined,
  InboxOutlined,
  LineChartOutlined,
  SettingOutlined,
  ShoppingOutlined,
  TagsOutlined,
  TeamOutlined,
} from '@ant-design/icons';
import type { ReactNode } from 'react';

import type { FeatureFlagKey } from '@/features/featureFlags';
import type { Permission } from '@/features/rbac';

export type AppMenuItem = {
  name?: string;
  path?: string;
  icon?: ReactNode;
  children?: AppMenuItem[];
  permission?: Permission;
  feature?: FeatureFlagKey;
};

export type AppMenuLeaf = {
  path: string;
  name: string;
  permission?: Permission;
  feature?: FeatureFlagKey;
};

type MenuLeafOptions = {
  permission?: Permission;
  feature?: FeatureFlagKey;
  icon?: ReactNode;
};

const leaf = (path: string, name: string, options: MenuLeafOptions = {}): AppMenuItem => ({
  path,
  name,
  ...options,
});

const group = (name: string, children: AppMenuItem[]): AppMenuItem => ({
  name,
  children,
});

export const flattenMenuItems = (items: AppMenuItem[]): AppMenuLeaf[] => {
  const result: AppMenuLeaf[] = [];
  const visit = (item: AppMenuItem) => {
    if (item.path && (!item.children || item.children.length === 0)) {
      result.push({
        path: item.path,
        name: item.name ?? item.path,
        permission: item.permission,
        feature: item.feature,
      });
      return;
    }
    item.children?.forEach((child) => visit(child as AppMenuItem));
  };
  items.forEach((item) => visit(item));
  return result;
};

export const appMenuItems: AppMenuItem[] = [
  {
    name: '首页',
    path: '/dashboard',
    icon: <DashboardOutlined />,
  },
  {
    name: '渠道',
    icon: <AppstoreOutlined />,
    children: [
      group('基础档案', [
        leaf('/channels', '渠道目录', { permission: 'channels.read' }),
      ]),
      group('接入配置', [
        leaf('/channels/shops', '店铺接入', { permission: 'settings.read' }),
        leaf('/channels/sync-jobs', '渠道同步任务', { permission: 'settings.read' }),
      ]),
      group('商品映射', [
        leaf('/channels/product-match', '商品匹配', { permission: 'shop_products.read' }),
      ]),
    ],
  },
  {
    name: '销售',
    icon: <ShoppingOutlined />,
    children: [
      group('订单中心', [
        leaf('/sales/new-order', '新建订单', { permission: 'orders.read' }),
        leaf('/sales/import-order', '导入订单', { permission: 'orders.read' }),
        leaf('/orders/review', '审核订单', { permission: 'orders.read' }),
        leaf('/orders/recheck', '订单复核', { permission: 'orders.read' }),
        leaf('/orders', '订单查询', { permission: 'orders.read' }),
        leaf('/orders/exceptions', '异常订单', { permission: 'orders.read' }),
      ]),
      group('网店订单', [
        leaf('/sales/shop-order-download', '网店订单下载', { permission: 'orders.read' }),
        leaf('/sales/logistics-upload', '物流上传', { permission: 'shipping.read' }),
        leaf('/channels/shop-orders', '网店订单查询', { permission: 'orders.read' }),
        leaf('/shop-products', '网店商品下载', { permission: 'shop_products.read' }),
        leaf('/channels/product-match', '网店商品匹配', { permission: 'shop_products.read' }),
        leaf('/sales/shop-product-match-query', '网店商品匹配查询', {
          permission: 'shop_products.read',
        }),
        leaf('/sales/shop-inventory-sync', '网店库存同步', { permission: 'inventory.read' }),
      ]),
      group('履约中心', [
        leaf('/shipments', '发货/包裹', { permission: 'shipping.read' }),
        leaf('/shipments/push-failures', '回传失败', { permission: 'shipping.read' }),
        leaf('/oms/shipments', 'OMS 包裹', { permission: 'shipping.read' }),
      ]),
      group('结算规则', [
        leaf('/orders/review-rules', '审单规则', { permission: 'orders.read' }),
        leaf('/orders/settlement', '销售结算', { permission: 'orders.read' }),
        leaf('/orders/reconciliations', '销售对账', { permission: 'orders.read' }),
        leaf('/sales/strategy-settings', '销售策略设置', { permission: 'settings.read' }),
      ]),
      group('业务设置', [leaf('/sales/settings', '销售设置', { permission: 'settings.read' })]),
    ],
  },
  {
    name: '商品',
    icon: <TagsOutlined />,
    children: [
      group('货品档案', [
        leaf('/products', '商品管理', { permission: 'skus.read' }),
        leaf('/products/barcodes', '商品条码', { permission: 'skus.read' }),
        leaf('/products/units', '商品单位', { permission: 'skus.read' }),
        leaf('/products/packaging-specs', '包装规格', { permission: 'skus.read' }),
        leaf('/kits', '组合装管理', { permission: 'kits.read' }),
      ]),
      group('渠道经营', [
        leaf('/products/channel-availability', '渠道可售与上架', { permission: 'skus.read' }),
      ]),
      group('商品治理', [
        leaf('/products/change-control', '商品变更控制', { permission: 'skus.read' }),
        leaf('/products/lifecycle', '商品生命周期', { permission: 'skus.read' }),
        leaf('/products/diff', '商品相差', { permission: 'skus.read' }),
      ]),
      group('标签体系', [
        leaf('/tags', '标签目录', {
          permission: 'order_tags.read',
          feature: 'directory.tag_system',
        }),
        leaf('/tags/orders', '订单标签', {
          permission: 'order_tags.read',
          feature: 'directory.tag_system',
        }),
        leaf('/tags/skus', '货品标签', {
          permission: 'sku_tags.read',
          feature: 'directory.tag_system',
        }),
      ]),
    ],
  },
  {
    name: '采购',
    icon: <InboxOutlined />,
    children: [
      group('供应网络', [
        leaf('/procurement/suppliers', '供应商目录', { permission: 'suppliers.read' }),
        leaf('/procurement/sourcing', '寻源与选品', {
          permission: 'procurement_sourcing.read',
        }),
        leaf('/procurement/performance', '供应商绩效', {
          permission: 'supplier_performances.read',
        }),
      ]),
      group('采购计划', [
        leaf('/procurement/requests', '采购申请', { permission: 'procurement_requests.read' }),
        leaf('/procurement/plans', '采购计划', { permission: 'procurement_plans.read' }),
        leaf('/procurement/suggestions', '采购建议', {
          permission: 'purchase_suggestions.read',
        }),
      ]),
      group('采购执行', [
        leaf('/procurement/orders', '采购订单', { permission: 'purchase_orders.read' }),
        leaf('/procurement/receipts', '采购到货', { permission: 'purchase_orders.read' }),
        leaf('/procurement/returns', '采购退货', { permission: 'purchase_returns.read' }),
        leaf('/procurement/inspections', '质检单', {
          permission: 'procurement_inspections.read',
        }),
      ]),
      group('供应结算', [
        leaf('/procurement/reconciliations', '供应商对账', {
          permission: 'procurement_reconciliations.read',
        }),
      ]),
    ],
  },
  {
    name: '计划',
    icon: <LineChartOutlined />,
    children: [
      group('需求预测', [leaf('/planning/forecast', '销售预测', { permission: 'inventory.read' })]),
      group('补货策略', [
        leaf('/planning/replenishment-rules', '安全库存与补货策略', {
          permission: 'inventory.read',
        }),
        leaf('/planning/purchase-plans', '采购补货计划', { permission: 'inventory.read' }),
        leaf('/planning/transfer-suggestions', '调拨建议', { permission: 'inventory.read' }),
        leaf('/planning/promo-stocking', '促销备货', { permission: 'inventory.read' }),
      ]),
      group('执行反馈', [
        leaf('/planning/plan-execution', '计划执行反馈', { permission: 'inventory.read' }),
      ]),
    ],
  },
  {
    name: '仓储',
    icon: <DatabaseOutlined />,
    children: [
      group('仓网档案', [
        leaf('/wms/warehouses', '仓库目录', { permission: 'warehouses.read' }),
        leaf('/wms/warehouse-areas', '库区目录', { permission: 'inventory.read' }),
        leaf('/wms/warehouse-locations', '货位目录', { permission: 'inventory.read' }),
        leaf('/wms/location-balances', '库位库存', { permission: 'inventory.read' }),
        leaf('/wms/containers', '容器目录', { permission: 'inventory.read' }),
        leaf('/wms/sender-addresses', '寄方目录', { permission: 'shipping.read' }),
        leaf('/wms/workflow-configs', '流程配置', { permission: 'wms.read' }),
        leaf('/wms/dock-yard', '月台管理', { permission: 'wms.write' }),
      ]),
      group('收货入仓', [
        leaf('/wms/inbounds', '收货入库', {
          permission: 'wms.inbounds.read',
          feature: 'inventory.enabled',
        }),
        leaf('/wms/inbounds/search', '收货查询', {
          permission: 'wms.inbounds.read',
          feature: 'inventory.enabled',
        }),
        leaf('/wms/inbounds/receive-entry', '收货登记', {
          permission: 'wms.inbounds.write',
          feature: 'inventory.enabled',
        }),
        leaf('/wms/return-inbounds', '退货入库', {
          permission: 'aftersales.read',
          feature: 'inventory.enabled',
        }),
        leaf('/wms/inbound-rejects', '拒收入库查询', {
          permission: 'wms.inbounds.read',
          feature: 'inventory.enabled',
        }),
      ]),
      group('发货出仓', [
        leaf('/wms/outbounds', '出库单', { permission: 'wms.read' }),
        leaf('/wms/waves', '波次管理', { permission: 'wms.read' }),
        leaf('/wms/pick-tasks', '拣货任务', { permission: 'wms.read' }),
        leaf('/wms/pack', '打包复核', { permission: 'wms.write' }),
        leaf('/wms/ship', '按单发货', { permission: 'wms.write' }),
        leaf('/wms/outbound-confirm', '出库确认', { permission: 'wms.write' }),
        leaf('/wms/labor', '劳务任务', { permission: 'wms.write' }),
      ]),
      group('库存控制', [
        leaf('/inventory', '总库存查询', {
          permission: 'inventory.read',
          feature: 'inventory.enabled',
        }),
        leaf('/inventory/warehouse', '分仓库存', {
          permission: 'inventory.read',
          feature: 'inventory.enabled',
        }),
        leaf('/inventory/costs', '库存成本', {
          permission: 'inventory.read',
          feature: 'inventory.enabled',
        }),
        leaf('/inventory/ledgers', '库存台账', {
          permission: 'inventory.read',
          feature: 'inventory.enabled',
        }),
        leaf('/inventory/locks', '库存锁定', {
          permission: 'inventory.read',
          feature: 'inventory.enabled',
        }),
        leaf('/inventory/sync-jobs', '库存同步', {
          permission: 'inventory.read',
          feature: 'inventory.enabled',
        }),
        leaf('/inventory/stocktakes', '盘点管理', { permission: 'inventory.read' }),
        leaf('/inventory/stocktake-tasks', '盘点任务', { permission: 'inventory.read' }),
        leaf('/wms/stock-transfers', '调拨管理', { permission: 'inventory.read' }),
        leaf('/wms/inventory-freezes', '库存冻结', { permission: 'inventory.read' }),
        leaf('/wms/stock-adjustments', '报损报溢', { permission: 'inventory.read' }),
        leaf('/wms/stock-moves', '移库管理', { permission: 'inventory.read' }),
        leaf('/inventory/insights/alerts', '库存预警看板', {
          permission: 'inventory_insights.read',
          feature: 'inventory_insights',
        }),
      ]),
      group('策略引擎', [
        leaf('/wms/shipment-preprocess-strategies', '发货预处理策略', {
          permission: 'shipping.read',
        }),
        leaf('/wms/pick-recommend-strategies', '拣货推荐策略', { permission: 'wms.read' }),
        leaf('/wms/stock-count-strategies', '盘点策略/建议', {
          permission: 'inventory.read',
        }),
        leaf('/wms/slotting', '库位优化', { permission: 'wms.write' }),
        leaf('/wms/replenishment', '补货任务', { permission: 'wms.write' }),
      ]),
    ],
  },
  {
    name: '物流',
    icon: <InboxOutlined />,
    children: [
      group('承运基础', [
        leaf('/logistics/couriers', '承运商目录', { permission: 'couriers.read' }),
        leaf('/logistics/products', '物流产品', { permission: 'logistics_products.read' }),
        leaf('/logistics/prices', '物流价格', {
          permission: 'logistics_prices.read',
          feature: 'directory.logistics_price',
        }),
        leaf('/logistics/interfaces', '接口配置', { permission: 'logistics_prices.read' }),
      ]),
      group('跟踪异常', [
        leaf('/logistics/tracking', '物流追踪', { permission: 'shipping.read' }),
        leaf('/logistics/exceptions', '物流异常', { permission: 'shipping.read' }),
      ]),
      group('结算优化', [
        leaf('/logistics/settlements', '物流结算', { permission: 'shipping.read' }),
        leaf('/logistics/warehouse-optimization', '仓配优化', {
          permission: 'shipping.read',
        }),
        leaf('/logistics/installations', '配送安装', { permission: 'shipping.read' }),
      ]),
    ],
  },
  {
    name: '客户',
    icon: <TeamOutlined />,
    children: [
      group('客户运营', [
        leaf('/customers', '客户档案', { permission: 'customers.read' }),
        leaf('/customers/categories', '客户分类', { permission: 'customers.read' }),
        leaf('/customers/member-levels', '会员等级', { permission: 'customers.read' }),
        leaf('/customers/tags', '客户标签', { permission: 'customers.read' }),
        leaf('/customers/loyalty', '积分与储值', { permission: 'customers.read' }),
        leaf('/customers/segments', '客户分群', { permission: 'customers.read' }),
        leaf('/customers/lifecycle', '生命周期运营', { permission: 'customers.read' }),
      ]),
      group('服务工单', [
        leaf('/customers/service-tickets', '客服工单', { permission: 'customers.read' }),
      ]),
      group('售后作业', [
        leaf('/after-sales', '售后单', { permission: 'aftersales.read' }),
        leaf('/after-sales/intercepts', '退款拦截', { permission: 'aftersales.read' }),
        leaf('/after-sales/exchanges', '换货补发', { permission: 'aftersales.read' }),
        leaf('/after-sales/inspections', '退货质检', { permission: 'aftersales.read' }),
        leaf('/after-sales/sla', '售后 SLA', { permission: 'aftersales.read' }),
        leaf('/after-sales/bulk-edit', '售后批量导入', { permission: 'aftersales.read' }),
      ]),
      group('平台售后', [
        leaf('/after-sale-cases', '平台售后', { permission: 'aftersales.read' }),
        leaf('/refund-docs', '退款处理', { permission: 'aftersales.read' }),
        leaf('/after-sales/settings', '售后设置', { permission: 'SYS.SETTINGS.READ' }),
      ]),
    ],
  },
  {
    name: '财务',
    icon: <AccountBookOutlined />,
    children: [
      group('应收应付', [
        leaf('/finance/ar', '应收管理', { permission: 'finance.read' }),
        leaf('/finance/ap', '应付管理', { permission: 'finance.read' }),
        leaf('/finance/reconcile', '对账中心', { permission: 'finance.read' }),
        leaf('/finance/ap-docs', '应付单据', { permission: 'finance.read' }),
      ]),
      group('核算利润', [
        leaf('/finance/expenses', '费用分摊', { permission: 'finance.read' }),
        leaf('/finance/costing', '成本核算', { permission: 'finance.read' }),
        leaf('/finance/profit', '利润报表', { permission: 'finance.read' }),
        leaf('/finance/consignment', '代销结算', { permission: 'finance.read' }),
      ]),
      group('税票管理', [
        leaf('/finance/invoices', '开票中心', { permission: 'finance.read' }),
        leaf('/finance/tax', '税务管理', { permission: 'finance.read' }),
      ]),
    ],
  },
  {
    name: '营销',
    icon: <LineChartOutlined />,
    children: [
      group('价格策略', [
        leaf('/marketing/prices', '渠道价表', { permission: 'marketing_prices.read' }),
      ]),
      group('促销活动', [
        leaf('/marketing/promotions', '促销活动', {
          permission: 'marketing_promotions.read',
        }),
        leaf('/marketing/gifts', '赠品策略', { permission: 'marketing_gifts.read' }),
        leaf('/marketing/coupons', '优惠券', { permission: 'marketing_coupons.read' }),
      ]),
      group('费用复盘', [
        leaf('/marketing/costs', '促销费用', { permission: 'marketing_costs.read' }),
        leaf('/marketing/postmortems', '活动复盘', {
          permission: 'marketing_postmortems.read',
        }),
      ]),
    ],
  },
  {
    name: '分析',
    icon: <BarChartOutlined />,
    feature: 'ui.enable_stats',
    children: [
      group('销售分析', [
        leaf('/stats/sales', '销售统计', { permission: 'stats.read' }),
        leaf('/stats/sales-daily', '销售日报表', { permission: 'stats.read' }),
        leaf('/stats/sales-trends', '销售趋势分析', { permission: 'stats.read' }),
        leaf('/stats/slow-moving', '滞销分析', { permission: 'stats.read' }),
        leaf('/stats/best-sellers', '畅销分析', { permission: 'stats.read' }),
        leaf('/stats/sales-cycle', '销售周期', { permission: 'stats.read' }),
      ]),
      group('履约分析', [leaf('/stats/shipping', '发货统计', { permission: 'stats.read' })]),
      group('数据输出', [
        leaf('/reports', '经营分析报表', { permission: 'reports.read' }),
        leaf('/exports', '导出任务', { permission: 'exports.read' }),
      ]),
    ],
  },
  {
    name: '集成',
    icon: <CloudDownloadOutlined />,
    children: [
      group('平台接入', [
        leaf('/integrations/center', '平台接入总览', { permission: 'SYS.SETTINGS.READ' }),
        leaf('/integrations/sync-jobs', '集成同步任务', { permission: 'SYS.SETTINGS.READ' }),
      ]),
      group('打印管理', [
        leaf('/integrations/print-devices', '打印设备', { permission: 'print_devices.read' }),
        leaf('/integrations/print-mappings', '打印映射', { permission: 'print_mappings.read' }),
        leaf('/print-jobs', '打印任务', { permission: 'print_jobs.read' }),
        leaf('/print-templates', '打印模板', { permission: 'print_templates.read' }),
      ]),
    ],
  },
  {
    name: '系统',
    icon: <SettingOutlined />,
    children: [
      group('基础配置', [
        leaf('/system/settings', '系统设置', { permission: 'SYS.SETTINGS.READ' }),
        leaf('/system/workflows', '单据编号/流程配置', { permission: 'SYS.SETTINGS.READ' }),
        leaf('/system/custom-fields', '自定义字段', { permission: 'SYS.SETTINGS.READ' }),
      ]),
      group('权限组织', [
        leaf('/system/users', '登录账户', { permission: 'SYS.USER.READ' }),
        leaf('/system/roles', '角色管理', { permission: 'SYS.ROLE.READ' }),
        leaf('/system/permissions', '权限目录', { permission: 'SYS.PERMISSION.READ' }),
        leaf('/system/data-scopes', '数据权限', { permission: 'SYS.USER.READ' }),
      ]),
      group('审计监控', [
        leaf('/system/audit-logs', '审计日志', { permission: 'SYS.AUDIT.READ' }),
        leaf('/system/operation-logs', '操作日志', { permission: 'operation_logs.read' }),
        leaf('/system/audit-reports', '审计报告', { permission: 'SYS.SETTINGS.READ' }),
        leaf('/system/observability', '可观测性面板', { permission: 'SYS.DLQ.READ' }),
        leaf('/system/frontend-route-observability', '前端路由留痕', {
          permission: 'operation_logs.read',
        }),
        leaf('/failure-queue', '失败队列', { permission: 'SYS.DLQ.READ' }),
      ]),
    ],
  },
];
