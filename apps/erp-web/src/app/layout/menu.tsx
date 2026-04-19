import { AppstoreOutlined, DashboardOutlined, ShoppingOutlined } from '@ant-design/icons';
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
    path: '/channels',
    icon: <AppstoreOutlined />,
    permission: 'channels.read',
  },
  {
    name: '销售',
    icon: <ShoppingOutlined />,
    children: [
      group('订单中心', [
        leaf('/orders/review', '审核订单', { permission: 'orders.read' }),
        leaf('/orders/recheck', '订单复核', { permission: 'orders.read' }),
      ]),
      group('业务设置', [
        leaf('/sales/settings', '销售设置', { permission: 'settings.read' }),
      ]),
    ],
  },
];
