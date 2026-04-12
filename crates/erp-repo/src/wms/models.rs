use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};

pub const SHIPMENT_SHIPPED_EVENT: &str = "SHIPMENT_SHIPPED";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundDoc {
    pub outbound_doc_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub doc_type: String,
    pub shipment_id: Option<uuid::Uuid>,
    pub order_id: Option<uuid::Uuid>,
    pub platform: Option<String>,
    pub platform_order_id: Option<String>,
    pub shop_id: Option<uuid::Uuid>,
    pub receiver_name: Option<String>,
    pub receiver_phone: Option<String>,
    pub receiver_address: Option<String>,
    pub carrier_code: Option<String>,
    pub tracking_no: Option<String>,
    pub exception_flags: Vec<String>,
    pub tags: Vec<String>,
    pub doc_no: String,
    pub status: String,
    pub outbound_status: String,
    pub pick_status: String,
    pub verify_status: String,
    pub pack_status: String,
    pub ship_status: String,
    pub picked_at: Option<DateTime<Utc>>,
    pub packed_at: Option<DateTime<Utc>>,
    pub shipped_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundDocItem {
    pub outbound_doc_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub planned_qty: i64,
    pub picked_qty: i64,
    pub packed_qty: i64,
    pub batch_enabled: bool,
    pub serial_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundPickTask {
    pub pick_task_id: uuid::Uuid,
    pub outbound_doc_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub from_location_id: Option<uuid::Uuid>,
    pub container_id: Option<uuid::Uuid>,
    pub qty: i64,
    pub picked_qty: i64,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackRecord {
    pub pack_record_id: uuid::Uuid,
    pub shipment_id: Option<uuid::Uuid>,
    pub package_no: String,
    pub weight_gram: Option<i32>,
    pub operator_id: Option<uuid::Uuid>,
    pub packed_at: DateTime<Utc>,
    pub pack_status: String,
    pub void_reason: Option<String>,
    pub voided_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct PackCheckEventInput {
    pub tenant_id: TenantId,
    pub outbound_doc_id: uuid::Uuid,
    pub shipment_id: Option<uuid::Uuid>,
    pub package_no: String,
    pub event_type: String,
    pub result: String,
    pub weight_gram: Option<i32>,
    pub reason: Option<String>,
    pub operator_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundDocDetail {
    pub doc: OutboundDoc,
    pub items: Vec<OutboundDocItem>,
    pub pick_tasks: Vec<OutboundPickTask>,
    pub pack_records: Vec<PackRecord>,
}

#[derive(Debug, Clone)]
pub struct OutboundDocListFilters {
    pub warehouse_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub outbound_status: Option<String>,
    pub pick_status: Option<String>,
    pub verify_status: Option<String>,
    pub pack_status: Option<String>,
    pub ship_status: Option<String>,
    pub doc_type: Option<String>,
    pub doc_no: Option<String>,
    pub doc_no_keyword: Option<String>,
    pub shipment_id: Option<uuid::Uuid>,
    pub shop_id: Option<uuid::Uuid>,
    pub carrier_code: Option<String>,
    pub tracking_no: Option<String>,
    pub tag: Option<String>,
    pub exception_flag: Option<String>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub offset: i64,
    pub limit: i64,
}

pub struct OutboundDocListResult {
    pub items: Vec<OutboundDoc>,
    pub total: i64,
}

#[derive(Debug, Clone)]
pub struct OutboundDocItemInput {
    pub sku_id: uuid::Uuid,
    pub planned_qty: i64,
}

#[derive(Debug, Clone)]
pub struct OutboundDocCreateInput {
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub doc_type: String,
    pub shipment_id: Option<uuid::Uuid>,
    pub doc_no: String,
    pub items: Vec<OutboundDocItemInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wave {
    pub wave_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct WaveListFilters {
    pub warehouse_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct WaveListResult {
    pub items: Vec<Wave>,
    pub total: i64,
}

pub struct WaveDetail {
    pub wave: Wave,
    pub docs: Vec<OutboundDoc>,
    pub tasks: Vec<OutboundPickTask>,
}

#[derive(Debug, Clone)]
pub struct WaveCreateInput {
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub cutoff_at: DateTime<Utc>,
    pub max_docs: i64,
    pub auto_pick: bool,
    pub outbound_doc_ids: Option<Vec<uuid::Uuid>>,
}

#[derive(Debug, Clone)]
pub struct WaveCandidate {
    pub outbound_doc_id: uuid::Uuid,
    pub total_qty: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PickTaskListFilters {
    pub warehouse_id: Option<uuid::Uuid>,
    pub outbound_doc_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct PickTaskListResult {
    pub items: Vec<OutboundPickTask>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveRule {
    pub rule_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub carrier_code: Option<String>,
    pub max_orders: Option<i32>,
    pub max_qty: Option<i64>,
    pub cutoff_time: Option<chrono::NaiveTime>,
    pub strategy: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct WaveRuleListFilters {
    pub warehouse_id: Option<uuid::Uuid>,
    pub carrier_code: Option<String>,
    pub enabled: Option<bool>,
    pub offset: i64,
    pub limit: i64,
}

pub struct WaveRuleListResult {
    pub items: Vec<WaveRule>,
    pub total: i64,
}

#[derive(Debug, Clone)]
pub struct WaveRuleCreateInput {
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub carrier_code: Option<String>,
    pub max_orders: Option<i32>,
    pub max_qty: Option<i64>,
    pub cutoff_time: Option<chrono::NaiveTime>,
    pub strategy: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct WaveRuleUpdateInput {
    pub carrier_code: Option<String>,
    pub max_orders: Option<i32>,
    pub max_qty: Option<i64>,
    pub cutoff_time: Option<chrono::NaiveTime>,
    pub strategy: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct PickTaskConfirmInput {
    pub tenant_id: TenantId,
    pub pick_task_id: uuid::Uuid,
    pub picked_qty: i64,
    pub container_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone)]
pub struct OutboundPackItemInput {
    pub sku_id: uuid::Uuid,
    pub packed_qty: i64,
}

#[derive(Debug, Clone)]
pub struct OutboundPackBatchInput {
    pub sku_id: uuid::Uuid,
    pub batch_id: Option<uuid::Uuid>,
    pub batch_no: Option<String>,
    pub qty: i64,
}

#[derive(Debug, Clone)]
pub struct OutboundPackInput {
    pub tenant_id: TenantId,
    pub outbound_doc_id: uuid::Uuid,
    pub package_no: String,
    pub weight_gram: Option<i32>,
    pub operator_id: Option<uuid::Uuid>,
    pub items: Vec<OutboundPackItemInput>,
    pub batches: Vec<OutboundPackBatchInput>,
}

#[derive(Debug, Clone)]
pub struct OutboundShipInput {
    pub tenant_id: TenantId,
    pub outbound_doc_id: uuid::Uuid,
    pub shipped_at: Option<DateTime<Utc>>,
    pub allow_unpacked: bool,
    pub inspect_required: bool,
}

#[derive(Debug, Clone)]
pub struct OutboundInspectInput {
    pub tenant_id: TenantId,
    pub outbound_doc_id: uuid::Uuid,
}

#[derive(Debug, Clone)]
pub struct OutboundVerifyStartInput {
    pub tenant_id: TenantId,
    pub outbound_doc_id: uuid::Uuid,
    pub shipment_no: String,
    pub tracking_no: Option<String>,
    pub package_no: String,
    pub started_by: Option<uuid::Uuid>,
}

#[derive(Debug, Clone)]
pub struct OutboundVerifyScanInput {
    pub tenant_id: TenantId,
    pub outbound_doc_id: uuid::Uuid,
    pub barcode: String,
    pub qty: i64,
    pub scanned_by: Option<uuid::Uuid>,
}

#[derive(Debug, Clone)]
pub struct OutboundVerifyFinishInput {
    pub tenant_id: TenantId,
    pub outbound_doc_id: uuid::Uuid,
    pub finished_by: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundVerifyState {
    pub outbound_doc_id: uuid::Uuid,
    pub session_id: uuid::Uuid,
    pub verify_status: String,
    pub scanned_total_qty: i64,
    pub missing_total_qty: i64,
    pub extra_total_qty: i64,
}
