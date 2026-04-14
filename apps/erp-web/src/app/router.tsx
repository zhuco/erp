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
import {
  attachRoutePrefetch,
  installRoutePrefetchRegistry,
} from '@/app/navigation/routePrefetch';
import {
  getLegacyRedirectTarget,
  routeAliases,
  toRelativeRoutePath
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

const AfterSalesDetailPage = lazyNamedPage(() => import('@/pages/afterSales/AfterSalesDetailPage'), 'AfterSalesDetailPage');
const AfterSalesExchangesPage = lazyNamedPage(() => import('@/pages/afterSales/AfterSalesExchangesPage'), 'AfterSalesExchangesPage');
const AfterSalesBulkEditPage = lazyNamedPage(() => import('@/pages/afterSales/AfterSalesBulkEditPage'), 'AfterSalesBulkEditPage');
const AfterSalesInterceptsPage = lazyNamedPage(() => import('@/pages/afterSales/AfterSalesInterceptsPage'), 'AfterSalesInterceptsPage');
const AfterSalesInspectionsPage = lazyNamedPage(() => import('@/pages/afterSales/AfterSalesInspectionsPage'), 'AfterSalesInspectionsPage');
const AfterSalesListPage = lazyNamedPage(() => import('@/pages/afterSales/AfterSalesListPage'), 'AfterSalesListPage');
const AfterSalesReceivePage = lazyNamedPage(() => import('@/pages/afterSales/AfterSalesReceivePage'), 'AfterSalesReceivePage');
const AfterSalesSlaPage = lazyNamedPage(() => import('@/pages/afterSales/AfterSalesSlaPage'), 'AfterSalesSlaPage');
const AfterSalesSettingsPage = lazyNamedPage(() => import('@/pages/afterSales/AfterSalesSettingsPage'), 'AfterSalesSettingsPage');
const AfterSaleCaseDetailPage = lazyNamedPage(() => import('@/pages/afterSales/AfterSaleCaseDetailPage'), 'AfterSaleCaseDetailPage');
const AfterSaleCaseListPage = lazyNamedPage(() => import('@/pages/afterSales/AfterSaleCaseListPage'), 'AfterSaleCaseListPage');
const RefundDocsPage = lazyNamedPage(() => import('@/pages/afterSales/RefundDocsPage'), 'RefundDocsPage');
const ForbiddenPage = lazyNamedPage(() => import('@/pages/Forbidden'), 'ForbiddenPage');
const LoginPage = lazyNamedPage(() => import('@/pages/LoginPage'), 'LoginPage');
const NotFoundPage = lazyNamedPage(() => import('@/pages/NotFound'), 'NotFoundPage');
const FailureQueuePage = lazyNamedPage(() => import('@/pages/failureQueue/FailureQueuePage'), 'FailureQueuePage');
const DashboardPage = lazyNamedPage(() => import('@/pages/Dashboard'), 'DashboardPage');
const InventoryCostsPage = lazyNamedPage(() => import('@/pages/inventory/InventoryCostsPage'), 'InventoryCostsPage');
const InventoryLedgerPage = lazyNamedPage(() => import('@/pages/inventory/InventoryLedgerPage'), 'InventoryLedgerPage');
const InventoryLocksPage = lazyNamedPage(() => import('@/pages/inventory/InventoryLocksPage'), 'InventoryLocksPage');
const InventoryPage = lazyNamedPage(() => import('@/pages/inventory/InventoryPage'), 'InventoryPage');
const InventoryAlertsPage = lazyNamedPage(() => import('@/pages/inventory/InventoryAlertsPage'), 'InventoryAlertsPage');
const InventorySummaryPage = lazyNamedPage(() => import('@/pages/inventory/InventorySummaryPage'), 'InventorySummaryPage');
const InventorySyncJobsPage = lazyNamedPage(() => import('@/pages/inventory/InventorySyncJobsPage'), 'InventorySyncJobsPage');
const StocktakesPage = lazyNamedPage(() => import('@/pages/inventory/StocktakesPage'), 'StocktakesPage');
const StocktakeTasksPage = lazyNamedPage(() => import('@/pages/inventory/StocktakeTasksPage'), 'StocktakeTasksPage');
const InboundDetailPage = lazyNamedPage(() => import('@/pages/inbounds/InboundDetailPage'), 'InboundDetailPage');
const InboundInspectPage = lazyNamedPage(() => import('@/pages/inbounds/InboundInspectPage'), 'InboundInspectPage');
const InboundListPage = lazyNamedPage(() => import('@/pages/inbounds/InboundListPage'), 'InboundListPage');
const InboundPutawayPage = lazyNamedPage(() => import('@/pages/inbounds/InboundPutawayPage'), 'InboundPutawayPage');
const InboundReceivePage = lazyNamedPage(() => import('@/pages/inbounds/InboundReceivePage'), 'InboundReceivePage');
const InboundQuickReceivePage = lazyNamedPage(() => import('@/pages/inbounds/InboundQuickReceivePage'), 'InboundQuickReceivePage');
const InboundRejectsPage = lazyNamedPage(() => import('@/pages/inbounds/InboundRejectsPage'), 'InboundRejectsPage');
const InboundSearchPage = lazyNamedPage(() => import('@/pages/inbounds/InboundSearchPage'), 'InboundSearchPage');
const ReturnInboundPage = lazyNamedPage(() => import('@/pages/inbounds/ReturnInboundPage'), 'ReturnInboundPage');
const CouriersPage = lazyNamedPage(() => import('@/pages/couriers/CouriersPage'), 'CouriersPage');
const LogisticsExceptionsPage = lazyNamedPage(() => import('@/pages/logistics/LogisticsExceptionsPage'), 'LogisticsExceptionsPage');
const LogisticsInterfacesPage = lazyNamedPage(() => import('@/pages/logistics/LogisticsInterfacesPage'), 'LogisticsInterfacesPage');
const LogisticsProductsPage = lazyNamedPage(() => import('@/pages/logistics/LogisticsProductsPage'), 'LogisticsProductsPage');
const LogisticsPricesPage = lazyNamedPage(() => import('@/pages/logistics/LogisticsPricesPage'), 'LogisticsPricesPage');
const LogisticsInstallationsPage = lazyNamedPage(() => import('@/pages/logistics/LogisticsInstallationsPage'), 'LogisticsInstallationsPage');
const LogisticsSettlementsPage = lazyNamedPage(() => import('@/pages/logistics/LogisticsSettlementsPage'), 'LogisticsSettlementsPage');
const WarehouseOptimizationPage = lazyNamedPage(() => import('@/pages/logistics/WarehouseOptimizationPage'), 'WarehouseOptimizationPage');
const WarehousesPage = lazyNamedPage(() => import('@/pages/warehouses/WarehousesPage'), 'WarehousesPage');
const ReportsPage = lazyNamedPage(() => import('@/pages/reports/ReportsPage'), 'ReportsPage');
const ExportJobsPage = lazyNamedPage(() => import('@/pages/exports/ExportJobsPage'), 'ExportJobsPage');
const KitEditorPage = lazyNamedPage(() => import('@/pages/kits/KitEditorPage'), 'KitEditorPage');
const KitsPage = lazyNamedPage(() => import('@/pages/kits/KitsPage'), 'KitsPage');
const ShipmentsPage = lazyNamedPage(() => import('@/pages/shipments/ShipmentsPage'), 'ShipmentsPage');
const OmsShipmentsPage = lazyNamedPage(() => import('@/pages/shipments/OmsShipmentsPage'), 'OmsShipmentsPage');
const LogisticsTrackingPage = lazyNamedPage(() => import('@/pages/logistics/LogisticsTrackingPage'), 'LogisticsTrackingPage');
const ShipmentPushFailuresPage = lazyNamedPage(() => import('@/pages/shipments/ShipmentPushFailuresPage'), 'ShipmentPushFailuresPage');
const OrderDetailPage = lazyNamedPage(() => import('@/pages/orders/OrderDetailPage'), 'OrderDetailPage');
const OrdersPage = lazyNamedPage(() => import('@/pages/orders/OrdersPage'), 'OrdersPage');
const OrderExceptionsPage = lazyNamedPage(() => import('@/pages/orders/OrderExceptionsPage'), 'OrderExceptionsPage');
const OrderReconciliationsPage = lazyNamedPage(() => import('@/pages/orders/OrderReconciliationsPage'), 'OrderReconciliationsPage');
const OrderRecheckPage = lazyNamedPage(() => import('@/pages/orders/OrderRecheckPage'), 'OrderRecheckPage');
const OrderReviewPage = lazyNamedPage(() => import('@/pages/orders/OrderReviewPage'), 'OrderReviewPage');
const OrderReviewRulesPage = lazyNamedPage(() => import('@/pages/orders/OrderReviewRulesPage'), 'OrderReviewRulesPage');
const OrderSettlementPage = lazyNamedPage(() => import('@/pages/orders/OrderSettlementPage'), 'OrderSettlementPage');
const SalesOrderCreatePage = lazyNamedPage(() => import('@/pages/sales/SalesOrderCreatePage'), 'SalesOrderCreatePage');
const SalesOrderImportPage = lazyNamedPage(() => import('@/pages/sales/SalesOrderImportPage'), 'SalesOrderImportPage');
const SalesSettingsPage = lazyNamedPage(() => import('@/pages/sales/SalesSettingsPage'), 'SalesSettingsPage');
const SalesStrategySettingsPage = lazyNamedPage(() => import('@/pages/sales/SalesStrategySettingsPage'), 'SalesStrategySettingsPage');
const ShopOrderDownloadPage = lazyNamedPage(
  () => import('@/pages/sales/SalesEcommercePlaceholderPages'),
  'ShopOrderDownloadPage'
);
const ShopProductMatchQueryPage = lazyNamedPage(
  () => import('@/pages/sales/SalesEcommercePlaceholderPages'),
  'ShopProductMatchQueryPage'
);
const ShopInventorySyncPage = lazyNamedPage(
  () => import('@/pages/sales/SalesEcommercePlaceholderPages'),
  'ShopInventorySyncPage'
);
const PrintJobsPage = lazyNamedPage(() => import('@/pages/printJobs/PrintJobsPage'), 'PrintJobsPage');
const PrintTemplatesPage = lazyNamedPage(() => import('@/pages/printTemplates/PrintTemplatesPage'), 'PrintTemplatesPage');
const InventoryFreezesPage = lazyNamedPage(() => import('@/pages/wms/InventoryFreezesPage'), 'InventoryFreezesPage');
const PickRecommendStrategiesPage = lazyNamedPage(() => import('@/pages/wms/PickRecommendStrategiesPage'), 'PickRecommendStrategiesPage');
const ShipmentPreprocessStrategiesPage = lazyNamedPage(() => import('@/pages/wms/ShipmentPreprocessStrategiesPage'), 'ShipmentPreprocessStrategiesPage');
const StockAdjustmentsPage = lazyNamedPage(() => import('@/pages/wms/StockAdjustmentsPage'), 'StockAdjustmentsPage');
const StockCountStrategiesPage = lazyNamedPage(() => import('@/pages/wms/StockCountStrategiesPage'), 'StockCountStrategiesPage');
const StockMovesPage = lazyNamedPage(() => import('@/pages/wms/StockMovesPage'), 'StockMovesPage');
const StockTransfersPage = lazyNamedPage(() => import('@/pages/wms/StockTransfersPage'), 'StockTransfersPage');
const WmsOutboundsPage = lazyNamedPage(() => import('@/pages/wms/WmsOutboundsPage'), 'WmsOutboundsPage');
const WmsWavesPage = lazyNamedPage(() => import('@/pages/wms/WmsWavesPage'), 'WmsWavesPage');
const WmsPickTasksPage = lazyNamedPage(() => import('@/pages/wms/WmsPickTasksPage'), 'WmsPickTasksPage');
const WmsPackPage = lazyNamedPage(() => import('@/pages/wms/WmsPackPage'), 'WmsPackPage');
const WmsShipPage = lazyNamedPage(() => import('@/pages/wms/WmsShipPage'), 'WmsShipPage');
const WmsOutboundConfirmPage = lazyNamedPage(() => import('@/pages/wms/WmsOutboundConfirmPage'), 'WmsOutboundConfirmPage');
const WmsLaborPage = lazyNamedPage(() => import('@/pages/wms/WmsLaborPage'), 'WmsLaborPage');
const WmsSlottingPage = lazyNamedPage(() => import('@/pages/wms/WmsSlottingPage'), 'WmsSlottingPage');
const WmsReplenishmentPage = lazyNamedPage(() => import('@/pages/wms/WmsReplenishmentPage'), 'WmsReplenishmentPage');
const WmsDockYardPage = lazyNamedPage(() => import('@/pages/wms/WmsDockYardPage'), 'WmsDockYardPage');
const WarehouseAreasPage = lazyNamedPage(() => import('@/pages/wms/WarehouseAreasPage'), 'WarehouseAreasPage');
const WarehouseLocationsPage = lazyNamedPage(() => import('@/pages/wms/WarehouseLocationsPage'), 'WarehouseLocationsPage');
const ContainersPage = lazyNamedPage(() => import('@/pages/wms/ContainersPage'), 'ContainersPage');
const SenderAddressesPage = lazyNamedPage(() => import('@/pages/wms/SenderAddressesPage'), 'SenderAddressesPage');
const LocationBalancesPage = lazyNamedPage(() => import('@/pages/wms/LocationBalancesPage'), 'LocationBalancesPage');
const WorkflowConfigPage = lazyNamedPage(() => import('@/pages/wms/WorkflowConfigPage'), 'WorkflowConfigPage');
const SuppliersPage = lazyNamedPage(() => import('@/pages/suppliers/SuppliersPage'), 'SuppliersPage');
const CustomerCategoriesPage = lazyNamedPage(() => import('@/pages/customers/CustomerCategoriesPage'), 'CustomerCategoriesPage');
const CustomerDetailPage = lazyNamedPage(() => import('@/pages/customers/CustomerDetailPage'), 'CustomerDetailPage');
const CustomerLifecyclePage = lazyNamedPage(() => import('@/pages/customers/CustomerLifecyclePage'), 'CustomerLifecyclePage');
const CustomerLoyaltyPage = lazyNamedPage(() => import('@/pages/customers/CustomerLoyaltyPage'), 'CustomerLoyaltyPage');
const CustomerSegmentsPage = lazyNamedPage(() => import('@/pages/customers/CustomerSegmentsPage'), 'CustomerSegmentsPage');
const CustomerServiceTicketsPage = lazyNamedPage(() => import('@/pages/customers/CustomerServiceTicketsPage'), 'CustomerServiceTicketsPage');
const CustomersPage = lazyNamedPage(() => import('@/pages/customers/CustomersPage'), 'CustomersPage');
const CustomerTagsPage = lazyNamedPage(() => import('@/pages/customers/CustomerTagsPage'), 'CustomerTagsPage');
const MemberLevelsPage = lazyNamedPage(() => import('@/pages/customers/MemberLevelsPage'), 'MemberLevelsPage');
const PurchaseOrdersPage = lazyNamedPage(() => import('@/pages/procurement/PurchaseOrdersPage'), 'PurchaseOrdersPage');
const PurchaseOrderDetailPage = lazyNamedPage(() => import('@/pages/procurement/PurchaseOrderDetailPage'), 'PurchaseOrderDetailPage');
const PurchaseReceiptsPage = lazyNamedPage(() => import('@/pages/procurement/PurchaseReceiptsPage'), 'PurchaseReceiptsPage');
const PurchaseSuggestionsPage = lazyNamedPage(() => import('@/pages/procurement/PurchaseSuggestionsPage'), 'PurchaseSuggestionsPage');
const PurchaseReturnsPage = lazyNamedPage(() => import('@/pages/procurement/PurchaseReturnsPage'), 'PurchaseReturnsPage');
const ProcurementRequestsPage = lazyNamedPage(() => import('@/pages/procurement/ProcurementRequestsPage'), 'ProcurementRequestsPage');
const ProcurementPlansPage = lazyNamedPage(() => import('@/pages/procurement/ProcurementPlansPage'), 'ProcurementPlansPage');
const ProcurementSourcingPage = lazyNamedPage(() => import('@/pages/procurement/ProcurementSourcingPage'), 'ProcurementSourcingPage');
const ProcurementInspectionsPage = lazyNamedPage(() => import('@/pages/procurement/ProcurementInspectionsPage'), 'ProcurementInspectionsPage');
const ProcurementReconciliationsPage = lazyNamedPage(() => import('@/pages/procurement/ProcurementReconciliationsPage'), 'ProcurementReconciliationsPage');
const SupplierPerformancePage = lazyNamedPage(() => import('@/pages/procurement/SupplierPerformancePage'), 'SupplierPerformancePage');
const ProductBarcodesPage = lazyNamedPage(() => import('@/pages/products/ProductBarcodesPage'), 'ProductBarcodesPage');
const ProductChannelAvailabilityPage = lazyNamedPage(() => import('@/pages/products/ProductChannelAvailabilityPage'), 'ProductChannelAvailabilityPage');
const ProductChangeControlPage = lazyNamedPage(() => import('@/pages/products/ProductChangeControlPage'), 'ProductChangeControlPage');
const ProductDiffPage = lazyNamedPage(() => import('@/pages/products/ProductDiffPage'), 'ProductDiffPage');
const ProductLifecyclePage = lazyNamedPage(() => import('@/pages/products/ProductLifecyclePage'), 'ProductLifecyclePage');
const ProductPackagingSpecsPage = lazyNamedPage(() => import('@/pages/products/ProductPackagingSpecsPage'), 'ProductPackagingSpecsPage');
const ProductsPage = lazyNamedPage(() => import('@/pages/products/ProductsPage'), 'ProductsPage');
const ProductUnitsPage = lazyNamedPage(() => import('@/pages/products/ProductUnitsPage'), 'ProductUnitsPage');
const SyncJobsPage = lazyNamedPage(() => import('@/pages/shops/SyncJobsPage'), 'SyncJobsPage');
const ShopProductDownloadPage = lazyNamedPage(() => import('@/pages/shopProducts/ShopProductDownloadPage'), 'ShopProductDownloadPage');
const OrderTagDefinitionsPage = lazyNamedPage(() => import('@/pages/tags/OrderTagDefinitionsPage'), 'OrderTagDefinitionsPage');
const SkuTagDefinitionsPage = lazyNamedPage(() => import('@/pages/tags/SkuTagDefinitionsPage'), 'SkuTagDefinitionsPage');
const TagDirectoryPage = lazyNamedPage(() => import('@/pages/tags/TagDirectoryPage'), 'TagDirectoryPage');

const AuditLogsPage = lazyNamedPage(() => import('@/pages/system/AuditLogsPage'), 'AuditLogsPage');
const DataScopesPage = lazyNamedPage(() => import('@/pages/system/DataScopesPage'), 'DataScopesPage');
const ObservabilityPage = lazyNamedPage(
  () => import('@/pages/system/ObservabilityPage'),
  'ObservabilityPage'
);
const SystemAuditReportsPage = lazyNamedPage(
  () => import('@/pages/system/SystemAuditReportsPage'),
  'SystemAuditReportsPage'
);
const SystemCustomFieldsPage = lazyNamedPage(
  () => import('@/pages/system/SystemCustomFieldsPage'),
  'SystemCustomFieldsPage'
);
const SystemIntegrationsPage = lazyNamedPage(
  () => import('@/pages/system/SystemIntegrationsPage'),
  'SystemIntegrationsPage'
);
const SystemSyncJobsPage = lazyNamedPage(
  () => import('@/pages/system/SystemSyncJobsPage'),
  'SystemSyncJobsPage'
);
const AgGridEnterpriseDemoPage = lazyNamedPage(
  () => import('@/pages/system/AgGridEnterpriseDemoPage'),
  'AgGridEnterpriseDemoPage'
);
const SystemWorkflowsPage = lazyNamedPage(
  () => import('@/pages/system/SystemWorkflowsPage'),
  'SystemWorkflowsPage'
);
const OperationLogsPage = lazyNamedPage(
  () => import('@/pages/system/OperationLogsPage'),
  'OperationLogsPage'
);
const PermissionsPage = lazyNamedPage(
  () => import('@/pages/system/PermissionsPage'),
  'PermissionsPage'
);
const ProfilePage = lazyNamedPage(() => import('@/pages/system/ProfilePage'), 'ProfilePage');
const PrintDevicesPage = lazyNamedPage(
  () => import('@/pages/system/PrintDevicesPage'),
  'PrintDevicesPage'
);
const PrintMappingsPage = lazyNamedPage(
  () => import('@/pages/system/PrintMappingsPage'),
  'PrintMappingsPage'
);
const RolesPage = lazyNamedPage(() => import('@/pages/system/RolesPage'), 'RolesPage');
const SystemSettingsPage = lazyNamedPage(
  () => import('@/pages/system/SystemSettingsPage'),
  'SystemSettingsPage'
);
const UsersPage = lazyNamedPage(() => import('@/pages/system/UsersPage'), 'UsersPage');
const ChannelDirectoryPage = lazyNamedPage(
  () => import('@/pages/channels/ChannelDirectoryPage'),
  'ChannelDirectoryPage'
);
const ShopAccessPage = lazyNamedPage(
  () => import('@/pages/channels/ShopAccessPage'),
  'ShopAccessPage'
);
const SalesProductMatchPage = lazyNamedPage(
  () => import('@/pages/sales/SalesProductMatchPage'),
  'SalesProductMatchPage'
);
const ShopOrderQueryPage = lazyNamedPage(
  () => import('@/pages/sales/ShopOrderQueryPage'),
  'ShopOrderQueryPage'
);
const LogisticsUploadPage = lazyNamedPage(
  () => import('@/pages/sales/LogisticsUploadPage'),
  'LogisticsUploadPage'
);
const SyncJobRunsPage = lazyNamedPage(
  () => import('@/pages/sales/SyncJobRunsPage'),
  'SyncJobRunsPage'
);

const MarketingCouponsPage = lazyNamedPage(
  () => import('@/pages/marketing/MarketingCouponsPage'),
  'MarketingCouponsPage'
);
const MarketingCostsPage = lazyNamedPage(
  () => import('@/pages/marketing/MarketingCostsPage'),
  'MarketingCostsPage'
);
const MarketingGiftPoliciesPage = lazyNamedPage(
  () => import('@/pages/marketing/MarketingGiftPoliciesPage'),
  'MarketingGiftPoliciesPage'
);
const MarketingPostmortemsPage = lazyNamedPage(
  () => import('@/pages/marketing/MarketingPostmortemsPage'),
  'MarketingPostmortemsPage'
);
const MarketingPriceListsPage = lazyNamedPage(
  () => import('@/pages/marketing/MarketingPriceListsPage'),
  'MarketingPriceListsPage'
);
const MarketingPromotionsPage = lazyNamedPage(
  () => import('@/pages/marketing/MarketingPromotionsPage'),
  'MarketingPromotionsPage'
);

const ShippingStatsPage = lazyNamedPage(
  () => import('@/pages/stats/ShippingStatsPage'),
  'ShippingStatsPage'
);
const SalesStatsPage = lazyNamedPage(() => import('@/pages/stats/SalesStatsPage'), 'SalesStatsPage');
const SalesDailyReportPage = lazyNamedPage(
  () => import('@/pages/stats/SalesDailyReportPage'),
  'SalesDailyReportPage'
);
const SalesTrendAnalysisPage = lazyNamedPage(
  () => import('@/pages/stats/SalesTrendAnalysisPage'),
  'SalesTrendAnalysisPage'
);
const SlowMovingAnalysisPage = lazyNamedPage(
  () => import('@/pages/stats/SlowMovingAnalysisPage'),
  'SlowMovingAnalysisPage'
);
const BestSellerAnalysisPage = lazyNamedPage(
  () => import('@/pages/stats/BestSellerAnalysisPage'),
  'BestSellerAnalysisPage'
);
const SalesCyclePage = lazyNamedPage(() => import('@/pages/stats/SalesCyclePage'), 'SalesCyclePage');

const ApDocsPage = lazyNamedPage(() => import('@/pages/finance/ApDocsPage'), 'ApDocsPage');
const ApManagementPage = lazyNamedPage(
  () => import('@/pages/finance/ApManagementPage'),
  'ApManagementPage'
);
const ArManagementPage = lazyNamedPage(
  () => import('@/pages/finance/ArManagementPage'),
  'ArManagementPage'
);
const ConsignmentPage = lazyNamedPage(
  () => import('@/pages/finance/ConsignmentPage'),
  'ConsignmentPage'
);
const CostingPage = lazyNamedPage(() => import('@/pages/finance/CostingPage'), 'CostingPage');
const ExpenseAllocationPage = lazyNamedPage(
  () => import('@/pages/finance/ExpenseAllocationPage'),
  'ExpenseAllocationPage'
);
const InvoicesPage = lazyNamedPage(() => import('@/pages/finance/InvoicesPage'), 'InvoicesPage');
const ProfitPage = lazyNamedPage(() => import('@/pages/finance/ProfitPage'), 'ProfitPage');
const ReconcilePage = lazyNamedPage(() => import('@/pages/finance/ReconcilePage'), 'ReconcilePage');
const TaxManagementPage = lazyNamedPage(
  () => import('@/pages/finance/TaxManagementPage'),
  'TaxManagementPage'
);

const PlanExecutionPage = lazyNamedPage(
  () => import('@/pages/planning/PlanExecutionPage'),
  'PlanExecutionPage'
);
const PromoStockingPage = lazyNamedPage(
  () => import('@/pages/planning/PromoStockingPage'),
  'PromoStockingPage'
);
const PurchasePlansPage = lazyNamedPage(
  () => import('@/pages/planning/PurchasePlansPage'),
  'PurchasePlansPage'
);
const ReplenishmentRulesPage = lazyNamedPage(
  () => import('@/pages/planning/ReplenishmentRulesPage'),
  'ReplenishmentRulesPage'
);
const SalesForecastPage = lazyNamedPage(
  () => import('@/pages/planning/SalesForecastPage'),
  'SalesForecastPage'
);
const TransferSuggestionsPage = lazyNamedPage(
  () => import('@/pages/planning/TransferSuggestionsPage'),
  'TransferSuggestionsPage'
);

function LegacyRouteRedirect() {
  const location = useLocation();
  const target = getLegacyRedirectTarget(location.pathname, location.search, location.hash);
  return target ? <Navigate to={target} replace /> : <NotFoundPage />;
}

const legacyRouteRedirects = routeAliases.map(({ legacyPath }) => ({
  path: toRelativeRoutePath(legacyPath),
  element: <LegacyRouteRedirect />
}));

const routeConfig = [
  {
    path: '/login',
    element: <LoginPage />
  },
  {
    element: <AuthGuard />,
    children: [
      {
        path: '/select-tenant',
        element: <Navigate to="/dashboard" replace />
      },
      {
        path: '/403',
        element: <ForbiddenPage />
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
                element: <Navigate to="dashboard" replace />
              },
              {
                path: 'dashboard',
                element: <DashboardPage />
              },
              {
                path: 'orders',
                element: (
                  <PermissionGuard permission="orders.read">
                    <OrdersPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'orders/settlement',
                element: (
                  <PermissionGuard permission="orders.read">
                    <OrderSettlementPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'orders/reconciliations',
                element: (
                  <PermissionGuard permission="orders.read">
                    <OrderReconciliationsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'orders/review',
                element: (
                  <PermissionGuard permission="orders.read">
                    <OrderReviewPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'orders/recheck',
                element: (
                  <PermissionGuard permission="orders.read">
                    <OrderRecheckPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'orders/exceptions',
                element: (
                  <PermissionGuard permission="orders.read">
                    <OrderExceptionsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'orders/review-rules',
                element: (
                  <PermissionGuard permission="orders.read">
                    <OrderReviewRulesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'orders/:order_id',
                element: (
                  <PermissionGuard permission="orders.read">
                    <OrderDetailPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'sales/new-order',
                element: (
                  <PermissionGuard permission="orders.read">
                    <SalesOrderCreatePage />
                  </PermissionGuard>
                )
              },
              {
                path: 'sales/import-order',
                element: (
                  <PermissionGuard permission="orders.read">
                    <SalesOrderImportPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'sales/shop-order-download',
                element: (
                  <PermissionGuard permission="orders.read">
                    <ShopOrderDownloadPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'channels/shops',
                element: (
                  <PermissionGuard permission="settings.read">
                    <ShopAccessPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'channels/product-match',
                element: (
                  <PermissionGuard permission="shop_products.read">
                    <SalesProductMatchPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'channels/shop-orders',
                element: (
                  <PermissionGuard permission="orders.read">
                    <ShopOrderQueryPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'sales/logistics-upload',
                element: (
                  <PermissionGuard permission="shipping.read">
                    <LogisticsUploadPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'sales/shop-product-match-query',
                element: (
                  <PermissionGuard permission="shop_products.read">
                    <ShopProductMatchQueryPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'sales/shop-inventory-sync',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <ShopInventorySyncPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'channels/sync-jobs',
                element: (
                  <PermissionGuard permission="settings.read">
                    <SyncJobRunsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'sales/strategy-settings',
                element: (
                  <PermissionGuard permission="settings.read">
                    <SalesStrategySettingsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'sales/settings',
                element: (
                  <PermissionGuard permission="settings.read">
                    <SalesSettingsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'shipments',
                element: (
                  <PermissionGuard permission="shipping.read">
                    <ShipmentsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'shipments/push-failures',
                element: (
                  <PermissionGuard permission="shipping.read">
                    <ShipmentPushFailuresPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'logistics/tracking',
                element: (
                  <PermissionGuard permission="shipping.read">
                    <LogisticsTrackingPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'logistics/exceptions',
                element: (
                  <PermissionGuard permission="shipping.read">
                    <LogisticsExceptionsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'logistics/warehouse-optimization',
                element: (
                  <PermissionGuard permission="shipping.read">
                    <WarehouseOptimizationPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'logistics/installations',
                element: (
                  <PermissionGuard permission="shipping.read">
                    <LogisticsInstallationsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'logistics/settlements',
                element: (
                  <PermissionGuard permission="shipping.read">
                    <LogisticsSettlementsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'marketing/prices',
                element: (
                  <PermissionGuard permission="marketing_prices.read">
                    <MarketingPriceListsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'marketing/promotions',
                element: (
                  <PermissionGuard permission="marketing_promotions.read">
                    <MarketingPromotionsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'marketing/gifts',
                element: (
                  <PermissionGuard permission="marketing_gifts.read">
                    <MarketingGiftPoliciesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'marketing/coupons',
                element: (
                  <PermissionGuard permission="marketing_coupons.read">
                    <MarketingCouponsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'marketing/costs',
                element: (
                  <PermissionGuard permission="marketing_costs.read">
                    <MarketingCostsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'marketing/postmortems',
                element: (
                  <PermissionGuard permission="marketing_postmortems.read">
                    <MarketingPostmortemsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'oms/shipments',
                element: (
                  <PermissionGuard permission="shipping.read">
                    <OmsShipmentsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'stats/shipping',
                element: (
                  <PermissionGuard permission="stats.read">
                    <ShippingStatsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'stats/sales',
                element: (
                  <PermissionGuard permission="stats.read">
                    <SalesStatsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'stats/sales-daily',
                element: (
                  <PermissionGuard permission="stats.read">
                    <SalesDailyReportPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'stats/sales-trends',
                element: (
                  <PermissionGuard permission="stats.read">
                    <SalesTrendAnalysisPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'stats/slow-moving',
                element: (
                  <PermissionGuard permission="stats.read">
                    <SlowMovingAnalysisPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'stats/best-sellers',
                element: (
                  <PermissionGuard permission="stats.read">
                    <BestSellerAnalysisPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'stats/sales-cycle',
                element: (
                  <PermissionGuard permission="stats.read">
                    <SalesCyclePage />
                  </PermissionGuard>
                )
              },
              {
                path: 'finance/ar',
                element: (
                  <PermissionGuard permission="finance.read">
                    <ArManagementPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'finance/ap',
                element: (
                  <PermissionGuard permission="finance.read">
                    <ApManagementPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'finance/expenses',
                element: (
                  <PermissionGuard permission="finance.read">
                    <ExpenseAllocationPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'finance/costing',
                element: (
                  <PermissionGuard permission="finance.read">
                    <CostingPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'finance/consignment',
                element: (
                  <PermissionGuard permission="finance.read">
                    <ConsignmentPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'finance/tax',
                element: (
                  <PermissionGuard permission="finance.read">
                    <TaxManagementPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'finance/reconcile',
                element: (
                  <PermissionGuard permission="finance.read">
                    <ReconcilePage />
                  </PermissionGuard>
                )
              },
              {
                path: 'finance/invoices',
                element: (
                  <PermissionGuard permission="finance.read">
                    <InvoicesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'finance/ap-docs',
                element: (
                  <PermissionGuard permission="finance.read">
                    <ApDocsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'finance/profit',
                element: (
                  <PermissionGuard permission="finance.read">
                    <ProfitPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/outbounds',
                element: (
                  <PermissionGuard permission="wms.read">
                    <WmsOutboundsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/waves',
                element: (
                  <PermissionGuard permission="wms.read">
                    <WmsWavesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/pick-tasks',
                element: (
                  <PermissionGuard permission="wms.read">
                    <WmsPickTasksPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/pack',
                element: (
                  <PermissionGuard permission="wms.write">
                    <WmsPackPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/ship',
                element: (
                  <PermissionGuard permission="wms.write">
                    <WmsShipPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/outbound-confirm',
                element: (
                  <PermissionGuard permission="wms.write">
                    <WmsOutboundConfirmPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/labor',
                element: (
                  <PermissionGuard permission="wms.write">
                    <WmsLaborPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/slotting',
                element: (
                  <PermissionGuard permission="wms.write">
                    <WmsSlottingPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/replenishment',
                element: (
                  <PermissionGuard permission="wms.write">
                    <WmsReplenishmentPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/dock-yard',
                element: (
                  <PermissionGuard permission="wms.write">
                    <WmsDockYardPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/warehouse-areas',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <WarehouseAreasPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/warehouse-locations',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <WarehouseLocationsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/location-balances',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <LocationBalancesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/containers',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <ContainersPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/sender-addresses',
                element: (
                  <PermissionGuard permission="shipping.read">
                    <SenderAddressesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/workflow-configs',
                element: (
                  <PermissionGuard permission="wms.read">
                    <WorkflowConfigPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/stock-counts',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <StocktakesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/stock-count-strategies',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <StockCountStrategiesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/stock-transfers',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <StockTransfersPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/shipment-preprocess-strategies',
                element: (
                  <PermissionGuard permission="shipping.read">
                    <ShipmentPreprocessStrategiesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/pick-recommend-strategies',
                element: (
                  <PermissionGuard permission="wms.read">
                    <PickRecommendStrategiesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/inventory-freezes',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <InventoryFreezesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/stock-adjustments',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <StockAdjustmentsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/stock-moves',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <StockMovesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'inventory',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <InventorySummaryPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'inventory/warehouse',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <InventoryPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'inventory/costs',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <InventoryCostsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'reports',
                element: (
                  <PermissionGuard permission="reports.read">
                    <ReportsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'exports',
                element: (
                  <PermissionGuard permission="exports.read">
                    <ExportJobsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'inventory/ledgers',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <InventoryLedgerPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'inventory/locks',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <InventoryLocksPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'inventory/sync-jobs',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <InventorySyncJobsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'inventory/insights/alerts',
                element: (
                  <PermissionGuard permission="inventory_insights.read">
                    <InventoryAlertsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'inventory/stocktake-tasks',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <StocktakeTasksPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'inventory/stocktakes',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <StocktakesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/inbounds',
                element: (
                  <PermissionGuard permission="wms.inbounds.read">
                    <InboundListPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/inbounds/search',
                element: (
                  <PermissionGuard permission="wms.inbounds.read">
                    <InboundSearchPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/inbounds/receive-entry',
                element: (
                  <PermissionGuard permission="wms.inbounds.write">
                    <InboundQuickReceivePage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/return-inbounds',
                element: (
                  <PermissionGuard permission="aftersales.read">
                    <ReturnInboundPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/inbound-rejects',
                element: (
                  <PermissionGuard permission="wms.inbounds.read">
                    <InboundRejectsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/inbounds/:doc_id',
                element: (
                  <PermissionGuard permission="wms.inbounds.read">
                    <InboundDetailPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/inbounds/:doc_id/receive',
                element: (
                  <PermissionGuard permission="wms.inbounds.write">
                    <InboundReceivePage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/inbounds/:doc_id/inspect',
                element: (
                  <PermissionGuard permission="wms.inbounds.write">
                    <InboundInspectPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/inbounds/:doc_id/putaway',
                element: (
                  <PermissionGuard permission="wms.inbounds.write">
                    <InboundPutawayPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'wms/warehouses',
                element: (
                  <PermissionGuard permission="warehouses.read">
                    <WarehousesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'logistics/couriers',
                element: (
                  <PermissionGuard permission="couriers.read">
                    <CouriersPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'channels',
                element: (
                  <PermissionGuard permission="channels.read">
                    <ChannelDirectoryPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'logistics/products',
                element: (
                  <PermissionGuard permission="logistics_products.read">
                    <LogisticsProductsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'logistics/prices',
                element: (
                  <PermissionGuard permission="logistics_prices.read">
                    <LogisticsPricesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'logistics/interfaces',
                element: (
                  <PermissionGuard permission="logistics_prices.read">
                    <LogisticsInterfacesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'procurement/suppliers',
                element: (
                  <PermissionGuard permission="suppliers.read">
                    <SuppliersPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'tags',
                element: (
                  <PermissionGuard permission="order_tags.read">
                    <TagDirectoryPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'tags/orders',
                element: (
                  <PermissionGuard permission="order_tags.read">
                    <OrderTagDefinitionsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'tags/skus',
                element: (
                  <PermissionGuard permission="sku_tags.read">
                    <SkuTagDefinitionsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'customers',
                element: (
                  <PermissionGuard permission="customers.read">
                    <CustomersPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'customers/categories',
                element: (
                  <PermissionGuard permission="customers.read">
                    <CustomerCategoriesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'customers/member-levels',
                element: (
                  <PermissionGuard permission="customers.read">
                    <MemberLevelsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'customers/tags',
                element: (
                  <PermissionGuard permission="customers.read">
                    <CustomerTagsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'customers/loyalty',
                element: (
                  <PermissionGuard permission="customers.read">
                    <CustomerLoyaltyPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'customers/segments',
                element: (
                  <PermissionGuard permission="customers.read">
                    <CustomerSegmentsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'customers/lifecycle',
                element: (
                  <PermissionGuard permission="customers.read">
                    <CustomerLifecyclePage />
                  </PermissionGuard>
                )
              },
              {
                path: 'customers/service-tickets',
                element: (
                  <PermissionGuard permission="customers.read">
                    <CustomerServiceTicketsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'customers/:customer_id',
                element: (
                  <PermissionGuard permission="customers.read">
                    <CustomerDetailPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'procurement/requests',
                element: (
                  <PermissionGuard permission="procurement_requests.read">
                    <ProcurementRequestsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'procurement/plans',
                element: (
                  <PermissionGuard permission="procurement_plans.read">
                    <ProcurementPlansPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'procurement/sourcing',
                element: (
                  <PermissionGuard permission="procurement_sourcing.read">
                    <ProcurementSourcingPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'procurement/inspections',
                element: (
                  <PermissionGuard permission="procurement_inspections.read">
                    <ProcurementInspectionsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'procurement/reconciliations',
                element: (
                  <PermissionGuard permission="procurement_reconciliations.read">
                    <ProcurementReconciliationsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'procurement/performance',
                element: (
                  <PermissionGuard permission="supplier_performances.read">
                    <SupplierPerformancePage />
                  </PermissionGuard>
                )
              },
              {
                path: 'procurement/orders',
                element: (
                  <PermissionGuard permission="purchase_orders.read">
                    <PurchaseOrdersPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'procurement/orders/:po_id',
                element: (
                  <PermissionGuard permission="purchase_orders.read">
                    <PurchaseOrderDetailPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'procurement/receipts',
                element: (
                  <PermissionGuard permission="purchase_orders.read">
                    <PurchaseReceiptsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'procurement/suggestions',
                element: (
                  <PermissionGuard permission="purchase_suggestions.read">
                    <PurchaseSuggestionsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'procurement/returns',
                element: (
                  <PermissionGuard permission="purchase_returns.read">
                    <PurchaseReturnsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'planning/forecast',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <SalesForecastPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'planning/replenishment-rules',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <ReplenishmentRulesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'planning/purchase-plans',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <PurchasePlansPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'planning/transfer-suggestions',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <TransferSuggestionsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'planning/promo-stocking',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <PromoStockingPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'planning/plan-execution',
                element: (
                  <PermissionGuard permission="inventory.read">
                    <PlanExecutionPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'products',
                element: (
                  <PermissionGuard permission="skus.read">
                    <ProductsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'products/barcodes',
                element: (
                  <PermissionGuard permission="skus.read">
                    <ProductBarcodesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'products/units',
                element: (
                  <PermissionGuard permission="skus.read">
                    <ProductUnitsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'products/channel-availability',
                element: (
                  <PermissionGuard permission="skus.read">
                    <ProductChannelAvailabilityPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'products/change-control',
                element: (
                  <PermissionGuard permission="skus.read">
                    <ProductChangeControlPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'products/diff',
                element: (
                  <PermissionGuard permission="skus.read">
                    <ProductDiffPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'products/lifecycle',
                element: (
                  <PermissionGuard permission="skus.read">
                    <ProductLifecyclePage />
                  </PermissionGuard>
                )
              },
              {
                path: 'products/packaging-specs',
                element: (
                  <PermissionGuard permission="skus.read">
                    <ProductPackagingSpecsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'kits',
                element: (
                  <PermissionGuard permission="kits.read">
                    <KitsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'kits/:kit_sku_id',
                element: (
                  <PermissionGuard permission="kits.read">
                    <KitEditorPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'shop-products',
                element: (
                  <PermissionGuard permission="shop_products.read">
                    <ShopProductDownloadPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'print-jobs',
                element: (
                  <PermissionGuard permission="print_jobs.read">
                    <PrintJobsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'print-templates',
                element: (
                  <PermissionGuard permission="print_templates.read">
                    <PrintTemplatesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'settings/sync-jobs',
                element: (
                  <PermissionGuard permission="settings.read">
                    <SyncJobsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'system/profile',
                element: <ProfilePage />
              },
              {
                path: 'account/profile',
                element: <ProfilePage />
              },
              {
                path: 'system/permissions',
                element: (
                  <PermissionGuard permission="SYS.PERMISSION.READ">
                    <PermissionsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'system/roles',
                element: (
                  <PermissionGuard permission="SYS.ROLE.READ">
                    <RolesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'system/users',
                element: (
                  <PermissionGuard permission="SYS.USER.READ">
                    <UsersPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'system/data-scopes',
                element: (
                  <PermissionGuard permission="SYS.USER.READ">
                    <DataScopesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'system/audit-logs',
                element: (
                  <PermissionGuard permission="SYS.AUDIT.READ">
                    <AuditLogsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'system/operation-logs',
                element: (
                  <PermissionGuard permission="operation_logs.read">
                    <OperationLogsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'system/settings',
                element: (
                  <PermissionGuard permission="SYS.SETTINGS.READ">
                    <SystemSettingsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'system/workflows',
                element: (
                  <PermissionGuard permission="SYS.SETTINGS.READ">
                    <SystemWorkflowsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'system/custom-fields',
                element: (
                  <PermissionGuard permission="SYS.SETTINGS.READ">
                    <SystemCustomFieldsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'integrations/center',
                element: (
                  <PermissionGuard permission="SYS.SETTINGS.READ">
                    <SystemIntegrationsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'integrations/sync-jobs',
                element: (
                  <PermissionGuard permission="SYS.SETTINGS.READ">
                    <SystemSyncJobsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'system/audit-reports',
                element: (
                  <PermissionGuard permission="SYS.SETTINGS.READ">
                    <SystemAuditReportsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'integrations/print-devices',
                element: (
                  <PermissionGuard permission="print_devices.read">
                    <PrintDevicesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'integrations/print-mappings',
                element: (
                  <PermissionGuard permission="print_mappings.read">
                    <PrintMappingsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'system/observability',
                element: (
                  <PermissionGuard permission="SYS.DLQ.READ">
                    <ObservabilityPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'system/frontend-route-observability',
                element: (
                  <PermissionGuard permission="operation_logs.read">
                    <ObservabilityPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'system/erp-data-grid-next',
                element: (
                  <PermissionGuard permission="orders.read">
                    <AgGridEnterpriseDemoPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'system/ag-grid-enterprise',
                element: (
                  <PermissionGuard permission="orders.read">
                    <AgGridEnterpriseDemoPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'failure-queue',
                element: (
                  <PermissionGuard permission="SYS.DLQ.READ">
                    <FailureQueuePage />
                  </PermissionGuard>
                )
              },
              {
                path: 'after-sale-cases',
                element: (
                  <PermissionGuard permission="aftersales.read">
                    <AfterSaleCaseListPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'after-sale-cases/:case_id',
                element: (
                  <PermissionGuard permission="aftersales.read">
                    <AfterSaleCaseDetailPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'after-sales',
                element: (
                  <PermissionGuard permission="aftersales.read">
                    <AfterSalesListPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'after-sales/intercepts',
                element: (
                  <PermissionGuard permission="aftersales.read">
                    <AfterSalesInterceptsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'after-sales/exchanges',
                element: (
                  <PermissionGuard permission="aftersales.read">
                    <AfterSalesExchangesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'after-sales/inspections',
                element: (
                  <PermissionGuard permission="aftersales.read">
                    <AfterSalesInspectionsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'after-sales/sla',
                element: (
                  <PermissionGuard permission="aftersales.read">
                    <AfterSalesSlaPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'after-sales/bulk-edit',
                element: (
                  <PermissionGuard permission="aftersales.read">
                    <AfterSalesBulkEditPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'after-sales/settings',
                element: (
                  <PermissionGuard permission="SYS.SETTINGS.READ">
                    <AfterSalesSettingsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'after-sales/:after_sales_id',
                element: (
                  <PermissionGuard permission="aftersales.read">
                    <AfterSalesDetailPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'after-sales/:after_sales_id/receive',
                element: (
                  <PermissionGuard permission="aftersales.write">
                    <AfterSalesReceivePage />
                  </PermissionGuard>
                )
              },
              {
                path: 'refund-docs',
                element: (
                  <PermissionGuard permission="aftersales.read">
                    <RefundDocsPage />
                  </PermissionGuard>
                )
              },
              ...legacyRouteRedirects,
              {
                path: '*',
                element: <NotFoundPage />
              }
            ]
          }
        ]
      }
    ]
  }
];

installRoutePrefetchRegistry(routeConfig);

const createRouter = routerMode === 'hash' ? createHashRouter : createBrowserRouter;
const routerBasename = routerMode === 'hash' ? '/' : routerBasePath;

export const router = createRouter(routeConfig, { basename: routerBasename });


