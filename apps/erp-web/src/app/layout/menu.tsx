import {
  AppstoreOutlined,
  BarChartOutlined,
  DashboardOutlined,
  DollarOutlined,
  SafetyCertificateOutlined,
  ShoppingOutlined,
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
    name: '目录',
    icon: <AppstoreOutlined />,
    children: [
      group('基础目录', [
        leaf('/channels', '渠道目录', { permission: 'channels.read' }),
        leaf('/couriers', '物流目录', { permission: 'couriers.read' }),
        leaf('/tags', '标签目录'),
        leaf('/warehouses', '仓库目录', { permission: 'warehouses.read' }),
        leaf('/logistics-fees', '物流费目录'),
        leaf('/suppliers', '供应商目录', { permission: 'suppliers.read' }),
      ]),
      group('客户目录', [
        leaf('/customers/categories', '客户分类目录', { permission: 'customers.read' }),
        leaf('/customers/member-levels', '会员等级目录', { permission: 'customers.read' }),
        leaf('/customers/tags', '客户标签目录', { permission: 'customers.read' }),
      ]),
      group('WMS 主数据', [
        leaf('/wms/containers', '容器目录', { permission: 'inventory.read' }),
        leaf('/wms/sender-addresses', '寄方目录', { permission: 'shipping.read' }),
        leaf('/wms/warehouse-areas', '库区目录', { permission: 'inventory.read' }),
        leaf('/wms/warehouse-locations', '货位目录', { permission: 'inventory.read' }),
      ]),
      group('系统目录', [
        leaf('/system/permissions', '权限目录', { permission: 'permissions.read' }),
        leaf('/system/roles', '角色目录', { permission: 'roles.read' }),
      ]),
    ],
  },
  {
    name: '销售',
    icon: <ShoppingOutlined />,
    children: [
      group('订单中心', [
        leaf('/sales/order-query', '订单查询', { permission: 'orders.read' }),
        leaf('/orders/review', '订单审核', { permission: 'orders.read' }),
        leaf('/orders/recheck', '订单复核', { permission: 'orders.read' }),
      ]),
      group('业务设置', [
        leaf('/sales/settings', '销售设置', { permission: 'settings.read' }),
      ]),
    ],
  },
  {
    name: '履约',
    icon: <SafetyCertificateOutlined />,
    children: [
      group('库存与仓储', [
        leaf('/inventory', '库存查询', { permission: 'inventory.read' }),
        leaf('/wms/waves', '波次执行', { permission: 'wms.wave.read' }),
        leaf('/wms/ship', '出库发货', { permission: 'shipping.read' }),
        leaf('/logistics/exceptions', '物流异常', { permission: 'shipping.read' }),
      ]),
      group('售后履约', [
        leaf('/after-sales/exchanges', '退换补发', { permission: 'aftersales.read' }),
      ]),
    ],
  },
  {
    name: '经营',
    icon: <DollarOutlined />,
    children: [
      group('财务与计划', [
        leaf('/finance/profit', '利润分析', { permission: 'finance.read' }),
        leaf('/planning/plan-execution', '计划执行', {
          permission: 'purchase_suggestions.read',
        }),
      ]),
    ],
  },
  {
    name: '报表',
    icon: <BarChartOutlined />,
    children: [
      group('统计与治理', [
        leaf('/stats/sales', '销售统计', { permission: 'stats.read' }),
        leaf('/integrations/center', '接口集成', { permission: 'channels.read' }),
        leaf('/system/observability', '系统观测', { permission: 'settings.read' }),
      ]),
    ],
  },
];
