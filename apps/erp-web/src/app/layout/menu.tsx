import {
  AccountBookOutlined,
  AppstoreOutlined,
  BarChartOutlined,
  CloudDownloadOutlined,
  DashboardOutlined,
  DatabaseOutlined,
  InboxOutlined,
  LineChartOutlined,
  PrinterOutlined,
  SettingOutlined,
  ShoppingOutlined,
  TagsOutlined,
  TeamOutlined,
  ToolOutlined
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

export const flattenMenuItems = (items: AppMenuItem[]): AppMenuLeaf[] => {
  const result: AppMenuLeaf[] = [];
  const visit = (item: AppMenuItem) => {
    if (item.path && (!item.children || item.children.length === 0)) {
      result.push({
        path: item.path,
        name: item.name ?? item.path,
        permission: item.permission,
        feature: item.feature
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
    name: '仪表盘',
    path: '/dashboard',
    icon: <DashboardOutlined />
  },
  {
    name: '系统',
    icon: <SettingOutlined />,
    children: [
      {
        path: '/system/settings',
        name: '系统设置',
        permission: 'SYS.SETTINGS.READ'
      },
      {
        path: '/system/workflows',
        name: '单据编号/流程配置',
        permission: 'SYS.SETTINGS.READ'
      },
      {
        path: '/system/custom-fields',
        name: '自定义字段',
        permission: 'SYS.SETTINGS.READ'
      },
      {
        path: '/system/integrations',
        name: '接口与集成',
        permission: 'SYS.SETTINGS.READ'
      },
      {
        path: '/system/sync-jobs',
        name: '同步任务',
        permission: 'SYS.SETTINGS.READ'
      },
      {
        path: '/system/audit-reports',
        name: '审计报告',
        permission: 'SYS.SETTINGS.READ'
      },
      {
        name: '打印设置',
        icon: <PrinterOutlined />,
        children: [
          {
            path: '/system/print-devices',
            name: '打印设备',
            permission: 'print_devices.read'
          },
          {
            path: '/system/print-mappings',
            name: '打印映射',
            permission: 'print_mappings.read'
          }
        ]
      },
      {
        path: '/system/users',
        name: '登录账户',
        permission: 'SYS.USER.READ'
      },
      {
        path: '/system/roles',
        name: '角色管理',
        permission: 'SYS.ROLE.READ'
      },
      {
        path: '/system/permissions',
        name: '权限目录',
        permission: 'SYS.PERMISSION.READ'
      },
      {
        path: '/system/data-scopes',
        name: '数据权限',
        permission: 'SYS.USER.READ'
      },
      {
        path: '/system/audit-logs',
        name: '审计日志',
        permission: 'SYS.AUDIT.READ'
      },
      {
        path: '/system/operation-logs',
        name: '操作日志',
        permission: 'operation_logs.read'
      },
      {
        path: '/system/observability',
        name: '可观测性面板',
        permission: 'SYS.DLQ.READ'
      },
      {
        path: '/system/ag-grid-enterprise',
        name: 'AG Grid Enterprise',
        permission: 'orders.read'
      },
      {
        path: '/failure-queue',
        name: '失败队列',
        permission: 'SYS.DLQ.READ'
      }
    ]
  },
  {
    name: '目录',
    icon: <AppstoreOutlined />,
    children: [
      {
        path: '/warehouses',
        name: '仓库目录',
        permission: 'warehouses.read'
      },
      {
        name: '物流目录',
        icon: <InboxOutlined />,
        children: [
          {
            path: '/couriers',
            name: '快递公司',
            permission: 'couriers.read'
          },
          {
            path: '/logistics/products',
            name: '物流产品',
            permission: 'logistics_products.read'
          },
          {
            path: '/logistics/prices',
            name: '物流价格',
            permission: 'logistics_prices.read',
            feature: 'directory.logistics_price'
          },
          {
            path: '/logistics/interfaces',
            name: '接口配置',
            permission: 'logistics_prices.read'
          },
        ]
      },
      {
        path: '/suppliers',
        name: '供应商目录',
        permission: 'suppliers.read'
      },
      {
        name: '标记目录',
        icon: <TagsOutlined />,
        feature: 'directory.tag_system',
        children: [
          {
            path: '/tags',
            name: '标记目录',
            permission: 'order_tags.read',
            feature: 'directory.tag_system'
          },
          {
            path: '/tags/orders',
            name: '订单标记',
            permission: 'order_tags.read',
            feature: 'directory.tag_system'
          },
          {
            path: '/tags/skus',
            name: '货品标记',
            permission: 'sku_tags.read',
            feature: 'directory.tag_system'
          }
        ]
      },
      {
        path: '/channels',
        name: '渠道目录',
        permission: 'channels.read'
      }
    ]
  },
  {
    name: '货品',
    icon: <TagsOutlined />,
    children: [
      {
        path: '/products',
        name: '货品管理',
        permission: 'skus.read'
      },
      {
        path: '/products/barcodes',
        name: '货品条码',
        permission: 'skus.read'
      },
      {
        path: '/products/units',
        name: '货品单位',
        permission: 'skus.read'
      },
      {
        path: '/products/channel-availability',
        name: '货品渠道可售与上架',
        permission: 'skus.read'
      },
      {
        path: '/products/change-control',
        name: '货品变更控制',
        permission: 'skus.read'
      },
      {
        path: '/products/lifecycle',
        name: '货品生命周期',
        permission: 'skus.read'
      },
      {
        path: '/products/diff',
        name: '货品相差',
        permission: 'skus.read'
      },
      {
        path: '/products/packaging-specs',
        name: '包装规格',
        permission: 'skus.read'
      },
      {
        path: '/kits',
        name: '组合装管理',
        permission: 'kits.read'
      }
    ]
  },
  {
    name: '采购',
    icon: <InboxOutlined />,
    children: [
      {
        path: '/procurement/requests',
        name: '采购申请',
        permission: 'procurement_requests.read'
      },
      {
        path: '/procurement/plans',
        name: '采购计划',
        permission: 'procurement_plans.read'
      },
      {
        path: '/procurement/sourcing',
        name: '寻源与选品',
        permission: 'procurement_sourcing.read'
      },
      {
        path: '/procurement/inspections',
        name: '质检单',
        permission: 'procurement_inspections.read'
      },
      {
        path: '/procurement/orders',
        name: '采购订单',
        permission: 'purchase_orders.read'
      },
      {
        path: '/procurement/receipts',
        name: '采购到货',
        permission: 'purchase_orders.read'
      },
      {
        path: '/procurement/suggestions',
        name: '采购建议',
        permission: 'purchase_suggestions.read'
      },
      {
        path: '/procurement/returns',
        name: '采购退货',
        permission: 'purchase_returns.read'
      },
      {
        path: '/procurement/reconciliations',
        name: '供应商对账',
        permission: 'procurement_reconciliations.read'
      },
      {
        path: '/procurement/performance',
        name: '供应商绩效',
        permission: 'supplier_performances.read'
      }
    ]
  },
  {
    name: '计划与补货',
    icon: <LineChartOutlined />,
    children: [
      {
        path: '/planning/forecast',
        name: '销售预测',
        permission: 'inventory.read'
      },
      {
        path: '/planning/replenishment-rules',
        name: '安全库存与补货策略',
        permission: 'inventory.read'
      },
      {
        path: '/planning/purchase-plans',
        name: '采购补货计划',
        permission: 'inventory.read'
      },
      {
        path: '/planning/transfer-suggestions',
        name: '调拨建议',
        permission: 'inventory.read'
      },
      {
        path: '/planning/promo-stocking',
        name: '促销备货',
        permission: 'inventory.read'
      },
      {
        path: '/planning/plan-execution',
        name: '计划执行反馈',
        permission: 'inventory.read'
      }
    ]
  },
  {
    name: '销售',
    icon: <ShoppingOutlined />,
    children: [
      {
        path: '/sales/new-order',
        name: '新建订单',
        permission: 'orders.read'
      },
      {
        path: '/sales/import-order',
        name: '导入订单',
        permission: 'orders.read'
      },
      {
        path: '/orders/review',
        name: '订单审核',
        permission: 'orders.read'
      },
      {
        path: '/orders/exceptions',
        name: '订单复核',
        permission: 'orders.read'
      },
      {
        path: '/orders/exception-sla',
        name: '异常 SLA',
        permission: 'orders.read'
      },
      {
        path: '/orders/split-merge',
        name: '拆单合单',
        permission: 'orders.read'
      },
      {
        path: '/orders/review-rules',
        name: '审单规则',
        permission: 'orders.read'
      },
      {
        path: '/orders',
        name: '订单查询',
        permission: 'orders.read'
      },
        {
          path: '/orders/settlement',
          name: '销售结算',
          permission: 'orders.read'
        },
      {
        path: '/orders/reconciliations',
        name: '销售对账',
        permission: 'orders.read'
      },
      {
        path: '/sales/shops',
        name: '网店订单下载',
        permission: 'settings.read'
      },
      {
        path: '/sales/product-match',
        name: '商品匹配',
        permission: 'shop_products.read'
      },
      {
        path: '/sales/shop-orders',
        name: '网店订单查询',
        permission: 'orders.read'
      },
      {
        path: '/sales/logistics-upload',
        name: '物流信息上传',
        permission: 'shipping.read'
      },
      {
        path: '/sales/sync-job-runs',
        name: '同步任务日志',
        permission: 'settings.read'
      },
      {
        path: '/sales/strategy-settings',
        name: '销售策略设置',
        permission: 'settings.read'
      },
      {
        path: '/sales/settings',
        name: '销售设置',
        permission: 'settings.read'
      },
      {
        path: '/shipments',
        name: '发货/包裹',
        permission: 'shipping.read'
      },
      {
        path: '/shipments/push-failures',
        name: '回传失败',
        permission: 'shipping.read'
      },
      {
        path: '/oms/shipments',
        name: 'OMS包裹',
        permission: 'shipping.read'
      },
      {
        path: '/logistics/tracking',
        name: '物流查询',
        permission: 'shipping.read'
      }
    ]
  },
  {
    name: '物流',
    icon: <InboxOutlined />,
    children: [
      {
        path: '/logistics/exceptions',
        name: '异常工单',
        permission: 'shipping.read'
      },
      {
        path: '/logistics/warehouse-optimization',
        name: '仓配优化',
        permission: 'shipping.read'
      },
      {
        path: '/logistics/installations',
        name: '配送安装',
        permission: 'shipping.read'
      },
      {
        path: '/logistics/settlements',
        name: '费用结算',
        permission: 'shipping.read'
      }
    ]
  },
  {
    name: '营销',
    icon: <LineChartOutlined />,
    children: [
      {
        path: '/marketing/prices',
        name: '渠道价表',
        permission: 'marketing_prices.read'
      },
      {
        path: '/marketing/promotions',
        name: '促销活动',
        permission: 'marketing_promotions.read'
      },
      {
        path: '/marketing/gifts',
        name: '赠品策略',
        permission: 'marketing_gifts.read'
      },
      {
        path: '/marketing/coupons',
        name: '优惠券',
        permission: 'marketing_coupons.read'
      },
      {
        path: '/marketing/costs',
        name: '促销费用',
        permission: 'marketing_costs.read'
      },
      {
        path: '/marketing/postmortems',
        name: '活动复盘',
        permission: 'marketing_postmortems.read'
      }
    ]
  },
  {
    name: '库存',
    icon: <DatabaseOutlined />,
    children: [
      {
        path: '/inventory',
        name: '总库存查询',
        permission: 'inventory.read',
        feature: 'inventory.enabled'
      },
      {
        path: '/inventory/warehouse',
        name: '分仓库存',
        permission: 'inventory.read',
        feature: 'inventory.enabled'
      },
      {
        path: '/inventory/costs',
        name: '库存成本',
        permission: 'inventory.read',
        feature: 'inventory.enabled'
      },
      {
        path: '/inventory/ledgers',
        name: '库存台账',
        permission: 'inventory.read',
        feature: 'inventory.enabled'
      },
      {
        path: '/inventory/locks',
        name: '库存锁定',
        permission: 'inventory.read',
        feature: 'inventory.enabled'
      },
      {
        path: '/inventory/sync-jobs',
        name: '库存同步',
        permission: 'inventory.read',
        feature: 'inventory.enabled'
      },
      {
        name: '数据分析',
        children: [
          {
            path: '/inventory/insights/alerts',
            name: '库存预警看板',
            permission: 'inventory_insights.read',
            feature: 'inventory_insights'
          }
        ]
      }
    ]
  },
  {
    name: '仓储',
    icon: <InboxOutlined />,
    children: [
      {
        name: '基础资料',
        icon: <DatabaseOutlined />,
        children: [
          {
            path: '/wms/warehouse-areas',
            name: '库区目录',
            permission: 'inventory.read'
          },
          {
            path: '/wms/warehouse-locations',
            name: '货位目录',
            permission: 'inventory.read'
          },
          {
            path: '/wms/location-balances',
            name: '库位库存',
            permission: 'inventory.read'
          },
          {
            path: '/wms/containers',
            name: '容器目录',
            permission: 'inventory.read'
          },
          {
            path: '/wms/sender-addresses',
            name: '寄方目录',
            permission: 'shipping.read'
          },
          {
            path: '/wms/workflow-configs',
            name: '流程配置',
            permission: 'wms.read'
          }
        ]
      },
      {
        name: '出库',
        icon: <InboxOutlined />,
        children: [
          {
            path: '/wms/outbounds',
            name: '出库单',
            permission: 'wms.read'
          },
          {
            path: '/wms/waves',
            name: '波次管理',
            permission: 'wms.read'
          },
          {
            path: '/wms/pick-tasks',
            name: '拣货任务',
            permission: 'wms.read'
          },
          {
            path: '/wms/pack',
            name: '打包复核',
            permission: 'wms.write'
          },
          {
            path: '/wms/ship',
            name: '按单发货',
            permission: 'wms.write'
          },
          {
            path: '/wms/outbound-confirm',
            name: '出库确认',
            permission: 'wms.write'
          },
          {
            path: '/wms/labor',
            name: '劳务任务',
            permission: 'wms.write'
          },
          {
            path: '/wms/slotting',
            name: '库位优化',
            permission: 'wms.write'
          },
          {
            path: '/wms/replenishment',
            name: '补货任务',
            permission: 'wms.write'
          },
          {
            path: '/wms/dock-yard',
            name: '月台管理',
            permission: 'wms.write'
          }
        ]
      },
      {
        name: '入库',
        icon: <InboxOutlined />,
        children: [
          {
            path: '/wms/inbounds',
            name: '收货入库',
            permission: 'wms.inbounds.read',
            feature: 'inventory.enabled'
          },
          {
            path: '/wms/inbounds/search',
            name: '收货查询',
            permission: 'wms.inbounds.read',
            feature: 'inventory.enabled'
          },
          {
            path: '/wms/inbounds/receive-entry',
            name: '收货登记',
            permission: 'wms.inbounds.write',
            feature: 'inventory.enabled'
          },
          {
            path: '/wms/return-inbounds',
            name: '退货入库',
            permission: 'aftersales.read',
            feature: 'inventory.enabled'
          },
          {
            path: '/wms/inbound-rejects',
            name: '拒收查询',
            permission: 'wms.inbounds.read',
            feature: 'inventory.enabled'
          }
        ]
      },
      {
        name: '库内管理',
        icon: <DatabaseOutlined />,
        children: [
          {
            path: '/inventory/stocktakes',
            name: '盘点管理',
            permission: 'inventory.read'
          },
          {
            path: '/inventory/stocktake-tasks',
            name: '盘点任务',
            permission: 'inventory.read'
          },
          {
            path: '/wms/stock-transfers',
            name: '调拨管理',
            permission: 'inventory.read'
          },
          {
            path: '/wms/inventory-freezes',
            name: '库存冻结',
            permission: 'inventory.read'
          },
          {
            path: '/wms/stock-adjustments',
            name: '报损报溢',
            permission: 'inventory.read'
          },
          {
            path: '/wms/stock-moves',
            name: '移库管理',
            permission: 'inventory.read'
          }
        ]
      },
      {
        name: '策略',
        icon: <ToolOutlined />,
        children: [
          {
            path: '/wms/shipment-preprocess-strategies',
            name: '发货预处理策略',
            permission: 'shipping.read'
          },
          {
            path: '/wms/pick-recommend-strategies',
            name: '拣货推荐策略',
            permission: 'wms.read'
          },
          {
            path: '/wms/stock-count-strategies',
            name: '盘点策略/建议',
            permission: 'inventory.read'
          }
        ]
      }
    ]
  },
  {
    name: '客户',
    icon: <TeamOutlined />,
    children: [
      {
        path: '/customers',
        name: '客户档案',
        permission: 'customers.read'
      },
      {
        path: '/customers/categories',
        name: '客户分类',
        permission: 'customers.read'
      },
      {
        path: '/customers/member-levels',
        name: '会员等级',
        permission: 'customers.read'
      },
      {
        path: '/customers/tags',
        name: '客户标签',
        permission: 'customers.read'
      },
      {
        path: '/customers/loyalty',
        name: '积分与储值',
        permission: 'customers.read'
      },
      {
        path: '/customers/segments',
        name: '客户分群',
        permission: 'customers.read'
      },
      {
        path: '/customers/lifecycle',
        name: '生命周期运营',
        permission: 'customers.read'
      },
      {
        path: '/customers/service-tickets',
        name: '客服工单',
        permission: 'customers.read'
      },
      {
        name: '售后处理',
        icon: <ToolOutlined />,
        children: [
          {
            path: '/after-sales',
            name: '售后单',
            permission: 'aftersales.read'
          },
            {
              path: '/after-sales/intercepts',
              name: '退款拦截',
              permission: 'aftersales.read'
            },
          {
            path: '/after-sales/exchanges',
            name: '换货补发',
            permission: 'aftersales.read'
          },
          {
            path: '/after-sales/inspections',
            name: '退货质检',
            permission: 'aftersales.read'
          },
          {
            path: '/after-sales/sla',
            name: 'SLA配置',
            permission: 'aftersales.read'
          },
          {
            path: '/after-sales/bulk-edit',
            name: '批量导入',
            permission: 'aftersales.read'
          }
        ]
      },
      {
        path: '/after-sale-cases',
        name: '平台售后',
        permission: 'aftersales.read'
      },
        {
          path: '/refund-docs',
          name: '退款处理',
          permission: 'aftersales.read'
        },
      {
        path: '/after-sales/settings',
        name: '售后设置',
        permission: 'SYS.SETTINGS.READ'
      }
    ]
  },
  {
    name: '服务',
    icon: <ToolOutlined />,
    children: [
      {
        path: '/print-jobs',
        name: '打印任务',
        permission: 'print_jobs.read'
      },
      {
        path: '/print-templates',
        name: '打印模板',
        permission: 'print_templates.read'
      }
    ]
  },
  {
    name: '统计',
    icon: <BarChartOutlined />,
    feature: 'ui.enable_stats',
    children: [
      {
        path: '/stats/shipping',
        name: '发货统计',
        permission: 'stats.read'
      },
      {
        path: '/stats/sales',
        name: '销售统计',
        permission: 'stats.read'
      },
      {
        path: '/stats/sales-daily',
        name: '销售日报表',
        permission: 'stats.read'
      },
      {
        path: '/stats/sales-trends',
        name: '销售趋势分析',
        permission: 'stats.read'
      },
      {
        path: '/stats/slow-moving',
        name: '滞销分析',
        permission: 'stats.read'
      },
      {
        path: '/stats/best-sellers',
        name: '畅销分析',
        permission: 'stats.read'
      },
      {
        path: '/stats/sales-cycle',
        name: '销售周期',
        permission: 'stats.read'
      },
      {
        name: '财务',
        icon: <AccountBookOutlined />,
        children: [
          {
            path: '/finance/ar',
            name: '应收管理',
            permission: 'finance.read'
          },
          {
            path: '/finance/ap',
            name: '应付管理',
            permission: 'finance.read'
          },
          {
            path: '/finance/expenses',
            name: '费用分摊',
            permission: 'finance.read'
          },
          {
            path: '/finance/costing',
            name: '成本核算',
            permission: 'finance.read'
          },
          {
            path: '/finance/consignment',
            name: '代销结算',
            permission: 'finance.read'
          },
          {
            path: '/finance/tax',
            name: '税务管理',
            permission: 'finance.read'
          },
          {
            path: '/finance/reconcile',
            name: '对账中心',
            permission: 'finance.read'
          },
          {
            path: '/finance/invoices',
            name: '开票中心',
            permission: 'finance.read'
          },
          {
            path: '/finance/ap-docs',
            name: '应付单据',
            permission: 'finance.read'
          },
          {
            path: '/finance/profit',
            name: '利润报表',
            permission: 'finance.read'
          }
        ]
      },
      {
        path: '/reports',
        name: '经营分析',
        icon: <BarChartOutlined />,
        permission: 'reports.read'
      },
      {
        path: '/exports',
        name: '导出任务',
        icon: <CloudDownloadOutlined />,
        permission: 'exports.read'
      }
    ]
  }
];

