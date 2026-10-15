use argon2::{PasswordHasher, PasswordVerifier};
use async_trait::async_trait;
use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::{Extension, Multipart, Path, Query, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    middleware,
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post, put},
};
use base64::{Engine as _, engine::general_purpose};
use calamine::{Data, Reader, open_workbook_auto};
use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use csv::ReaderBuilder;
use erp_adapter::{PlatformAdapter, SyncCursor};
use erp_common::{
    feature_flags::{self, FeatureFlagKey, FeatureScope},
    sales_settings::{self, SalesShopConfig},
    telemetry::metrics,
    trace::{REQUEST_ID_HEADER, TRACE_ID_HEADER, generate_trace_id},
};
use erp_domain::{
    DLQ_REPLAY_REQUESTED_EVENT, DeadLetterReplayPayload, OrderProjection, OrderUpsertedPayload,
    PROFIT_COMPUTE_REQUESTED_EVENT, ProfitComputeRequestedPayload, RefundRequestedPayload,
    ShipmentRequestedPayload, ShipmentShippedPayload, TokenRefreshRequestedPayload,
    oms::{
        OrderReviewCondition, OrderReviewContext, ReviewAction, ReviewMode, apply_review_decision,
        derive_exception_status, derive_order_status, ensure_order_review_reopen_allowed,
        evaluate_conditions,
    },
};
use erp_repo::{
    access_plan::{AccessPlanRepo, SeatAdmission},
    after_sale_cases::AfterSaleCaseRepo,
    after_sales::{
        AfterSalesCreateInput, AfterSalesItemCreate, AfterSalesListFilters, AfterSalesRepo,
        AfterSalesShipmentCreate, ExchangeDocCreateInput, REFUND_REQUESTED_EVENT,
        RefundDocCreateInput, RefundDocListFilters, ReturnReceiptCreateInput,
    },
    after_sales_import_jobs::{
        AfterSalesImportJobCreate, AfterSalesImportJobListFilters, AfterSalesImportJobsRepo,
    },
    atp_inventory::{AtpAllocation, AtpInventoryRepo, AtpQuote},
    audit_logs::{AuditLogCreate, AuditLogCursor, AuditLogListFilters, AuditLogsRepo},
    auth::AuthRepo,
    carriers::{Carrier, CarrierCreate, CarrierListFilters, CarrierUpdate, CarriersRepo},
    channel_field_mappings::{
        ChannelFieldMapping, ChannelFieldMappingCreate, ChannelFieldMappingListFilters,
        ChannelFieldMappingUpdate, ChannelFieldMappingsRepo,
    },
    channel_sku_mappings::{
        ChannelSkuMapping, ChannelSkuMappingCreate, ChannelSkuMappingCursor,
        ChannelSkuMappingListFilters, ChannelSkuMappingListResult, ChannelSkuMappingUpdate,
        ChannelSkuMappingView, ChannelSkuMappingViewListFilters, ChannelSkuMappingsRepo,
    },
    channels::{
        Channel, ChannelCreate, ChannelListFilters, ChannelListResult, ChannelUpdate, ChannelsRepo,
    },
    containers::{
        Container, ContainerCreate, ContainerListFilters, ContainerUpdate, ContainersRepo,
    },
    couriers::{
        Courier, CourierCreate, CourierListFilters, CourierListResult, CourierUpdate, CouriersRepo,
    },
    customer_blacklist::{
        CustomerBlacklistCreate, CustomerBlacklistEntry, CustomerBlacklistListFilters,
        CustomerBlacklistRepo,
    },
    customers::{
        Customer, CustomerCategoriesRepo, CustomerCategory, CustomerCategoryCreate,
        CustomerCategoryListFilters, CustomerCategoryUpdate, CustomerCreate, CustomerListFilters,
        CustomerTag, CustomerTagCreate, CustomerTagListFilters, CustomerTagUpdate,
        CustomerTagsRepo, CustomerUpdate, CustomersRepo, MemberLevel, MemberLevelCreate,
        MemberLevelListFilters, MemberLevelUpdate, MemberLevelsRepo,
    },
    data_scopes::DataScopesRepo,
    db::Db,
    dead_letters::{DeadLetterCursor, DeadLetterListFilters, DeadLettersRepo},
    exception_reasons::{ExceptionReason, ExceptionReasonListFilters, ExceptionReasonsRepo},
    export_jobs::{
        ExportJob as ExportJobRecord, ExportJobCreate as ExportJobCreateInput,
        ExportJobListFilters as ExportJobListFiltersRepo,
        ExportJobListResult as ExportJobListResultRepo, ExportJobUpdate, ExportJobsRepo,
    },
    feature_flags::{FeatureFlag, FeatureFlagsRepo},
    finance::{
        InvoiceCreate, InvoiceListFilters, InvoicesRepo, PAYMENT_STATEMENT_STATUS_FAILED,
        PAYMENT_STATEMENT_STATUS_IMPORTED, PAYMENT_STATEMENT_STATUS_RECONCILED,
        PAYMENT_STATEMENT_STATUS_RECONCILING, PLATFORM_STATEMENT_STATUS_IMPORTED,
        PLATFORM_STATEMENT_STATUS_RECONCILED, PLATFORM_STATEMENT_STATUS_RECONCILING,
        PaymentReconcileItemCreate, PaymentReconcileItemListFilters, PaymentReconcileJobCreate,
        PaymentReconcileJobListFilters, PaymentReconcileJobUpdate, PaymentReconcileJobsRepo,
        PaymentStatementCreate, PaymentStatementLineInput, PaymentStatementsRepo,
        PlatformStatementCreate, PlatformStatementLineInput, PlatformStatementsRepo,
        ProfitSnapshotListFilters, ProfitSnapshotUpsert, ProfitSnapshotsRepo,
        RECONCILE_JOB_STATUS_DONE, RECONCILE_JOB_STATUS_FAILED, RECONCILE_JOB_STATUS_RUNNING,
        RECONCILE_JOB_TYPE_PLATFORM, RECONCILE_JOB_TYPE_SHIPPING, RECONCILE_REASON_AMOUNT_MISMATCH,
        RECONCILE_REASON_ORDER_MISSING, RECONCILE_REASON_SHIPMENT_MISSING,
        RECONCILE_REASON_SHIPPING_MISMATCH, RECONCILE_REF_TYPE_ORDER, RECONCILE_REF_TYPE_SHIPMENT,
        ReconcileJobCreate, ReconcileJobItemCreate, ReconcileJobItemListFilters,
        ReconcileJobListFilters, ReconcileJobUpdate, ReconcileJobsRepo, ShippingStatementLineInput,
        ShippingStatementsRepo, WriteoffRecordListFilters, WriteoffRecordsRepo,
    },
    frontend_route_diagnostics::{
        FrontendRouteDiagnosticEvent as RepoFrontendRouteDiagnosticEvent,
        FrontendRouteDiagnosticEventCreate, FrontendRouteDiagnosticsFilters,
        FrontendRouteDiagnosticsPathSummary as RepoFrontendRouteDiagnosticsPathSummary,
        FrontendRouteDiagnosticsRepo,
        FrontendRouteDiagnosticsResult as RepoFrontendRouteDiagnosticsResult,
        FrontendRouteDiagnosticsSummary as RepoFrontendRouteDiagnosticsSummary,
    },
    fulfillment::{
        FulfillmentRepo, OmsShipmentListFilters, SHIPMENT_TYPE_EXCHANGE, SHIPMENT_TYPE_OMS,
        SHIPMENT_TYPE_RESEND,
    },
    inbound_rejects::{
        InboundReject, InboundRejectCreateInput, InboundRejectCreateItem, InboundRejectListFilters,
        InboundRejectsRepo,
    },
    inbounds::{
        INBOUND_STATUS_DRAFT, InboundDocCreate, InboundDocDetail, InboundDocItemCreate,
        InboundDocListFilters, InboundDocSummary, InboundInspectInput, InboundInspectItem,
        InboundInspection, InboundPutawayInput, InboundPutawayItem, InboundReceiveInput,
        InboundReceiveItem, InboundsRepo,
    },
    inventory::{
        InventoryAdjustment, InventoryBalance, InventoryCostListFilters, InventoryCostRecord,
        InventoryCursor, InventoryError, InventoryLedgerListFilters, InventoryLedgerRecord,
        InventoryListFilters, InventoryListResult, InventoryRecord,
        InventoryRepo as InventoryRepoCore, InventoryReservationAction, InventoryReservationInput,
        InventorySummaryListFilters, InventorySummaryRecord,
    },
    inventory_freezes::{
        InventoryFreeze, InventoryFreezeCreateInput, InventoryFreezeListFilters,
        InventoryFreezesRepo,
    },
    inventory_reconciliation_center::{
        InventoryReconciliationCenterRepo, InventoryReconciliationCheckInput,
        InventoryReconciliationRecord,
    },
    kits::{
        KitBomSaveInput, KitDetail, KitListFilters, KitListResult, KitProductListFilters, KitsRepo,
    },
    logistics_exceptions::{
        LogisticsExceptionTypeCreate, LogisticsExceptionTypeFilters, LogisticsExceptionTypeUpdate,
        LogisticsExceptionWorkOrder, LogisticsExceptionWorkOrderCreate,
        LogisticsExceptionWorkOrderFilters, LogisticsExceptionsRepo,
    },
    logistics_installations::{
        LogisticsInstallationAppointment, LogisticsInstallationAppointmentCreate,
        LogisticsInstallationAppointmentsRepo, LogisticsInstallationTask,
        LogisticsInstallationTaskCreate, LogisticsInstallationTaskListFilters,
        LogisticsInstallationTaskUpdate, LogisticsInstallationTasksRepo,
    },
    logistics_interfaces::{
        LogisticsInterface, LogisticsInterfaceApprovalCreate, LogisticsInterfaceCreate,
        LogisticsInterfaceListFilters, LogisticsInterfaceUpdate, LogisticsInterfacesRepo,
    },
    logistics_match_hits::{
        LogisticsMatchHit, LogisticsMatchHitCreate, LogisticsMatchHitListFilters,
        LogisticsMatchHitsRepo,
    },
    logistics_match_rules::{
        LogisticsMatchRule, LogisticsMatchRuleCreate, LogisticsMatchRuleListFilters,
        LogisticsMatchRuleUpdate, LogisticsMatchRulesRepo,
    },
    logistics_prices::{
        LogisticsPrice, LogisticsPriceCreate, LogisticsPriceListFilters, LogisticsPriceUpdate,
        LogisticsPricesRepo,
    },
    logistics_products::{
        LogisticsProduct, LogisticsProductCreate, LogisticsProductListFilters,
        LogisticsProductUpdate, LogisticsProductsRepo,
    },
    logistics_settlements::{
        LogisticsSettlementBatch, LogisticsSettlementBatchListFilters,
        LogisticsSettlementBatchesRepo, LogisticsSettlementRuleUpsert,
        LogisticsSettlementRulesRepo,
    },
    logistics_warehouse_optimization::{
        LogisticsWarehouseStrategiesRepo, LogisticsWarehouseStrategy,
        LogisticsWarehouseStrategyCreate, LogisticsWarehouseStrategyHistory,
        LogisticsWarehouseStrategyHistoryCreate, LogisticsWarehouseStrategyListFilters,
        LogisticsWarehouseStrategyUpdate, LogisticsWarehouseSuggestion,
        LogisticsWarehouseSuggestionListFilters, LogisticsWarehouseSuggestionsRepo,
    },
    mapping_rules::{
        MappingRuleCreate, MappingRuleListFilters, MappingRuleUpdate, MappingRulesRepo,
    },
    operation_logs::{
        OP_ACTION_AP_DOC_CLOSED, OP_ACTION_AP_DOC_PAID, OP_ACTION_INVENTORY_ADJUSTED,
        OP_ACTION_INVENTORY_FREEZE, OP_ACTION_INVENTORY_INBOUND, OP_ACTION_INVENTORY_OUTBOUND,
        OP_ACTION_INVENTORY_SYNC_RETRY, OP_ACTION_INVENTORY_SYNC_RUN, OP_ACTION_INVENTORY_UNFREEZE,
        OP_ACTION_KIT_BOM_SAVED,
        OP_ACTION_OMS_EXCEPTION_REJECT, OP_ACTION_OMS_EXCEPTION_RESOLVE,
        OP_ACTION_OMS_EXCEPTION_REVOKE, OP_ACTION_OMS_ORDER_CANCEL,
        OP_ACTION_OMS_REVIEW_FLAG, OP_ACTION_OMS_REVIEW_PASS,
        OP_ACTION_OMS_REVIEW_REJECT, OP_ACTION_OMS_SHIPMENT_CANCEL_REQUESTED,
        OP_ACTION_PRODUCT_CREATED,
        OP_ACTION_PRODUCT_DELETED, OP_ACTION_PRODUCT_UPDATED, OP_ACTION_SHIPMENT_CALLBACK,
        OP_ACTION_SHIPMENT_CONFIRMED, OP_ACTION_SHIPMENT_REQUESTED,
        OP_ACTION_SHOP_PRODUCT_AUTO_MATCH, OP_ACTION_SHOP_PRODUCT_MANUAL_MATCH,
        OP_ACTION_SKU_COPIED, OP_ACTION_SKU_COST_ADJUSTED, OP_ACTION_SKU_CREATED,
        OP_ACTION_SKU_DELETED, OP_ACTION_SKU_UPDATED, OP_ACTION_STOCK_ADJUST_APPLIED,
        OP_ACTION_STOCK_ADJUST_CREATED, OP_ACTION_STOCK_COUNT_APPLIED,
        OP_ACTION_STOCK_COUNT_CANCELLED, OP_ACTION_STOCK_COUNT_CONFIRMED,
        OP_ACTION_STOCK_COUNT_CREATED, OP_ACTION_STOCK_MOVE_APPLIED, OP_ACTION_STOCK_MOVE_CREATED,
        OP_ACTION_STOCK_TRANSFER_CREATED, OP_ACTION_STOCK_TRANSFER_IN,
        OP_ACTION_STOCK_TRANSFER_OUT, OP_ACTION_WMS_INBOUND_AUDITED, OP_ACTION_WMS_INBOUND_CREATED,
        OP_ACTION_WMS_INBOUND_INSPECTED, OP_ACTION_WMS_INBOUND_PUTAWAY,
        OP_ACTION_WMS_INBOUND_RECEIVED, OP_ACTION_WMS_OUTBOUND_ADD_PACKAGE,
        OP_ACTION_WMS_OUTBOUND_AUDITED, OP_ACTION_WMS_OUTBOUND_CHECKED,
        OP_ACTION_WMS_OUTBOUND_CREATED, OP_ACTION_WMS_OUTBOUND_INSPECTED,
        OP_ACTION_WMS_OUTBOUND_LOGISTICS_UPDATED, OP_ACTION_WMS_OUTBOUND_SHIPPED,
        OP_ACTION_WMS_OUTBOUND_WEIGHED, OP_ACTION_WMS_SHIPMENT_CANCEL_REQUESTED,
        OP_ACTION_WMS_TRACKING_RESERVED, OperationLogCreate, OperationLogCursor,
        OperationLogListFilters, OperationLogsRepo,
    },
    order_action_logs::{OrderActionLogCreate, OrderActionLogsRepo},
    order_review_rules::{
        OrderReviewRule, OrderReviewRuleCreate, OrderReviewRuleListFilters, OrderReviewRuleUpdate,
        OrderReviewRulesRepo,
    },
    order_status_history::{OrderStatusHistoryCreate, OrderStatusHistoryRepo},
    order_tag_definitions::{
        OrderTagDefinition, OrderTagDefinitionCreate, OrderTagDefinitionListFilters,
        OrderTagDefinitionUpdate, OrderTagDefinitionsRepo,
    },
    orders::{
        OmsOrderListFilters, OrderEditInput, OrderEditItemInput, OrderNoteCreate,
        OrderReceiverUpdate, OrderReviewUpdate, OrderTagCreate, OrdersRepo,
    },
    outbox::{OutboxCursor, OutboxListFilters, OutboxRepo},
    permissions::{PermissionDefinition, PermissionsRepo},
    pick_recommend_hits::{
        PickRecommendHit, PickRecommendHitCreate, PickRecommendHitListFilters,
        PickRecommendHitsRepo,
    },
    pick_recommend_strategies::{
        PickRecommendStrategiesRepo, PickRecommendStrategy, PickRecommendStrategyCreate,
        PickRecommendStrategyListFilters, PickRecommendStrategyUpdate,
    },
    platform_tokens::{PlatformToken, PlatformTokenUpsert, PlatformTokensRepo},
    print_devices::{
        PrintDevice, PrintDeviceCreate, PrintDeviceCursor, PrintDeviceListFilters,
        PrintDeviceListResult, PrintDeviceUpdate, PrintDevicesRepo as PrintDevicesRepoCore,
    },
    print_jobs::{
        PrintJob, PrintJobCreate, PrintJobCursor, PrintJobListFilters, PrintJobListResult,
        PrintJobUpdate, PrintJobsRepo as PrintJobsRepoCore,
    },
    print_mappings::{PrintMappingUpsert, PrintMappingsRepo},
    print_templates::{
        PrintTemplate, PrintTemplateCreate, PrintTemplateCursor, PrintTemplateListFilters,
        PrintTemplateListResult, PrintTemplateUpdate, PrintTemplatesRepo as PrintTemplatesRepoCore,
    },
    procurement::{
        ApDocListFilters, ApDocsRepo, ProcurementInspection, ProcurementInspectionCreate,
        ProcurementInspectionDetail, ProcurementInspectionItem, ProcurementInspectionItemInput,
        ProcurementInspectionListFilters, ProcurementInspectionUpdate, ProcurementInspectionsRepo,
        ProcurementPlan, ProcurementPlanCreate, ProcurementPlanDetail, ProcurementPlanItem,
        ProcurementPlanItemInput, ProcurementPlanListFilters, ProcurementPlanUpdate,
        ProcurementPlansRepo, ProcurementReconciliation, ProcurementReconciliationCreate,
        ProcurementReconciliationListFilters, ProcurementReconciliationUpdate,
        ProcurementReconciliationsRepo, ProcurementRequest, ProcurementRequestCreate,
        ProcurementRequestDetail, ProcurementRequestItem, ProcurementRequestItemInput,
        ProcurementRequestListFilters, ProcurementRequestUpdate, ProcurementRequestsRepo,
        ProcurementSourcing, ProcurementSourcingCreate, ProcurementSourcingListFilters,
        ProcurementSourcingRepo, ProcurementSourcingUpdate, PurchaseOrder, PurchaseOrderCreate,
        PurchaseOrderDetail, PurchaseOrderItem, PurchaseOrderItemInput, PurchaseOrderListFilters,
        PurchaseOrderReceiveInput, PurchaseOrderReceiveItem, PurchaseOrderUpdate,
        PurchaseOrdersRepo, PurchaseReceipt, PurchaseReceiptDetail, PurchaseReceiptItem,
        PurchaseReceiptListFilters, PurchaseReceiptSummary, PurchaseReceiptsRepo, PurchaseReturn,
        PurchaseReturnCreate, PurchaseReturnDetail, PurchaseReturnItem, PurchaseReturnListFilters,
        PurchaseReturnsRepo, PurchaseSuggestion, PurchaseSuggestionListFilters,
        PurchaseSuggestionsRepo, SupplierPerformance, SupplierPerformanceCreate,
        SupplierPerformanceListFilters, SupplierPerformanceRepo, SupplierPerformanceUpdate,
    },
    product_barcodes::{
        ProductBarcode, ProductBarcodeCreate, ProductBarcodeListFilters, ProductBarcodeUpdate,
        ProductBarcodesRepo,
    },
    product_brands::{
        ProductBrand, ProductBrandCreate, ProductBrandListFilters, ProductBrandUpdate,
        ProductBrandsRepo,
    },
    product_categories::{
        ProductCategoriesRepo, ProductCategory, ProductCategoryCreate, ProductCategoryListFilters,
        ProductCategoryUpdate,
    },
    product_change_requests::{
        ProductChangeHistoryItem, ProductChangeRequest, ProductChangeRequestCreate,
        ProductChangeRequestListFilters, ProductChangeRequestUpdate, ProductChangeRequestsRepo,
    },
    product_channel_availability::{
        ProductChannelAvailability, ProductChannelAvailabilityCreate,
        ProductChannelAvailabilityListFilters, ProductChannelAvailabilityRepo,
        ProductChannelAvailabilityUpdate,
    },
    product_lifecycles::{
        ProductLifecycle, ProductLifecycleCreate, ProductLifecycleListFilters,
        ProductLifecycleUpdate, ProductLifecyclesRepo,
    },
    product_packaging_specs::{
        ProductPackagingSpec, ProductPackagingSpecUpsert, ProductPackagingSpecsRepo,
    },
    product_unit_conversions::{
        ProductUnitConversion, ProductUnitConversionCreate, ProductUnitConversionListFilters,
        ProductUnitConversionUpdate, ProductUnitConversionsRepo,
    },
    product_units::{
        ProductUnit, ProductUnitCreate, ProductUnitListFilters, ProductUnitUpdate, ProductUnitsRepo,
    },
    product_warehouses::{
        ProductWarehouse, ProductWarehouseCreate, ProductWarehouseUpdate, ProductWarehousesRepo,
    },
    products::{
        Product, ProductCreate, ProductCursor, ProductListFilters, ProductListResult,
        ProductUpdate, ProductsRepo,
    },
    rbac::{RbacRepo, Role, RoleCreate, RoleUpdate, User, UserCreate, UserListFilters, UserUpdate},
    reports::{
        DashboardReportFilters, FinanceReportFilters, FulfillmentReportFilters,
        InventoryReportFilters, ReportsRepo,
    },
    sales_orders::{SalesOrderAfterSalesUpdate, SalesOrderWmsUpdate, SalesOrdersRepo},
    sender_addresses::{
        SenderAddress, SenderAddressCreate, SenderAddressListFilters, SenderAddressUpdate,
        SenderAddressesRepo,
    },
    settings::{SettingListFilters, SettingUpsert, SettingsRepo},
    shared_contract::{SharedContractRepo, SharedWrite},
    shipment_preprocess_hits::{
        ShipmentPreprocessHit, ShipmentPreprocessHitCreate, ShipmentPreprocessHitListFilters,
        ShipmentPreprocessHitsRepo,
    },
    shipment_preprocess_strategies::{
        ShipmentPreprocessStrategiesRepo, ShipmentPreprocessStrategy,
        ShipmentPreprocessStrategyCreate, ShipmentPreprocessStrategyListFilters,
        ShipmentPreprocessStrategyUpdate,
    },
    shipment_push_attempts::{
        ShipmentPushAttemptCursor, ShipmentPushAttemptListFilters, ShipmentPushAttemptsRepo,
    },
    shipments::{
        SHIPMENT_CONFIRMED_EVENT, SHIPMENT_REQUESTED_EVENT, SHIPMENT_SHIPPED_EVENT,
        SHIPMENT_TYPE_PLATFORM, ShipmentCreateInput, ShipmentListFilters,
        ShipmentPackageUpsertInput, ShipmentsRepo,
    },
    shop_carrier_accounts::{
        ShopCarrierAccount, ShopCarrierAccountCreate, ShopCarrierAccountListFilters,
        ShopCarrierAccountUpdate, ShopCarrierAccountsRepo,
    },
    shop_product_tasks::{
        ShopProductTask, ShopProductTaskCreate, ShopProductTaskListFilters, ShopProductTaskUpdate,
        ShopProductTasksRepo,
    },
    shop_products::{
        ShopProductSkuListFilters, ShopProductSkuUpsert, ShopProductSkuView, ShopProductUpsert,
        ShopProductsRepo,
    },
    shops::{Shop, ShopCreate, ShopListFilters as RepoShopListFilters, ShopUpdate, ShopsRepo},
    sku_batch_strategies::{
        SkuBatchStrategiesRepo, SkuBatchStrategy, SkuBatchStrategyCreate,
        SkuBatchStrategyListFilters, SkuBatchStrategyUpdate,
    },
    sku_batches::{
        SkuBatch, SkuBatchCreate, SkuBatchCursor, SkuBatchListFilters, SkuBatchListResult,
        SkuBatchUpdate, SkuBatchesRepo,
    },
    sku_costs::{
        SkuCostAdjustInput, SkuCostLedger, SkuCostLedgerListFilters, SkuCostLedgerListResult,
        SkuCostsRepo,
    },
    sku_serial_rules::{
        SkuSerialRule, SkuSerialRuleCreate, SkuSerialRuleListFilters, SkuSerialRuleUpdate,
        SkuSerialRulesRepo,
    },
    sku_serials::{
        ShipmentSerialBindInput as RepoShipmentSerialBindInput, ShipmentSerialCount, SkuSerial,
        SkuSerialCursor, SkuSerialGenerateInput, SkuSerialListFilters, SkuSerialListResult,
        SkuSerialsRepo,
    },
    sku_suppliers::{SkuSupplier, SkuSuppliersRepo},
    sku_tag_definitions::{
        SkuTagDefinition, SkuTagDefinitionCreate, SkuTagDefinitionListFilters,
        SkuTagDefinitionUpdate, SkuTagDefinitionsRepo,
    },
    skus::{
        Sku, SkuCreate, SkuCursor, SkuListFilters, SkuListResult, SkuSearchFilters,
        SkuSearchItem as RepoSkuSearchItem, SkuUpdate, SkusRepo,
    },
    stock_adjustments::{
        StockAdjustCreateInput, StockAdjustDetail, StockAdjustDoc, StockAdjustItem,
        StockAdjustItemInput, StockAdjustListFilters, StockAdjustmentsRepo,
    },
    stock_count_strategies::{
        StockCountStrategiesRepo, StockCountStrategy, StockCountStrategyCreate,
        StockCountStrategyListFilters, StockCountStrategyUpdate,
    },
    stock_count_suggestions::{
        StockCountSuggestion, StockCountSuggestionCreate, StockCountSuggestionListFilters,
        StockCountSuggestionsRepo,
    },
    stock_counts::{
        StockCount, StockCountCreateInput, StockCountCreateWithItemsInput, StockCountDetail,
        StockCountImportInput, StockCountImportItemInput, StockCountItem, StockCountItemUpdate,
        StockCountListFilters, StockCountsRepo,
    },
    stock_moves::{
        StockMoveCreateInput, StockMoveDetail, StockMoveDoc, StockMoveItem, StockMoveItemInput,
        StockMoveListFilters, StockMovesRepo,
    },
    stock_sync_job_events::StockSyncJobEventsRepo,
    stock_sync_jobs::{StockSyncJob, StockSyncJobListFilters, StockSyncJobsRepo},
    stock_transfers::{
        StockTransfer, StockTransferCreateInput, StockTransferDetail, StockTransferItem,
        StockTransferItemInput, StockTransferListFilters, StockTransfersRepo,
    },
    suppliers::{
        Supplier, SupplierCreate, SupplierListFilters, SupplierListResult, SupplierUpdate,
        SuppliersRepo,
    },
    sync_job_runs::{SyncJobRun, SyncJobRunCreate, SyncJobRunListFilters, SyncJobRunsRepo},
    sync_jobs::{
        SyncJob, SyncJobListFilters as RepoSyncJobListFilters, SyncJobUpsert, SyncJobsRepo,
    },
    tenants::TenantsRepo,
    user_shortcuts::UserShortcutsRepo,
    warehouse_areas::{
        WarehouseArea, WarehouseAreaCreate, WarehouseAreaListFilters, WarehouseAreaUpdate,
        WarehouseAreasRepo,
    },
    warehouse_locations::{
        LocationBalanceListFilters, LocationBalanceRecord, WarehouseLocation,
        WarehouseLocationCreate, WarehouseLocationListFilters, WarehouseLocationUpdate,
        WarehouseLocationsRepo,
    },
    warehouses::{
        Warehouse, WarehouseCreate, WarehouseListFilters, WarehouseListResult, WarehouseUpdate,
        WarehousesRepo,
    },
    waybills::{
        WAYBILL_STATUS_AVAILABLE, WAYBILL_STATUS_CANCELLED, WAYBILL_STATUS_RESERVED,
        WAYBILL_STATUS_USED, Waybill, WaybillListFilters, WaybillsRepo,
    },
    wms::{
        OutboundDoc, OutboundDocCreateInput, OutboundDocDetail, OutboundDocItemInput,
        OutboundDocListFilters, OutboundInspectInput, OutboundPackBatchInput, OutboundPackInput,
        OutboundPackItemInput, OutboundPickTask, OutboundShipInput, OutboundVerifyFinishInput,
        OutboundVerifyScanInput, OutboundVerifyStartInput, PackCheckEventInput, PackRecord,
        PickTaskConfirmInput, PickTaskListFilters, Wave, WaveCreateInput, WaveListFilters,
        WaveRule, WaveRuleCreateInput, WaveRuleListFilters, WaveRuleUpdateInput, WmsRepo,
    },
};
use erp_types::{
    AfterSaleCaseId, AfterSaleState, AfterSalesStatus, AfterSalesType, CompensationStatus,
    EvidenceStatus, InboundQcStatus, InboundStatus, OmsOrderStatus, OrderExceptionStatus,
    OrderFulfillmentStatus, OrderId, OrderMainStatus, OrderPaymentStatus, OrderReviewStatus,
    OrderStatus, OutboundStatus, PackStatus, PermissionCode, PickStatus, Platform, PutawayStatus,
    ReceiptStatus, RefundChannel, RefundState, RefundStatus, RoleId, ShipStatus, ShopId, TenantId,
    UserId, VerifyStatus,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Cursor,
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};
use tower_http::trace::TraceLayer;
use tracing::warn;

