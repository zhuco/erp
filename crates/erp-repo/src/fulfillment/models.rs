use chrono::{DateTime, Utc};
use erp_types::TenantId;

pub const SHIPMENT_TYPE_OMS: &str = "OMS";
pub const SHIPMENT_TYPE_EXCHANGE: &str = "EXCHANGE";
pub const SHIPMENT_TYPE_RESEND: &str = "RESEND";
pub const SHIPMENT_STATUS_CREATED: &str = "CREATED";
pub const SHIPMENT_STATUS_SHIPPED: &str = "SHIPPED";
pub const SHIPMENT_STATUS_PARTIALLY_SHIPPED: &str = "PARTIALLY_SHIPPED";

#[derive(Debug, Clone)]
pub struct WarehouseRule {
    pub rule_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub province: Option<String>,
    pub city: Option<String>,
    pub carrier_code: Option<String>,
    pub priority: i32,
    pub is_active: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OmsShipment {
    pub shipment_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub shipment_no: Option<String>,
    pub shipment_type: String,
    pub origin_shipment_id: Option<uuid::Uuid>,
    pub order_id: uuid::Uuid,
    pub shop_id: uuid::Uuid,
    pub platform: String,
    pub platform_order_id: String,
    pub warehouse_id: uuid::Uuid,
    pub carrier_code: Option<String>,
    pub tracking_no: Option<String>,
    pub status: String,
    pub weight_gram: Option<i32>,
    pub shipped_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OmsShipmentItem {
    pub shipment_item_id: uuid::Uuid,
    pub shipment_id: uuid::Uuid,
    pub order_item_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub qty: i64,
}

#[derive(Debug, Clone)]
pub struct ShipmentCancelRequest {
    pub cancel_request_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub shipment_id: uuid::Uuid,
    pub order_id: uuid::Uuid,
    pub status: String,
    pub reason: Option<String>,
    pub requested_by: Option<uuid::Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OmsShipmentListFilters {
    pub order_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub warehouse_id: Option<uuid::Uuid>,
    pub shipment_types: Vec<String>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone)]
pub struct OmsShipmentListResult {
    pub items: Vec<OmsShipment>,
    pub total: i64,
}

#[derive(Debug, Clone)]
pub struct AllocationShortage {
    pub order_item_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub requested: i64,
    pub available: i64,
}

#[derive(Debug, Clone)]
pub struct AllocationResult {
    pub shipments: Vec<OmsShipment>,
    pub shortages: Vec<AllocationShortage>,
}

#[derive(Debug, Clone)]
pub struct ReleaseResult {
    pub released: i64,
}
