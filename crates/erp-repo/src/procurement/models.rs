use chrono::{DateTime, NaiveDate, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};

pub const PURCHASE_ORDER_STATUS_DRAFT: &str = "DRAFT";
pub const PURCHASE_ORDER_STATUS_SENT: &str = "SENT";
pub const PURCHASE_ORDER_STATUS_CONFIRMED: &str = "CONFIRMED";
pub const PURCHASE_ORDER_STATUS_RECEIVING: &str = "RECEIVING";
pub const PURCHASE_ORDER_STATUS_DONE: &str = "DONE";
pub const PURCHASE_ORDER_STATUS_CANCELLED: &str = "CANCELLED";
pub const PURCHASE_ORDER_STATUS_PENDING_APPROVAL: &str = "PENDING_APPROVAL";
pub const PURCHASE_ORDER_STATUS_PLACED: &str = "PLACED";
pub const PURCHASE_ORDER_STATUS_PARTIALLY_RECEIVED: &str = "PARTIALLY_RECEIVED";
pub const PURCHASE_ORDER_STATUS_RECEIVED: &str = "RECEIVED";
pub const PURCHASE_ORDER_STATUS_CLOSED: &str = "CLOSED";

pub const PURCHASE_RECEIPT_STATUS_CONFIRMED: &str = "CONFIRMED";
pub const PURCHASE_RECEIPT_STATUS_NOT_RECEIVED: &str = "NOT_RECEIVED";
pub const PURCHASE_RECEIPT_STATUS_PART_RECEIVED: &str = "PART_RECEIVED";
pub const PURCHASE_RECEIPT_STATUS_RECEIVED: &str = "RECEIVED";
pub const PURCHASE_RETURN_STATUS_DRAFT: &str = "DRAFT";
pub const PURCHASE_RETURN_STATUS_DONE: &str = "DONE";

pub const AP_STATUS_OPEN: &str = "OPEN";
pub const AP_STATUS_PAID: &str = "PAID";
pub const AP_STATUS_CLOSED: &str = "CLOSED";
pub const AP_SETTLEMENT_STATUS_UNSETTLED: &str = "UNSETTLED";
pub const AP_SETTLEMENT_STATUS_SETTLING: &str = "SETTLING";
pub const AP_SETTLEMENT_STATUS_SETTLED: &str = "SETTLED";

pub const OUTBOUND_STATUS_SHIPPED: &str = "SHIPPED";
pub const INBOUND_STATUS_PUTAWAY_DONE: &str = "PUTAWAY_DONE";

pub const INBOUND_DOC_TYPE_PURCHASE: &str = "PURCHASE";
pub const OUTBOUND_DOC_TYPE_PURCHASE_RETURN: &str = "PURCHASE_RETURN";