mod adapter_registry;
mod admin_shortcut_handlers;
mod after_sales_handlers;
pub mod app_state;
mod audit_helpers;
mod auth_context;
mod auth_handlers;
mod auth_layers;
mod auth_tokens;
mod common_parse_helpers;
mod crm_handlers;
mod export_job_views;
mod file_paths;
mod finance_handlers;
mod finance_management;
#[path = "middleware/idempotency.rs"]
mod idempotency;
#[path = "middleware/web_security.rs"]
mod web_security;
mod inventory_print_report_handlers;
mod inventory_views;
mod logistics_handlers;
mod manufacturing;
mod onboarding;
mod mapping_handlers;
mod marketing;
mod order_sku_mapping_helpers;
mod orders;
mod permission_catalog_handlers;
mod permission_catalog_helpers;
mod permission_normalization_helpers;
mod planning;
mod procurement_handlers;
mod procurement_views;
mod product_copy_batch_handlers;
mod product_handlers;
mod product_selection_handlers;
mod rbac_role_handlers;
mod rbac_user_handlers;
mod repo_ports;
pub mod router;
mod sales_config_helpers;
mod sales_order_helpers;
mod sales_orders;
mod settings_payloads;
mod shipment_serials;
mod shop_product_download_helpers;
mod shop_runtime_helpers;
mod shop_sync_handlers;
mod shop_views;
mod sku_action_handlers;
mod sku_auto_match_helpers;
mod sku_grid_handlers;
mod sku_handlers;
mod sku_import_file_helpers;
mod sku_import_lookup_helpers;
mod sku_import_parser_helpers;
mod stats_helpers;
mod support;
mod system_catalog_handlers;
mod system_observability_handlers;
mod system_settings_handlers;
mod warehouse_master_handlers;
mod warehouse_tree_handlers;
mod wms_config_helpers;
mod wms_inbound_views;
mod wms_normalizers;
mod wms_operation_handlers;
pub(crate) use adapter_registry::*;
pub(crate) use admin_shortcut_handlers::*;
pub(crate) use after_sales_handlers::*;
pub use app_state::AppState;
pub(crate) use audit_helpers::*;
pub(crate) use auth_context::*;
pub(crate) use auth_handlers::*;
pub(crate) use auth_layers::*;
pub use auth_tokens::{Claims, JwtService};
pub(crate) use common_parse_helpers::*;
pub(crate) use crm_handlers::*;
pub(crate) use export_job_views::*;
pub(crate) use file_paths::*;
pub(crate) use finance_handlers::*;
pub(crate) use inventory_print_report_handlers::*;
pub(crate) use inventory_views::*;
pub(crate) use logistics_handlers::*;
pub(crate) use mapping_handlers::*;
pub(crate) use order_sku_mapping_helpers::*;
pub(crate) use permission_catalog_handlers::*;
pub(crate) use permission_catalog_helpers::*;
pub(crate) use permission_normalization_helpers::*;
pub(crate) use procurement_handlers::*;
pub(crate) use procurement_views::*;
pub(crate) use product_copy_batch_handlers::*;
pub(crate) use product_handlers::*;
pub(crate) use product_selection_handlers::*;
pub(crate) use rbac_role_handlers::*;
pub(crate) use rbac_user_handlers::*;
pub use repo_ports::*;
pub use router::build_app;
pub use web_security::public_cors_allows;
pub(crate) use sales_config_helpers::*;
pub(crate) use sales_order_helpers::*;
pub(crate) use settings_payloads::*;
use shipment_serials::*;
pub(crate) use shop_product_download_helpers::*;
pub(crate) use shop_runtime_helpers::*;
pub(crate) use shop_sync_handlers::*;
pub(crate) use shop_views::*;
pub(crate) use sku_action_handlers::*;
pub(crate) use sku_auto_match_helpers::*;
pub(crate) use sku_grid_handlers::*;
pub(crate) use sku_handlers::*;
pub(crate) use sku_import_file_helpers::*;
pub(crate) use sku_import_lookup_helpers::*;
pub(crate) use sku_import_parser_helpers::*;
pub(crate) use stats_helpers::*;
pub(crate) use support::*;
pub(crate) use system_catalog_handlers::*;
pub(crate) use system_observability_handlers::*;
pub(crate) use system_settings_handlers::*;
pub(crate) use warehouse_master_handlers::*;
pub(crate) use warehouse_tree_handlers::*;
pub(crate) use wms_config_helpers::*;
pub(crate) use wms_inbound_views::*;
pub(crate) use wms_normalizers::*;
pub(crate) use wms_operation_handlers::*;

#[derive(Debug, Deserialize)]
struct AfterSaleCaseListQuery {
    shop_id: Option<String>,
    platform: Option<String>,
    refund_state: Option<String>,
    after_sale_state: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Serialize)]
struct AfterSaleCaseListResponse {
    data: Vec<erp_domain::AfterSaleCase>,
    limit: i64,
    offset: i64,
}

#[derive(Debug, Deserialize)]
struct AfterSalesListQuery {
    status: Option<String>,
    after_sales_type: Option<String>,
    order_id: Option<String>,
    created_from: Option<String>,
    created_to: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct RefundDocListQuery {
    status: Option<String>,
    after_sales_id: Option<String>,
    created_from: Option<String>,
    created_to: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AfterSalesItemRequest {
    sku_id: String,
    qty: i64,
    refund_cent: i64,
}

#[derive(Debug, Deserialize, Serialize)]
struct AfterSalesCreateRequest {
    order_id: String,
    #[serde(rename = "type")]
    after_sales_type: String,
    reason_code: Option<String>,
    amount_cent: Option<i64>,
    compensation_status: Option<String>,
    evidence_status: Option<String>,
    #[serde(default)]
    items: Vec<AfterSalesItemRequest>,
}

#[derive(Debug, Deserialize)]
struct AfterSalesReceiveRequest {
    warehouse_id: String,
    putaway_done: Option<bool>,
}

#[derive(Deserialize)]
struct AfterSalesReasonListQuery {
    category: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Deserialize)]
struct AfterSalesReasonRequest {
    reason_code: Option<String>,
    label: String,
    category: Option<String>,
    status: Option<String>,
    sort_order: Option<i32>,
}

#[derive(Deserialize)]
struct AfterSalesQualityRuleListQuery {
    after_sales_type: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Deserialize)]
