import { Suspense, lazy } from 'react';
import type { ComponentType } from 'react';
import { Navigate, createBrowserRouter, createHashRouter } from 'react-router-dom';

import { AuthGuard } from '@/app/guards/AuthGuard';
import { PermissionGuard } from '@/app/guards/PermissionGuard';
import { TenantGuard } from '@/app/guards/TenantGuard';
import { WorkbenchLayout } from '@/app/layout/WorkbenchLayout';
import { AfterSalesDetailPage } from '@/pages/afterSales/AfterSalesDetailPage';
import { AfterSalesExchangesPage } from '@/pages/afterSales/AfterSalesExchangesPage';
import { AfterSalesBulkEditPage } from '@/pages/afterSales/AfterSalesBulkEditPage';
import { AfterSalesInterceptsPage } from '@/pages/afterSales/AfterSalesInterceptsPage';
import { AfterSalesInspectionsPage } from '@/pages/afterSales/AfterSalesInspectionsPage';
import { AfterSalesListPage } from '@/pages/afterSales/AfterSalesListPage';
import { AfterSalesReceivePage } from '@/pages/afterSales/AfterSalesReceivePage';
import { AfterSalesSlaPage } from '@/pages/afterSales/AfterSalesSlaPage';
import { AfterSalesSettingsPage } from '@/pages/afterSales/AfterSalesSettingsPage';
import { AfterSaleCaseDetailPage } from '@/pages/afterSales/AfterSaleCaseDetailPage';
import { AfterSaleCaseListPage } from '@/pages/afterSales/AfterSaleCaseListPage';
import { RefundDocsPage } from '@/pages/afterSales/RefundDocsPage';
import { ForbiddenPage } from '@/pages/Forbidden';
import { LoginPage } from '@/pages/LoginPage';
import { NotFoundPage } from '@/pages/NotFound';
import { FailureQueuePage } from '@/pages/failureQueue/FailureQueuePage';
import { DashboardPage } from '@/pages/Dashboard';
import { InventoryCostsPage } from '@/pages/inventory/InventoryCostsPage';
import { InventoryLedgerPage } from '@/pages/inventory/InventoryLedgerPage';
import { InventoryLocksPage } from '@/pages/inventory/InventoryLocksPage';
import { InventoryPage } from '@/pages/inventory/InventoryPage';
import { InventoryAlertsPage } from '@/pages/inventory/InventoryAlertsPage';
import { InventorySummaryPage } from '@/pages/inventory/InventorySummaryPage';
import { InventorySyncJobsPage } from '@/pages/inventory/InventorySyncJobsPage';
import { StocktakesPage } from '@/pages/inventory/StocktakesPage';
import { StocktakeTasksPage } from '@/pages/inventory/StocktakeTasksPage';
import { InboundDetailPage } from '@/pages/inbounds/InboundDetailPage';
import { InboundInspectPage } from '@/pages/inbounds/InboundInspectPage';
import { InboundListPage } from '@/pages/inbounds/InboundListPage';
import { InboundPutawayPage } from '@/pages/inbounds/InboundPutawayPage';
import { InboundReceivePage } from '@/pages/inbounds/InboundReceivePage';
import { InboundQuickReceivePage } from '@/pages/inbounds/InboundQuickReceivePage';
import { InboundRejectsPage } from '@/pages/inbounds/InboundRejectsPage';
import { InboundSearchPage } from '@/pages/inbounds/InboundSearchPage';
import { ReturnInboundPage } from '@/pages/inbounds/ReturnInboundPage';
import { ChannelDirectoryPage } from '@/pages/channels/ChannelDirectoryPage';
import { CouriersPage } from '@/pages/couriers/CouriersPage';
import { LogisticsExceptionsPage } from '@/pages/logistics/LogisticsExceptionsPage';
import { LogisticsInterfacesPage } from '@/pages/logistics/LogisticsInterfacesPage';
import { LogisticsProductsPage } from '@/pages/logistics/LogisticsProductsPage';
import { LogisticsPricesPage } from '@/pages/logistics/LogisticsPricesPage';
import { LogisticsInstallationsPage } from '@/pages/logistics/LogisticsInstallationsPage';
import { LogisticsSettlementsPage } from '@/pages/logistics/LogisticsSettlementsPage';
import { WarehouseOptimizationPage } from '@/pages/logistics/WarehouseOptimizationPage';
import { WarehousesPage } from '@/pages/warehouses/WarehousesPage';
import { ReportsPage } from '@/pages/reports/ReportsPage';
import { ExportJobsPage } from '@/pages/exports/ExportJobsPage';
import { KitEditorPage } from '@/pages/kits/KitEditorPage';
import { KitsPage } from '@/pages/kits/KitsPage';
import { ShipmentsPage } from '@/pages/shipments/ShipmentsPage';
import { OmsShipmentsPage } from '@/pages/shipments/OmsShipmentsPage';
import { LogisticsTrackingPage } from '@/pages/logistics/LogisticsTrackingPage';
import { ShipmentPushFailuresPage } from '@/pages/shipments/ShipmentPushFailuresPage';
import { OrderDetailPage } from '@/pages/orders/OrderDetailPage';
import { OrdersPage } from '@/pages/orders/OrdersPage';
import { OrderExceptionSlaPage } from '@/pages/orders/OrderExceptionSlaPage';
import { OrderExceptionsPage } from '@/pages/orders/OrderExceptionsPage';
import { OrderReconciliationsPage } from '@/pages/orders/OrderReconciliationsPage';
import { OrderReviewPage } from '@/pages/orders/OrderReviewPage';
import { OrderReviewRulesPage } from '@/pages/orders/OrderReviewRulesPage';
import { OrderSettlementPage } from '@/pages/orders/OrderSettlementPage';
import { OrderSplitMergePage } from '@/pages/orders/OrderSplitMergePage';
import { ChannelMappingsPage } from '@/pages/mappings/ChannelMappingsPage';
import { SalesOrderCreatePage } from '@/pages/sales/SalesOrderCreatePage';
import { SalesOrderImportPage } from '@/pages/sales/SalesOrderImportPage';
import { SalesSettingsPage } from '@/pages/sales/SalesSettingsPage';
import { SalesShopConfigPage } from '@/pages/sales/SalesShopConfigPage';
import { SalesProductMatchPage } from '@/pages/sales/SalesProductMatchPage';
import { SalesStrategySettingsPage } from '@/pages/sales/SalesStrategySettingsPage';
import { ShopOrderQueryPage } from '@/pages/sales/ShopOrderQueryPage';
import { LogisticsUploadPage } from '@/pages/sales/LogisticsUploadPage';
import { SyncJobRunsPage } from '@/pages/sales/SyncJobRunsPage';
import { PrintJobsPage } from '@/pages/printJobs/PrintJobsPage';
import { PrintTemplatesPage } from '@/pages/printTemplates/PrintTemplatesPage';
import { InventoryFreezesPage } from '@/pages/wms/InventoryFreezesPage';
import { PickRecommendStrategiesPage } from '@/pages/wms/PickRecommendStrategiesPage';
import { ShipmentPreprocessStrategiesPage } from '@/pages/wms/ShipmentPreprocessStrategiesPage';
import { StockAdjustmentsPage } from '@/pages/wms/StockAdjustmentsPage';
import { StockCountStrategiesPage } from '@/pages/wms/StockCountStrategiesPage';
import { StockMovesPage } from '@/pages/wms/StockMovesPage';
import { StockTransfersPage } from '@/pages/wms/StockTransfersPage';
import { WmsOutboundsPage } from '@/pages/wms/WmsOutboundsPage';
import { WmsWavesPage } from '@/pages/wms/WmsWavesPage';
import { WmsPickTasksPage } from '@/pages/wms/WmsPickTasksPage';
import { WmsPackPage } from '@/pages/wms/WmsPackPage';
import { WmsShipPage } from '@/pages/wms/WmsShipPage';
import { WmsOutboundConfirmPage } from '@/pages/wms/WmsOutboundConfirmPage';
import { WmsLaborPage } from '@/pages/wms/WmsLaborPage';
import { WmsSlottingPage } from '@/pages/wms/WmsSlottingPage';
import { WmsReplenishmentPage } from '@/pages/wms/WmsReplenishmentPage';
import { WmsDockYardPage } from '@/pages/wms/WmsDockYardPage';
import { WarehouseAreasPage } from '@/pages/wms/WarehouseAreasPage';
import { WarehouseLocationsPage } from '@/pages/wms/WarehouseLocationsPage';
import { ContainersPage } from '@/pages/wms/ContainersPage';
import { SenderAddressesPage } from '@/pages/wms/SenderAddressesPage';
import { LocationBalancesPage } from '@/pages/wms/LocationBalancesPage';
import { WorkflowConfigPage } from '@/pages/wms/WorkflowConfigPage';
import { SuppliersPage } from '@/pages/suppliers/SuppliersPage';
import { CustomerCategoriesPage } from '@/pages/customers/CustomerCategoriesPage';
import { CustomerDetailPage } from '@/pages/customers/CustomerDetailPage';
import { CustomerLifecyclePage } from '@/pages/customers/CustomerLifecyclePage';
import { CustomerLoyaltyPage } from '@/pages/customers/CustomerLoyaltyPage';
import { CustomerSegmentsPage } from '@/pages/customers/CustomerSegmentsPage';
import { CustomerServiceTicketsPage } from '@/pages/customers/CustomerServiceTicketsPage';
import { CustomersPage } from '@/pages/customers/CustomersPage';
import { CustomerTagsPage } from '@/pages/customers/CustomerTagsPage';
import { MemberLevelsPage } from '@/pages/customers/MemberLevelsPage';
import { PurchaseOrdersPage } from '@/pages/procurement/PurchaseOrdersPage';
import { PurchaseOrderDetailPage } from '@/pages/procurement/PurchaseOrderDetailPage';
import { PurchaseReceiptsPage } from '@/pages/procurement/PurchaseReceiptsPage';
import { PurchaseSuggestionsPage } from '@/pages/procurement/PurchaseSuggestionsPage';
import { PurchaseReturnsPage } from '@/pages/procurement/PurchaseReturnsPage';
import { ProcurementRequestsPage } from '@/pages/procurement/ProcurementRequestsPage';
import { ProcurementPlansPage } from '@/pages/procurement/ProcurementPlansPage';
import { ProcurementSourcingPage } from '@/pages/procurement/ProcurementSourcingPage';
import { ProcurementInspectionsPage } from '@/pages/procurement/ProcurementInspectionsPage';
import { ProcurementReconciliationsPage } from '@/pages/procurement/ProcurementReconciliationsPage';
import { SupplierPerformancePage } from '@/pages/procurement/SupplierPerformancePage';
import { ProductBarcodesPage } from '@/pages/products/ProductBarcodesPage';
import { ProductChannelAvailabilityPage } from '@/pages/products/ProductChannelAvailabilityPage';
import { ProductChangeControlPage } from '@/pages/products/ProductChangeControlPage';
import { ProductDiffPage } from '@/pages/products/ProductDiffPage';
import { ProductLifecyclePage } from '@/pages/products/ProductLifecyclePage';
import { ProductPackagingSpecsPage } from '@/pages/products/ProductPackagingSpecsPage';
import { ProductsPage } from '@/pages/products/ProductsPage';
import { ProductUnitsPage } from '@/pages/products/ProductUnitsPage';
import { SyncJobsPage } from '@/pages/shops/SyncJobsPage';
import { ShopProductDownloadPage } from '@/pages/shopProducts/ShopProductDownloadPage';
import { OrderTagDefinitionsPage } from '@/pages/tags/OrderTagDefinitionsPage';
import { SkuTagDefinitionsPage } from '@/pages/tags/SkuTagDefinitionsPage';
import { TagDirectoryPage } from '@/pages/tags/TagDirectoryPage';
import { routerBasePath, routerMode } from '@/utils/env';