pub const INVENTORY_REASON_PURCHASE_RECEIPT: &str = "PURCHASE_RECEIPT";
pub const INVENTORY_REASON_PURCHASE_RETURN: &str = "PURCHASE_RETURN";
pub const PROCUREMENT_REQUEST_STATUS_DRAFT: &str = "DRAFT";
pub const PROCUREMENT_REQUEST_STATUS_APPROVED: &str = "APPROVED";
pub const PROCUREMENT_REQUEST_STATUS_REJECTED: &str = "REJECTED";
pub const PROCUREMENT_PLAN_STATUS_DRAFT: &str = "DRAFT";
pub const PROCUREMENT_PLAN_STATUS_APPROVED: &str = "APPROVED";
pub const PROCUREMENT_PLAN_STATUS_RELEASED: &str = "RELEASED";
pub const PROCUREMENT_SOURCING_STATUS_OPEN: &str = "OPEN";
pub const PROCUREMENT_SOURCING_STATUS_REVIEWED: &str = "REVIEWED";
pub const PROCUREMENT_INSPECTION_STATUS_PENDING: &str = "PENDING";
pub const PROCUREMENT_INSPECTION_STATUS_PASSED: &str = "PASSED";
pub const PROCUREMENT_INSPECTION_STATUS_FAILED: &str = "FAILED";
pub const PROCUREMENT_RECONCILIATION_STATUS_OPEN: &str = "OPEN";
pub const PROCUREMENT_RECONCILIATION_STATUS_CONFIRMED: &str = "CONFIRMED";
pub const SUPPLIER_PERFORMANCE_STATUS_ACTIVE: &str = "ACTIVE";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrder {
    pub po_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub supplier_id: uuid::Uuid,
    pub supplier_name: Option<String>,
    pub warehouse_id: Option<uuid::Uuid>,
    pub expected_at: Option<DateTime<Utc>>,
    pub remark: Option<String>,
    pub status: String,
    pub purchase_order_status: String,
    pub total_cent: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PurchaseOrderItem {
    pub sku_id: uuid::Uuid,
    pub sku_code: Option<String>,
    pub qty: i64,
    pub price_cent: i64,
    pub received_qty: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PurchaseOrderDetail {
    pub order: PurchaseOrder,
    pub items: Vec<PurchaseOrderItem>,
    pub receipts: Vec<PurchaseReceiptSummary>,
}

#[derive(Debug, Clone)]
pub struct PurchaseOrderItemInput {
    pub sku_id: uuid::Uuid,
    pub qty: i64,
    pub price_cent: i64,
}

#[derive(Debug, Clone)]
pub struct PurchaseOrderCreate {
    pub tenant_id: TenantId,
    pub supplier_id: uuid::Uuid,
    pub warehouse_id: uuid::Uuid,
    pub expected_at: Option<DateTime<Utc>>,
    pub remark: Option<String>,
    pub items: Vec<PurchaseOrderItemInput>,
}

#[derive(Debug, Clone)]
pub struct PurchaseOrderUpdate {
    pub supplier_id: Option<uuid::Uuid>,
    pub warehouse_id: Option<uuid::Uuid>,
    pub expected_at: Option<DateTime<Utc>>,
    pub remark: Option<String>,
    pub items: Option<Vec<PurchaseOrderItemInput>>,
}

#[derive(Debug, Clone)]
pub struct PurchaseOrderListFilters {
    pub supplier_id: Option<uuid::Uuid>,
    pub warehouse_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct PurchaseOrderListResult {
    pub items: Vec<PurchaseOrder>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseReceipt {
    pub receipt_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub po_id: uuid::Uuid,
    pub supplier_id: Option<uuid::Uuid>,
    pub supplier_name: Option<String>,
    pub warehouse_id: uuid::Uuid,
    pub status: String,
    pub receiving_status: String,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PurchaseReceiptItem {
    pub sku_id: uuid::Uuid,
    pub qty: i64,
}

#[derive(Debug, Clone)]
pub struct PurchaseReceiptDetail {
    pub receipt: PurchaseReceipt,
    pub items: Vec<PurchaseReceiptItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PurchaseReceiptSummary {
    pub receipt_id: uuid::Uuid,
    pub po_id: uuid::Uuid,
    pub supplier_id: uuid::Uuid,
    pub supplier_name: Option<String>,
    pub warehouse_id: uuid::Uuid,
    pub status: String,
    pub receiving_status: String,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PurchaseReceiptListFilters {
    pub supplier_id: Option<uuid::Uuid>,
    pub warehouse_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct PurchaseReceiptListResult {
    pub items: Vec<PurchaseReceiptSummary>,
    pub total: i64,
}

#[derive(Debug, Clone)]
pub struct PurchaseOrderReceiveItem {
    pub sku_id: uuid::Uuid,
    pub qty: i64,
}

#[derive(Debug, Clone)]
pub struct PurchaseOrderReceiveInput {
    pub tenant_id: TenantId,
    pub po_id: uuid::Uuid,
    pub warehouse_id: uuid::Uuid,
    pub items: Vec<PurchaseOrderReceiveItem>,
    pub update_cost: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PurchaseOrderReceiveResult {
    pub order: PurchaseOrder,
    pub receipt: PurchaseReceipt,
    pub receipt_items: Vec<PurchaseReceiptItem>,
    pub inbound_doc_id: uuid::Uuid,
    pub ap_doc_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseReturn {
    pub return_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub po_id: uuid::Uuid,
    pub supplier_id: Option<uuid::Uuid>,
    pub supplier_name: Option<String>,
    pub warehouse_id: uuid::Uuid,
    pub status: String,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PurchaseReturnItem {
    pub sku_id: uuid::Uuid,
    pub qty: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PurchaseReturnDetail {
    pub purchase_return: PurchaseReturn,
    pub items: Vec<PurchaseReturnItem>,
}

#[derive(Debug, Clone)]
pub struct PurchaseReturnCreate {
    pub tenant_id: TenantId,
    pub po_id: uuid::Uuid,
    pub warehouse_id: uuid::Uuid,
    pub items: Vec<PurchaseReturnItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PurchaseReturnConfirmResult {
    pub purchase_return: PurchaseReturn,
    pub outbound_doc_id: uuid::Uuid,
}

#[derive(Debug, Clone)]
pub struct PurchaseReturnListFilters {
    pub supplier_id: Option<uuid::Uuid>,
    pub warehouse_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct PurchaseReturnListResult {
    pub items: Vec<PurchaseReturn>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseSuggestion {
    pub tenant_id: TenantId,
    pub sku_id: uuid::Uuid,
    pub sku_code: Option<String>,
    pub suggested_qty: i64,
    pub reason: String,
    pub computed_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PurchaseSuggestionListFilters {
    pub sku_id: Option<uuid::Uuid>,
    pub reason: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct PurchaseSuggestionListResult {
    pub items: Vec<PurchaseSuggestion>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApDoc {
    pub ap_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub supplier_id: uuid::Uuid,
    pub supplier_name: Option<String>,
    pub ref_type: String,
    pub ref_id: String,
    pub amount_cent: i64,
    pub status: String,
    pub settlement_status: String,
    pub paid_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ApDocListFilters {
    pub supplier_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct ApDocListResult {
    pub items: Vec<ApDoc>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcurementRequest {
    pub request_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub department: Option<String>,
    pub requester: Option<String>,
    pub status: String,
    pub budget_cent: i64,
    pub expected_at: Option<DateTime<Utc>>,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProcurementRequestItem {
    pub sku_id: uuid::Uuid,
    pub sku_code: Option<String>,
    pub qty: i64,
    pub price_cent: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcurementRequestDetail {
    pub request: ProcurementRequest,
    pub items: Vec<ProcurementRequestItem>,
}

#[derive(Debug, Clone)]
pub struct ProcurementRequestItemInput {
    pub sku_id: uuid::Uuid,
    pub qty: i64,
    pub price_cent: i64,
}

#[derive(Debug, Clone)]
pub struct ProcurementRequestCreate {
    pub tenant_id: TenantId,
    pub department: Option<String>,
    pub requester: Option<String>,
    pub expected_at: Option<DateTime<Utc>>,
    pub remark: Option<String>,
    pub items: Vec<ProcurementRequestItemInput>,
}

#[derive(Debug, Clone)]
pub struct ProcurementRequestUpdate {
    pub department: Option<String>,
    pub requester: Option<String>,
    pub status: Option<String>,
    pub expected_at: Option<DateTime<Utc>>,
    pub remark: Option<String>,
    pub items: Option<Vec<ProcurementRequestItemInput>>,
}

#[derive(Debug, Clone)]
pub struct ProcurementRequestListFilters {
    pub department: Option<String>,
    pub requester: Option<String>,
    pub status: Option<String>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct ProcurementRequestListResult {
    pub items: Vec<ProcurementRequest>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcurementPlan {
    pub plan_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub request_id: Option<uuid::Uuid>,
    pub supplier_id: Option<uuid::Uuid>,
    pub supplier_name: Option<String>,
    pub status: String,
    pub total_cent: i64,
    pub planned_at: Option<DateTime<Utc>>,
    pub expected_at: Option<DateTime<Utc>>,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProcurementPlanItem {
    pub sku_id: uuid::Uuid,
    pub sku_code: Option<String>,
    pub qty: i64,
    pub price_cent: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcurementPlanDetail {
    pub plan: ProcurementPlan,
    pub items: Vec<ProcurementPlanItem>,
}

#[derive(Debug, Clone)]
pub struct ProcurementPlanItemInput {
    pub sku_id: uuid::Uuid,
    pub qty: i64,
    pub price_cent: i64,
}

#[derive(Debug, Clone)]
pub struct ProcurementPlanCreate {
    pub tenant_id: TenantId,
    pub request_id: Option<uuid::Uuid>,
    pub supplier_id: Option<uuid::Uuid>,
    pub planned_at: Option<DateTime<Utc>>,
    pub expected_at: Option<DateTime<Utc>>,
    pub remark: Option<String>,
    pub items: Vec<ProcurementPlanItemInput>,
}

#[derive(Debug, Clone)]
pub struct ProcurementPlanUpdate {
    pub supplier_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub planned_at: Option<DateTime<Utc>>,
    pub expected_at: Option<DateTime<Utc>>,
    pub remark: Option<String>,
    pub items: Option<Vec<ProcurementPlanItemInput>>,
}

#[derive(Debug, Clone)]
pub struct ProcurementPlanListFilters {
    pub supplier_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct ProcurementPlanListResult {
    pub items: Vec<ProcurementPlan>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcurementSourcing {
    pub sourcing_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub title: String,
    pub category: Option<String>,
    pub supplier_id: Option<uuid::Uuid>,
    pub supplier_name: Option<String>,
    pub status: String,
    pub target_price_cent: i64,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ProcurementSourcingCreate {
    pub tenant_id: TenantId,
    pub title: String,
    pub category: Option<String>,
    pub supplier_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub target_price_cent: i64,
    pub remark: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProcurementSourcingUpdate {
    pub title: Option<String>,
    pub category: Option<String>,
    pub supplier_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub target_price_cent: Option<i64>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProcurementSourcingListFilters {
    pub keyword: Option<String>,
    pub supplier_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct ProcurementSourcingListResult {
    pub items: Vec<ProcurementSourcing>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcurementInspection {
    pub inspection_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub receipt_id: Option<uuid::Uuid>,
    pub supplier_id: Option<uuid::Uuid>,
    pub supplier_name: Option<String>,
    pub warehouse_id: Option<uuid::Uuid>,
    pub status: String,
    pub inspected_at: Option<DateTime<Utc>>,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProcurementInspectionItem {
    pub sku_id: uuid::Uuid,
    pub sku_code: Option<String>,
    pub qty: i64,
    pub passed_qty: i64,
    pub failed_qty: i64,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcurementInspectionDetail {
    pub inspection: ProcurementInspection,
    pub items: Vec<ProcurementInspectionItem>,
}

#[derive(Debug, Clone)]
pub struct ProcurementInspectionItemInput {
    pub sku_id: uuid::Uuid,
    pub qty: i64,
    pub passed_qty: i64,
    pub failed_qty: i64,
    pub result: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProcurementInspectionCreate {
    pub tenant_id: TenantId,
    pub receipt_id: Option<uuid::Uuid>,
    pub supplier_id: Option<uuid::Uuid>,
    pub warehouse_id: Option<uuid::Uuid>,
    pub inspected_at: Option<DateTime<Utc>>,
    pub remark: Option<String>,
    pub items: Vec<ProcurementInspectionItemInput>,
}

#[derive(Debug, Clone)]
pub struct ProcurementInspectionUpdate {
    pub status: Option<String>,
    pub inspected_at: Option<DateTime<Utc>>,
    pub remark: Option<String>,
    pub items: Option<Vec<ProcurementInspectionItemInput>>,
}

#[derive(Debug, Clone)]
pub struct ProcurementInspectionListFilters {
    pub supplier_id: Option<uuid::Uuid>,
    pub warehouse_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct ProcurementInspectionListResult {
    pub items: Vec<ProcurementInspection>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcurementReconciliation {
    pub reconciliation_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub supplier_id: uuid::Uuid,
    pub supplier_name: Option<String>,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub amount_cent: i64,
    pub status: String,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ProcurementReconciliationCreate {
    pub tenant_id: TenantId,
    pub supplier_id: uuid::Uuid,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub amount_cent: i64,
    pub status: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProcurementReconciliationUpdate {
    pub supplier_id: Option<uuid::Uuid>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub amount_cent: Option<i64>,
    pub status: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProcurementReconciliationListFilters {
    pub supplier_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub period_from: Option<NaiveDate>,
    pub period_to: Option<NaiveDate>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct ProcurementReconciliationListResult {
    pub items: Vec<ProcurementReconciliation>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplierPerformance {
    pub performance_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub supplier_id: uuid::Uuid,
    pub supplier_name: Option<String>,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub delivery_score: i32,
    pub quality_score: i32,
    pub price_score: i32,
    pub total_score: i32,
    pub status: String,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SupplierPerformanceCreate {
    pub tenant_id: TenantId,
    pub supplier_id: uuid::Uuid,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub delivery_score: i32,
    pub quality_score: i32,
    pub price_score: i32,
    pub status: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SupplierPerformanceUpdate {
    pub supplier_id: Option<uuid::Uuid>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub delivery_score: Option<i32>,
    pub quality_score: Option<i32>,
    pub price_score: Option<i32>,
    pub status: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SupplierPerformanceListFilters {
    pub supplier_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub period_from: Option<NaiveDate>,
    pub period_to: Option<NaiveDate>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct SupplierPerformanceListResult {
    pub items: Vec<SupplierPerformance>,
    pub total: i64,
}
