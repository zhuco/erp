import { Suspense, lazy, useEffect } from 'react';
import type { ComponentType } from 'react';
import { Navigate, createBrowserRouter, createHashRouter, useLocation } from 'react-router-dom';

import { AuthGuard } from '@/app/guards/AuthGuard';
import { PermissionGuard } from '@/app/guards/PermissionGuard';
import { TenantGuard } from '@/app/guards/TenantGuard';
import { WorkbenchLayout } from '@/app/layout/WorkbenchLayout';
import {
  markRouteLoadingReady,
  markRouteLoadingStart,
  useRouteLoadingSnapshot,
} from '@/app/navigation/routeLoadingDiagnostics';
import { attachRoutePrefetch, installRoutePrefetchRegistry } from '@/app/navigation/routePrefetch';
import {
  getLegacyRedirectTarget,
  routeAliases,
  toRelativeRoutePath,
} from '@/app/navigation/routeAliases';
import { routerBasePath, routerMode } from '@/utils/env';

function RouteLoadingFallback() {
  const location = useLocation();
  const routeSnapshot = useRouteLoadingSnapshot(location.pathname);

  useEffect(() => {
    markRouteLoadingStart(location.pathname);
  }, [location.pathname]);

  return (
    <div
      data-testid="route-page-loading"
      data-route-loading-status={routeSnapshot.status}
      data-route-loading-path={location.pathname}
    >
      页面加载中...
      {routeSnapshot.status === 'timeout' ? (
        <div data-testid="route-page-loading-hint">当前页面加载较慢，系统仍在继续尝试打开。</div>
      ) : null}
    </div>
  );
}

function lazyNamedPage(loader: () => Promise<Record<string, unknown>>, exportName: string) {
  let modulePromise: Promise<Record<string, unknown>> | null = null;
  const loadModule = () => {
    if (!modulePromise) {
      modulePromise = loader();
    }
    return modulePromise;
  };

  const LazyPage = lazy(async () => {
    const module = await loadModule();
    const component = module[exportName];
    if (!component) {
      throw new Error(`页面导出不存在: ${exportName}`);
    }
    return { default: component as ComponentType };
  });

  function WrappedPage() {
    const location = useLocation();

    useEffect(() => {
      markRouteLoadingReady(location.pathname, {
        routeLabel: exportName,
      });
    }, [location.pathname]);

    return (
      <Suspense fallback={<RouteLoadingFallback />}>
        <LazyPage />
      </Suspense>
    );
  }

  WrappedPage.displayName = `LazyPage(${exportName})`;
  return attachRoutePrefetch(WrappedPage, {
    routeLabel: exportName,
    prefetch: async () => {
      await loadModule();
    },
  });
}

function LegacyRouteRedirect() {
  const location = useLocation();
  const redirectTarget = getLegacyRedirectTarget(
    location.pathname,
    location.search,
    location.hash,
  );

  if (!redirectTarget) {
    return <Navigate to="/404" replace />;
  }

  return <Navigate to={redirectTarget} replace />;
}