function RouteLoadingFallback() {
  return <div data-testid="route-page-loading">页面加载中...</div>;
}

function lazyNamedPage(loader: () => Promise<Record<string, unknown>>, exportName: string) {
  const LazyPage = lazy(async () => {
    const module = await loader();
    const component = module[exportName];
    if (!component) {
      throw new Error(`页面导出不存在: ${exportName}`);
    }
    return { default: component as ComponentType };
  });

  function WrappedPage() {
    return (
      <Suspense fallback={<RouteLoadingFallback />}>
        <LazyPage />
      </Suspense>
    );
  }

  WrappedPage.displayName = `LazyPage(${exportName})`;
  return WrappedPage;
}

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
                path: 'orders/exceptions',
                element: (
                  <PermissionGuard permission="orders.read">
                    <OrderExceptionsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'orders/exception-sla',
                element: (
                  <PermissionGuard permission="orders.read">
                    <OrderExceptionSlaPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'orders/split-merge',
                element: (
                  <PermissionGuard permission="orders.read">
                    <OrderSplitMergePage />
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
                path: 'mappings',
                element: (
                  <PermissionGuard permission="channel_sku_mappings.read">
                    <ChannelMappingsPage />
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
                path: 'sales/shops',
                element: (
                  <PermissionGuard permission="settings.read">
                    <SalesShopConfigPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'sales/product-match',
                element: (
                  <PermissionGuard permission="shop_products.read">
                    <SalesProductMatchPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'sales/shop-orders',
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
                path: 'sales/sync-job-runs',
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
                path: 'warehouses',
                element: (
                  <PermissionGuard permission="warehouses.read">
                    <WarehousesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'couriers',
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
                path: 'suppliers',
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
                path: 'system/integrations',
                element: (
                  <PermissionGuard permission="SYS.SETTINGS.READ">
                    <SystemIntegrationsPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'system/sync-jobs',
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
                path: 'system/print-devices',
                element: (
                  <PermissionGuard permission="print_devices.read">
                    <PrintDevicesPage />
                  </PermissionGuard>
                )
              },
              {
                path: 'system/print-mappings',
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

const createRouter = routerMode === 'hash' ? createHashRouter : createBrowserRouter;
const routerBasename = routerMode === 'hash' ? '/' : routerBasePath;

export const router = createRouter(routeConfig, { basename: routerBasename });
