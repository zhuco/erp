import { appMenuItems, flattenMenuItems } from '@/app/layout/menu';

export type RouteMaturity = 'draft' | 'wired' | 'p0-ready' | 'deprecated';

export type AppRouteRegistration = {
  path: string;
  title: string;
  permission?: string;
  maturity: RouteMaturity;
  usesPlaceholder?: boolean;
};

const normalizeRoutePath = (path: string): string => {
  if (!path) {
    return '/';
  }
  return path.startsWith('/') ? path : `/${path}`;
};

export const appRouteRegistry: AppRouteRegistration[] = [
  { path: '/dashboard', title: '首页', maturity: 'p0-ready' },
  { path: '/channels', title: '渠道目录', permission: 'channels.read', maturity: 'p0-ready' },
  {
    path: '/channels/service-purchase',
    title: '服务订购',
    permission: 'channels.read',
    maturity: 'wired',
  },
  {
    path: '/channels/authorize',
    title: '渠道授权',
    permission: 'channels.read',
    maturity: 'wired',
  },
  { path: '/couriers', title: '物流目录', permission: 'couriers.read', maturity: 'p0-ready' },
  { path: '/tags', title: '记号目录', maturity: 'p0-ready' },
  { path: '/warehouses', title: '仓库目录', permission: 'warehouses.read', maturity: 'p0-ready' },
  { path: '/logistics-fees', title: '物流费目录', maturity: 'p0-ready' },
  { path: '/suppliers', title: '供应商目录', permission: 'suppliers.read', maturity: 'p0-ready' },
  { path: '/sales/order-query', title: '订单查询', permission: 'orders.read', maturity: 'p0-ready' },
  { path: '/orders/review', title: '订单审核', permission: 'orders.read', maturity: 'p0-ready' },
  { path: '/orders/recheck', title: '订单复核', permission: 'orders.read', maturity: 'p0-ready' },
  { path: '/sales/settings', title: '销售设置', permission: 'settings.read', maturity: 'p0-ready' },
  { path: '/integrations/center', title: '接口集成', permission: 'channels.read', maturity: 'p0-ready' },
  { path: '/inventory', title: '库存查询', permission: 'inventory.read', maturity: 'p0-ready' },
  { path: '/wms/waves', title: '波次执行', permission: 'wms.wave.read', maturity: 'p0-ready' },
  { path: '/wms/ship', title: '出库发货', permission: 'shipping.read', maturity: 'p0-ready' },
  {
    path: '/after-sales/exchanges',
    title: '退换补发',
    permission: 'aftersales.read',
    maturity: 'p0-ready',
  },
  {
    path: '/logistics/exceptions',
    title: '物流异常',
    permission: 'shipping.read',
    maturity: 'p0-ready',
  },
  { path: '/finance/profit', title: '利润分析', permission: 'finance.read', maturity: 'p0-ready' },
  {
    path: '/planning/plan-execution',
    title: '计划执行',
    permission: 'purchase_suggestions.read',
    maturity: 'p0-ready',
  },
  { path: '/stats/sales', title: '销售统计', permission: 'stats.read', maturity: 'p0-ready' },
  { path: '/system/observability', title: '系统观测', permission: 'settings.read', maturity: 'p0-ready' },
];

export const registeredRoutePathSet = new Set(
  appRouteRegistry.map((route) => normalizeRoutePath(route.path)),
);

export const p0RouteRegistrations = appRouteRegistry.filter(
  (route) => route.maturity === 'p0-ready',
);

export const draftRouteRegistrations = appRouteRegistry.filter(
  (route) => route.maturity === 'draft',
);

export const findUnregisteredMenuPaths = (): string[] =>
  flattenMenuItems(appMenuItems)
    .map((item) => normalizeRoutePath(item.path))
    .filter((path) => !registeredRoutePathSet.has(path));

export const findP0PlaceholderRoutes = (): AppRouteRegistration[] =>
  p0RouteRegistrations.filter((route) => route.usesPlaceholder === true);