const ForbiddenPage = lazyNamedPage(() => import('@/pages/Forbidden'), 'ForbiddenPage');
const LoginPage = lazyNamedPage(() => import('@/pages/LoginPage'), 'LoginPage');
const NotFoundPage = lazyNamedPage(() => import('@/pages/NotFound'), 'NotFoundPage');
const DashboardPage = lazyNamedPage(() => import('@/pages/Dashboard'), 'DashboardPage');
const ChannelDirectoryPage = lazyNamedPage(
  () => import('@/pages/channels/ChannelDirectoryPage'),
  'ChannelDirectoryPage',
);
const ChannelServicePurchasePage = lazyNamedPage(
  () => import('@/pages/channels/ChannelServicePurchasePage'),
  'ChannelServicePurchasePage',
);
const ChannelAuthorizePage = lazyNamedPage(
  () => import('@/pages/channels/ChannelAuthorizePage'),
  'ChannelAuthorizePage',
);
const CouriersPage = lazyNamedPage(() => import('@/pages/couriers/CouriersPage'), 'CouriersPage');
const TagDirectoryPage = lazyNamedPage(
  () => import('@/pages/tags/TagDirectoryPage'),
  'TagDirectoryPage',
);
const WarehouseDirectoryPage = lazyNamedPage(
  () => import('@/pages/warehouses/WarehouseDirectoryPage'),
  'WarehouseDirectoryPage',
);
const LogisticsFeeDirectoryPage = lazyNamedPage(
  () => import('@/pages/logistics/LogisticsFeeDirectoryPage'),
  'LogisticsFeeDirectoryPage',
);
const SuppliersPage = lazyNamedPage(
  () => import('@/pages/suppliers/SuppliersPage'),
  'SuppliersPage',
);
const SalesOrderQueryWorkbenchPage = lazyNamedPage(() => import('@/pages/sales/SalesOrderQueryWorkbenchPage'), 'SalesOrderQueryWorkbenchPage');
const OrderReviewPage = lazyNamedPage(() => import('@/pages/orders/OrderReviewPage'), 'OrderReviewPage');
const OrderRecheckPage = lazyNamedPage(() => import('@/pages/orders/OrderRecheckPage'), 'OrderRecheckPage');
const SalesSettingsPage = lazyNamedPage(() => import('@/pages/sales/SalesSettingsPage'), 'SalesSettingsPage');
const SystemIntegrationsPage = lazyNamedPage(
  () => import('@/pages/system/SystemIntegrationsPage'),
  'SystemIntegrationsPage',
);
const InventoryPage = lazyNamedPage(() => import('@/pages/inventory/InventoryPage'), 'InventoryPage');
const WmsWavesPage = lazyNamedPage(() => import('@/pages/wms/WmsWavesPage'), 'WmsWavesPage');
const WmsShipPage = lazyNamedPage(() => import('@/pages/wms/WmsShipPage'), 'WmsShipPage');
const AfterSalesExchangesPage = lazyNamedPage(
  () => import('@/pages/afterSales/AfterSalesExchangesPage'),
  'AfterSalesExchangesPage',
);
const LogisticsExceptionsPage = lazyNamedPage(
  () => import('@/pages/logistics/LogisticsExceptionsPage'),
  'LogisticsExceptionsPage',
);
const ProfitPage = lazyNamedPage(() => import('@/pages/finance/ProfitPage'), 'ProfitPage');
const PlanExecutionPage = lazyNamedPage(
  () => import('@/pages/planning/PlanExecutionPage'),
  'PlanExecutionPage',
);
const SalesStatsPage = lazyNamedPage(() => import('@/pages/stats/SalesStatsPage'), 'SalesStatsPage');
const ObservabilityPage = lazyNamedPage(
  () => import('@/pages/system/ObservabilityPage'),
  'ObservabilityPage',
);

const legacyRouteRedirects = routeAliases.map(({ legacyPath }) => ({
  path: toRelativeRoutePath(legacyPath),
  element: <LegacyRouteRedirect />,
}));