struct AfterSalesQualityRuleRequest {
    after_sales_type: String,
    result_code: String,
    status: Option<String>,
    sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct AfterSalesRefundRequest {
    amount_cent: Option<i64>,
    channel: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AfterSalesShipmentRequest {
    origin_shipment_id: Option<String>,
    carrier_code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AfterSalesInterceptListQuery {
    status: Option<String>,
    order_id: Option<String>,
    tracking_no: Option<String>,
    requested_from: Option<String>,
    requested_to: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AfterSalesInterceptRequest {
    order_id: String,
    after_sales_id: Option<String>,
    carrier_name: Option<String>,
    tracking_no: Option<String>,
    status: Option<String>,
    reason: Option<String>,
    requested_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AfterSalesInspectionListQuery {
    after_sales_id: Option<String>,
    sku_id: Option<String>,
    warehouse_id: Option<String>,
    result_code: Option<String>,
    inspected_from: Option<String>,
    inspected_to: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AfterSalesInspectionRequest {
    after_sales_id: String,
    sku_id: String,
    warehouse_id: Option<String>,
    qty: i64,
    result_code: Option<String>,
    inspector_name: Option<String>,
    inspected_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AfterSalesInspectionUpdateRequest {
    warehouse_id: Option<String>,
    qty: Option<i64>,
    result_code: Option<String>,
    inspector_name: Option<String>,
    inspected_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AfterSalesSlaRuleListQuery {
    after_sales_type: Option<String>,
    status: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AfterSalesSlaRuleRequest {
    after_sales_type: String,
    response_hours: i32,
    resolution_hours: i32,
    escalation_role: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AfterSalesSlaRuleUpdateRequest {
    after_sales_type: Option<String>,
    response_hours: Option<i32>,
    resolution_hours: Option<i32>,
    escalation_role: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AfterSalesImportJobListQuery {
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AfterSalesAuditLogQuery {
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct AfterSalesView {
    id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    order_id: uuid::Uuid,
    #[serde(rename = "type")]
    after_sales_type: String,
    status: String,
    after_sale_status: String,
    compensation_status: Option<String>,
    evidence_status: Option<String>,
    reason_code: Option<String>,
    amount_cent: i64,
    sla_deadline_at: Option<DateTime<Utc>>,
    sla_status: Option<String>,
    sla_remaining_minutes: Option<i64>,
    escalation_role: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct AfterSalesItemView {
    sku_id: uuid::Uuid,
    qty: i64,
    refund_cent: i64,
}

#[derive(Debug, Serialize)]
struct ReturnReceiptView {
    id: uuid::Uuid,
    after_sales_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    status: String,
    return_status: String,
    received_at: Option<DateTime<Utc>>,
    putaway_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct RefundDocView {
    id: uuid::Uuid,
    after_sales_id: uuid::Uuid,
    amount_cent: i64,
    status: String,
    refund_status: String,
    channel: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct AfterSalesAuditLogView {
    audit_id: uuid::Uuid,
    action: String,
    user_id: Option<uuid::Uuid>,
    before_json: Option<Value>,
    after_json: Option<Value>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct AfterSalesDetailView {
    after_sales: AfterSalesView,
    items: Vec<AfterSalesItemView>,
    return_receipt: Option<ReturnReceiptView>,
    refund_doc: Option<RefundDocView>,
    shipments: Vec<AfterSalesShipmentView>,
    raw_json: Value,
    audit_logs: Vec<AfterSalesAuditLogView>,
}

#[derive(Debug, Serialize)]
struct AfterSalesShipmentView {
    shipment_id: uuid::Uuid,
    shipment_type: String,
    origin_shipment_id: Option<uuid::Uuid>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct AfterSalesReasonView {
    reason_code: String,
    label: String,
    category: Option<String>,
    status: String,
    sort_order: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct AfterSalesReasonListResponse {
    items: Vec<AfterSalesReasonView>,
    total: i64,
    limit: i64,
    offset: i64,
}

#[derive(Debug, Serialize)]
struct AfterSalesQualityRuleView {
    rule_id: uuid::Uuid,
    after_sales_type: String,
    result_code: String,
    status: String,
    sort_order: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct AfterSalesQualityRuleListResponse {
    items: Vec<AfterSalesQualityRuleView>,
    total: i64,
    limit: i64,
    offset: i64,
}

#[derive(Debug, Serialize)]
struct AfterSalesImportJobView {
    job_id: uuid::Uuid,
    file_name: Option<String>,
    file_type: String,
    status: String,
    progress: i32,
    total_rows: i32,
    success_rows: i32,
    failed_rows: i32,
    attempts: i32,
    last_error: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct AfterSalesImportJobListResponse {
    items: Vec<AfterSalesImportJobView>,
    total: i64,
    limit: i64,
    offset: i64,
}

#[derive(Debug, Serialize)]
struct AfterSalesInterceptView {
    intercept_id: uuid::Uuid,
    order_id: uuid::Uuid,
    after_sales_id: Option<uuid::Uuid>,
    carrier_name: Option<String>,
    tracking_no: Option<String>,
    status: String,
    reason: Option<String>,
    requested_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct AfterSalesInterceptListResponse {
    items: Vec<AfterSalesInterceptView>,
    total: i64,
    limit: i64,
    offset: i64,
}

#[derive(Debug, Serialize)]
struct AfterSalesInspectionView {
    inspection_id: uuid::Uuid,
    after_sales_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    warehouse_id: Option<uuid::Uuid>,
    qty: i64,
    result_code: String,
    inspector_name: Option<String>,
    inspected_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct AfterSalesInspectionListResponse {
    items: Vec<AfterSalesInspectionView>,
    total: i64,
    limit: i64,
    offset: i64,
}

#[derive(Debug, Serialize)]
struct AfterSalesSlaRuleView {
    rule_id: uuid::Uuid,
    after_sales_type: String,
    response_hours: i32,
    resolution_hours: i32,
    escalation_role: Option<String>,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct AfterSalesSlaRuleListResponse {
    items: Vec<AfterSalesSlaRuleView>,
    total: i64,
    limit: i64,
    offset: i64,
}

#[derive(Debug, Serialize)]
struct AfterSalesListResponse {
    items: Vec<AfterSalesView>,
    total: i64,
    limit: i64,
    offset: i64,
}

#[derive(Debug, Serialize)]
struct RefundDocListResponse {
    items: Vec<RefundDocView>,
    total: i64,
    limit: i64,
    offset: i64,
}

#[derive(Debug, Deserialize)]
struct ProductListQuery {
    keyword: Option<String>,
    brand: Option<String>,
    category: Option<String>,
    status: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ProductCreateRequest {
    product_code: Option<String>,
    name: String,
    alias: Option<String>,
    brand: Option<String>,
    category: Option<String>,
    base_unit: Option<String>,
    second_unit: Option<String>,
    default_supplier_id: Option<String>,
    default_warehouse_id: Option<String>,
    primary_barcode: Option<String>,
    product_tags: Option<Vec<String>>,
    remark: Option<String>,
    warning_stock: Option<Value>,
    batch_enabled: Option<bool>,
    shelf_life_days: Option<i32>,
    forbid_sale_days: Option<i32>,
    serial_enabled: Option<bool>,
    serial_push_platform: Option<bool>,
    attrs: Option<Value>,
    #[allow(dead_code)]
    allow_negative_stock: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ProductUpdateRequest {
    product_code: Option<String>,
    name: Option<String>,
    alias: Option<String>,
    brand: Option<String>,
    category: Option<String>,
    base_unit: Option<String>,
    second_unit: Option<String>,
    default_supplier_id: Option<String>,
    default_warehouse_id: Option<String>,
    primary_barcode: Option<String>,
    product_tags: Option<Vec<String>>,
    remark: Option<String>,
    warning_stock: Option<Value>,
    batch_enabled: Option<bool>,
    shelf_life_days: Option<i32>,
    forbid_sale_days: Option<i32>,
    serial_enabled: Option<bool>,
    serial_push_platform: Option<bool>,
    attrs: Option<Value>,
    status: Option<String>,
    #[allow(dead_code)]
    allow_negative_stock: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ProductCategoryListQuery {
    keyword: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ProductCategoryListResponse {
    items: Vec<ProductCategory>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ProductCategoryCreateRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct ProductCategoryUpdateRequest {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProductBrandListQuery {
    keyword: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ProductBrandListResponse {
    items: Vec<ProductBrand>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ProductBrandCreateRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct ProductBrandUpdateRequest {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProductUnitListQuery {
    keyword: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ProductUnitListResponse {
    items: Vec<ProductUnit>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ProductUnitCreateRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct ProductUnitUpdateRequest {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProductWarehouseCreateRequest {
    warehouse_id: String,
    is_default: Option<bool>,
    allow_negative_stock: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ProductWarehouseUpdateRequest {
    is_default: Option<bool>,
    allow_negative_stock: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ProductWarehouseListResponse {
    items: Vec<ProductWarehouse>,
}

#[derive(Debug, Deserialize)]
struct ProductKitListQuery {
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ProductBarcodeListQuery {
    sku_id: Option<String>,
    barcode: Option<String>,
    status: Option<String>,
    level: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ProductBarcodeListResponse {
    items: Vec<ProductBarcode>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ProductBarcodeCreateRequest {
    sku_id: String,
    barcode: String,
    level: String,
    unit: String,
    ratio: Option<i32>,
    is_default: Option<bool>,
    status: Option<String>,
    print_template: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProductBarcodeUpdateRequest {
    barcode: Option<String>,
    level: Option<String>,
    unit: Option<String>,
    ratio: Option<i32>,
    is_default: Option<bool>,
    status: Option<String>,
    print_template: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
struct ProductUnitConversionListQuery {
    sku_id: Option<String>,
    scenario: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ProductUnitConversionListResponse {
    items: Vec<ProductUnitConversion>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ProductUnitConversionCreateRequest {
    sku_id: String,
    base_unit: String,
    aux_unit: String,
    ratio: Option<i32>,
    rounding: Option<String>,
    scenario: String,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProductUnitConversionUpdateRequest {
    base_unit: Option<String>,
    aux_unit: Option<String>,
    ratio: Option<i32>,
    rounding: Option<String>,
    scenario: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProductChannelAvailabilityListQuery {
    sku_id: Option<String>,
    channel_id: Option<String>,
    warehouse_id: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ProductChannelAvailabilityListResponse {
    items: Vec<ProductChannelAvailability>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ProductChannelAvailabilityCreateRequest {
    sku_id: String,
    channel_id: Option<String>,
    warehouse_id: Option<String>,
    status: Option<String>,
    available_qty: Option<i32>,
    rule: Option<String>,
    on_shelf_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProductChannelAvailabilityUpdateRequest {
    channel_id: Option<Option<String>>,
    warehouse_id: Option<Option<String>>,
    status: Option<String>,
    available_qty: Option<i32>,
    rule: Option<Option<String>>,
    on_shelf_at: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
struct ProductChangeRequestListQuery {
    sku_id: Option<String>,
    status: Option<String>,
    change_type: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ProductChangeRequestListResponse {
    items: Vec<ProductChangeRequest>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ProductChangeRequestCreateRequest {
    sku_id: String,
    title: String,
    change_type: String,
    status: Option<String>,
    applicant: String,
    approver: Option<String>,
    reason: Option<String>,
    history: Option<Vec<ProductChangeHistoryItem>>,
}

#[derive(Debug, Deserialize)]
struct ProductChangeRequestUpdateRequest {
    title: Option<String>,
    change_type: Option<String>,
    status: Option<String>,
    approver: Option<Option<String>>,
    reason: Option<Option<String>>,
    history: Option<Vec<ProductChangeHistoryItem>>,
}

#[derive(Debug, Deserialize)]
struct ProductLifecycleListQuery {
    sku_id: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ProductLifecycleListResponse {
    items: Vec<ProductLifecycle>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ProductLifecycleCreateRequest {
    sku_id: String,
    status: Option<String>,
    inventory_qty: Option<i32>,
    remark: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProductLifecycleUpdateRequest {
    status: Option<String>,
    inventory_qty: Option<i32>,
    remark: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
struct SkuListQuery {
    product_id: Option<String>,
    sku_code: Option<String>,
    barcode: Option<String>,
    keyword: Option<String>,
    is_active: Option<bool>,
    #[serde(alias = "page_token")]
    cursor: Option<String>,
    #[serde(alias = "page_size")]
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct SkuCreateRequest {
    product_id: String,
    sku_code: String,
    barcode: Option<String>,
    name: Option<String>,
    note: Option<String>,
    spec: Option<Value>,
    tags: Option<Vec<String>>,
    image_urls: Option<Vec<String>>,
    supplier_ids: Option<Vec<String>>,
    primary_supplier_id: Option<String>,
    weight_gram: Option<i32>,
    length_mm: Option<i32>,
    width_mm: Option<i32>,
    height_mm: Option<i32>,
    volume_cm3: Option<i32>,
    sku_kind: Option<String>,
    cost_cent: Option<i64>,
    is_active: Option<bool>,
    status: Option<String>,
    batch_enabled: Option<bool>,
    serial_enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct SkuUpdateRequest {
    sku_code: Option<String>,
    barcode: Option<String>,
    name: Option<String>,
    note: Option<String>,
    spec: Option<Value>,
    tags: Option<Vec<String>>,
    image_urls: Option<Vec<String>>,
    supplier_ids: Option<Vec<String>>,
    primary_supplier_id: Option<String>,
    weight_gram: Option<i32>,
    length_mm: Option<i32>,
    width_mm: Option<i32>,
    height_mm: Option<i32>,
    volume_cm3: Option<i32>,
    sku_kind: Option<String>,
    cost_cent: Option<i64>,
    is_active: Option<bool>,
    status: Option<String>,
    batch_enabled: Option<bool>,
    serial_enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct SkuSearchQuery {
    keyword: Option<String>,
    sku_kind: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct SkuSearchItem {
    sku_id: uuid::Uuid,
    sku_code: String,
    name: Option<String>,
    barcode: Option<String>,
    sku_kind: Option<String>,
}

#[derive(Debug, Serialize)]
struct SkuSearchResponse {
    items: Vec<SkuSearchItem>,
}

#[derive(Debug, Serialize)]
struct SkuDetailResponse {
    sku: Sku,
    product: Option<Product>,
    suppliers: Vec<SkuSupplier>,
}

#[derive(Debug, Deserialize)]
struct SkuCostLedgerListQuery {
    sku_id: Option<String>,
    start_at: Option<String>,
    end_at: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct SkuCostLedgerListResponse {
    items: Vec<SkuCostLedger>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct SkuCostAdjustRequest {
    sku_id: String,
    new_cost_cent: i64,
    ref_type: Option<String>,
    ref_id: Option<String>,
    request_id: Option<String>,
    item_code: Option<String>,
    qty_micros: Option<i64>,
    period_code: Option<String>,
    kind: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SkuImportPrecheckRequest {
    file_name: Option<String>,
    file_type: Option<String>,
    content_base64: String,
    request_id: Option<String>,
    file_digest: Option<String>,
    column_map: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SkuImportRequest {
    file_name: Option<String>,
    file_type: Option<String>,
    content_base64: String,
}

#[derive(Debug, Serialize)]
struct SkuImportUploadResponse {
    file_id: uuid::Uuid,
    file_name: String,
    file_type: String,
}

#[derive(Debug, Deserialize)]
struct SkuImportJobCreateRequest {
    file_id: String,
    file_name: Option<String>,
    file_type: Option<String>,
    duplicate_strategy: Option<String>,
}

#[derive(Debug, Serialize)]
struct SkuImportJobResponse {
    job_id: uuid::Uuid,
    status: String,
    progress: i32,
    total_rows: i64,
    success_rows: i64,
    failed_rows: i64,
    file_name: Option<String>,
    file_type: Option<String>,
    duplicate_strategy: Option<String>,
    failure_log_url: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct SkuImportFailureRow {
    row: i64,
    sku_code: Option<String>,
    field: Option<String>,
    message: String,
}

#[derive(Debug, Deserialize)]
struct SkuImportFailureListQuery {
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct SkuImportFailureListResponse {
    items: Vec<SkuImportFailureRow>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct SkuImportIssue {
    row: i64,
    field: Option<String>,
    message: String,
}

#[derive(Debug, Serialize)]
struct SkuImportPreviewRow {
    row: i64,
    product_id: Option<String>,
    product_name: Option<String>,
    sku_code: String,
    barcode: Option<String>,
    name: Option<String>,
    sku_kind: Option<String>,
    tags: Vec<String>,
    image_urls: Vec<String>,
    cost_cent: Option<i64>,
    supplier_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SkuImportPrecheckResponse {
    total: i64,
    valid: i64,
    errors: Vec<SkuImportIssue>,
    preview: Vec<SkuImportPreviewRow>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct SkuImportRow {
    row: i64,
    product_id: Option<String>,
    product_name: Option<String>,
    sku_code: String,
    barcode: Option<String>,
    aux_barcode: Option<String>,
    name: Option<String>,
    spec: Option<String>,
    unit: Option<String>,
    category_id: Option<String>,
    category_name: Option<String>,
    sku_kind: Option<String>,
    tags: Vec<String>,
    image_urls: Vec<String>,
    cost_cent: Option<String>,
    supplier_ids: Vec<String>,
    weight_gram: Option<String>,
    length_mm: Option<String>,
    width_mm: Option<String>,
    height_mm: Option<String>,
    volume_cm3: Option<String>,
}

#[derive(Default)]
struct SkuImportHeaderIndex {
    product_id: Option<usize>,
    product_name: Option<usize>,
    sku_code: Option<usize>,
    barcode: Option<usize>,
    aux_barcode: Option<usize>,
    name: Option<usize>,
    spec: Option<usize>,
    unit: Option<usize>,
    category_id: Option<usize>,
    category_name: Option<usize>,
    sku_kind: Option<usize>,
    tags: Option<usize>,
    image_urls: Option<usize>,
    cost_cent: Option<usize>,
    supplier_ids: Option<usize>,
    weight_gram: Option<usize>,
    length_mm: Option<usize>,
    width_mm: Option<usize>,
    height_mm: Option<usize>,
    volume_cm3: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct KitListQuery {
    keyword: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct KitBomComponentRequest {
    component_sku_id: String,
    qty: i64,
    expire_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KitBomSaveRequest {
    #[serde(default)]
    components: Vec<KitBomComponentRequest>,
}

#[derive(Debug, Deserialize)]
struct ChannelSkuMappingListQuery {
    shop_id: Option<String>,
    platform: Option<String>,
    sku_id: Option<String>,
    platform_sku_id: Option<String>,
    platform_outer_sku_id: Option<String>,
    status: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ChannelSkuMappingCreateRequest {
    shop_id: String,
    platform: String,
    platform_sku_id: String,
    platform_outer_sku_id: Option<String>,
    sku_id: String,
    status: Option<String>,
    confidence: Option<i32>,
    match_source: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChannelSkuMappingUpdateRequest {
    platform_sku_id: Option<String>,
    platform_outer_sku_id: Option<String>,
    sku_id: Option<String>,
    status: Option<String>,
    conflict_reason: Option<String>,
    confidence: Option<i32>,
    match_source: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MappingListQuery {
    platform: Option<String>,
    shop_id: Option<String>,
    platform_sku_id: Option<String>,
    sku_code: Option<String>,
    barcode: Option<String>,
    status: Option<String>,
    conflict_reason: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct MappingPayload {
    platform: String,
    shop_id: String,
    platform_sku_id: String,
    outer_sku_id: Option<String>,
    sku_id: String,
    sku_code: Option<String>,
    barcode: Option<String>,
    resolve_conflict: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct MappingBatchRequest {
    #[serde(default)]
    items: Vec<MappingPayload>,
}

#[derive(Debug, Serialize)]
struct MappingConflict {
    platform: String,
    shop_id: String,
    platform_sku_id: String,
    incoming_sku_id: String,
    existing_mapping_id: String,
    existing_sku_id: String,
    existing_sku_code: Option<String>,
    existing_barcode: Option<String>,
    conflict_reason: String,
}

#[derive(Debug, Serialize)]
struct MappingView {
    mapping_id: uuid::Uuid,
    tenant_id: TenantId,
    platform: Platform,
    shop_id: ShopId,
    platform_sku_id: String,
    outer_sku_id: Option<String>,
    sku_id: uuid::Uuid,
    sku_code: String,
    barcode: Option<String>,
    status: String,
    conflict_reason: String,
    match_source: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct MappingListResponse {
    items: Vec<MappingView>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct MappingBatchResponse {
    items: Vec<MappingView>,
    conflicts: Vec<MappingConflict>,
}

#[derive(Debug, Deserialize)]
struct ShopProductSkuListQuery {
    platform: Option<String>,
    shop_ids: Option<String>,
    product_type: Option<String>,
    status: Option<String>,
    keyword: Option<String>,
    match_status: Option<String>,
    mapping_status: Option<String>,
    conflict_reason: Option<String>,
    match_source: Option<String>,
    updated_from: Option<String>,
    updated_to: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ShopProductSkuListResponse {
    items: Vec<ShopProductSkuView>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ShopProductTaskListQuery {
    task_type: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ShopProductTaskListResponse {
    items: Vec<ShopProductTask>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct ShopProductTaskResponse {
    task: ShopProductTask,
}

#[derive(Debug, Serialize, Clone)]
struct ShopProductMatchFailure {
    sku_record_id: uuid::Uuid,
    platform_sku_id: String,
    merchant_sku_code: Option<String>,
    reason: String,
}

#[derive(Debug, Serialize)]
struct ShopProductMatchResponse {
    task: ShopProductTask,
    failures: Vec<ShopProductMatchFailure>,
}

#[derive(Debug, Serialize)]
struct ShopProductDownloadResponse {
    task: ShopProductTask,
    auto_match_task: Option<ShopProductTask>,
    auto_match_failures: Vec<ShopProductMatchFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ShopProductSkuFilterPayload {
    platform: Option<String>,
    shop_ids: Option<Vec<String>>,
    product_type: Option<String>,
    status: Option<String>,
    keyword: Option<String>,
    match_status: Option<String>,
    mapping_status: Option<String>,
    conflict_reason: Option<String>,
    match_source: Option<String>,
    updated_from: Option<String>,
    updated_to: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ShopProductDownloadSku {
    platform_sku_id: String,
    merchant_sku_code: Option<String>,
    title: Option<String>,
    spec_text: Option<String>,
    price_cent: Option<i64>,
    stock_qty: Option<i64>,
    status: Option<String>,
    barcode: Option<String>,
    image_url: Option<String>,
    updated_at: Option<String>,
    raw_json: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct ShopProductDownloadItem {
    platform_product_id: String,
    merchant_code: Option<String>,
    title: String,
    product_type: Option<String>,
    status: Option<String>,
    updated_at: Option<String>,
    raw_json: Option<Value>,
    skus: Vec<ShopProductDownloadSku>,
}

#[derive(Debug, Deserialize)]
struct ShopProductDownloadRequest {
    platform: Option<String>,
    shop_ids: Option<Vec<String>>,
    product_type: Option<String>,
    status: Option<String>,
    updated_from: Option<String>,
    updated_to: Option<String>,
    keyword: Option<String>,
    auto_match: Option<bool>,
    products: Option<Vec<ShopProductDownloadItem>>,
}

#[derive(Debug, Deserialize)]
struct ShopProductCreateSkuRequest {
    sku_record_ids: Option<Vec<String>>,
    filters: Option<ShopProductSkuFilterPayload>,
    import_price: Option<bool>,
    import_image: Option<bool>,
    import_stock: Option<bool>,
    warehouse_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ShopProductAutoMatchRequest {
    sku_record_ids: Option<Vec<String>>,
    filters: Option<ShopProductSkuFilterPayload>,
}

#[derive(Debug, Deserialize)]
struct ShopProductManualMatchRequest {
    sku_record_ids: Vec<String>,
    sku_id: String,
}

#[derive(Debug, Deserialize)]
struct ShopProductDeleteMatchRequest {
    sku_record_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ShopProductParseRequest {
    sku_record_ids: Option<Vec<String>>,
    filters: Option<ShopProductSkuFilterPayload>,
}

#[derive(Debug, Serialize)]
struct ShopProductParseItem {
    sku_record_id: uuid::Uuid,
    shop_id: ShopId,
    platform: Platform,
    platform_sku_id: String,
    merchant_sku_code: Option<String>,
    resolved_sku_id: Option<uuid::Uuid>,
    resolved_sku_code: Option<String>,
    resolved_sku_name: Option<String>,
    parse_source: String,
    reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct ShopProductParseResponse {
    items: Vec<ShopProductParseItem>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ChannelFieldMappingListQuery {
    platform: Option<String>,
    entity_type: Option<String>,
    platform_field: Option<String>,
    internal_field: Option<String>,
    enabled: Option<bool>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ChannelFieldMappingListResponse {
    items: Vec<ChannelFieldMapping>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ChannelFieldMappingCreateRequest {
    platform: String,
    entity_type: String,
    platform_field: String,
    internal_field: String,
    default_value: Option<String>,
    transform_rule: Option<serde_json::Value>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ChannelFieldMappingUpdateRequest {
    platform: Option<String>,
    entity_type: Option<String>,
    platform_field: Option<String>,
    internal_field: Option<String>,
    default_value: Option<Option<String>>,
    transform_rule: Option<serde_json::Value>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct SkuBatchListQuery {
    sku_id: Option<String>,
    batch_no: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct SkuBatchCreateRequest {
    sku_id: String,
    batch_no: String,
    production_date: Option<String>,
    expire_at: Option<String>,
    request_id: Option<String>,
    serialized: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct SkuBatchUpdateRequest {
    batch_no: Option<String>,
    production_date: Option<String>,
    expire_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SkuBatchStrategyListQuery {
    sku_id: Option<String>,
    enabled: Option<bool>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct SkuBatchStrategyListResponse {
    items: Vec<SkuBatchStrategy>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct SkuBatchStrategyCreateRequest {
    sku_id: Option<String>,
    name: String,
    enabled: Option<bool>,
    expire_threshold_days: Option<i32>,
    fifo_enabled: Option<bool>,
    request_id: Option<String>,
    rule_code: Option<String>,
    request_ref: Option<String>,
    requested_qty: Option<i64>,
    lots: Option<Vec<AllocationLotRequest>>,
}

#[derive(Debug, Deserialize)]
struct AllocationLotRequest {
    batch_code: String,
    location_code: String,
    available: i64,
    sort_date: String,
    stock_version: i64,
    frozen: bool,
    expired: bool,
}

#[derive(Debug, Deserialize)]
struct SkuBatchStrategyUpdateRequest {
    sku_id: Option<Option<String>>,
    name: Option<String>,
    enabled: Option<bool>,
    expire_threshold_days: Option<i32>,
    fifo_enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct SkuSerialListQuery {
    sku_id: Option<String>,
    status: Option<String>,
    serial_no: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct SkuSerialGenerateRequest {
    sku_id: String,
    prefix: Option<String>,
    rule_id: Option<String>,
    count: i64,
}

#[derive(Debug, Deserialize)]
struct SkuSerialRuleListQuery {
    sku_id: Option<String>,
    enabled: Option<bool>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct SkuSerialRuleListResponse {
    items: Vec<SkuSerialRule>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct SkuSerialRuleCreateRequest {
    sku_id: Option<String>,
    name: String,
    prefix: String,
    seq_length: Option<i32>,
    increment_step: Option<i32>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct SkuSerialRuleUpdateRequest {
    sku_id: Option<Option<String>>,
    name: Option<String>,
    prefix: Option<String>,
    seq_length: Option<i32>,
    increment_step: Option<i32>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct SkuSerialPrintRequest {
    template_id: String,
    serial_ids: Vec<String>,
    device_fingerprint: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ShipmentSerialEntry {
    line_id: String,
    sku_id: Option<String>,
    serial_nos: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ShipmentSerialBindRequest {
    #[serde(default)]
    serials: Vec<ShipmentSerialEntry>,
}

#[derive(Debug, Deserialize)]
struct WarehouseListQuery {
    keyword: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct WarehouseListResponse {
    items: Vec<Warehouse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct CourierListQuery {
    keyword: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct CourierListResponse {
    items: Vec<Courier>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct LogisticsProductListQuery {
    keyword: Option<String>,
    carrier_code: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct LogisticsProductListResponse {
    items: Vec<LogisticsProduct>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct LogisticsProductCreateRequest {
    carrier_code: String,
    code: String,
    name: String,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LogisticsProductUpdateRequest {
    carrier_code: Option<String>,
    code: Option<String>,
    name: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LogisticsPriceListQuery {
    carrier_code: Option<String>,
    product_id: Option<String>,
    region_code: Option<String>,
    weight_kg: Option<f64>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct LogisticsPriceListResponse {
    items: Vec<LogisticsPrice>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct LogisticsPriceCreateRequest {
    carrier_code: String,
    product_id: Option<String>,
    region_code: String,
    weight_min_kg: f64,
    weight_max_kg: f64,
    price_cent: i64,
}

#[derive(Debug, Deserialize)]
struct LogisticsPriceUpdateRequest {
    carrier_code: Option<String>,
    product_id: Option<String>,
    region_code: Option<String>,
    weight_min_kg: Option<f64>,
    weight_max_kg: Option<f64>,
    price_cent: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct LogisticsPriceEstimateRequest {
    shipment_id: Option<String>,
    order_id: Option<String>,
    carrier_code: Option<String>,
    product_id: Option<String>,
    region_code: Option<String>,
    weight_kg: Option<f64>,
}

#[derive(Debug, Serialize)]
struct LogisticsPriceEstimateResponse {
    matched: bool,
    carrier_code: String,
    product_id: Option<uuid::Uuid>,
    region_code: String,
    weight_kg: f64,
    price: Option<LogisticsPrice>,
}

#[derive(Debug, Deserialize)]
struct LogisticsMatchRuleListQuery {
    keyword: Option<String>,
    enabled: Option<bool>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct LogisticsMatchRuleListResponse {
    items: Vec<LogisticsMatchRule>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct LogisticsMatchRuleCreateRequest {
    name: String,
    enabled: Option<bool>,
    priority: Option<i32>,
    product_id: Option<String>,
    conditions_json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct LogisticsMatchRuleUpdateRequest {
    name: Option<String>,
    enabled: Option<bool>,
    priority: Option<i32>,
    product_id: Option<Option<String>>,
    conditions_json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct LogisticsMatchHitListQuery {
    rule_id: Option<String>,
    shipment_id: Option<String>,
    order_id: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct LogisticsMatchHitListResponse {
    items: Vec<LogisticsMatchHit>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct LogisticsMatchPreviewRequest {
    order_id: String,
    warehouse_id: Option<String>,
    weight_gram: Option<i32>,
}

#[derive(Debug, Serialize)]
struct LogisticsMatchPreviewResponse {
    matched: bool,
    rule: Option<LogisticsMatchRule>,
    product: Option<LogisticsProduct>,
}

#[derive(Debug, Deserialize)]
struct LogisticsInterfaceListQuery {
    keyword: Option<String>,
    status: Option<String>,
    approval_status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct LogisticsInterfaceListResponse {
    items: Vec<LogisticsInterface>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct LogisticsInterfaceCreateRequest {
    carrier_code: String,
    carrier_name: String,
    api_base_url: String,
    template_name: String,
    status: Option<String>,
    last_checked_at: Option<DateTime<Utc>>,
    retry_policy: String,
    alert_channel: String,
    approval_status: Option<String>,
    approved_by: Option<String>,
    approved_at: Option<DateTime<Utc>>,
    notes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LogisticsInterfaceUpdateRequest {
    carrier_code: Option<String>,
    carrier_name: Option<String>,
    api_base_url: Option<String>,
    template_name: Option<String>,
    status: Option<String>,
    last_checked_at: Option<DateTime<Utc>>,
    retry_policy: Option<String>,
    alert_channel: Option<String>,
    approval_status: Option<String>,
    approved_by: Option<Option<String>>,
    approved_at: Option<Option<DateTime<Utc>>>,
    notes: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
struct LogisticsInterfaceApprovalCreateRequest {
    action: String,
    operator: String,
    comment: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LogisticsExceptionTypeListQuery {
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct LogisticsExceptionTypeCreateRequest {
    type_code: String,
    name: String,
    sla_hours: Option<i32>,
    severity: Option<String>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct LogisticsExceptionTypeUpdateRequest {
    name: Option<String>,
    sla_hours: Option<i32>,
    severity: Option<String>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct LogisticsExceptionWorkOrderListQuery {
    status: Option<String>,
    exception_type: Option<String>,
    assignee: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct LogisticsExceptionWorkOrderListResponse {
    items: Vec<LogisticsExceptionWorkOrder>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct LogisticsExceptionWorkOrderCreateRequest {
    order_id: String,
    tracking_no: String,
    carrier_code: String,
    exception_type: String,
    status: Option<String>,
    assignee: Option<String>,
    due_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct LogisticsExceptionWorkOrderAssignRequest {
    assignee: String,
}

#[derive(Debug, Deserialize)]
struct LogisticsExceptionWorkOrderResolveRequest {
    resolution: String,
}

#[derive(Debug, Deserialize)]
struct LogisticsWarehouseStrategyListQuery {
    enabled: Option<bool>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct LogisticsWarehouseStrategyListResponse {
    items: Vec<LogisticsWarehouseStrategy>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct LogisticsWarehouseStrategyCreateRequest {
    name: String,
    priority: Option<String>,
    enabled: Option<bool>,
    updated_by: Option<String>,
    history_summary: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LogisticsWarehouseStrategyUpdateRequest {
    name: Option<String>,
    priority: Option<String>,
    enabled: Option<bool>,
    updated_by: Option<String>,
    history_summary: Option<String>,
}

#[derive(Debug, Serialize)]
struct LogisticsWarehouseStrategyHistoryListResponse {
    items: Vec<LogisticsWarehouseStrategyHistory>,
}

#[derive(Debug, Deserialize)]
struct LogisticsWarehouseSuggestionListQuery {
    applied: Option<bool>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct LogisticsWarehouseSuggestionListResponse {
    items: Vec<LogisticsWarehouseSuggestion>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct LogisticsInstallationTaskResponse {
    task_id: uuid::Uuid,
    tenant_id: TenantId,
    order_id: String,
    customer_name: String,
    address: String,
    service_type: String,
    assigned_team: Option<String>,
    status: String,
    remedy_ticket_id: Option<String>,
    exception_note: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    appointments: Vec<LogisticsInstallationAppointment>,
}

#[derive(Debug, Serialize)]
struct LogisticsInstallationTaskListResponse {
    items: Vec<LogisticsInstallationTaskResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct LogisticsInstallationTaskListQuery {
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct LogisticsInstallationTaskCreateRequest {
    order_id: String,
    customer_name: String,
    address: String,
    service_type: String,
    assigned_team: Option<String>,
    scheduled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct LogisticsInstallationTaskUpdateRequest {
    assigned_team: Option<String>,
    status: Option<String>,
    remedy_ticket_id: Option<String>,
    exception_note: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LogisticsInstallationAppointmentCreateRequest {
    scheduled_at: DateTime<Utc>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LogisticsSettlementBatchListQuery {
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct LogisticsSettlementBatchListResponse {
    items: Vec<LogisticsSettlementBatch>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct LogisticsSettlementRuleUpdateRequest {
    weight_rounding: String,
    min_fee_cent: i64,
    oversize_fee_cent: i64,
    overtime_fee_cent: i64,
}

#[derive(Debug, Deserialize)]
struct LogisticsSettlementBatchConfirmRequest {
    payment_plan_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CourierCreateRequest {
    name: String,
    code: String,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CourierUpdateRequest {
    name: Option<String>,
    code: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CarrierListQuery {
    keyword: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct CarrierListResponse {
    items: Vec<Carrier>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct CarrierCreateRequest {
    carrier_code: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct CarrierUpdateRequest {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ShopCarrierAccountListQuery {
    shop_id: Option<String>,
    carrier_code: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ShopCarrierAccountListResponse {
    items: Vec<ShopCarrierAccount>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ShopCarrierAccountCreateRequest {
    shop_id: String,
    carrier_code: String,
    account_meta_json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ShopCarrierAccountUpdateRequest {
    account_meta_json: Option<Value>,
}
#[derive(Debug, Deserialize)]
struct ChannelListQuery {
    keyword: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ChannelListResponse {
    items: Vec<Channel>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ChannelCreateRequest {
    platform: String,
    name: String,
    department: Option<String>,
    owner: Option<String>,
    allow_manual_order: Option<bool>,
    allow_manual_after_sales: Option<bool>,
    default_warehouse_id: Option<String>,
    commission_formula: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChannelUpdateRequest {
    platform: Option<String>,
    name: Option<String>,
    department: Option<String>,
    owner: Option<String>,
    allow_manual_order: Option<bool>,
    allow_manual_after_sales: Option<bool>,
    default_warehouse_id: Option<String>,
    commission_formula: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ShopListQuery {
    platform: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ShopTokenStatus {
    token_status: String,
    has_access_token: bool,
    has_refresh_token: bool,
    expires_at: Option<DateTime<Utc>>,
    expires_in_minutes: Option<i64>,
    is_expired: bool,
    is_expiring_soon: bool,
    scopes: Option<String>,
    last_refreshed_at: Option<DateTime<Utc>>,
    refresh_fail_count: i32,
    last_error: String,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ShopView {
    shop_id: ShopId,
    tenant_id: TenantId,
    platform: Platform,
    platform_shop_id: String,
    name: String,
    status: String,
    auth_meta_json: Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    token_status: Option<ShopTokenStatus>,
}

#[derive(Debug, Serialize)]
struct ShopListResponse {
    items: Vec<ShopView>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct ShopIntegrationOverviewResponse {
    items: Vec<ShopIntegrationOverviewItem>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct ShopIntegrationOverviewItem {
    shop_id: ShopId,
    platform: Platform,
    platform_shop_id: String,
    name: String,
    shop_status: String,
    oauth_status: String,
    integration_status: String,
    webhook_mode: Option<String>,
    webhook_status: String,
    webhook_sla_status: String,
    last_webhook_received_at: Option<DateTime<Utc>>,
    last_webhook_processed_at: Option<DateTime<Utc>>,
    webhook_failed_count: i64,
    webhook_verify_failed_count: i64,
    webhook_pending_count: i64,
    webhook_retrying_count: i64,
    webhook_dlq_count: i64,
    webhook_next_retry_at: Option<DateTime<Utc>>,
    token_status: Option<String>,
    token_expires_at: Option<DateTime<Utc>>,
    last_sync_at: Option<DateTime<Utc>>,
    enabled_job_count: i64,
    failed_job_count: i64,
    warning_count: i64,
    alert_messages: Vec<String>,
    jobs: Vec<ShopIntegrationOverviewJob>,
}

#[derive(Debug, Clone)]
struct ShopWebhookHealthSummary {
    webhook_status: String,
    webhook_sla_status: String,
    last_webhook_received_at: Option<DateTime<Utc>>,
    last_webhook_processed_at: Option<DateTime<Utc>>,
    webhook_failed_count: i64,
    webhook_verify_failed_count: i64,
    webhook_pending_count: i64,
    webhook_retrying_count: i64,
    webhook_dlq_count: i64,
    webhook_next_retry_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct ShopIntegrationOverviewJob {
    job_type: String,
    status: String,
    enabled: bool,
    last_error: Option<String>,
    last_success_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct ShopCreateRequest {
    platform: String,
    platform_shop_id: String,
    name: String,
    status: Option<String>,
    credentials: Option<Value>,
    auth_meta_json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ShopUpdateRequest {
    name: Option<String>,
    status: Option<String>,
    credentials: Option<Value>,
    auth_meta_json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct AdminShopListQuery {
    tenant_id: String,
    platform: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AdminShopCreateRequest {
    tenant_id: String,
    platform: String,
    platform_shop_id: String,
    name: String,
    status: Option<String>,
    credentials: Option<Value>,
    auth_meta_json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct AdminShopUpdateRequest {
    tenant_id: String,
    name: Option<String>,
    status: Option<String>,
    credentials: Option<Value>,
    auth_meta_json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct AdminShopTokenQuery {
    tenant_id: String,
}

#[derive(Debug, Deserialize)]
struct AdminSyncJobListQuery {
    tenant_id: String,
    shop_id: Option<String>,
    job_type: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ShopAuthorizeRequest {
    redirect_uri: Option<String>,
}

#[derive(Debug, Serialize)]
struct ShopAuthorizeResponse {
    authorize_url: String,
    state: String,
}

#[derive(Debug, Deserialize)]
struct ShopAuthCallbackRequest {
    access_token: Option<String>,
    refresh_token: Option<String>,
    code: Option<String>,
    redirect_uri: Option<String>,
    expires_at: Option<String>,
    scopes: Option<String>,
    auth_meta_json: Option<Value>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug, Serialize)]
struct TokenRefreshRequestResponse {
    outbox_id: uuid::Uuid,
    token_status: Option<String>,
    expires_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SyncJobListQuery {
    shop_id: Option<String>,
    job_type: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct SyncJobListResponse {
    items: Vec<SyncJob>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct SyncJobRunRequest {
    job_type: String,
    cursor: Option<String>,
    #[serde(alias = "last_time")]
    watermark: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlatformOrderManualDownloadRequest {
    cursor: Option<String>,
    watermark: Option<String>,
    #[serde(alias = "last_time")]
    last_time: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlatformOrderTimeRangeDownloadRequest {
    start_time: String,
    end_time: String,
    cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlatformOrderNoDownloadRequest {
    order_nos: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PlatformOrderDownloadResponse {
    job: SyncJob,
    criteria: Value,
}

#[derive(Debug, Serialize)]
struct PlatformOrderNoDownloadResult {
    platform_order_id: String,
    success: bool,
    order_id: Option<OrderId>,
    message: Option<String>,
}

#[derive(Debug, Serialize)]
struct PlatformOrderNoDownloadResponse {
    total: i64,
    success_count: i64,
    failure_count: i64,
    items: Vec<PlatformOrderNoDownloadResult>,
}

#[derive(Debug, Deserialize)]
struct SyncJobRunListQuery {
    job_id: Option<String>,
    shop_id: Option<String>,
    job_type: Option<String>,
    status: Option<String>,
    started_from: Option<String>,
    started_to: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct SyncJobRunListResponse {
    items: Vec<SyncJobRun>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ShopSalesConfigListQuery {
    platform: Option<String>,
    status: Option<String>,
    shop_id: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ShopSalesConfigRequest {
    auto_download_orders: bool,
    auto_to_review: bool,
    download_interval_minutes: i64,
    auto_merge_non_shipping_time: bool,
    non_shipping_time_start: Option<String>,
    non_shipping_time_end: Option<String>,
    enable_stock_sync: bool,
    enable_after_sales_download: bool,
    enable_logistics_upload: bool,
}

#[derive(Debug, Serialize)]
struct ShopSalesConfigView {
    shop_id: ShopId,
    platform: Platform,
    platform_shop_id: String,
    name: String,
    status: String,
    auto_download_orders: bool,
    auto_to_review: bool,
    download_interval_minutes: i64,
    auto_merge_non_shipping_time: bool,
    non_shipping_time_start: Option<String>,
    non_shipping_time_end: Option<String>,
    enable_stock_sync: bool,
    enable_after_sales_download: bool,
    enable_logistics_upload: bool,
}

#[derive(Debug, Serialize)]
struct ShopSalesConfigListResponse {
    items: Vec<ShopSalesConfigView>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct OrderSyncSettingsView {
    shop_id: ShopId,
    auto_download_orders: bool,
    download_interval_minutes: i64,
}

#[derive(Debug, Deserialize)]
struct OrderSyncSettingsUpdateRequest {
    auto_download_orders: Option<bool>,
    download_interval_minutes: Option<i64>,
}

#[derive(Debug, Serialize)]
struct InventorySyncSettingsView {
    shop_id: ShopId,
    enable_stock_sync: bool,
}

#[derive(Debug, Deserialize)]
struct InventorySyncSettingsUpdateRequest {
    enable_stock_sync: bool,
}

#[derive(Debug, Serialize)]
struct AfterSalesSyncSettingsView {
    shop_id: ShopId,
    enable_after_sales_download: bool,
}

#[derive(Debug, Deserialize)]
struct AfterSalesSyncSettingsUpdateRequest {
    enable_after_sales_download: bool,
}

#[derive(Debug, Deserialize)]
struct PlatformInventorySyncRunRequest {
    sku_id: Option<String>,
    warehouse_id: Option<String>,
    status: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct PlatformInventorySyncRunResponse {
    requested: i64,
    skipped: i64,
    jobs: Vec<StockSyncJobResponse>,
}

#[derive(Debug, Deserialize)]
struct PlatformAfterSalesSyncRunRequest {
    cursor: Option<String>,
    watermark: Option<String>,
    #[serde(alias = "last_time")]
    last_time: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MappingRuleListQuery {
    platform: Option<String>,
    entity_type: Option<String>,
    enabled: Option<bool>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct MappingRuleListResponse {
    items: Vec<erp_repo::mapping_rules::MappingRule>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct MappingRuleCreateRequest {
    platform: String,
    entity_type: String,
    version: i32,
    enabled: Option<bool>,
    rule_json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct MappingRuleUpdateRequest {
    enabled: Option<bool>,
    rule_json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct SupplierListQuery {
    keyword: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct SupplierListResponse {
    items: Vec<Supplier>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct SupplierCreateRequest {
    name: String,
    contact: Option<String>,
    settlement_terms: Option<String>,
    status: Option<String>,
    request_id: Option<String>,
    customer_no: Option<String>,
    supplier_no: Option<String>,
    term_days: Option<i32>,
    tax_no: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SupplierUpdateRequest {
    name: Option<String>,
    contact: Option<String>,
    settlement_terms: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PurchaseOrderListQuery {
    supplier_id: Option<String>,
    warehouse_id: Option<String>,
    status: Option<String>,
    created_from: Option<String>,
    created_to: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct PurchaseOrderItemRequest {
    sku_id: String,
    qty: i64,
    price_cent: i64,
}

#[derive(Debug, Deserialize)]
struct PurchaseOrderCreateRequest {
    supplier_id: String,
    warehouse_id: String,
    expected_at: Option<String>,
    remark: Option<String>,
    items: Vec<PurchaseOrderItemRequest>,
    /// Shared command identity. A trade purchase with this field does not read a BOM.
    #[serde(default)]
    request_id: Option<String>,
    #[serde(default)]
    partner_code: Option<String>,
    #[serde(default)]
    currency_code: Option<String>,
    #[serde(default)]
    document_date: Option<String>,
    #[serde(default)]
    revision_action: Option<String>,
    #[serde(default)]
    po_id: Option<String>,
    #[serde(default)]
    line_no: Option<i32>,
    #[serde(default)]
    due_date: Option<String>,
    #[serde(default)]
    tax_cent: Option<i64>,
    #[serde(default)]
    receive_qty: Option<i64>,
    #[serde(default)]
    arrival_qty: Option<i64>,
    #[serde(default)]
    release_qty: Option<i64>,
    #[serde(default)]
    return_qty: Option<i64>,
    #[serde(default)]
    issue_qty: Option<i64>,
    #[serde(default)]
    quality_id: Option<String>,
    #[serde(default)]
    batch_code: Option<String>,
    #[serde(default)]
    location_id: Option<String>,
    #[serde(default)]
    unit_cost_cent: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct PurchaseOrderUpdateRequest {
    supplier_id: Option<String>,
    warehouse_id: Option<String>,
    expected_at: Option<String>,
    remark: Option<String>,
    items: Option<Vec<PurchaseOrderItemRequest>>,
}

#[derive(Debug, Deserialize)]
struct PurchaseOrderReceiveItemRequest {
    sku_id: String,
    qty: i64,
}

#[derive(Debug, Deserialize)]
struct PurchaseOrderReceiveRequest {
    warehouse_id: String,
    items: Vec<PurchaseOrderReceiveItemRequest>,
    update_cost: Option<bool>,
}

#[derive(Debug, Serialize)]
struct PurchaseOrderSummaryResponse {
    po_id: uuid::Uuid,
    supplier_id: uuid::Uuid,
    supplier_name: Option<String>,
    warehouse_id: Option<uuid::Uuid>,
    expected_at: Option<DateTime<Utc>>,
    remark: Option<String>,
    status: String,
    purchase_order_status: String,
    total_cent: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct PurchaseOrderListResponse {
    items: Vec<PurchaseOrderSummaryResponse>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct PurchaseOrderItemResponse {
    sku_id: uuid::Uuid,
    sku_code: Option<String>,
    qty: i64,
    price_cent: i64,
    received_qty: i64,
}

#[derive(Debug, Serialize)]
struct PurchaseReceiptSummaryResponse {
    receipt_id: uuid::Uuid,
    po_id: uuid::Uuid,
    supplier_id: uuid::Uuid,
    supplier_name: Option<String>,
    warehouse_id: uuid::Uuid,
    status: String,
    receiving_status: String,
    confirmed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct PurchaseOrderDetailResponse {
    order: PurchaseOrderSummaryResponse,
    items: Vec<PurchaseOrderItemResponse>,
    receipts: Vec<PurchaseReceiptSummaryResponse>,
}

#[derive(Debug, Serialize)]
struct PurchaseReceiptResponse {
    receipt_id: uuid::Uuid,
    po_id: uuid::Uuid,
    supplier_id: Option<uuid::Uuid>,
    supplier_name: Option<String>,
    warehouse_id: uuid::Uuid,
    status: String,
    receiving_status: String,
    confirmed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct PurchaseReceiptDetailResponse {
    receipt: PurchaseReceiptResponse,
    items: Vec<PurchaseReceiptItemResponse>,
}

#[derive(Debug, Serialize)]
struct PurchaseReceiptListResponse {
    items: Vec<PurchaseReceiptSummaryResponse>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct PurchaseReceiptItemResponse {
    sku_id: uuid::Uuid,
    qty: i64,
}

#[derive(Debug, Serialize)]
struct PurchaseOrderReceiveResponse {
    order: PurchaseOrderSummaryResponse,
    receipt: PurchaseReceiptResponse,
    receipt_items: Vec<PurchaseReceiptItemResponse>,
    inbound_doc_id: uuid::Uuid,
    ap_doc_id: Option<uuid::Uuid>,
}

#[derive(Debug, Deserialize)]
struct PurchaseReceiptListQuery {
    supplier_id: Option<String>,
    warehouse_id: Option<String>,
    status: Option<String>,
    created_from: Option<String>,
    created_to: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct PurchaseSuggestionListQuery {
    sku_id: Option<String>,
    reason: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct PurchaseSuggestionResponse {
    sku_id: uuid::Uuid,
    sku_code: Option<String>,
    suggested_qty: i64,
    reason: String,
    computed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct PurchaseSuggestionListResponse {
    items: Vec<PurchaseSuggestionResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct PurchaseReturnListQuery {
    supplier_id: Option<String>,
    warehouse_id: Option<String>,
    status: Option<String>,
    created_from: Option<String>,
    created_to: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct PurchaseReturnItemRequest {
    sku_id: String,
    qty: i64,
}

#[derive(Debug, Deserialize)]
struct PurchaseReturnCreateRequest {
    po_id: String,
    warehouse_id: String,
    items: Vec<PurchaseReturnItemRequest>,
}

#[derive(Debug, Serialize)]
struct PurchaseReturnSummaryResponse {
    return_id: uuid::Uuid,
    po_id: uuid::Uuid,
    supplier_id: Option<uuid::Uuid>,
    supplier_name: Option<String>,
    warehouse_id: uuid::Uuid,
    status: String,
    confirmed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct PurchaseReturnListResponse {
    items: Vec<PurchaseReturnSummaryResponse>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct PurchaseReturnItemResponse {
    sku_id: uuid::Uuid,
    qty: i64,
}

#[derive(Debug, Serialize)]
struct PurchaseReturnDetailResponse {
    purchase_return: PurchaseReturnSummaryResponse,
    items: Vec<PurchaseReturnItemResponse>,
}

#[derive(Debug, Serialize)]
struct PurchaseReturnConfirmResponse {
    purchase_return: PurchaseReturnSummaryResponse,
    outbound_doc_id: uuid::Uuid,
}

#[derive(Debug, Deserialize)]
struct ProcurementRequestListQuery {
    department: Option<String>,
    requester: Option<String>,
    status: Option<String>,
    created_from: Option<String>,
    created_to: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ProcurementRequestItemRequest {
    sku_id: String,
    qty: i64,
    price_cent: i64,
}

#[derive(Debug, Deserialize)]
struct ProcurementRequestCreateRequest {
    department: Option<String>,
    requester: Option<String>,
    expected_at: Option<String>,
    remark: Option<String>,
    items: Vec<ProcurementRequestItemRequest>,
}

#[derive(Debug, Deserialize)]
struct ProcurementRequestUpdateRequest {
    department: Option<String>,
    requester: Option<String>,
    status: Option<String>,
    expected_at: Option<String>,
    remark: Option<String>,
    items: Option<Vec<ProcurementRequestItemRequest>>,
}

#[derive(Debug, Serialize)]
struct ProcurementRequestResponse {
    request_id: uuid::Uuid,
    department: Option<String>,
    requester: Option<String>,
    status: String,
    budget_cent: i64,
    expected_at: Option<DateTime<Utc>>,
    remark: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ProcurementRequestItemResponse {
    sku_id: uuid::Uuid,
    sku_code: Option<String>,
    qty: i64,
    price_cent: i64,
}

#[derive(Debug, Serialize)]
struct ProcurementRequestDetailResponse {
    request: ProcurementRequestResponse,
    items: Vec<ProcurementRequestItemResponse>,
}

#[derive(Debug, Serialize)]
struct ProcurementRequestListResponse {
    items: Vec<ProcurementRequestResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ProcurementPlanListQuery {
    supplier_id: Option<String>,
    status: Option<String>,
    created_from: Option<String>,
    created_to: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ProcurementPlanItemRequest {
    sku_id: String,
    qty: i64,
    price_cent: i64,
}

#[derive(Debug, Deserialize)]
struct ProcurementPlanCreateRequest {
    request_id: Option<String>,
    supplier_id: Option<String>,
    planned_at: Option<String>,
    expected_at: Option<String>,
    remark: Option<String>,
    items: Vec<ProcurementPlanItemRequest>,
}

#[derive(Debug, Deserialize)]
struct ProcurementPlanGenerateRequest {
    request_id: Option<String>,
    supplier_id: Option<String>,
    sku_id: Option<String>,
    reason: Option<String>,
    min_suggested_qty: Option<i64>,
    max_items: Option<i64>,
    default_price_cent: Option<i64>,
    planned_at: Option<String>,
    expected_at: Option<String>,
    remark: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProcurementPlanUpdateRequest {
    supplier_id: Option<String>,
    status: Option<String>,
    planned_at: Option<String>,
    expected_at: Option<String>,
    remark: Option<String>,
    items: Option<Vec<ProcurementPlanItemRequest>>,
}

#[derive(Debug, Serialize)]
struct ProcurementPlanResponse {
    plan_id: uuid::Uuid,
    request_id: Option<uuid::Uuid>,
    supplier_id: Option<uuid::Uuid>,
    supplier_name: Option<String>,
    status: String,
    total_cent: i64,
    planned_at: Option<DateTime<Utc>>,
    expected_at: Option<DateTime<Utc>>,
    remark: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ProcurementPlanItemResponse {
    sku_id: uuid::Uuid,
    sku_code: Option<String>,
    qty: i64,
    price_cent: i64,
}

#[derive(Debug, Serialize)]
struct ProcurementPlanDetailResponse {
    plan: ProcurementPlanResponse,
    items: Vec<ProcurementPlanItemResponse>,
}

#[derive(Debug, Serialize)]
struct ProcurementPlanListResponse {
    items: Vec<ProcurementPlanResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ProcurementSourcingListQuery {
    keyword: Option<String>,
    supplier_id: Option<String>,
    status: Option<String>,
    created_from: Option<String>,
    created_to: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ProcurementSourcingCreateRequest {
    title: String,
    category: Option<String>,
    supplier_id: Option<String>,
    status: Option<String>,
    target_price_cent: i64,
    remark: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProcurementSourcingUpdateRequest {
    title: Option<String>,
    category: Option<String>,
    supplier_id: Option<String>,
    status: Option<String>,
    target_price_cent: Option<i64>,
    remark: Option<String>,
}

#[derive(Debug, Serialize)]
struct ProcurementSourcingResponse {
    sourcing_id: uuid::Uuid,
    title: String,
    category: Option<String>,
    supplier_id: Option<uuid::Uuid>,
    supplier_name: Option<String>,
    status: String,
    target_price_cent: i64,
    remark: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ProcurementSourcingListResponse {
    items: Vec<ProcurementSourcingResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ProcurementInspectionListQuery {
    supplier_id: Option<String>,
    warehouse_id: Option<String>,
    status: Option<String>,
    created_from: Option<String>,
    created_to: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ProcurementInspectionItemRequest {
    sku_id: String,
    qty: i64,
    passed_qty: i64,
    failed_qty: i64,
    result: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProcurementInspectionCreateRequest {
    receipt_id: Option<String>,
    supplier_id: Option<String>,
    warehouse_id: Option<String>,
    inspected_at: Option<String>,
    remark: Option<String>,
    items: Vec<ProcurementInspectionItemRequest>,
}

#[derive(Debug, Deserialize)]
struct ProcurementInspectionUpdateRequest {
    status: Option<String>,
    inspected_at: Option<String>,
    remark: Option<String>,
    items: Option<Vec<ProcurementInspectionItemRequest>>,
}

#[derive(Debug, Serialize)]
struct ProcurementInspectionResponse {
    inspection_id: uuid::Uuid,
    receipt_id: Option<uuid::Uuid>,
    supplier_id: Option<uuid::Uuid>,
    supplier_name: Option<String>,
    warehouse_id: Option<uuid::Uuid>,
    status: String,
    inspected_at: Option<DateTime<Utc>>,
    remark: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ProcurementInspectionItemResponse {
    sku_id: uuid::Uuid,
    sku_code: Option<String>,
    qty: i64,
    passed_qty: i64,
    failed_qty: i64,
    result: Option<String>,
}

#[derive(Debug, Serialize)]
struct ProcurementInspectionDetailResponse {
    inspection: ProcurementInspectionResponse,
    items: Vec<ProcurementInspectionItemResponse>,
}

#[derive(Debug, Serialize)]
struct ProcurementInspectionListResponse {
    items: Vec<ProcurementInspectionResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ProcurementReconciliationListQuery {
    supplier_id: Option<String>,
    status: Option<String>,
    period_from: Option<String>,
    period_to: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ProcurementReconciliationCreateRequest {
    supplier_id: String,
    period_start: String,
    period_end: String,
    amount_cent: i64,
    status: Option<String>,
    remark: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProcurementReconciliationUpdateRequest {
    supplier_id: Option<String>,
    period_start: Option<String>,
    period_end: Option<String>,
    amount_cent: Option<i64>,
    status: Option<String>,
    remark: Option<String>,
}

#[derive(Debug, Serialize)]
struct ProcurementReconciliationResponse {
    reconciliation_id: uuid::Uuid,
    supplier_id: uuid::Uuid,
    supplier_name: Option<String>,
    period_start: chrono::NaiveDate,
    period_end: chrono::NaiveDate,
    amount_cent: i64,
    status: String,
    remark: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ProcurementReconciliationListResponse {
    items: Vec<ProcurementReconciliationResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct SupplierPerformanceListQuery {
    supplier_id: Option<String>,
    status: Option<String>,
    period_from: Option<String>,
    period_to: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct SupplierPerformanceCreateRequest {
    supplier_id: String,
    period_start: String,
    period_end: String,
    delivery_score: i32,
    quality_score: i32,
    price_score: i32,
    status: Option<String>,
    remark: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SupplierPerformanceUpdateRequest {
    supplier_id: Option<String>,
    period_start: Option<String>,
    period_end: Option<String>,
    delivery_score: Option<i32>,
    quality_score: Option<i32>,
    price_score: Option<i32>,
    status: Option<String>,
    remark: Option<String>,
}

#[derive(Debug, Serialize)]
struct SupplierPerformanceResponse {
    performance_id: uuid::Uuid,
    supplier_id: uuid::Uuid,
    supplier_name: Option<String>,
    period_start: chrono::NaiveDate,
    period_end: chrono::NaiveDate,
    delivery_score: i32,
    quality_score: i32,
    price_score: i32,
    total_score: i32,
    status: String,
    remark: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct SupplierPerformanceListResponse {
    items: Vec<SupplierPerformanceResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct InventoryListQuery {
    sku_id: Option<String>,
    warehouse_id: Option<String>,
    sku_code: Option<String>,
    barcode: Option<String>,
    tag: Option<String>,
    on_hand_negative: Option<bool>,
    available_below_safety: Option<bool>,
    available_min: Option<String>,
    available_max: Option<String>,
    cursor: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct InventorySummaryListQuery {
    sku_id: Option<String>,
    warehouse_id: Option<String>,
    sku_code: Option<String>,
    barcode: Option<String>,
    tag: Option<String>,
    on_hand_negative: Option<bool>,
    available_below_safety: Option<bool>,
    available_min: Option<String>,
    available_max: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct InventoryCostListQuery {
    sku_id: Option<String>,
    warehouse_id: Option<String>,
    sku_code: Option<String>,
    barcode: Option<String>,
    tag: Option<String>,
    on_hand_negative: Option<bool>,
    available_below_safety: Option<bool>,
    available_min: Option<String>,
    available_max: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct InventoryLedgerListQuery {
    sku_id: Option<String>,
    sku_code: Option<String>,
    warehouse_id: Option<String>,
    occurred_from: Option<String>,
    occurred_to: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct InventoryAtpQuery {
    warehouse_id: String,
    sku_id: String,
    request_qty: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProductPackagingSpecUpsertRequest {
    level_code: String,
    unit_name: String,
    units_per_level: i64,
    length_mm: Option<i32>,
    width_mm: Option<i32>,
    height_mm: Option<i32>,
    weight_gram: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct InventoryReconciliationCheckRequest {
    warehouse_id: String,
    sku_id: String,
    expected_on_hand: i64,
    source_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InventoryAdjustmentRequest {
    warehouse_id: String,
    sku_id: String,
    batch_id: Option<String>,
    quantity_delta: i64,
    reason: Option<String>,
    reference_id: Option<String>,
    #[serde(default)]
    request_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InventoryMovementRequest {
    warehouse_id: String,
    sku_id: String,
    batch_id: Option<String>,
    quantity: i64,
    reason: Option<String>,
    reference_id: Option<String>,
    /// Original identity for an atomic movement and its immutable result.
    /// Legacy callers that omit it cannot recover an unknown outcome by identity.
    #[serde(default)]
    request_id: Option<String>,
    #[serde(default)]
    location_id: Option<String>,
    #[serde(default)]
    batch_code: Option<String>,
    #[serde(default)]
    quality_status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InventoryReservationRequest {
    warehouse_id: String,
    sku_id: String,
    quantity: i64,
    order_id: Option<String>,
    order_item_id: Option<String>,
    reference_id: Option<String>,
    #[serde(default)]
    request_id: Option<String>,
    #[serde(default)]
    location_id: Option<String>,
    #[serde(default)]
    batch_code: Option<String>,
    #[serde(default)]
    quality_status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InventoryReleaseRequest {
    warehouse_id: String,
    sku_id: String,
    quantity: i64,
    order_item_id: Option<String>,
    reference_id: Option<String>,
    #[serde(default)]
    request_id: Option<String>,
    #[serde(default)]
    location_id: Option<String>,
    #[serde(default)]
    batch_code: Option<String>,
    #[serde(default)]
    quality_status: Option<String>,
    #[serde(default)]
    order_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InboundDocListQuery {
    status: Option<String>,
    inbound_status: Option<String>,
    receipt_status: Option<String>,
    inbound_qc_status: Option<String>,
    putaway_status: Option<String>,
    doc_type: Option<String>,
    warehouse_id: Option<String>,
    doc_no: Option<String>,
    doc_no_keyword: Option<String>,
    ref_type: Option<String>,
    ref_id: Option<String>,
    supplier_id: Option<String>,
    sku_id: Option<String>,
    created_from: Option<String>,
    created_to: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct InboundDocItemCreateRequest {
    sku_id: String,
    planned_qty: i64,
}

#[derive(Debug, Deserialize)]
struct InboundDocCreateRequest {
    warehouse_id: String,
    doc_type: String,
    doc_no: String,
    ref_type: Option<String>,
    ref_id: Option<String>,
    #[serde(default)]
    items: Vec<InboundDocItemCreateRequest>,
}

#[derive(Debug, Deserialize)]
struct InboundReceiveItemRequest {
    sku_id: String,
    qty: i64,
    operation_id: String,
}

#[derive(Debug, Deserialize)]
struct InboundReceiveRequest {
    #[serde(default)]
    items: Vec<InboundReceiveItemRequest>,
}

#[derive(Debug, Deserialize)]
struct InboundInspectItemRequest {
    sku_id: String,
    qty: i64,
    result: String,
    note: Option<String>,
    operation_id: String,
}

#[derive(Debug, Deserialize)]
struct InboundInspectRequest {
    #[serde(default)]
    items: Vec<InboundInspectItemRequest>,
}

#[derive(Debug, Deserialize)]
struct InboundPutawayItemRequest {
    sku_id: String,
    qty: i64,
    operation_id: String,
    from_area: Option<String>,
    to_location_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InboundPutawayRequest {
    #[serde(default)]
    items: Vec<InboundPutawayItemRequest>,
}

#[derive(Debug, Deserialize)]
struct InboundRejectItemRequest {
    sku_id: String,
    qty: i64,
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InboundRejectCreateRequest {
    #[serde(default)]
    items: Vec<InboundRejectItemRequest>,
}

#[derive(Debug, Deserialize)]
struct InboundRejectListQuery {
    doc_no_keyword: Option<String>,
    sku_id: Option<String>,
    created_from: Option<String>,
    created_to: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct InboundRejectResponse {
    reject_id: uuid::Uuid,
    doc_id: uuid::Uuid,
    doc_no: String,
    doc_type: String,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    sku_code: Option<String>,
    barcode: Option<String>,
    qty: i64,
    reason: Option<String>,
    operator_id: Option<uuid::Uuid>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct InboundInspectionListResponse {
    items: Vec<InboundInspection>,
}

#[derive(Debug, Serialize)]
struct InboundRejectListResponse {
    items: Vec<InboundRejectResponse>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct InboundRejectCreateResponse {
    inserted: i64,
}

#[derive(Debug, Deserialize)]
struct ShipmentRequest {
    order_id: String,
    warehouse_id: Option<String>,
    carrier_code: Option<String>,
    tracking_no: Option<String>,
    weight_gram: Option<i32>,
    serials: Option<Vec<ShipmentSerialEntry>>,
}

struct ShipmentRequestArgs {
    tenant_id: TenantId,
    order_id: OrderId,
    warehouse_id: Option<uuid::Uuid>,
    carrier_code: Option<String>,
    tracking_no: Option<String>,
    weight_gram: Option<i32>,
    serials: Vec<ShipmentSerialEntry>,
    idempotency_key: String,
}

#[derive(Debug, Deserialize)]
struct ShipOrderRequest {
    warehouse_id: Option<String>,
    carrier_code: Option<String>,
    tracking_no: Option<String>,
    weight_gram: Option<i32>,
    print_label: Option<bool>,
    serials: Option<Vec<ShipmentSerialEntry>>,
}

#[derive(Debug, Deserialize)]
struct ShipmentListQuery {
    shop_id: Option<String>,
    status: Option<String>,
    platform: Option<String>,
    tracking_no: Option<String>,
    updated_from: Option<String>,
    updated_to: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct LogisticsShipmentListQuery {
    tracking_no: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Serialize)]
struct LogisticsShipmentSummary {
    shipment_id: uuid::Uuid,
    carrier_code: Option<String>,
    tracking_no: Option<String>,
    status: String,
    weight_gram: Option<i32>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct LogisticsShipmentListResponse {
    items: Vec<LogisticsShipmentSummary>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct ShipmentTrackingResponse {
    shipment_id: uuid::Uuid,
    carrier_code: Option<String>,
    tracking_no: Option<String>,
    latest_status: String,
    latest_description: Option<String>,
    latest_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ShipmentView {
    shipment_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    order_id: uuid::Uuid,
    shop_id: uuid::Uuid,
    platform: String,
    warehouse_id: uuid::Uuid,
    carrier_code: String,
    tracking_no: Option<String>,
    status: String,
    ship_status: String,
    weight_gram: Option<i32>,
    sender_address_id: Option<uuid::Uuid>,
    last_push_error: String,
    push_status: Option<String>,
    push_error_message: Option<String>,
    push_attempted_at: Option<DateTime<Utc>>,
    printed_label: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct ShipmentPackageRequest {
    package_no: String,
    carrier_code: Option<String>,
    tracking_no: Option<String>,
    weight_gram: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct ShipmentPackageUpsertRequest {
    packages: Vec<ShipmentPackageRequest>,
}

#[derive(Debug, Serialize)]
struct ShipmentPackageResponse {
    package_id: uuid::Uuid,
    shipment_id: uuid::Uuid,
    package_no: String,
    carrier_code: Option<String>,
    tracking_no: Option<String>,
    weight_gram: Option<i32>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ShipmentPackageListResponse {
    items: Vec<ShipmentPackageResponse>,
}

#[derive(Debug, Serialize)]
struct ShipmentListResponse {
    data: Vec<ShipmentView>,
}

#[derive(Debug, Clone)]
struct ShipmentPushSummary {
    status: String,
    error_message: String,
    attempted_at: DateTime<Utc>,
}

async fn load_latest_push_attempts(
    pool: &PgPool,
    tenant_id: TenantId,
    shipment_ids: &[uuid::Uuid],
) -> anyhow::Result<HashMap<uuid::Uuid, ShipmentPushSummary>> {
    if shipment_ids.is_empty() {
        return Ok(HashMap::new());
    }

    #[derive(sqlx::FromRow)]
    #[allow(dead_code)]
    struct Row {
        shipment_id: uuid::Uuid,
        status: String,
        error_message: String,
        created_at: DateTime<Utc>,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        select distinct on (shipment_id)
            shipment_id, status, error_message, created_at
        from shipment_push_attempts
        where tenant_id = $1 and shipment_id = any($2)
        order by shipment_id, created_at desc, attempt_id desc
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(shipment_ids)
    .fetch_all(pool)
    .await?;

    let mut map = HashMap::new();
    for row in rows {
        map.insert(
            row.shipment_id,
            ShipmentPushSummary {
                status: row.status,
                error_message: row.error_message,
                attempted_at: row.created_at,
            },
        );
    }
    Ok(map)
}

async fn load_latest_logistics_match_product_id(
    pool: &PgPool,
    tenant_id: TenantId,
    shipment_id: Option<uuid::Uuid>,
    order_id: Option<uuid::Uuid>,
) -> anyhow::Result<Option<uuid::Uuid>> {
    if let Some(shipment_id) = shipment_id {
        let product_id: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            select product_id
            from logistics_match_hits
            where tenant_id = $1
              and shipment_id = $2
              and success = true
              and product_id is not null
            order by created_at desc, hit_id desc
            limit 1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .fetch_optional(pool)
        .await?;
        return Ok(product_id);
    }

    if let Some(order_id) = order_id {
        let product_id: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            select product_id
            from logistics_match_hits
            where tenant_id = $1
              and order_id = $2
              and success = true
              and product_id is not null
            order by created_at desc, hit_id desc
            limit 1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(order_id)
        .fetch_optional(pool)
        .await?;
        return Ok(product_id);
    }

    Ok(None)
}

#[derive(Debug, Deserialize)]
struct ShipmentPreprocessPreviewRequest {
    warehouse_id: Option<String>,
    carrier_code: Option<String>,
    tracking_no: Option<String>,
    weight_gram: Option<i32>,
}

#[derive(Debug, Serialize)]
struct ShipmentPreprocessPreviewResult {
    warehouse_id: Option<uuid::Uuid>,
    carrier_code: Option<String>,
    tracking_no: Option<String>,
    weight_gram: Option<i32>,
    sender_address_id: Option<uuid::Uuid>,
    auto_freeze: bool,
    auto_waybill_reserve: bool,
}

#[derive(Debug, Serialize)]
struct ShipmentPreprocessPreviewResponse {
    matched: bool,
    strategy_id: Option<uuid::Uuid>,
    hit_reason: Option<String>,
    result: ShipmentPreprocessPreviewResult,
}

#[derive(Debug, Deserialize)]
struct ShipmentPreprocessStrategyRequest {
    name: String,
    enabled: Option<bool>,
    priority: Option<i32>,
    effective_from: Option<String>,
    effective_to: Option<String>,
    conditions_json: Option<Value>,
    rules_json: Option<Value>,
    auto_freeze: Option<bool>,
    auto_waybill_reserve: Option<bool>,
    sender_address_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ShipmentPreprocessStrategyUpdateRequest {
    name: Option<String>,
    enabled: Option<bool>,
    priority: Option<i32>,
    effective_from: Option<String>,
    effective_to: Option<String>,
    conditions_json: Option<Value>,
    rules_json: Option<Value>,
    auto_freeze: Option<bool>,
    auto_waybill_reserve: Option<bool>,
    sender_address_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ShipmentPreprocessStrategyListQuery {
    enabled: Option<bool>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ShipmentPreprocessStrategyListResponse {
    items: Vec<ShipmentPreprocessStrategy>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ShipmentPreprocessHitListQuery {
    strategy_id: Option<String>,
    shipment_id: Option<String>,
    order_id: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ShipmentPreprocessHitListResponse {
    items: Vec<ShipmentPreprocessHit>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct PickRecommendStrategyRequest {
    name: String,
    enabled: Option<bool>,
    priority: Option<i32>,
    method: Option<String>,
    prefer_location_priority: Option<bool>,
    prefer_pick_zone: Option<bool>,
    prefer_sufficient: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct PickRecommendStrategyUpdateRequest {
    name: Option<String>,
    enabled: Option<bool>,
    priority: Option<i32>,
    method: Option<String>,
    prefer_location_priority: Option<bool>,
    prefer_pick_zone: Option<bool>,
    prefer_sufficient: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct PickRecommendStrategyListQuery {
    enabled: Option<bool>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct PickRecommendStrategyListResponse {
    items: Vec<PickRecommendStrategy>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct PickRecommendHitListQuery {
    strategy_id: Option<String>,
    pick_task_id: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct PickRecommendHitListResponse {
    items: Vec<PickRecommendHit>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct WaybillImportRequest {
    shop_id: String,
    carrier_code: String,
    waybills: Option<Vec<String>>,
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WaybillImportQuery {
    shop_id: Option<String>,
    carrier_code: Option<String>,
}

#[derive(Debug, Serialize)]
struct WaybillImportResponse {
    inserted: i64,
    total: i64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct WaybillAllocateRequest {
    shipment_id: Option<String>,
    shop_id: Option<String>,
    carrier_code: Option<String>,
    waybill_no: Option<String>,
    count: Option<i32>,
    platform: Option<String>,
}

#[derive(Debug, Serialize)]
struct WaybillAllocateResponse {
    waybills: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct WaybillReleaseRequest {
    shipment_id: String,
}

#[derive(Debug, Serialize)]
struct WaybillSummaryResponse {
    available: i64,
    reserved: i64,
    used: i64,
    cancelled: i64,
}

#[derive(Debug, Deserialize)]
struct WaybillListQuery {
    shop_id: Option<String>,
    carrier_code: Option<String>,
    status: Option<String>,
    shipment_id: Option<String>,
    waybill_no: Option<String>,
    created_from: Option<String>,
    created_to: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Serialize)]
struct WaybillView {
    waybill_id: uuid::Uuid,
    shop_id: uuid::Uuid,
    carrier_code: String,
    waybill_no: String,
    status: String,
    reserved_at: Option<DateTime<Utc>>,
    used_at: Option<DateTime<Utc>>,
    shipment_id: Option<uuid::Uuid>,
}

#[derive(Debug, Serialize)]
struct WaybillListResponse {
    items: Vec<WaybillView>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct OmsAllocationRequest {
    carrier_code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OmsResendRequest {
    origin_shipment_id: Option<String>,
    carrier_code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OmsShipmentCancelRequest {
    reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct OmsAllocationShortageView {
    order_item_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    requested: i64,
    available: i64,
}

#[derive(Debug, Serialize)]
struct OmsShipmentItemView {
    order_item_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    sku_code: Option<String>,
    qty: i64,
}

#[derive(Debug, Serialize)]
struct OmsShipmentSummary {
    shipment_id: uuid::Uuid,
    shipment_no: Option<String>,
    shipment_type: String,
    origin_shipment_id: Option<uuid::Uuid>,
    tenant_id: uuid::Uuid,
    order_id: uuid::Uuid,
    shop_id: uuid::Uuid,
    platform: String,
    platform_order_id: String,
    warehouse_id: uuid::Uuid,
    carrier_code: Option<String>,
    tracking_no: Option<String>,
    status: String,
    weight_gram: Option<i32>,
    shipped_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    items: Option<Vec<OmsShipmentItemView>>,
}

#[derive(Debug, Serialize)]
struct OmsShipmentListResponse {
    items: Vec<OmsShipmentSummary>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct OmsAllocationResponse {
    status: String,
    shipments: Vec<OmsShipmentSummary>,
    shortages: Vec<OmsAllocationShortageView>,
}

#[derive(Debug, Serialize)]
struct OmsReleaseResponse {
    released: i64,
}

#[derive(Debug, Serialize)]
struct OmsShipmentCancelResponse {
    cancel_request_id: uuid::Uuid,
    status: String,
}

#[derive(Debug, Deserialize)]
struct OmsShipmentListQuery {
    order_id: Option<String>,
    warehouse_id: Option<String>,
    status: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct OmsShipRequest {
    carrier_code: Option<String>,
    tracking_no: Option<String>,
    shipped_at: Option<String>,
    weight_gram: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct WmsOutboundListQuery {
    warehouse_id: Option<String>,
    status: Option<String>,
    outbound_status: Option<String>,
    pick_status: Option<String>,
    verify_status: Option<String>,
    pack_status: Option<String>,
    ship_status: Option<String>,
    doc_type: Option<String>,
    doc_no: Option<String>,
    doc_no_keyword: Option<String>,
    shipment_id: Option<String>,
    shop_id: Option<String>,
    carrier_code: Option<String>,
    tracking_no: Option<String>,
    tag: Option<String>,
    exception_flag: Option<String>,
    created_from: Option<String>,
    created_to: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct WmsWaitingShipListQuery {
    warehouse_id: Option<String>,
    shipment_id: Option<String>,
    shop_id: Option<String>,
    carrier_code: Option<String>,
    tracking_no: Option<String>,
    doc_no_keyword: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct WmsOutboundItemRequest {
    sku_id: String,
    planned_qty: i64,
}

#[derive(Debug, Deserialize)]
struct WmsOutboundCreateRequest {
    sales_order_id: Option<String>,
    warehouse_id: String,
    doc_type: String,
    shipment_id: Option<String>,
    doc_no: String,
    items: Vec<WmsOutboundItemRequest>,
}

#[derive(Debug, Serialize)]
struct WmsOutboundDocResponse {
    outbound_doc_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    doc_type: String,
    shipment_id: Option<uuid::Uuid>,
    order_id: Option<uuid::Uuid>,
    platform: Option<String>,
    platform_order_id: Option<String>,
    shop_id: Option<uuid::Uuid>,
    receiver_name: Option<String>,
    receiver_phone: Option<String>,
    receiver_address: Option<String>,
    carrier_code: Option<String>,
    tracking_no: Option<String>,
    exception_flags: Vec<String>,
    tags: Vec<String>,
    doc_no: String,
    status: String,
    outbound_status: String,
    pick_status: String,
    verify_status: String,
    pack_status: String,
    ship_status: String,
    picked_at: Option<DateTime<Utc>>,
    packed_at: Option<DateTime<Utc>>,
    shipped_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct WmsOutboundItemResponse {
    sku_id: uuid::Uuid,
    planned_qty: i64,
    picked_qty: i64,
    packed_qty: i64,
    batch_enabled: bool,
    serial_enabled: bool,
}

#[derive(Debug, Serialize)]
struct WmsPickTaskResponse {
    pick_task_id: uuid::Uuid,
    outbound_doc_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    from_location_id: Option<uuid::Uuid>,
    container_id: Option<uuid::Uuid>,
    qty: i64,
    picked_qty: i64,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct WmsPackRecordResponse {
    pack_record_id: uuid::Uuid,
    shipment_id: Option<uuid::Uuid>,
    package_no: String,
    weight_gram: Option<i32>,
    operator_id: Option<uuid::Uuid>,
    packed_at: DateTime<Utc>,
    pack_status: String,
    void_reason: Option<String>,
    voided_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct WmsOutboundDetailResponse {
    doc: WmsOutboundDocResponse,
    items: Vec<WmsOutboundItemResponse>,
    pick_tasks: Vec<WmsPickTaskResponse>,
    pack_records: Vec<WmsPackRecordResponse>,
}

#[derive(Debug, Serialize)]
struct WmsOutboundListResponse {
    items: Vec<WmsOutboundDocResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct WmsWaveListQuery {
    warehouse_id: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct WmsWaveCreateRequest {
    warehouse_id: String,
    cutoff_at: Option<String>,
    max_docs: Option<i64>,
    auto_pick: Option<bool>,
    outbound_doc_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct WmsWaveRunRequest {
    warehouse_id: String,
    cutoff_at: Option<String>,
    max_docs: Option<i64>,
    min_docs: Option<i64>,
    auto_pick: Option<bool>,
}

#[derive(Debug, Serialize)]
struct WmsWaveResponse {
    wave_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct WmsWaveListResponse {
    items: Vec<WmsWaveResponse>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct WmsWaveDetailResponse {
    wave: WmsWaveResponse,
    docs: Vec<WmsOutboundDocResponse>,
    tasks: Vec<WmsPickTaskResponse>,
}

#[derive(Debug, Serialize)]
struct WmsWaveRunResponse {
    status: String,
    wave: Option<WmsWaveDetailResponse>,
}

#[derive(Debug, Deserialize)]
struct WmsWaveShipRequest {
    shipped_at: Option<String>,
    continue_on_error: Option<bool>,
    direct_ship: Option<bool>,
}

#[derive(Debug, Serialize)]
struct WmsWaveShipDocResult {
    outbound_doc_id: uuid::Uuid,
    status: String,
    message: Option<String>,
}

#[derive(Debug, Serialize)]
struct WmsWaveShipResponse {
    wave_id: uuid::Uuid,
    total: i64,
    success: i64,
    failed: i64,
    direct_ship: bool,
    items: Vec<WmsWaveShipDocResult>,
}

#[derive(Debug, Deserialize)]
struct WmsWaveRuleListQuery {
    warehouse_id: Option<String>,
    carrier_code: Option<String>,
    enabled: Option<bool>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct WmsWaveRuleCreateRequest {
    warehouse_id: String,
    carrier_code: Option<String>,
    max_orders: Option<i32>,
    max_qty: Option<i64>,
    cutoff_time: Option<String>,
    strategy: Option<String>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct WmsWaveRuleUpdateRequest {
    carrier_code: Option<String>,
    max_orders: Option<i32>,
    max_qty: Option<i64>,
    cutoff_time: Option<String>,
    strategy: Option<String>,
    enabled: Option<bool>,
}

#[derive(Debug, Serialize)]
struct WmsWaveRuleResponse {
    rule_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    carrier_code: Option<String>,
    max_orders: Option<i32>,
    max_qty: Option<i64>,
    cutoff_time: Option<String>,
    strategy: Option<String>,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct WmsWaveRuleListResponse {
    items: Vec<WmsWaveRuleResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct WmsPickTaskListQuery {
    warehouse_id: Option<String>,
    outbound_doc_id: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct WmsPickTaskListResponse {
    items: Vec<WmsPickTaskResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct WmsPickTaskConfirmRequest {
    picked_qty: i64,
    container_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WmsLaborTaskListQuery {
    warehouse_id: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct WmsLaborTaskAssignRequest {
    assignee: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct WmsLaborTaskResponse {
    task_id: String,
    warehouse_id: String,
    zone_code: String,
    task_type: String,
    assignee: Option<String>,
    planned_qty: i64,
    finished_qty: i64,
    status: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct WmsLaborTaskListResponse {
    items: Vec<WmsLaborTaskResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct WmsSlottingSuggestionListQuery {
    warehouse_id: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct WmsSlottingSuggestionResponse {
    suggestion_id: String,
    warehouse_id: String,
    sku_id: String,
    from_location_id: String,
    to_location_id: String,
    reason: String,
    priority: String,
    status: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct WmsSlottingSuggestionListResponse {
    items: Vec<WmsSlottingSuggestionResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct WmsReplenishmentTaskListQuery {
    warehouse_id: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct WmsReplenishmentTaskClaimRequest {
    assignee: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct WmsReplenishmentTaskResponse {
    task_id: String,
    warehouse_id: String,
    sku_id: String,
    from_location_id: String,
    to_location_id: String,
    qty: i64,
    assignee: Option<String>,
    status: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct WmsReplenishmentTaskListResponse {
    items: Vec<WmsReplenishmentTaskResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct WmsDockAppointmentListQuery {
    warehouse_id: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct WmsDockAppointmentResponse {
    appointment_id: String,
    warehouse_id: String,
    dock_no: String,
    carrier_name: String,
    vehicle_no: String,
    eta_at: String,
    status: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct WmsDockAppointmentListResponse {
    items: Vec<WmsDockAppointmentResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct WmsPackItemRequest {
    sku_id: String,
    packed_qty: i64,
}

#[derive(Debug, Deserialize)]
struct WmsPackBatchEntry {
    sku_id: String,
    batch_id: Option<String>,
    batch_no: Option<String>,
    qty: i64,
}

#[derive(Debug, Deserialize)]
struct WmsShipmentSerialEntry {
    sku_id: String,
    serial_nos: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct WmsPackRequest {
    package_no: String,
    weight_gram: Option<i32>,
    operator_id: Option<String>,
    items: Vec<WmsPackItemRequest>,
    serials: Option<Vec<WmsShipmentSerialEntry>>,
    batches: Option<Vec<WmsPackBatchEntry>>,
}

#[derive(Debug, Deserialize)]
struct WmsOutboundCheckRequest {
    package_no: String,
    passed: bool,
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WmsOutboundVerifyStartRequest {
    shipment_no: String,
    tracking_no: Option<String>,
    package_no: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WmsOutboundVerifyScanItemRequest {
    barcode: String,
    qty: i64,
}

#[derive(Debug, Serialize)]
struct WmsOutboundVerifyStateResponse {
    outbound_doc_id: uuid::Uuid,
    verify_status: String,
    scanned_total_qty: i64,
    missing_total_qty: i64,
    extra_total_qty: i64,
}

#[derive(Debug, Deserialize)]
struct WmsOutboundWeighRequest {
    package_no: String,
    weight_gram: i32,
}

#[derive(Debug, Deserialize)]
struct WmsShipRequest {
    shipped_at: Option<String>,
    serials: Option<Vec<WmsShipmentSerialEntry>>,
}

#[derive(Debug, Deserialize)]
struct WmsDirectShipRequest {
    shipped_at: Option<String>,
    serials: Option<Vec<WmsShipmentSerialEntry>>,
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WmsReserveTrackingRequest {
    carrier_code: Option<String>,
    waybill_no: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WmsLogisticsUpdateRequest {
    carrier_code: Option<String>,
    tracking_no: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WmsAddPackageRequest {
    doc_no: String,
    carrier_code: Option<String>,
    tracking_no: Option<String>,
    shipment_type: Option<String>,
    items: Vec<WmsOutboundItemRequest>,
}

#[derive(Debug, Deserialize)]
struct WmsShipmentCancelRequest {
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WmsOutboundOperatorRegisterRequest {
    picker_user_id: Option<String>,
    packer_user_id: Option<String>,
    note: Option<String>,
}

#[derive(Debug, Serialize)]
struct WmsOutboundOperatorRegisterResponse {
    outbound_doc_id: uuid::Uuid,
    picker_user_id: Option<uuid::Uuid>,
    packer_user_id: Option<uuid::Uuid>,
    note: Option<String>,
    registered_by: uuid::Uuid,
    registered_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct WarehouseAreaListQuery {
    warehouse_id: Option<String>,
    area_type: Option<String>,
    status: Option<String>,
    keyword: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct WarehouseAreaCreateRequest {
    warehouse_id: String,
    code: String,
    name: String,
    area_type: Option<String>,
    status: Option<String>,
    priority: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct WarehouseAreaUpdateRequest {
    name: Option<String>,
    area_type: Option<String>,
    status: Option<String>,
    priority: Option<i32>,
}

#[derive(Debug, Serialize)]
struct WarehouseAreaResponse {
    area_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    code: String,
    name: String,
    area_type: String,
    status: String,
    priority: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct WarehouseAreaListResponse {
    items: Vec<WarehouseAreaResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct WarehouseLocationListQuery {
    warehouse_id: Option<String>,
    area_id: Option<String>,
    keyword: Option<String>,
    is_picking: Option<bool>,
    is_disabled: Option<bool>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct WarehouseLocationCreateRequest {
    warehouse_id: String,
    area_id: Option<String>,
    code: String,
    name: Option<String>,
    barcode: Option<String>,
    volume: Option<f64>,
    max_weight: Option<f64>,
    is_picking: Option<bool>,
    is_disabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct WarehouseLocationBatchItem {
    area_id: Option<String>,
    code: String,
    name: Option<String>,
    barcode: Option<String>,
    volume: Option<f64>,
    max_weight: Option<f64>,
    is_picking: Option<bool>,
    is_disabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct WarehouseLocationBatchRequest {
    warehouse_id: String,
    area_id: Option<String>,
    locations: Vec<WarehouseLocationBatchItem>,
}

#[derive(Debug, Deserialize)]
struct WarehouseLocationUpdateRequest {
    area_id: Option<Option<String>>,
    name: Option<Option<String>>,
    barcode: Option<Option<String>>,
    volume: Option<Option<f64>>,
    max_weight: Option<Option<f64>>,
    is_picking: Option<bool>,
    is_disabled: Option<bool>,
}

#[derive(Debug, Serialize)]
struct WarehouseLocationResponse {
    location_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    area_id: Option<uuid::Uuid>,
    code: String,
    name: Option<String>,
    barcode: Option<String>,
    volume: Option<f64>,
    max_weight: Option<f64>,
    is_picking: bool,
    is_disabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct WarehouseLocationListResponse {
    items: Vec<WarehouseLocationResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct LocationBalanceListQuery {
    warehouse_id: Option<String>,
    location_id: Option<String>,
    sku_id: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct LocationBalanceResponse {
    warehouse_id: uuid::Uuid,
    location_id: uuid::Uuid,
    location_code: Option<String>,
    sku_id: uuid::Uuid,
    sku_code: Option<String>,
    on_hand: i64,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct LocationBalanceListResponse {
    items: Vec<LocationBalanceResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ContainerListQuery {
    keyword: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ContainerCreateRequest {
    code: String,
    container_type: String,
    capacity: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ContainerUpdateRequest {
    container_type: Option<String>,
    capacity: Option<Option<f64>>,
}

#[derive(Debug, Serialize)]
struct ContainerResponse {
    container_id: uuid::Uuid,
    code: String,
    container_type: String,
    capacity: Option<f64>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ContainerListResponse {
    items: Vec<ContainerResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct SenderAddressListQuery {
    keyword: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct SenderAddressCreateRequest {
    name: String,
    phone: String,
    province: String,
    city: String,
    district: String,
    address: String,
    is_default: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct SenderAddressUpdateRequest {
    name: Option<String>,
    phone: Option<String>,
    province: Option<String>,
    city: Option<String>,
    district: Option<String>,
    address: Option<String>,
    is_default: Option<bool>,
}

#[derive(Debug, Serialize)]
struct SenderAddressResponse {
    sender_address_id: uuid::Uuid,
    name: String,
    phone: String,
    province: String,
    city: String,
    district: String,
    address: String,
    is_default: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct SenderAddressListResponse {
    items: Vec<SenderAddressResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct WarehouseCreateRequest {
    name: String,
    code: String,
    status: Option<String>,
    is_default: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct WarehouseUpdateRequest {
    name: Option<String>,
    code: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WarehouseWorkflowConfigPayload {
    enable_wave: bool,
    require_inspection: bool,
    require_weighing: bool,
    require_pack: bool,
}

#[derive(Debug, Serialize)]
struct WarehouseWorkflowConfigResponse {
    warehouse_id: uuid::Uuid,
    enable_wave: bool,
    require_inspection: bool,
    require_weighing: bool,
    require_pack: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SystemWorkflowCatalogPayload {
    rules: Vec<SystemNumberRule>,
    workflows: Vec<SystemWorkflowDefinition>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SystemNumberRule {
    rule_id: String,
    doc_type: String,
    org_scope: String,
    prefix: String,
    date_format: String,
    sequence_length: i32,
    next_number: i32,
    status: String,
    updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SystemWorkflowDefinition {
    workflow_id: String,
    doc_type: String,
    name: String,
    version: String,
    status: String,
    steps: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    governance: Option<SystemWorkflowGovernance>,
    updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SystemWorkflowGovernance {
    engine: String,
    process_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    definition_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    decision_key: Option<String>,
    publish_status: String,
    approval_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    effective_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_deploy_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_deploy_result: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SystemCustomFieldsCatalogPayload {
    fields: Vec<SystemCustomField>,
    audits: Vec<SystemCustomFieldAudit>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SystemCustomField {
    field_id: String,
    entity: String,
    label: String,
    field_key: String,
    field_type: String,
    required: bool,
    compatible: bool,
    status: String,
    description: Option<String>,
    updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SystemCustomFieldAudit {
    record_id: String,
    field_id: String,
    action: String,
    operator: String,
    comment: Option<String>,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SystemAuditReportsCatalogPayload {
    reports: Vec<SystemAuditReport>,
    rules: Vec<SystemAuditRule>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SystemAuditReport {
    report_id: String,
    module: String,
    period: String,
    owner: String,
    anomalies: i32,
    status: String,
    generated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SystemAuditRule {
    rule_id: String,
    module: String,
    name: String,
    condition: String,
    enabled: bool,
    updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SystemIntegrationsCatalogPayload {
    configs: Vec<SystemIntegrationConfig>,
    changes: Vec<SystemIntegrationChange>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SystemIntegrationConfig {
    integration_id: String,
    name: String,
    category: String,
    provider: String,
    status: String,
    success_rate: f64,
    retry_policy: String,
    alert_channel: String,
    last_checked_at: Option<String>,
    updated_at: String,
    description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SystemIntegrationChange {
    record_id: String,
    integration_id: String,
    action: String,
    operator: String,
    comment: Option<String>,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct OperationLogListQuery {
    action: Option<String>,
    ref_type: Option<String>,
    ref_id: Option<String>,
    actor_user_id: Option<String>,
    created_from: Option<String>,
    created_to: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct FrontendRouteDiagnosticsQuery {
    path_keyword: Option<String>,
    status: Option<String>,
    event_type: Option<String>,
    window_hours: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct FrontendRouteDiagnosticsReportRequest {
    events: Vec<FrontendRouteDiagnosticEventCreate>,
}

#[derive(Debug, Serialize)]
struct FrontendRouteDiagnosticsReportResponse {
    accepted: usize,
}

#[derive(Debug, Serialize)]
struct FrontendRouteDiagnosticsResponse {
    summary: RepoFrontendRouteDiagnosticsSummary,
    paths: Vec<RepoFrontendRouteDiagnosticsPathSummary>,
    events: Vec<RepoFrontendRouteDiagnosticEvent>,
}

#[derive(Debug, Deserialize)]
struct PrintDeviceListQuery {
    fingerprint: Option<String>,
    status: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct PrintMappingUpsertRequest {
    doc_type: String,
    template_id: String,
    device_fingerprint: Option<String>,
    printer_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PrintDeviceCreateRequest {
    name: String,
    fingerprint: String,
    status: Option<String>,
    is_default: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct PrintDeviceRegisterRequest {
    device_fingerprint: String,
    name: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Serialize)]
struct PrintDeviceRegisterResponse {
    device_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    name: String,
    fingerprint: String,
    status: String,
    is_default: bool,
    device_token: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct PrintDeviceUpdateRequest {
    name: Option<String>,
    status: Option<String>,
    is_default: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct PrintTemplateListQuery {
    platform: Option<String>,
    shop_id: Option<String>,
    template_type: Option<String>,
    enabled: Option<bool>,
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct PrintTemplateCreateRequest {
    shop_id: String,
    platform: String,
    template_type: String,
    platform_template_id: Option<String>,
    version: Option<i32>,
    content_format: Option<String>,
    content: Option<Value>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct PrintTemplateUpdateRequest {
    platform_template_id: Option<String>,
    content_format: Option<String>,
    content: Option<Value>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct PrintJobListQuery {
    job_id: Option<String>,
    job_type: Option<String>,
    ref_type: Option<String>,
    ref_id: Option<String>,
    status: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct PrintJobCreateRequest {
    job_type: String,
    ref_type: String,
    ref_id: String,
    template_id: Option<String>,
    device_fingerprint: Option<String>,
    payload: Option<Value>,
    request_id: Option<String>,
    scan_code: Option<String>,
    scan_kind: Option<String>,
    #[serde(default)]
    scan_exception: Option<bool>,
    #[serde(default)]
    scan_repeat: Option<bool>,
    print_state: Option<String>,
    copies: Option<i64>,
    reason: Option<String>,
    actor_kind: Option<String>,
    #[serde(default)]
    document_kind: Option<String>,
    #[serde(default)]
    company_name: Option<String>,
    #[serde(default)]
    print_id: Option<String>,
    #[serde(default)]
    print_lines: Option<Vec<PrintLineRequest>>,
}

#[derive(Debug, Deserialize)]
struct PrintLineRequest {
    line_no: i32,
    item_code: String,
    qty_micros: i64,
    amount_cent: i64,
    tax_cent: i64,
    discount_cent: i64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct PrintShippingLabelPayload {
    order_id: String,
    platform: Option<String>,
    shop_id: String,
    shipment_id: Option<String>,
    tracking_no: Option<String>,
    carrier_code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PrintJobUpdateRequest {
    status: Option<String>,
    payload: Option<Value>,
    output_url: Option<String>,
    last_error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PrintAgentJobQuery {
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct PrintAgentJob {
    job_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    job_type: Option<String>,
    ref_type: Option<String>,
    ref_id: Option<uuid::Uuid>,
    template_id: uuid::Uuid,
    template_version: i32,
    content_format: String,
    template_content: String,
    payload: Value,
    attempts: i32,
}

#[derive(Debug, Serialize)]
struct PrintAgentJobListResponse {
    items: Vec<PrintAgentJob>,
}

#[derive(Debug, Deserialize)]
struct PrintAgentJobCompleteRequest {
    status: String,
    output_content: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReportRangeQuery {
    start_at: Option<String>,
    end_at: Option<String>,
    platform: Option<String>,
    shop_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FulfillmentReportQuery {
    start_at: Option<String>,
    end_at: Option<String>,
    overdue_hours: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct InventoryReportQuery {
    start_at: Option<String>,
    end_at: Option<String>,
    warehouse_id: Option<String>,
    warning_threshold: Option<i64>,
    warning_quantity: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ExportJobCreateRequest {
    #[serde(rename = "type")]
    job_type: String,
    params: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ExportJobListQuery {
    status: Option<String>,
    #[serde(rename = "type")]
    job_type: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ExportJobResponse {
    job_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    #[serde(rename = "type")]
    job_type: String,
    params: Value,
    status: String,
    output_url: Option<String>,
    progress: i32,
    attempts: i32,
    last_error: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ExportJobListResponse {
    items: Vec<ExportJobResponse>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct ListResponse<T> {
    data: Vec<T>,
    next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
struct InventoryRecordResponse {
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    sku_code: Option<String>,
    barcode: Option<String>,
    cost_cent: Option<i64>,
    on_hand: i64,
    reserved: i64,
    frozen: i64,
    in_transit: i64,
    available: i64,
    updated_at: DateTime<Utc>,
    quantities: Option<erp_repo::inventory_reads::InventoryReadQuantities>,
    quantity_state: &'static str,
}

#[derive(Debug, Serialize)]
struct InventorySummaryResponse {
    sku_id: uuid::Uuid,
    sku_code: Option<String>,
    barcode: Option<String>,
    on_hand: i64,
    reserved: i64,
    frozen: i64,
    in_transit: i64,
    available: i64,
    updated_at: DateTime<Utc>,
    quantities: erp_repo::inventory_reads::InventoryReadQuantities,
}

#[derive(Debug, Serialize)]
struct InventorySummaryListResponse {
    items: Vec<InventorySummaryResponse>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct InventoryCostResponse {
    sku_id: uuid::Uuid,
    sku_code: Option<String>,
    barcode: Option<String>,
    on_hand: i64,
    reserved: i64,
    frozen: i64,
    in_transit: i64,
    available: i64,
    unit_cost_cent: Option<i64>,
    total_cost_cent: Option<i64>,
    estimated_value: Option<String>,
    valuation_source: &'static str,
    updated_at: DateTime<Utc>,
    quantities: erp_repo::inventory_reads::InventoryReadQuantities,
}

#[derive(Debug, Serialize)]
struct InventoryCostListResponse {
    items: Vec<InventoryCostResponse>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct InventoryMutationResponse {
    record: InventoryRecordResponse,
}

#[derive(Debug, Serialize)]
struct InventoryLedgerResponse {
    ledger_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    sku_code: Option<String>,
    delta_on_hand: i64,
    delta_reserved: i64,
    delta_in_transit: i64,
    delta_frozen: i64,
    reason_type: String,
    ref_type: Option<String>,
    ref_id: Option<String>,
    occurred_at: DateTime<Utc>,
    quantities: erp_repo::inventory_reads::InventoryReadDelta,
}

#[derive(Debug, Serialize)]
struct InventoryLedgerListResponse {
    items: Vec<InventoryLedgerResponse>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct InventoryAtpAllocationResponse {
    batch_id: uuid::Uuid,
    batch_no: String,
    expire_at: Option<DateTime<Utc>>,
    allocated_qty: i64,
    quantity: String,
}

#[derive(Debug, Serialize)]
struct InventoryAtpResponse {
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    on_hand: i64,
    reserved: i64,
    frozen: i64,
    available: i64,
    request_qty: i64,
    atp_qty: i64,
    shortage_qty: i64,
    quantities: erp_repo::atp_inventory::AtpQuantities,
    allocations: Vec<InventoryAtpAllocationResponse>,
}

#[derive(Debug, Serialize)]
struct ProductPackagingSpecResponse {
    spec_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    level_code: String,
    unit_name: String,
    units_per_level: i64,
    length_mm: Option<i32>,
    width_mm: Option<i32>,
    height_mm: Option<i32>,
    weight_gram: Option<i32>,
    is_default: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ProductPackagingSpecListResponse {
    items: Vec<ProductPackagingSpecResponse>,
}

#[derive(Debug, Serialize)]
struct InventoryReconciliationResponse {
    reconciliation_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    expected_on_hand: i64,
    actual_on_hand: i64,
    diff_qty: i64,
    status: String,
    source_ref: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct StockCountListQuery {
    warehouse_id: Option<String>,
    status: Option<String>,
    count_type: Option<String>,
    scope_type: Option<String>,
    keyword: Option<String>,
    created_from: Option<String>,
    created_to: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct StockCountCreateRequest {
    warehouse_id: String,
    strategy: Option<String>,
    dimension: Option<String>,
    recent_days: Option<i64>,
    brand: Option<String>,
    category: Option<String>,
    scope_type: Option<String>,
    scope_ids: Option<Vec<String>>,
    count_mode: Option<String>,
    count_type: Option<String>,
    freeze_stock: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct StockCountImportItemRequest {
    sku_id: String,
    location_id: Option<String>,
    counted_qty: i64,
    diff_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StockCountImportRequest {
    warehouse_id: String,
    dimension: Option<String>,
    scope_type: Option<String>,
    count_mode: Option<String>,
    count_type: Option<String>,
    freeze_stock: Option<bool>,
    #[serde(default)]
    items: Vec<StockCountImportItemRequest>,
}

#[derive(Debug, Deserialize)]
struct StockCountItemUpdateRequest {
    count_item_id: String,
    counted_qty: i64,
    diff_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StockCountItemsUpdateRequest {
    #[serde(default)]
    items: Vec<StockCountItemUpdateRequest>,
}

#[derive(Debug, Serialize)]
struct StockCountSummaryResponse {
    count_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    status: String,
    strategy: String,
    dimension: String,
    filters: StockCountFiltersResponse,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct StockCountFiltersResponse {
    recent_days: Option<i64>,
    brand: Option<String>,
    category: Option<String>,
    scope_type: Option<String>,
    scope_ids: Option<Vec<String>>,
    count_mode: Option<String>,
    count_type: Option<String>,
    freeze_stock: Option<bool>,
    created_by: Option<String>,
}

#[derive(Debug, Serialize)]
struct StockCountItemResponse {
    count_item_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    location_id: Option<uuid::Uuid>,
    system_qty: i64,
    counted_qty: i64,
    diff_qty: i64,
    diff_reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct StockCountDetailResponse {
    count: StockCountSummaryResponse,
    items: Vec<StockCountItemResponse>,
    audit_required: bool,
}

#[derive(Debug, Serialize)]
struct StockCountListResponse {
    items: Vec<StockCountSummaryResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct StockCountStrategyRequest {
    name: String,
    enabled: Option<bool>,
    strategy_type: String,
    config_json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct StockCountStrategyUpdateRequest {
    name: Option<String>,
    enabled: Option<bool>,
    strategy_type: Option<String>,
    config_json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct StockCountStrategyListQuery {
    enabled: Option<bool>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct StockCountStrategyListResponse {
    items: Vec<StockCountStrategy>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct StockCountSuggestionRefreshRequest {
    warehouse_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StockCountSuggestionListQuery {
    warehouse_id: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct StockCountSuggestionListResponse {
    items: Vec<StockCountSuggestion>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct StockTransferListQuery {
    warehouse_id: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct StockTransferItemRequest {
    sku_id: String,
    qty: i64,
}

#[derive(Debug, Deserialize)]
struct StockTransferCreateRequest {
    from_warehouse_id: String,
    to_warehouse_id: String,
    #[serde(default)]
    items: Vec<StockTransferItemRequest>,
}

#[derive(Debug, Serialize)]
struct StockTransferSummaryResponse {
    transfer_id: uuid::Uuid,
    from_warehouse_id: uuid::Uuid,
    to_warehouse_id: uuid::Uuid,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct StockTransferItemResponse {
    transfer_item_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    qty: i64,
}

#[derive(Debug, Serialize)]
struct StockTransferDetailResponse {
    transfer: StockTransferSummaryResponse,
    items: Vec<StockTransferItemResponse>,
}

#[derive(Debug, Serialize)]
struct StockTransferListResponse {
    items: Vec<StockTransferSummaryResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct InventoryFreezeListQuery {
    warehouse_id: Option<String>,
    sku_id: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct InventoryFreezeCreateRequest {
    warehouse_id: String,
    sku_id: String,
    qty: i64,
    reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct InventoryFreezeResponse {
    freeze_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    qty: i64,
    reason: Option<String>,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct InventoryFreezeListResponse {
    items: Vec<InventoryFreezeResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct StockAdjustListQuery {
    warehouse_id: Option<String>,
    status: Option<String>,
    reason: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct StockAdjustItemRequest {
    sku_id: String,
    qty: i64,
}

#[derive(Debug, Deserialize)]
struct StockAdjustCreateRequest {
    warehouse_id: String,
    reason: String,
    #[serde(default)]
    items: Vec<StockAdjustItemRequest>,
    #[serde(default)]
    request_id: Option<String>,
    #[serde(default)]
    location_id: Option<String>,
    #[serde(default)]
    sku_id: Option<String>,
    #[serde(default)]
    batch_code: Option<String>,
    #[serde(default)]
    cutoff_at: Option<String>,
    #[serde(default)]
    counted_qty: Option<i64>,
}

#[derive(Debug, Serialize)]
struct StockAdjustSummaryResponse {
    adjust_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    reason: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct StockAdjustItemResponse {
    adjust_item_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    qty: i64,
}

#[derive(Debug, Serialize)]
struct StockAdjustDetailResponse {
    doc: StockAdjustSummaryResponse,
    items: Vec<StockAdjustItemResponse>,
}

#[derive(Debug, Serialize)]
struct StockAdjustListResponse {
    items: Vec<StockAdjustSummaryResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct StockMoveListQuery {
    warehouse_id: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct StockMoveItemRequest {
    sku_id: String,
    qty: i64,
}

#[derive(Debug, Deserialize)]
struct StockMoveCreateRequest {
    warehouse_id: String,
    from_location_id: Option<String>,
    to_location_id: Option<String>,
    #[serde(default)]
    items: Vec<StockMoveItemRequest>,
}

#[derive(Debug, Serialize)]
struct StockMoveSummaryResponse {
    move_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    from_location_id: Option<uuid::Uuid>,
    to_location_id: Option<uuid::Uuid>,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct StockMoveItemResponse {
    move_item_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    qty: i64,
}

#[derive(Debug, Serialize)]
struct StockMoveDetailResponse {
    doc: StockMoveSummaryResponse,
    items: Vec<StockMoveItemResponse>,
}

#[derive(Debug, Serialize)]
struct StockMoveListResponse {
    items: Vec<StockMoveSummaryResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct StockSyncJobListQuery {
    shop_id: Option<String>,
    sku_id: Option<String>,
    warehouse_id: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct StockSyncJobResponse {
    job_id: uuid::Uuid,
    shop_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    target_qty: i64,
    status: String,
    attempts: i32,
    last_error: Option<String>,
    success_count: i32,
    failure_count: i32,
    last_duration_ms: i32,
    last_success_at: Option<DateTime<Utc>>,
    next_retry_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct StockSyncJobListResponse {
    items: Vec<StockSyncJobResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct StockSyncJobEventListQuery {
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct StockSyncJobEventResponse {
    event_id: uuid::Uuid,
    status: String,
    attempt: i32,
    payload: serde_json::Value,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct StockSyncJobEventListResponse {
    items: Vec<StockSyncJobEventResponse>,
}

#[derive(Debug, Serialize)]
struct InboundDocSummaryResponse {
    doc_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    doc_type: String,
    doc_no: String,
    status: String,
    inbound_status: String,
    receipt_status: String,
    inbound_qc_status: String,
    putaway_status: String,
    ref_type: Option<String>,
    ref_id: Option<String>,
    supplier_id: Option<uuid::Uuid>,
    supplier_name: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct InboundDocListResponse {
    items: Vec<InboundDocSummaryResponse>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct InboundDocResponse {
    doc_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    doc_type: String,
    doc_no: String,
    status: String,
    inbound_status: String,
    receipt_status: String,
    inbound_qc_status: String,
    putaway_status: String,
    ref_type: Option<String>,
    ref_id: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct InboundDocItemResponse {
    sku_id: uuid::Uuid,
    sku_code: Option<String>,
    barcode: Option<String>,
    planned_qty: i64,
    received_qty: i64,
    putaway_qty: i64,
}

#[derive(Debug, Serialize)]
struct InboundDocDetailResponse {
    doc: InboundDocResponse,
    items: Vec<InboundDocItemResponse>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    code: String,
    message: String,
    details: Option<Value>,
    request_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum StatementImportType {
    Platform,
    Shipping,
    Payment,
}

impl StatementImportType {
    fn as_str(&self) -> &'static str {
        match self {
            StatementImportType::Platform => "PLATFORM",
            StatementImportType::Shipping => "SHIPPING",
            StatementImportType::Payment => "PAYMENT",
        }
    }
}

#[derive(Debug, Deserialize)]
struct PlatformStatementLineRequest {
    platform_order_id: String,
    gross_cent: Option<i64>,
    fee_cent: Option<i64>,
    refund_cent: Option<i64>,
    net_cent: Option<i64>,
    occurred_at: Option<DateTime<Utc>>,
    raw_json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ShippingStatementLineRequest {
    tracking_no: String,
    fee_cent: Option<i64>,
    weight: Option<f64>,
    occurred_at: Option<DateTime<Utc>>,
    raw_json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct PaymentStatementLineRequest {
    platform_order_id: String,
    paid_cent: Option<i64>,
    fee_cent: Option<i64>,
    net_cent: Option<i64>,
    occurred_at: Option<DateTime<Utc>>,
    raw_json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct StatementImportRequest {
    statement_type: StatementImportType,
    platform: Option<String>,
    shop_id: Option<String>,
    channel: Option<String>,
    period_start: Option<NaiveDate>,
    period_end: Option<NaiveDate>,
    lines: Option<Vec<PlatformStatementLineRequest>>,
    shipping_lines: Option<Vec<ShippingStatementLineRequest>>,
    payment_lines: Option<Vec<PaymentStatementLineRequest>>,
}

#[derive(Debug, Deserialize)]
struct StatementImportCsvQuery {
    statement_type: Option<StatementImportType>,
    platform: Option<String>,
    shop_id: Option<String>,
    channel: Option<String>,
    period_start: Option<NaiveDate>,
    period_end: Option<NaiveDate>,
}

#[derive(Debug, Serialize)]
struct StatementImportResponse {
    statement_type: String,
    statement_id: Option<uuid::Uuid>,
    inserted_lines: i64,
}

#[derive(Debug, Deserialize)]
struct CsvPlatformStatementLine {
    platform_order_id: String,
    gross_cent: Option<i64>,
    fee_cent: Option<i64>,
    refund_cent: Option<i64>,
    net_cent: Option<i64>,
    occurred_at: Option<String>,
    raw_json: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CsvShippingStatementLine {
    tracking_no: String,
    fee_cent: Option<i64>,
    weight: Option<f64>,
    occurred_at: Option<String>,
    raw_json: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CsvPaymentStatementLine {
    platform_order_id: String,
    paid_cent: Option<i64>,
    fee_cent: Option<i64>,
    net_cent: Option<i64>,
    occurred_at: Option<String>,
    raw_json: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReconcileRunRequest {
    job_type: String,
    statement_id: Option<String>,
    occurred_from: Option<DateTime<Utc>>,
    occurred_to: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct ReconcileRunResponse {
    job_id: uuid::Uuid,
    status: String,
    diff_count: i64,
}

#[derive(Debug, Deserialize)]
struct ReconcileJobListQuery {
    job_type: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ReconcileJobResponse {
    job_id: uuid::Uuid,
    job_type: String,
    status: String,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ReconcileJobListResponse {
    items: Vec<ReconcileJobResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ReconcileJobItemListQuery {
    job_id: String,
    ref_type: Option<String>,
    diff_only: Option<bool>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ReconcileJobItemResponse {
    job_id: uuid::Uuid,
    ref_type: String,
    ref_id: String,
    expected_cent: i64,
    actual_cent: i64,
    diff_cent: i64,
    diff_reason: Option<String>,
    raw_json: Value,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ReconcileJobItemListResponse {
    items: Vec<ReconcileJobItemResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct PaymentReconcileRunRequest {
    statement_id: String,
}

#[derive(Debug, Serialize)]
struct PaymentReconcileRunResponse {
    job_id: uuid::Uuid,
    status: String,
    diff_count: i64,
}

#[derive(Debug, Deserialize)]
struct PaymentReconcileJobListQuery {
    status: Option<String>,
    statement_id: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct PaymentReconcileJobResponse {
    job_id: uuid::Uuid,
    statement_id: Option<uuid::Uuid>,
    status: String,
    diff_count: i64,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct PaymentReconcileJobListResponse {
    items: Vec<PaymentReconcileJobResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct PaymentReconcileItemListQuery {
    job_id: String,
    diff_only: Option<bool>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct PaymentReconcileItemResponse {
    job_id: uuid::Uuid,
    platform_order_id: String,
    order_id: Option<uuid::Uuid>,
    expected_cent: i64,
    actual_cent: i64,
    diff_cent: i64,
    diff_reason: Option<String>,
    raw_json: Value,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct PaymentReconcileItemListResponse {
    items: Vec<PaymentReconcileItemResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct WriteoffRecordListQuery {
    ref_type: Option<String>,
    ref_id: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct WriteoffRecordResponse {
    writeoff_id: uuid::Uuid,
    ref_type: String,
    ref_id: String,
    amount_cent: i64,
    reason: Option<String>,
    created_by: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct WriteoffRecordListResponse {
    items: Vec<WriteoffRecordResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct InvoiceApplyRequest {
    order_id: String,
    invoice_title: String,
    tax_no: Option<String>,
}

#[derive(Debug, Serialize)]
struct InvoiceResponse {
    invoice_id: uuid::Uuid,
    order_id: String,
    invoice_title: String,
    tax_no: Option<String>,
    status: String,
    issued_at: Option<DateTime<Utc>>,
    result_json: Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct InvoiceListQuery {
    status: Option<String>,
    order_id: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct InvoiceListResponse {
    items: Vec<InvoiceResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ApDocListQuery {
    status: Option<String>,
    supplier_id: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ApDocResponse {
    ap_id: uuid::Uuid,
    supplier_id: uuid::Uuid,
    supplier_name: Option<String>,
    ref_type: String,
    ref_id: String,
    amount_cent: i64,
    status: String,
    settlement_status: String,
    paid_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ApDocListResponse {
    items: Vec<ApDocResponse>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ProfitListQuery {
    shop_id: Option<String>,
    sku_id: Option<String>,
    created_from: Option<DateTime<Utc>>,
    created_to: Option<DateTime<Utc>>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ProfitSnapshotResponse {
    order_id: String,
    shop_id: String,
    platform_order_id: String,
    order_created_at: DateTime<Utc>,
    revenue_cent: i64,
    cost_cent: i64,
    platform_fee_cent: i64,
    shipping_fee_cent: i64,
    gross_profit_cent: i64,
    computed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ProfitSnapshotListResponse {
    items: Vec<ProfitSnapshotResponse>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct ProfitSourceRevenueItem {
    line_id: String,
    sku_id: Option<String>,
    qty: i64,
    amount_cent: Option<i64>,
    pay_amount_cent: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ProfitSourceCogsItem {
    sku_id: uuid::Uuid,
    qty: i64,
    unit_cost_cent: i64,
    total_cost_cent: i64,
    ref_type: String,
    ref_id: String,
    shipment_id: Option<uuid::Uuid>,
    outbound_doc_id: Option<uuid::Uuid>,
    order_item_id: Option<uuid::Uuid>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ProfitSourcePlatformFeeLine {
    statement_id: uuid::Uuid,
    platform_order_id: String,
    fee_cent: i64,
    occurred_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct ProfitSourceShippingFeeLine {
    line_id: uuid::Uuid,
    tracking_no: String,
    fee_cent: i64,
    occurred_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct ProfitSnapshotDetailResponse {
    snapshot: ProfitSnapshotResponse,
    revenue_items: Vec<ProfitSourceRevenueItem>,
    cogs_items: Vec<ProfitSourceCogsItem>,
    platform_fee_lines: Vec<ProfitSourcePlatformFeeLine>,
    shipping_fee_lines: Vec<ProfitSourceShippingFeeLine>,
}

#[derive(Debug, Deserialize)]
struct OmsOrderListQuery {
    platform: Option<String>,
    shop_id: Option<String>,
    oms_status: Option<String>,
    order_status: Option<String>,
    review_status: Option<String>,
    fulfillment_status: Option<String>,
    exception_status: Option<String>,
    payment_status: Option<String>,
    exception_flags: Option<String>,
    tags: Option<String>,
    tagged: Option<bool>,
    created_from: Option<String>,
    created_to: Option<String>,
    updated_from: Option<String>,
    updated_to: Option<String>,
    ship_warehouse_id: Option<String>,
    platform_order_id: Option<String>,
    item_keyword: Option<String>,
    total_amount_cent: Option<String>,
    total_amount_min_cent: Option<String>,
    total_amount_max_cent: Option<String>,
    goods_qty: Option<String>,
    customer_account: Option<String>,
    receiver_name: Option<String>,
    receiver_phone: Option<String>,
    receiver_address: Option<String>,
    remark: Option<String>,
    order_source: Option<String>,
    logistics_company: Option<String>,
    estimated_postage_cent: Option<String>,
    estimated_profit_cent: Option<String>,
    estimated_profit_rate: Option<String>,
    keyword: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct OrderReviewRequest {
    action: String,
    reason: Option<String>,
    version: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct OrderForceReviewRequest {
    password: String,
    reason: Option<String>,
    version: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct OrderMergeRequest {
    order_ids: Vec<String>,
    target_order_id: Option<String>,
    force: Option<bool>,
}

#[derive(Debug, Serialize)]
struct OrderMergeResponse {
    target_order_id: String,
    merged_order_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct OrderSplitRequest {
    split_count: Option<i32>,
    children: Option<Vec<OrderSplitChildRequest>>,
}

#[derive(Debug, Deserialize)]
struct OrderSplitChildRequest {
    items: Vec<OrderSplitItemRequest>,
}

#[derive(Debug, Deserialize)]
struct OrderSplitItemRequest {
    line_id: String,
    qty: i32,
}

#[derive(Debug, Serialize)]
struct OrderSplitResponse {
    source_order_id: String,
    split_count: i32,
    created_at: DateTime<Utc>,
    child_orders: Vec<OrderSplitChildResponse>,
}

#[derive(Debug, Serialize)]
struct OrderSplitChildResponse {
    order_id: String,
    items: Vec<OrderSplitItemResponse>,
}

#[derive(Debug, Serialize)]
struct OrderSplitItemResponse {
    line_id: String,
    sku_id: Option<String>,
    title: Option<String>,
    qty: i32,
    price_cent: i64,
}

#[derive(Debug, Deserialize)]
struct OrderEditItemRequest {
    line_id: Option<String>,
    sku_id: Option<String>,
    title: Option<String>,
    qty: i32,
    price_cent: i64,
}

#[derive(Debug, Deserialize)]
struct OrderItemEditRequest {
    line_id: String,
    qty: Option<i32>,
    price_cent: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct OrderEditRequest {
    expected_version: Option<i32>,
    receiver_name: Option<String>,
    receiver_phone: Option<String>,
    receiver_tel: Option<String>,
    receiver_address: Option<String>,
    logistics_company: Option<String>,
    tracking_no: Option<String>,
    order_type: Option<String>,
    sales_channel: Option<String>,
    order_source: Option<String>,
    salesperson: Option<String>,
    ship_warehouse_id: Option<String>,
    shipment_no: Option<String>,
    estimated_weight_gram: Option<i32>,
    package_weight_gram: Option<i32>,
    customer_account: Option<String>,
    paid_amount_cent: Option<i64>,
    subsidy_amount_cent: Option<i64>,
    receivable_amount_cent: Option<i64>,
    other_fee_cent: Option<i64>,
    estimated_postage_cent: Option<i64>,
    invoice_type: Option<String>,
    invoice_title: Option<String>,
    payment_status: Option<String>,
    buyer_message: Option<String>,
    seller_remark: Option<String>,
    freeze_reason: Option<String>,
    cancel_reason: Option<String>,
    exception_reason: Option<String>,
    extra_remark: Option<String>,
    items: Option<Vec<OrderEditItemRequest>>,
    item_edits: Option<Vec<OrderItemEditRequest>>,
}

#[derive(Debug, Deserialize)]
struct OrderCancelRequest {
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OrderExceptionHandleRequest {
    action: String,
    reason: Option<String>,
    flags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct OrderExceptionPoolRequest {
    order_id: String,
    action: String,
    reason: Option<String>,
    flags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Default)]
struct OrderExceptionListQuery {
    status: Option<String>,
    keyword: Option<String>,
    assignee_id: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
struct OrderExceptionActionRequest {
    operator_id: Option<String>,
    operator_name: Option<String>,
    assignee_id: Option<String>,
    assignee_name: Option<String>,
    reason: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct OrderExceptionActionRecordResponse {
    action: String,
    message: String,
    operator_id: Option<String>,
    operator_name: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Clone)]
struct OrderExceptionSummaryResponse {
    order_id: String,
    order_no: String,
    platform: String,
    shop_id: String,
    receiver_name: Option<String>,
    total_amount_cent: i64,
    audit_status: String,
    exception_status: String,
    fulfillment_status: String,
    reserve_status: Option<String>,
    target_warehouse_id: Option<String>,
    blocking_reason: String,
    assignee_id: Option<String>,
    assignee_name: Option<String>,
    latest_action: Option<String>,
    latest_action_at: Option<DateTime<Utc>>,
    sla_deadline_at: Option<DateTime<Utc>>,
    sla_status: String,
    sla_remaining_minutes: i64,
}

#[derive(Debug, Serialize)]
struct OrderExceptionListResponse {
    items: Vec<OrderExceptionSummaryResponse>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct OrderExceptionDetailResponse {
    order_id: String,
    order_no: String,
    platform: String,
    shop_id: String,
    receiver_name: Option<String>,
    total_amount_cent: i64,
    audit_status: String,
    exception_status: String,
    fulfillment_status: String,
    reserve_status: Option<String>,
    target_warehouse_id: Option<String>,
    blocking_reason: String,
    assignee_id: Option<String>,
    assignee_name: Option<String>,
    latest_action: Option<String>,
    latest_action_at: Option<DateTime<Utc>>,
    sla_deadline_at: Option<DateTime<Utc>>,
    sla_status: String,
    sla_remaining_minutes: i64,
    blocking_reasons: Vec<String>,
    action_logs: Vec<OrderExceptionActionRecordResponse>,
    order: OmsOrderDetailResponse,
}

#[derive(Debug, Deserialize, Default)]
struct OrderSettlementListQuery {
    platform: Option<String>,
    shop_id: Option<String>,
    keyword: Option<String>,
    status: Option<String>,
    created_from: Option<String>,
    created_to: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct OrderSettlementRuleUpdateRequest {
    stack_mode: String,
    member_discount_rate: i64,
    promo_discount_rate: i64,
    max_discount_rate: i64,
    expected_version: Option<i32>,
}

#[derive(Debug, Deserialize, Default)]
struct OrderSettlementActionRequest {
    operator_id: Option<String>,
    operator_name: Option<String>,
    export_job_id: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct OrderSettlementRuleResponse {
    stack_mode: String,
    member_discount_rate: i64,
    promo_discount_rate: i64,
    max_discount_rate: i64,
    version: i32,
    updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Clone)]
struct OrderSettlementPromotionResponse {
    name: String,
    promotion_type: String,
    rate: i64,
    amount_cent: i64,
}

#[derive(Debug, Serialize, Clone)]
struct OrderSettlementActionRecordResponse {
    action: String,
    message: String,
    operator_id: Option<String>,
    operator_name: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Clone)]
struct OrderSettlementSummaryResponse {
    order_id: String,
    order_no: String,
    platform: String,
    shop_id: String,
    receiver_name: Option<String>,
    created_at: DateTime<Utc>,
    total_amount_cent: i64,
    discount_amount_cent: i64,
    receivable_amount_cent: i64,
    settlement_status: String,
    locked_at: Option<DateTime<Utc>>,
    export_job_id: Option<String>,
    latest_action: Option<String>,
    latest_action_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct OrderSettlementListSummaryResponse {
    total_orders: i64,
    open_orders: i64,
    locked_orders: i64,
    exported_orders: i64,
    total_amount_cent: i64,
    receivable_amount_cent: i64,
}

#[derive(Debug, Serialize)]
struct OrderSettlementListResponse {
    items: Vec<OrderSettlementSummaryResponse>,
    total: i64,
    summary: OrderSettlementListSummaryResponse,
}

#[derive(Debug, Serialize)]
struct OrderSettlementDetailResponse {
    order_id: String,
    order_no: String,
    platform: String,
    shop_id: String,
    receiver_name: Option<String>,
    created_at: DateTime<Utc>,
    total_amount_cent: i64,
    discount_amount_cent: i64,
    receivable_amount_cent: i64,
    settlement_status: String,
    locked_at: Option<DateTime<Utc>>,
    export_job_id: Option<String>,
    latest_action: Option<String>,
    latest_action_at: Option<DateTime<Utc>>,
    promotions: Vec<OrderSettlementPromotionResponse>,
    action_logs: Vec<OrderSettlementActionRecordResponse>,
}

#[derive(Debug, Deserialize, Default)]
struct OrderReconciliationListQuery {
    platform: Option<String>,
    shop_id: Option<String>,
    keyword: Option<String>,
    status: Option<String>,
    created_from: Option<String>,
    created_to: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
struct OrderReconciliationActionRequest {
    operator_id: Option<String>,
    operator_name: Option<String>,
    note: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct OrderReconciliationSummaryResponse {
    batch_id: String,
    platform: String,
    shop_id: String,
    period_start: NaiveDate,
    period_end: NaiveDate,
    order_count: i64,
    expected_cent: i64,
    actual_cent: i64,
    diff_cent: i64,
    status: String,
    reasons: Vec<String>,
    settled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct OrderReconciliationListSummaryResponse {
    total_batches: i64,
    pending_batches: i64,
    done_batches: i64,
    expected_total_cent: i64,
    diff_total_cent: i64,
}

#[derive(Debug, Serialize)]
struct OrderReconciliationListResponse {
    items: Vec<OrderReconciliationSummaryResponse>,
    total: i64,
    summary: OrderReconciliationListSummaryResponse,
}

#[derive(Debug, Serialize, Clone)]
struct OrderReconciliationOrderResponse {
    order_id: String,
    order_no: String,
    receiver_name: Option<String>,
    total_amount_cent: i64,
    receivable_amount_cent: i64,
    settlement_status: String,
    audit_status: String,
    exception_status: String,
    blocking_reason: String,
}

#[derive(Debug, Serialize)]
struct OrderReconciliationDetailResponse {
    batch_id: String,
    platform: String,
    shop_id: String,
    period_start: NaiveDate,
    period_end: NaiveDate,
    order_count: i64,
    expected_cent: i64,
    actual_cent: i64,
    diff_cent: i64,
    status: String,
    reasons: Vec<String>,
    settled_at: Option<DateTime<Utc>>,
    note: Option<String>,
    orders: Vec<OrderReconciliationOrderResponse>,
}

#[derive(Debug, Serialize)]
struct OrderExceptionReasonStatResponse {
    reason_code: String,
    label: Option<String>,
    total: i64,
    open: i64,
    processed: i64,
}

#[derive(Debug, Serialize)]
struct OrderExceptionStatsResponse {
    total: i64,
    open: i64,
    processed: i64,
    reasons: Vec<OrderExceptionReasonStatResponse>,
}

#[derive(Debug, Deserialize)]
struct OrderReturnToDownloadRequest {
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OrderRollbackReviewRequest {
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OrderTagsRequest {
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct OrderAutoTagApplyRequest {
    order_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
struct OrderAutoTagApplyResponse {
    tagged_orders: Vec<String>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct OrderReviewHitsExportRequest {
    order_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct OrderNoteRequest {
    note: String,
}

#[derive(Debug, Deserialize)]
struct OrderReceiverUpdateRequest {
    receiver_name: String,
    receiver_phone: String,
    receiver_address: String,
}

#[derive(Debug, Deserialize)]
struct TagDefinitionListQuery {
    keyword: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct OrderTagDefinitionListResponse {
    items: Vec<OrderTagDefinition>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct SkuTagDefinitionListResponse {
    items: Vec<SkuTagDefinition>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct TagDefinitionCreateRequest {
    name: String,
    color: Option<String>,
    priority: Option<i32>,
    status: Option<String>,
    auto_apply: Option<bool>,
    conditions_json: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct TagDefinitionUpdateRequest {
    name: Option<String>,
    color: Option<String>,
    priority: Option<i32>,
    status: Option<String>,
    auto_apply: Option<bool>,
    conditions_json: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct CustomerCategoryListQuery {
    keyword: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct CustomerCategoryListResponse {
    items: Vec<CustomerCategory>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct CustomerCategoryCreateRequest {
    name: String,
    description: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CustomerCategoryUpdateRequest {
    name: Option<String>,
    description: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MemberLevelListQuery {
    keyword: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct MemberLevelListResponse {
    items: Vec<MemberLevel>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct MemberLevelCreateRequest {
    name: String,
    level_rank: Option<i32>,
    discount_rate: Option<f64>,
    rules: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MemberLevelUpdateRequest {
    name: Option<String>,
    level_rank: Option<i32>,
    discount_rate: Option<f64>,
    rules: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CustomerTagListQuery {
    keyword: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct CustomerTagListResponse {
    items: Vec<CustomerTag>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct CustomerTagCreateRequest {
    name: String,
    color: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CustomerTagUpdateRequest {
    name: Option<String>,
    color: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CustomerListQuery {
    keyword: Option<String>,
    status: Option<String>,
    category_id: Option<String>,
    member_level_id: Option<String>,
    tag_id: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct CustomerListResponse {
    items: Vec<Customer>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct CustomerDetailResponse {
    customer: Customer,
    tags: Vec<CustomerTag>,
}

#[derive(Debug, Deserialize)]
struct CustomerCreateRequest {
    name: String,
    phone: Option<String>,
    address: Option<String>,
    note: Option<String>,
    status: Option<String>,
    category_id: Option<String>,
    member_level_id: Option<String>,
    request_id: Option<String>,
    customer_no: Option<String>,
    supplier_no: Option<String>,
    term_days: Option<i32>,
    tax_no: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CustomerUpdateRequest {
    name: Option<String>,
    phone: Option<String>,
    address: Option<String>,
    note: Option<String>,
    status: Option<String>,
    category_id: Option<String>,
    member_level_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CustomerTagReplaceRequest {
    tag_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CrmCustomerSegmentListQuery {
    keyword: Option<String>,
    segment_type: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct CrmCustomerSegment {
    segment_id: uuid::Uuid,
    name: String,
    segment_type: String,
    rule_summary: Option<String>,
    customer_count: i64,
    status: Option<String>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct CrmCustomerSegmentListResponse {
    items: Vec<CrmCustomerSegment>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct CrmCustomerSegmentCreateRequest {
    name: String,
    segment_type: String,
    rule_summary: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CrmCustomerSegmentUpdateRequest {
    name: Option<String>,
    segment_type: Option<String>,
    rule_summary: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CrmLifecycleTaskListQuery {
    keyword: Option<String>,
    stage: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct CrmLifecycleTask {
    task_id: uuid::Uuid,
    stage: String,
    strategy: String,
    trigger_rule: Option<String>,
    owner: Option<String>,
    status: Option<String>,
    last_triggered_at: Option<DateTime<Utc>>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct CrmLifecycleTaskListResponse {
    items: Vec<CrmLifecycleTask>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct CrmLifecycleTaskCreateRequest {
    stage: String,
    strategy: String,
    trigger_rule: Option<String>,
    owner: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CrmLifecycleTaskUpdateRequest {
    stage: Option<String>,
    strategy: Option<String>,
    trigger_rule: Option<String>,
    owner: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CrmServiceTicketListQuery {
    keyword: Option<String>,
    status: Option<String>,
    priority: Option<String>,
    channel: Option<String>,
    category: Option<String>,
    handler: Option<String>,
    updated_from: Option<String>,
    updated_to: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct CrmServiceTicket {
    ticket_id: uuid::Uuid,
    customer_id: Option<String>,
    customer_name: String,
    contact_phone: Option<String>,
    channel: Option<String>,
    category: Option<String>,
    priority: Option<String>,
    status: Option<String>,
    related_order_id: Option<String>,
    satisfaction_score: Option<i32>,
    handler: Option<String>,
    description: Option<String>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct CrmServiceTicketListResponse {
    items: Vec<CrmServiceTicket>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct CrmServiceTicketCreateRequest {
    customer_id: Option<String>,
    customer_name: String,
    contact_phone: Option<String>,
    channel: Option<String>,
    category: Option<String>,
    priority: Option<String>,
    status: Option<String>,
    related_order_id: Option<String>,
    satisfaction_score: Option<i32>,
    handler: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CrmServiceTicketUpdateRequest {
    customer_id: Option<String>,
    customer_name: Option<String>,
    contact_phone: Option<String>,
    channel: Option<String>,
    category: Option<String>,
    priority: Option<String>,
    status: Option<String>,
    related_order_id: Option<String>,
    satisfaction_score: Option<i32>,
    handler: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CrmLedgerListQuery {
    keyword: Option<String>,
    customer_id: Option<String>,
    created_from: Option<String>,
    created_to: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct CrmPointLedger {
    ledger_id: uuid::Uuid,
    customer_id: Option<String>,
    customer_name: String,
    change_points: i64,
    balance_points: i64,
    reason: String,
    ref_type: Option<String>,
    ref_id: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct CrmPointLedgerListResponse {
    items: Vec<CrmPointLedger>,
    total: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct CrmStoredValueLedger {
    ledger_id: uuid::Uuid,
    customer_id: Option<String>,
    customer_name: String,
    change_cent: i64,
    balance_cent: i64,
    reason: String,
    ref_type: Option<String>,
    ref_id: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct CrmStoredValueLedgerListResponse {
    items: Vec<CrmStoredValueLedger>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct StatsReportQuery {
    start_at: Option<String>,
    end_at: Option<String>,
    compare_start_at: Option<String>,
    compare_end_at: Option<String>,
    dimension: Option<String>,
    metrics: Option<String>,
}

#[derive(Debug, Serialize)]
struct StatsBreakdownItemResponse {
    dimension: String,
    dimension_label: Option<String>,
    metrics: HashMap<String, f64>,
}

#[derive(Debug, Serialize)]
struct StatsReportResponse {
    stats_type: String,
    current: HashMap<String, f64>,
    compare: Option<HashMap<String, f64>>,
    breakdown: Option<Vec<StatsBreakdownItemResponse>>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct StatsInfraHealthResponse {
    analytics_backend: String,
    clickhouse_ready: bool,
    sso_ready: bool,
    checked_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct OmsOrderDetailResponse {
    order: erp_repo::orders::OmsOrder,
    items: Vec<erp_repo::orders::OmsOrderItem>,
    review_hits: Vec<erp_repo::orders::OrderReviewHit>,
    tags: Vec<erp_repo::orders::OrderTag>,
    notes: Vec<erp_repo::orders::OrderNote>,
    audit_logs: Vec<erp_repo::audit_logs::AuditLog>,
}

#[derive(Debug, Deserialize)]
struct OrderReviewRuleRequest {
    name: String,
    enabled: Option<bool>,
    priority: Option<i32>,
    conditions_json: Option<Value>,
    action: String,
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OrderReviewRuleUpdateRequest {
    name: Option<String>,
    enabled: Option<bool>,
    priority: Option<i32>,
    conditions_json: Option<Value>,
    action: Option<String>,
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OrderReviewRuleListQuery {
    enabled: Option<bool>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct OrderReviewRuleListResponse {
    items: Vec<OrderReviewRule>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct CustomerBlacklistRequest {
    key_type: String,
    key_value: String,
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CustomerBlacklistListQuery {
    key_type: Option<String>,
    keyword: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct CustomerBlacklistListResponse {
    items: Vec<CustomerBlacklistEntry>,
    total: i64,
}

#[derive(Debug, Serialize)]
struct ExceptionReasonListResponse {
    items: Vec<ExceptionReason>,
    total: i64,
}

#[derive(Debug, Deserialize)]
struct ExceptionReasonListQuery {
    category: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct RoleCreateRequest {
    name: String,
    permissions: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct RoleUpdateRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct RolePermissionsRequest {
    permissions: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct UserCreateRequest {
    email: String,
    password: String,
    status: Option<String>,
    role_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct UserUpdateRequest {
    email: Option<String>,
    password: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserRolesRequest {
    role_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
struct UserRolesResponse {
    roles: Vec<Role>,
}

#[derive(Debug, Deserialize)]
struct DataScopeUpdateRequest {
    shop_ids: Vec<String>,
    warehouse_ids: Vec<String>,
    merchant_ids: Vec<String>,
    organization_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
struct DataScopeResponse {
    shop_ids: Vec<String>,
    warehouse_ids: Vec<String>,
    merchant_ids: Vec<String>,
    organization_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CurrentUserResponse {
    user: User,
    roles: Vec<Role>,
    permissions: Vec<String>,
    data_scopes: DataScopeResponse,
}

#[derive(Debug, Deserialize)]
struct AuditLogListQuery {
    action: Option<String>,
    entity_type: Option<String>,
    entity_id: Option<String>,
    user_id: Option<String>,
    created_from: Option<String>,
    created_to: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ShipmentPushAttemptListQuery {
    status: Option<String>,
    platform_order_id: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ShipmentPushAttemptResponse {
    attempt_id: String,
    outbox_id: String,
    shipment_id: String,
    shop_id: String,
    platform: String,
    platform_order_id: String,
    carrier_code: Option<String>,
    tracking_no: Option<String>,
    shipped_at: Option<DateTime<Utc>>,
    status: String,
    attempt: i32,
    duration_ms: i32,
    error_code: String,
    error_message: String,
    request_json: Value,
    response_json: Value,
    next_retry_at: Option<DateTime<Utc>>,
    dead_letter_id: Option<String>,
    trigger: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct DeadLetterListQuery {
    status: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct OutboxEventListQuery {
    status: Option<String>,
    event_type: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct OutboxRequeueResponse {
    outbox_id: uuid::Uuid,
}

#[derive(Debug, Deserialize)]
struct SettingsListQuery {
    scope_type: Option<String>,
    scope_id: Option<String>,
    key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SettingUpsertRequest {
    scope_type: String,
    scope_id: Option<String>,
    key: String,
    value: Value,
    expected_version: Option<i32>,
}

#[derive(Debug, Serialize)]
struct WmsInboundSettingsResponse {
    require_audit: bool,
    require_inspection: bool,
    require_putaway_location: bool,
    enable_return_inbound_link: bool,
}

#[derive(Debug, Deserialize)]
struct WmsInboundSettingsUpdateRequest {
    require_audit: Option<bool>,
    require_inspection: Option<bool>,
    require_putaway_location: Option<bool>,
    enable_return_inbound_link: Option<bool>,
}

#[allow(dead_code)]
async fn export_order_review_hits(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OrderReviewHitsExportRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "exports.write") {
        return resp.into_response();
    }
    if req.order_ids.is_empty() {
        return bad_request("order_ids required").into_response();
    }

    let repo = ExportJobsRepo::new(state.db.pool());
    let job = match repo
        .create_job(ExportJobCreateInput {
            tenant_id,
            job_type: "ORDER_REVIEW_HITS_EXPORT".to_string(),
            params_json: serde_json::json!({ "order_ids": req.order_ids }),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let orders_repo = OrdersRepo::new(state.db.pool());
    let mut rows: Vec<Vec<String>> = Vec::new();
    rows.push(vec![
        "订单ID".to_string(),
        "平台订单号".to_string(),
        "规则名称".to_string(),
        "动作".to_string(),
        "是否命中".to_string(),
        "说明".to_string(),
        "时间".to_string(),
    ]);

    for raw_id in &job.params_json["order_ids"]
        .as_array()
        .cloned()
        .unwrap_or_default()
    {
        let Some(raw) = raw_id.as_str() else {
            continue;
        };
        let order_id = match OrderId::from_str(raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let detail = match orders_repo.get_order_detail(tenant_id, order_id).await {
            Ok(Some(v)) => v,
            Ok(None) => continue,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        for hit in detail.review_hits {
            rows.push(vec![
                order_id.to_string(),
                detail.order.platform_order_id.clone(),
                hit.rule_name,
                hit.action,
                if hit.hit { "命中" } else { "未命中" }.to_string(),
                hit.message,
                hit.created_at.to_rfc3339(),
            ]);
        }
    }

    let csv = rows
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|cell| format!("\"{}\"", cell.replace('"', "\"\"")))
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let file_path = export_job_file_path(tenant_id, job.job_id);
    if let Some(parent) = file_path.parent() {
        if let Err(err) = tokio::fs::create_dir_all(parent).await {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
        }
    }
    if let Err(err) = tokio::fs::write(&file_path, format!("\u{FEFF}{csv}")).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    let output_url = Some(format!("/exports/jobs/{}/file", job.job_id));
    let updated = match repo
        .update_job(
            tenant_id,
            job.job_id,
            ExportJobUpdate {
                status: Some("DONE".to_string()),
                output_url: Some(output_url.clone()),
                progress: Some(100),
                last_error: None,
                params_json: None,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => job,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "ORDER.REVIEW_HITS_EXPORT".to_string(),
            entity_type: "EXPORT_JOB".to_string(),
            entity_id: Some(updated.job_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({ "output_url": output_url })),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(export_job_response(updated))).into_response()
}

#[allow(dead_code)]
async fn add_order_note(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OrderNoteRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };
    let note = req.note.trim();
    if note.is_empty() {
        return bad_request("note required").into_response();
    }

    let orders_repo = OrdersRepo::new(state.db.pool());
    let result = match orders_repo
        .add_order_note(
            tenant_id,
            order_id,
            OrderNoteCreate {
                note: note.to_string(),
                created_by: Some(ctx.user_id),
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "OMS.ORDER_NOTE_ADD".to_string(),
            entity_type: "ORDER".to_string(),
            entity_id: Some(order_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({ "note": note })),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(result)).into_response()
}

async fn list_operation_logs(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<OperationLogListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "operation_logs.read") {
        return resp.into_response();
    }

    let actor_user_id = match query.actor_user_id.as_deref() {
        Some(raw) => match UserId::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad actor_user_id").into_response(),
        },
        None => None,
    };
    let parse_dt = |raw: &str, label: &str| -> Result<DateTime<Utc>, Response> {
        match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Ok(v.with_timezone(&Utc)),
            Err(_) => Err(bad_request(&format!("bad {label}")).into_response()),
        }
    };
    let created_from = match query.created_from.as_deref() {
        Some(raw) => match parse_dt(raw, "created_from") {
            Ok(v) => Some(v),
            Err(resp) => return resp,
        },
        None => None,
    };
    let created_to = match query.created_to.as_deref() {
        Some(raw) => match parse_dt(raw, "created_to") {
            Ok(v) => Some(v),
            Err(resp) => return resp,
        },
        None => None,
    };
    let cursor = match query.cursor.as_deref() {
        Some(raw) => match OperationLogCursor::decode(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad cursor").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = OperationLogsRepo::new(state.db.pool());
    let result = match repo
        .list_logs(
            tenant_id,
            OperationLogListFilters {
                action: query.action,
                ref_type: query.ref_type,
                ref_id: query.ref_id,
                actor_user_id,
                created_from,
                created_to,
                cursor,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response: ListResponse<erp_repo::operation_logs::OperationLog> = ListResponse {
        data: result.items,
        next_cursor: result.next_cursor.map(|c| c.encode()),
    };
    (StatusCode::OK, Json(response)).into_response()
}

fn normalize_frontend_route_diagnostic_status(
    raw: Option<String>,
) -> Result<Option<String>, Response> {
    let Some(value) = raw.and_then(non_empty) else {
        return Ok(None);
    };
    let normalized = value.to_lowercase();
    if matches!(
        normalized.as_str(),
        "ready" | "timeout" | "error" | "loading" | "idle"
    ) {
        Ok(Some(normalized))
    } else {
        Err(bad_request("bad status").into_response())
    }
}

fn normalize_frontend_route_diagnostic_event_type(
    raw: Option<String>,
) -> Result<Option<String>, Response> {
    let Some(value) = raw.and_then(non_empty) else {
        return Ok(None);
    };
    let normalized = value.to_lowercase();
    if matches!(
        normalized.as_str(),
        "loading-ready"
            | "loading-timeout"
            | "loading-error"
            | "prefetch-ready"
            | "prefetch-error"
            | "prefetch-skipped"
    ) {
        Ok(Some(normalized))
    } else {
        Err(bad_request("bad event_type").into_response())
    }
}

async fn report_frontend_route_diagnostics(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<FrontendRouteDiagnosticsReportRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) =
        ensure_permission_any(&ctx, &["stats.read", "reports.read", "operation_logs.read"])
    {
        return resp.into_response();
    }
    if req.events.len() > 200 {
        return bad_request("too many events").into_response();
    }

    for event in &req.events {
        if event.session_id.trim().is_empty() {
            return bad_request("session_id required").into_response();
        }
        if event.path.trim().is_empty() {
            return bad_request("path required").into_response();
        }
        if event.page_url.trim().is_empty() {
            return bad_request("page_url required").into_response();
        }
        if event.build_version.trim().is_empty() {
            return bad_request("build_version required").into_response();
        }
        if event.user_agent.trim().is_empty() {
            return bad_request("user_agent required").into_response();
        }
        if event.event_at <= 0 {
            return bad_request("bad event_at").into_response();
        }
        if normalize_frontend_route_diagnostic_status(Some(event.status.clone())).is_err() {
            return bad_request("bad status").into_response();
        }
        if normalize_frontend_route_diagnostic_event_type(Some(event.event_type.clone())).is_err() {
            return bad_request("bad event_type").into_response();
        }
    }

    let repo = FrontendRouteDiagnosticsRepo::new(state.db.pool());
    let accepted = match repo
        .ingest_events(tenant_id, Some(ctx.user_id), &req.events)
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    for event in req
        .events
        .iter()
        .filter(|event| !event.path.trim().is_empty())
    {
        metrics().inc_frontend_route_diagnostic_ingested();
        if event.status.trim().eq_ignore_ascii_case("timeout") {
            metrics().inc_frontend_route_timeout();
        }
        if event.status.trim().eq_ignore_ascii_case("error") {
            metrics().inc_frontend_route_error();
        }
        if event
            .event_type
            .trim()
            .eq_ignore_ascii_case("prefetch-ready")
        {
            metrics().inc_frontend_route_prefetch_ready();
        }
    }

    (
        StatusCode::OK,
        Json(FrontendRouteDiagnosticsReportResponse { accepted }),
    )
        .into_response()
}

async fn get_frontend_route_diagnostics(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<FrontendRouteDiagnosticsQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) =
        ensure_permission_any(&ctx, &["stats.read", "reports.read", "operation_logs.read"])
    {
        return resp.into_response();
    }

    let status = match normalize_frontend_route_diagnostic_status(query.status) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let event_type = match normalize_frontend_route_diagnostic_event_type(query.event_type) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let window_hours = query.window_hours.unwrap_or(24).clamp(1, 24 * 7);
    let limit = query.limit.unwrap_or(20).clamp(1, 200);

    let repo = FrontendRouteDiagnosticsRepo::new(state.db.pool());
    let RepoFrontendRouteDiagnosticsResult {
        summary,
        paths,
        events,
    } = match repo
        .list(
            tenant_id,
            &FrontendRouteDiagnosticsFilters {
                path_keyword: query.path_keyword.and_then(non_empty),
                status,
                event_type,
                created_from: Some(Utc::now() - Duration::hours(window_hours)),
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(FrontendRouteDiagnosticsResponse {
            summary,
            paths,
            events,
        }),
    )
        .into_response()
}

async fn build_feature_flag_map(state: &AppState, tenant_id: TenantId) -> Result<Value, Response> {
    let read_only = match read_only_enabled(state, tenant_id, None).await {
        Ok(v) => v,
        Err(resp) => return Err(resp.into_response()),
    };
    let read_only_state = if read_only { "enabled" } else { "disabled" };
    let mut flags = serde_json::Map::new();
    let to_state =
        |enabled: bool| Value::String(if enabled { "enabled" } else { "disabled" }.into());

    flags.insert(
        "system.read_only".to_string(),
        Value::String(read_only_state.to_string()),
    );

    let inventory_enabled =
        match feature_flag_enabled(state, tenant_id, None, FeatureFlagKey::InventoryWrite).await {
            Ok(v) => v,
            Err(resp) => return Err(resp.into_response()),
        };
    flags.insert("inventory.enabled".to_string(), to_state(inventory_enabled));

    let cost_enabled =
        match feature_flag_enabled(state, tenant_id, None, FeatureFlagKey::CostManagement).await {
            Ok(v) => v,
            Err(resp) => return Err(resp.into_response()),
        };
    flags.insert("cost.enabled".to_string(), to_state(cost_enabled));

    let logistics_price_enabled =
        match feature_flag_enabled(state, tenant_id, None, FeatureFlagKey::LogisticsPrice).await {
            Ok(v) => v,
            Err(resp) => return Err(resp.into_response()),
        };
    let tag_system_enabled =
        match feature_flag_enabled(state, tenant_id, None, FeatureFlagKey::TagSystem).await {
            Ok(v) => v,
            Err(resp) => return Err(resp.into_response()),
        };
    flags.insert(
        "directory.logistics_price".to_string(),
        to_state(logistics_price_enabled),
    );
    flags.insert(
        "directory.tag_system".to_string(),
        to_state(tag_system_enabled),
    );

    let order_sync_enabled =
        match feature_flag_enabled(state, tenant_id, None, FeatureFlagKey::OrderSync).await {
            Ok(v) => v,
            Err(resp) => return Err(resp.into_response()),
        };
    let order_merge_enabled =
        match feature_flag_enabled(state, tenant_id, None, FeatureFlagKey::OrderMerge).await {
            Ok(v) => v,
            Err(resp) => return Err(resp.into_response()),
        };
    let inventory_sync_enabled =
        match feature_flag_enabled(state, tenant_id, None, FeatureFlagKey::InventorySync).await {
            Ok(v) => v,
            Err(resp) => return Err(resp.into_response()),
        };
    let aftersales_sync_enabled =
        match feature_flag_enabled(state, tenant_id, None, FeatureFlagKey::AfterSalesSync).await {
            Ok(v) => v,
            Err(resp) => return Err(resp.into_response()),
        };
    let shipping_enabled =
        match feature_flag_enabled(state, tenant_id, None, FeatureFlagKey::ShippingWrite).await {
            Ok(v) => v,
            Err(resp) => return Err(resp.into_response()),
        };
    let printing_enabled =
        match feature_flag_enabled(state, tenant_id, None, FeatureFlagKey::Printing).await {
            Ok(v) => v,
            Err(resp) => return Err(resp.into_response()),
        };
    let waybill_enabled = match feature_flag_enabled(
        state,
        tenant_id,
        None,
        FeatureFlagKey::WaybillAutoReserve,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return Err(resp.into_response()),
    };
    let auto_freeze_enabled =
        match feature_flag_enabled(state, tenant_id, None, FeatureFlagKey::AutoFreeze).await {
            Ok(v) => v,
            Err(resp) => return Err(resp.into_response()),
        };
    let pack_performance_enabled =
        match feature_flag_enabled(state, tenant_id, None, FeatureFlagKey::PackPerformance).await {
            Ok(v) => v,
            Err(resp) => return Err(resp.into_response()),
        };
    let pick_recommend_enabled =
        match feature_flag_enabled(state, tenant_id, None, FeatureFlagKey::PickRecommend).await {
            Ok(v) => v,
            Err(resp) => return Err(resp.into_response()),
        };
    let stock_count_suggest_enabled =
        match feature_flag_enabled(state, tenant_id, None, FeatureFlagKey::StockCountSuggest).await
        {
            Ok(v) => v,
            Err(resp) => return Err(resp.into_response()),
        };
    let wave_enabled =
        match feature_flag_enabled(state, tenant_id, None, FeatureFlagKey::WmsWave).await {
            Ok(v) => v,
            Err(resp) => return Err(resp.into_response()),
        };
    let outbound_inspection_enabled = match feature_flag_enabled(
        state,
        tenant_id,
        None,
        FeatureFlagKey::WmsOutboundInspection,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return Err(resp.into_response()),
    };
    let shipment_cancel_enabled =
        match feature_flag_enabled(state, tenant_id, None, FeatureFlagKey::WmsShipmentCancel).await
        {
            Ok(v) => v,
            Err(resp) => return Err(resp.into_response()),
        };
    let print_local_agent_enabled =
        match feature_flag_enabled(state, tenant_id, None, FeatureFlagKey::PrintLocalAgent).await {
            Ok(v) => v,
            Err(resp) => return Err(resp.into_response()),
        };

    let orders_ship_enabled = shipping_enabled && !read_only;
    let orders_print_waybill_enabled = waybill_enabled && printing_enabled && !read_only;

    flags.insert(
        "flows.order_auto_download".to_string(),
        to_state(order_sync_enabled),
    );
    flags.insert(
        "flows.merge_outside_ship_time".to_string(),
        to_state(order_merge_enabled),
    );
    flags.insert(
        "flows.inventory_sync".to_string(),
        to_state(inventory_sync_enabled),
    );
    flags.insert(
        "flows.aftersales_download".to_string(),
        to_state(aftersales_sync_enabled),
    );
    flags.insert(
        "flows.logistics_upload".to_string(),
        to_state(orders_ship_enabled),
    );
    flags.insert(
        "flows.waybill_reserve".to_string(),
        to_state(waybill_enabled && !read_only),
    );
    flags.insert(
        "flows.auto_freeze".to_string(),
        to_state(auto_freeze_enabled),
    );
    flags.insert(
        "flows.pack_performance".to_string(),
        to_state(pack_performance_enabled),
    );
    flags.insert(
        "flows.pick_recommend".to_string(),
        to_state(pick_recommend_enabled),
    );
    flags.insert(
        "flows.stock_count_suggest".to_string(),
        to_state(stock_count_suggest_enabled),
    );
    flags.insert(
        "wms.enable_wave".to_string(),
        to_state(wave_enabled && !read_only),
    );
    flags.insert(
        "wms.enable_reserve_tracking".to_string(),
        to_state(waybill_enabled && !read_only),
    );
    flags.insert(
        "wms.enable_outbound_inspection".to_string(),
        to_state(outbound_inspection_enabled && !read_only),
    );
    flags.insert(
        "wms.enable_pack_performance".to_string(),
        to_state(pack_performance_enabled && !read_only),
    );
    flags.insert(
        "wms.enable_shipment_cancel".to_string(),
        to_state(shipment_cancel_enabled && !read_only),
    );
    flags.insert(
        "print.enable_local_agent".to_string(),
        to_state(print_local_agent_enabled && printing_enabled && !read_only),
    );

    flags.insert("orders.ship".to_string(), to_state(orders_ship_enabled));
    flags.insert(
        "orders.print_waybill".to_string(),
        to_state(orders_print_waybill_enabled),
    );
    flags.insert(
        "orders.refund_sync".to_string(),
        to_state(aftersales_sync_enabled),
    );

    Ok(Value::Object(flags))
}

async fn get_feature_flags(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }

    match build_feature_flag_map(&state, tenant_id).await {
        Ok(flags) => (StatusCode::OK, Json(flags)).into_response(),
        Err(resp) => resp,
    }
}

#[derive(Deserialize)]
struct FeatureFlagsUpdatePayload {
    flags: HashMap<String, bool>,
}

#[derive(Serialize)]
struct FeatureFlagsUpdateResponse {
    flags: Value,
}

async fn update_feature_flags(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(payload): Json<FeatureFlagsUpdatePayload>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysSettingsWrite.as_str()) {
        return resp.into_response();
    }
    if payload.flags.is_empty() {
        return bad_request("flags required").into_response();
    }

    let repo = FeatureFlagsRepo::new(state.db.pool());
    for (key, enabled) in payload.flags {
        let flag_key = match key.as_str() {
            "flows.order_auto_download" => FeatureFlagKey::OrderSync,
            "flows.merge_outside_ship_time" => FeatureFlagKey::OrderMerge,
            "flows.inventory_sync" => FeatureFlagKey::InventorySync,
            "flows.aftersales_download" => FeatureFlagKey::AfterSalesSync,
            "flows.logistics_upload" => FeatureFlagKey::ShippingWrite,
            "flows.waybill_reserve" => FeatureFlagKey::WaybillAutoReserve,
            "flows.auto_freeze" => FeatureFlagKey::AutoFreeze,
            "flows.pack_performance" => FeatureFlagKey::PackPerformance,
            "flows.pick_recommend" => FeatureFlagKey::PickRecommend,
            "flows.stock_count_suggest" => FeatureFlagKey::StockCountSuggest,
            "cost.enabled" => FeatureFlagKey::CostManagement,
            "directory.logistics_price" => FeatureFlagKey::LogisticsPrice,
            "directory.tag_system" => FeatureFlagKey::TagSystem,
            "wms.enable_wave" => FeatureFlagKey::WmsWave,
            "wms.enable_reserve_tracking" => FeatureFlagKey::WaybillAutoReserve,
            "wms.enable_outbound_inspection" => FeatureFlagKey::WmsOutboundInspection,
            "wms.enable_pack_performance" => FeatureFlagKey::PackPerformance,
            "wms.enable_shipment_cancel" => FeatureFlagKey::WmsShipmentCancel,
            "print.enable_local_agent" => FeatureFlagKey::PrintLocalAgent,
            _ => return bad_request("unsupported feature flag").into_response(),
        };
        if let Err(e) = repo
            .upsert_flag(
                tenant_id,
                FeatureScope::Tenant,
                None,
                FeatureFlag::from_common_key(flag_key),
                enabled,
            )
            .await
        {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to update feature flag: {e}"),
            )
                .into_response();
        }
    }

    match build_feature_flag_map(&state, tenant_id).await {
        Ok(flags) => (StatusCode::OK, Json(FeatureFlagsUpdateResponse { flags })).into_response(),
        Err(resp) => resp,
    }
}

#[derive(Deserialize)]
struct InventoryFeatureFlagPayload {
    inventory_enabled: bool,
}

async fn update_inventory_feature_flag(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(payload): Json<InventoryFeatureFlagPayload>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysSettingsWrite.as_str()) {
        return resp.into_response();
    }

    let repo = FeatureFlagsRepo::new(state.db.pool());
    if let Err(e) = repo
        .upsert_flag(
            tenant_id,
            FeatureScope::Tenant,
            None,
            FeatureFlag::InventoryWrite,
            payload.inventory_enabled,
        )
        .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to update inventory flag: {e}"),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "inventory_enabled": payload.inventory_enabled
        })),
    )
        .into_response()
}

#[derive(Serialize, Deserialize)]
struct UserShortcutItem {
    path: String,
    name: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct UserShortcutsPayload {
    shortcuts: Vec<UserShortcutItem>,
}

async fn get_user_shortcuts(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }

    let repo = UserShortcutsRepo::new(state.db.pool());
    let shortcuts = match repo.get_shortcuts(tenant_id, ctx.user_id).await {
        Ok(Some(record)) => serde_json::from_value(record.shortcuts_json).unwrap_or_default(),
        Ok(None) => Vec::new(),
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to load user shortcuts: {e}"),
            )
                .into_response();
        }
    };

    (StatusCode::OK, Json(UserShortcutsPayload { shortcuts })).into_response()
}

async fn update_user_shortcuts(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(payload): Json<UserShortcutsPayload>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }

    let shortcuts_json = match serde_json::to_value(&payload.shortcuts) {
        Ok(v) => v,
        Err(_) => return bad_request("invalid shortcuts").into_response(),
    };

    let repo = UserShortcutsRepo::new(state.db.pool());
    let record = match repo
        .upsert_shortcuts(erp_repo::user_shortcuts::UserShortcutUpsert {
            tenant_id,
            user_id: ctx.user_id,
            shortcuts_json,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to update user shortcuts: {e}"),
            )
                .into_response();
        }
    };

    let shortcuts = serde_json::from_value(record.shortcuts_json).unwrap_or_default();
    (StatusCode::OK, Json(UserShortcutsPayload { shortcuts })).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use axum::{Extension, Json};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    struct RecordingSkuSerialRepo {
        calls: Mutex<Vec<(uuid::Uuid, uuid::Uuid)>>,
        fail_mark_shipped: bool,
    }

    impl RecordingSkuSerialRepo {
        fn new(fail_mark_shipped: bool) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                fail_mark_shipped,
            }
        }

        fn calls(&self) -> Vec<(uuid::Uuid, uuid::Uuid)> {
            self.calls.lock().expect("calls mutex poisoned").clone()
        }
    }

    #[async_trait]
    impl SkuSerialRepo for RecordingSkuSerialRepo {
        async fn list_serials(
            &self,
            _tenant_id: TenantId,
            _filters: SkuSerialListFilters,
        ) -> anyhow::Result<SkuSerialListResult> {
            Err(anyhow!("list_serials not implemented in test mock"))
        }

        async fn generate_serials(
            &self,
            _input: SkuSerialGenerateInput,
        ) -> anyhow::Result<Vec<SkuSerial>> {
            Err(anyhow!("generate_serials not implemented in test mock"))
        }

        async fn bind_serials_to_shipment(
            &self,
            _input: RepoShipmentSerialBindInput,
        ) -> anyhow::Result<Vec<SkuSerial>> {
            Err(anyhow!(
                "bind_serials_to_shipment not implemented in test mock"
            ))
        }

        async fn count_shipment_serials(
            &self,
            _tenant_id: TenantId,
            _shipment_id: uuid::Uuid,
        ) -> anyhow::Result<Vec<ShipmentSerialCount>> {
            Err(anyhow!(
                "count_shipment_serials not implemented in test mock"
            ))
        }

        async fn mark_shipment_serials_shipped(
            &self,
            tenant_id: TenantId,
            shipment_id: uuid::Uuid,
        ) -> anyhow::Result<()> {
            self.calls
                .lock()
                .expect("calls mutex poisoned")
                .push((uuid::Uuid::from(tenant_id), shipment_id));
            if self.fail_mark_shipped {
                Err(anyhow!("mark_shipment_serials_shipped failed"))
            } else {
                Ok(())
            }
        }
    }

    fn sample_shop(status: &str) -> Shop {
        let now = Utc::now();
        Shop {
            shop_id: ShopId(uuid::Uuid::new_v4()),
            tenant_id: TenantId(uuid::Uuid::new_v4()),
            platform: Platform::Taobao,
            platform_shop_id: "TB-TEST".to_string(),
            name: "测试店铺".to_string(),
            status: status.to_string(),
            auth_meta_json: json!({}),
            created_at: now,
            updated_at: now,
        }
    }

    fn sample_active_token(shop: &Shop) -> PlatformToken {
        let now = Utc::now();
        PlatformToken {
            tenant_id: shop.tenant_id,
            shop_id: shop.shop_id,
            token_status: "ACTIVE".to_string(),
            access_token: Some("access-token".to_string()),
            refresh_token: Some("refresh-token".to_string()),
            expires_at: Some(now + Duration::hours(2)),
            scopes: Some("orders,inventory".to_string()),
            last_refreshed_at: Some(now - Duration::minutes(10)),
            refresh_fail_count: 0,
            last_error: String::new(),
            created_at: now - Duration::hours(1),
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn sync_shipment_serials_after_wms_ship_marks_bound_serials_shipped() {
        let db = Db::connect_lazy("postgres://postgres@localhost/postgres")
            .expect("lazy db should be constructible");
        let jwt = JwtService::new("test-secret".to_string());
        let mut state = AppState::new(db, Arc::new(jwt));
        let repo = Arc::new(RecordingSkuSerialRepo::new(false));
        state.sku_serials_repo = repo.clone();

        let tenant_id = TenantId(uuid::Uuid::new_v4());
        let shipment_id = uuid::Uuid::new_v4();

        sync_shipment_serials_after_wms_ship(&state, tenant_id, Some(shipment_id)).await;

        assert_eq!(
            repo.calls(),
            vec![(uuid::Uuid::from(tenant_id), shipment_id)]
        );
    }

    #[tokio::test]
    async fn sync_shipment_serials_after_wms_ship_ignores_missing_shipment_id() {
        let db = Db::connect_lazy("postgres://postgres@localhost/postgres")
            .expect("lazy db should be constructible");
        let jwt = JwtService::new("test-secret".to_string());
        let mut state = AppState::new(db, Arc::new(jwt));
        let repo = Arc::new(RecordingSkuSerialRepo::new(false));
        state.sku_serials_repo = repo.clone();

        sync_shipment_serials_after_wms_ship(&state, TenantId(uuid::Uuid::new_v4()), None).await;

        assert!(repo.calls().is_empty());
    }

    #[tokio::test]
    async fn sync_shipment_serials_after_wms_ship_swallows_repo_errors() {
        let db = Db::connect_lazy("postgres://postgres@localhost/postgres")
            .expect("lazy db should be constructible");
        let jwt = JwtService::new("test-secret".to_string());
        let mut state = AppState::new(db, Arc::new(jwt));
        let repo = Arc::new(RecordingSkuSerialRepo::new(true));
        state.sku_serials_repo = repo.clone();

        let tenant_id = TenantId(uuid::Uuid::new_v4());
        let shipment_id = uuid::Uuid::new_v4();

        sync_shipment_serials_after_wms_ship(&state, tenant_id, Some(shipment_id)).await;

        assert_eq!(
            repo.calls(),
            vec![(uuid::Uuid::from(tenant_id), shipment_id)]
        );
    }

    #[tokio::test]
    async fn wms_verify_handlers_validate_path_params_before_db_access() {
        let db = Db::connect_lazy("postgres://postgres@localhost/postgres")
            .expect("lazy db should be constructible");
        let jwt = JwtService::new("test-secret".to_string());
        let state = AppState::new(db, Arc::new(jwt));
        let ctx = AuthContext {
            tenant_id: TenantId(uuid::Uuid::new_v4()),
            user_id: UserId(uuid::Uuid::new_v4()),
            perms: vec!["wms.verify.write".to_string()],
            is_admin: false,
            shop_ids: Vec::new(),
            warehouse_ids: Vec::new(),
            merchant_ids: Vec::new(),
            organization_ids: Vec::new(),
        session_id: None,
        };
        let req_ctx = RequestContext {
            request_id: "req-wms-verify".to_string(),
            trace_id: "trace-wms-verify".to_string(),
            ip: None,
            user_agent: None,
        };

        let start_resp = start_wms_outbound_verify(
            State(state.clone()),
            Path(("not-a-uuid".to_string(), "not-a-uuid".to_string())),
            Extension(ctx.clone()),
            Extension(req_ctx.clone()),
            Json(WmsOutboundVerifyStartRequest {
                shipment_no: "SHP-1".to_string(),
                tracking_no: None,
                package_no: None,
            }),
        )
        .await
        .into_response();
        assert_eq!(start_resp.status(), StatusCode::BAD_REQUEST);

        let scan_resp = scan_wms_outbound_verify_item(
            State(state.clone()),
            Path(("not-a-uuid".to_string(), "not-a-uuid".to_string())),
            Extension(ctx.clone()),
            Extension(req_ctx.clone()),
            Json(WmsOutboundVerifyScanItemRequest {
                barcode: "6901234567890".to_string(),
                qty: 1,
            }),
        )
        .await
        .into_response();
        assert_eq!(scan_resp.status(), StatusCode::BAD_REQUEST);

        let finish_resp = finish_wms_outbound_verify(
            State(state),
            Path(("not-a-uuid".to_string(), "not-a-uuid".to_string())),
            Extension(ctx),
            Extension(req_ctx),
        )
        .await
        .into_response();
        assert_eq!(finish_resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn derive_webhook_health_summary_is_idle_without_events() {
        let now = Utc::now();
        let summary =
            derive_webhook_health_summary_with_clock(None, None, 0, 0, 0, 0, 0, None, None, now);
        assert_eq!(summary.webhook_status, "IDLE");
        assert_eq!(summary.webhook_sla_status, "IDLE");
        assert_eq!(summary.last_webhook_received_at, None);
        assert_eq!(summary.last_webhook_processed_at, None);
        assert_eq!(summary.webhook_failed_count, 0);
        assert_eq!(summary.webhook_verify_failed_count, 0);
        assert_eq!(summary.webhook_pending_count, 0);
        assert_eq!(summary.webhook_retrying_count, 0);
        assert_eq!(summary.webhook_dlq_count, 0);
        assert_eq!(summary.webhook_next_retry_at, None);
    }

    #[test]
    fn derive_webhook_health_summary_is_warning_when_verify_failed() {
        let received_at = Utc::now() - Duration::minutes(5);
        let summary = derive_webhook_health_summary_with_clock(
            Some(received_at),
            None,
            1,
            1,
            0,
            0,
            0,
            None,
            None,
            Utc::now(),
        );
        assert_eq!(summary.webhook_status, "WARNING");
        assert_eq!(summary.webhook_sla_status, "HEALTHY");
        assert_eq!(summary.last_webhook_received_at, Some(received_at));
        assert_eq!(summary.last_webhook_processed_at, None);
        assert_eq!(summary.webhook_failed_count, 1);
        assert_eq!(summary.webhook_verify_failed_count, 1);
        assert_eq!(summary.webhook_pending_count, 0);
        assert_eq!(summary.webhook_retrying_count, 0);
        assert_eq!(summary.webhook_dlq_count, 0);
    }

    #[test]
    fn derive_webhook_health_summary_is_dead_when_dlq_exists() {
        let now = Utc::now();
        let received_at = now - Duration::minutes(8);
        let summary = derive_webhook_health_summary_with_clock(
            Some(received_at),
            None,
            1,
            0,
            0,
            0,
            1,
            None,
            None,
            now,
        );
        assert_eq!(summary.webhook_status, "WARNING");
        assert_eq!(summary.webhook_sla_status, "DEAD");
        assert_eq!(summary.webhook_dlq_count, 1);
    }

    #[test]
    fn derive_webhook_health_summary_is_warning_when_retrying() {
        let now = Utc::now();
        let received_at = now - Duration::minutes(6);
        let next_retry_at = now + Duration::minutes(3);
        let summary = derive_webhook_health_summary_with_clock(
            Some(received_at),
            None,
            0,
            0,
            1,
            1,
            0,
            Some(next_retry_at),
            Some(received_at),
            now,
        );
        assert_eq!(summary.webhook_status, "HEALTHY");
        assert_eq!(summary.webhook_sla_status, "WARNING");
        assert_eq!(summary.webhook_pending_count, 1);
        assert_eq!(summary.webhook_retrying_count, 1);
        assert_eq!(summary.webhook_next_retry_at, Some(next_retry_at));
    }

    #[test]
    fn derive_shop_integration_status_downgrades_on_webhook_warning() {
        let shop = sample_shop("AUTHORIZED");
        let token = sample_active_token(&shop);
        let token_status = shop_token_status(&token);
        let webhook = derive_webhook_health_summary_with_clock(
            Some(Utc::now() - Duration::minutes(5)),
            None,
            1,
            1,
            0,
            0,
            0,
            None,
            None,
            Utc::now(),
        );

        let integration_status =
            derive_shop_integration_status(&shop, "SUCCESS", Some(&token_status), 0, &webhook);

        assert_eq!(integration_status, "WARNING");
    }

    #[test]
    fn derive_shop_integration_status_downgrades_on_webhook_sla_warning() {
        let now = Utc::now();
        let shop = sample_shop("AUTHORIZED");
        let token = sample_active_token(&shop);
        let token_status = shop_token_status(&token);
        let webhook = derive_webhook_health_summary_with_clock(
            Some(now - Duration::minutes(6)),
            Some(now - Duration::minutes(7)),
            0,
            0,
            1,
            1,
            0,
            Some(now + Duration::minutes(2)),
            Some(now - Duration::minutes(6)),
            now,
        );

        let integration_status =
            derive_shop_integration_status(&shop, "SUCCESS", Some(&token_status), 0, &webhook);

        assert_eq!(integration_status, "WARNING");
    }
}