const routeConfig = [
  {
    path: '/login',
    element: <LoginPage />,
  },
  {
    element: <AuthGuard />,
    children: [
      {
        path: '/select-tenant',
        element: <Navigate to="/dashboard" replace />,
      },
      {
        path: '/403',
        element: <ForbiddenPage />,
      },
      {
        path: '/404',
        element: <NotFoundPage />,
      },
      {
        element: <TenantGuard />,
        children: [
          {
            path: '/',
            element: <WorkbenchLayout />,
            children: [
              {
                index: true,
                element: <Navigate to="/dashboard" replace />,
              },
              {
                path: 'dashboard',
                element: <DashboardPage />,
              },
              {
                path: 'channels',
                element: (
                  <PermissionGuard permission="channels.read">
                    <ChannelDirectoryPage />
                  </PermissionGuard>
                ),
              },
              {
                path: 'channels/service-purchase',
                element: (
                  <PermissionGuard permission="channels.read">
                    <ChannelServicePurchasePage />
                  </PermissionGuard>
                ),
              },
              {
                path: 'channels/authorize',
                element: (
                  <PermissionGuard permission="channels.read">
                    <ChannelAuthorizePage />
                  </PermissionGuard>
                ),
              },
              {
                path: 'couriers',
                element: (
                  <PermissionGuard permission="couriers.read">
                    <CouriersPage />
                  </PermissionGuard>
                ),
              },
              {
                path: 'tags',
                element: <TagDirectoryPage />,
              },
              {
                path: 'warehouses',
                element: (
                  <PermissionGuard permission="warehouses.read">
                    <WarehouseDirectoryPage />
                  </PermissionGuard>
                ),
              },
              {
                path: 'logistics-fees',
                element: <LogisticsFeeDirectoryPage />,
              },
              {
                path: 'suppliers',
                element: (
                  <PermissionGuard permission="suppliers.read">
                    <SuppliersPage />
                  </PermissionGuard>
                ),
              },
              {
                path: 'sales/order-query',
                element: (
                  <PermissionGuard permission="orders.read">
                    <SalesOrderQueryWorkbenchPage />
                  </PermissionGuard>
                ),
              },
              {
                path: 'orders/review',
                element: (
                  <PermissionGuard permission="orders.read">
                    <OrderReviewPage />
                  </PermissionGuard>
                ),
              },
              {
                path: 'orders/recheck',
                element: (
                  <PermissionGuard permission="orders.read">
                    <OrderRecheckPage />
                  </PermissionGuard>
                ),
              },
              {
                path: 'sales/settings',
                element: (
                  <PermissionGuard permission="settings.read">
                    <SalesSettingsPage />
                  </PermissionGuard>
                ),
              },
              {
                path: 'integrations/center',
                element: (
                  <PermissionGuard permission="channels.read">
                    <SystemIntegrationsPage />
                  </PermissionGuard>
                ),
              },
              {
                path: 'inventory',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <InventoryPage />
                  </PermissionGuard>
                ),
              },
              {
                path: 'wms/waves',
                element: (
                  <PermissionGuard permission="wms.wave.read">
                    <WmsWavesPage />
                  </PermissionGuard>
                ),
              },
              {
                path: 'wms/ship',
                element: (
                  <PermissionGuard permission="shipping.read">
                    <WmsShipPage />
                  </PermissionGuard>
                ),
              },
              {
                path: 'after-sales/exchanges',
                element: (
                  <PermissionGuard permission="aftersales.read">
                    <AfterSalesExchangesPage />
                  </PermissionGuard>
                ),
              },
              {
                path: 'logistics/exceptions',
                element: (
                  <PermissionGuard permission="shipping.read">
                    <LogisticsExceptionsPage />
                  </PermissionGuard>
                ),
              },
              {
                path: 'finance/profit',
                element: (
                  <PermissionGuard permission="finance.read">
                    <ProfitPage />
                  </PermissionGuard>
                ),
              },
              {
                path: 'planning/plan-execution',
                element: (
                  <PermissionGuard permission="purchase_suggestions.read">
                    <PlanExecutionPage />
                  </PermissionGuard>
                ),
              },
              {
                path: 'stats/sales',
                element: (
                  <PermissionGuard permission="stats.read">
                    <SalesStatsPage />
                  </PermissionGuard>
                ),
              },
              {
                path: 'system/observability',
                element: (
                  <PermissionGuard permission="settings.read">
                    <ObservabilityPage />
                  </PermissionGuard>
                ),
              },
              ...legacyRouteRedirects,
              {
                path: '*',
                element: <NotFoundPage />,
              },
            ],
          },
        ],
      },
    ],
  },
];

installRoutePrefetchRegistry(routeConfig);

const createRouter = routerMode === 'hash' ? createHashRouter : createBrowserRouter;
const routerBasename = routerMode === 'hash' ? '/' : routerBasePath;

export const router = createRouter(routeConfig, { basename: routerBasename });
