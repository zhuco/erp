use chrono::{DateTime, NaiveDate, Utc};
use erp_types::{
    OmsOrderStatus, OrderExceptionStatus, OrderFulfillmentStatus, OrderId, OrderMainStatus,
    OrderPaymentStatus, OrderReviewStatus, OrderStatus, Platform, ShopId, UserId,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct OrderShippingInfo {
    pub status: OrderStatus,
    pub platform: Platform,
    pub platform_order_id: String,
    pub shop_id: ShopId,
    pub order_month: NaiveDate,
}

#[derive(Debug, Clone)]
pub struct OrderItemLine {
    pub line_id: String,
    pub sku_id: Option<String>,
    pub qty: i64,
}

#[derive(Debug, Clone)]
pub struct OrderItemStockPolicy {
    pub line_id: String,
    pub sku_id: Option<String>,
    pub sku_exists: bool,
    pub allow_negative_stock: bool,
}

#[derive(Debug, Clone)]
pub struct OrderReviewCheck {
    pub logistics_company: Option<String>,
    pub tracking_no: Option<String>,
    pub inventory_shortage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmsOrderSummary {
    pub id: OrderId,
    pub platform: Platform,
    pub shop_id: ShopId,
    pub platform_order_id: String,
    pub platform_status: OrderStatus,
    pub oms_status: OmsOrderStatus,
    pub order_status: OrderMainStatus,
    pub review_status: OrderReviewStatus,
    pub fulfillment_status: OrderFulfillmentStatus,
    pub exception_status: OrderExceptionStatus,
    pub payment_status: Option<OrderPaymentStatus>,
    pub total_amount_cent: i64,
    pub receiver_name: Option<String>,
    pub receiver_phone: Option<String>,
    pub receiver_phone_last4: Option<String>,
    pub customer_id: Option<uuid::Uuid>,
    pub customer_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub exception_flags: Vec<String>,
    pub mapping_missing: bool,
    pub inventory_shortage: bool,
    pub shipping_failed: bool,
    pub tracking_no: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct OmsOrderListFilters {
    pub platform: Option<Platform>,
    pub platforms: Option<Vec<Platform>>,
    pub shop_id: Option<ShopId>,
    pub shop_ids: Option<Vec<ShopId>>,
    pub shop_ids_filter: Option<Vec<ShopId>>,
    pub oms_status: Option<OmsOrderStatus>,
    pub oms_statuses: Option<Vec<OmsOrderStatus>>,
    pub order_status: Option<OrderMainStatus>,
    pub review_status: Option<OrderReviewStatus>,
    pub fulfillment_status: Option<OrderFulfillmentStatus>,
    pub exception_status: Option<OrderExceptionStatus>,
    pub payment_status: Option<OrderPaymentStatus>,
    pub exception_flags: Vec<String>,
    pub tags: Vec<String>,
    pub tagged: Option<bool>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub updated_from: Option<DateTime<Utc>>,
    pub updated_to: Option<DateTime<Utc>>,
    pub ship_warehouse_id: Option<uuid::Uuid>,
    pub ship_warehouse_ids: Option<Vec<uuid::Uuid>>,
    pub platform_order_id: Option<String>,
    pub item_keyword: Option<String>,
    pub total_amount_cent: Option<i64>,
    pub total_amount_min_cent: Option<i64>,
    pub total_amount_max_cent: Option<i64>,
    pub goods_qty: Option<i64>,
    pub customer_account: Option<String>,
    pub receiver_name: Option<String>,
    pub receiver_phone: Option<String>,
    pub receiver_address: Option<String>,
    pub remark: Option<String>,
    pub order_source: Option<String>,
    pub logistics_company: Option<String>,
    pub estimated_postage_cent: Option<i64>,
    pub estimated_profit_cent: Option<i64>,
    pub estimated_profit_rate: Option<f64>,
    pub keyword: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmsOrderListResult {
    pub items: Vec<OmsOrderSummary>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmsOrderDetail {
    pub order: OmsOrder,
    pub items: Vec<OmsOrderItem>,
    pub review_hits: Vec<OrderReviewHit>,
    pub tags: Vec<OrderTag>,
    pub notes: Vec<OrderNote>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmsOrder {
    pub id: OrderId,
    pub platform: Platform,
    pub shop_id: ShopId,
    pub platform_order_id: String,
    pub order_type: Option<String>,
    pub sales_channel: Option<String>,
    pub order_source: Option<String>,
    pub salesperson: Option<String>,
    pub platform_status: OrderStatus,
    pub oms_status: OmsOrderStatus,
    pub order_status: OrderMainStatus,
    pub review_status: OrderReviewStatus,
    pub fulfillment_status: OrderFulfillmentStatus,
    pub exception_status: OrderExceptionStatus,
    pub exception_flags: Vec<String>,
    pub total_amount_cent: i64,
    pub goods_amount_cent: Option<i64>,
    pub postage_amount_cent: Option<i64>,
    pub discount_amount_cent: Option<i64>,
    pub receiver_name: Option<String>,
    pub receiver_phone: Option<String>,
    pub receiver_tel: Option<String>,
    pub receiver_address: Option<String>,
    pub customer_id: Option<uuid::Uuid>,
    pub customer_account: Option<String>,
    pub customer_name: Option<String>,
    pub customer_status: Option<String>,
    pub logistics_company: Option<String>,
    pub tracking_no: Option<String>,
    pub shipment_no: Option<String>,
    pub ship_warehouse_id: Option<uuid::Uuid>,
    pub ship_warehouse_name: Option<String>,
    pub estimated_weight_gram: Option<i32>,
    pub package_weight_gram: Option<i32>,
    pub paid_amount_cent: Option<i64>,
    pub subsidy_amount_cent: Option<i64>,
    pub receivable_amount_cent: Option<i64>,
    pub other_fee_cent: Option<i64>,
    pub estimated_postage_cent: Option<i64>,
    pub invoice_type: Option<String>,
    pub invoice_title: Option<String>,
    pub payment_status: Option<OrderPaymentStatus>,
    pub pay_at: Option<DateTime<Utc>>,
    pub ship_at: Option<DateTime<Utc>>,
    pub buyer_message: Option<String>,
    pub seller_remark: Option<String>,
    pub freeze_reason: Option<String>,
    pub cancel_reason: Option<String>,
    pub exception_reason: Option<String>,
    pub extra_remark: Option<String>,
    pub order_created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmsOrderItem {
    pub id: String,
    pub platform_sku_id: Option<String>,
    pub outer_sku_id: Option<String>,
    pub title: Option<String>,
    pub quantity: i32,
    pub price_cent: i64,
    pub mapping_missing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderReviewHit {
    pub rule_id: uuid::Uuid,
    pub rule_name: String,
    pub action: String,
    pub hit: bool,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderTag {
    pub tag: String,
    pub created_by: Option<UserId>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderNote {
    pub note_id: uuid::Uuid,
    pub note: String,
    pub created_by: Option<UserId>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OrderEditItemInput {
    pub line_id: Option<String>,
    pub sku_id: Option<String>,
    pub title: Option<String>,
    pub qty: i32,
    pub price_cent: i64,
}

#[derive(Debug, Clone)]
pub struct OrderItemEditInput {
    pub line_id: String,
    pub qty: Option<i32>,
    pub price_cent: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct OrderEditInput {
    /// Required when replacing `items` or applying `item_edits`; optional for scalar-only edits.
    pub expected_version: Option<i32>,
    pub receiver_name: Option<String>,
    pub receiver_phone: Option<String>,
    pub receiver_tel: Option<String>,
    pub receiver_address: Option<String>,
    pub logistics_company: Option<String>,
    pub tracking_no: Option<String>,
    pub order_type: Option<String>,
    pub sales_channel: Option<String>,
    pub order_source: Option<String>,
    pub salesperson: Option<String>,
    pub ship_warehouse_id: Option<uuid::Uuid>,
    pub shipment_no: Option<String>,
    pub estimated_weight_gram: Option<i32>,
    pub package_weight_gram: Option<i32>,
    pub customer_account: Option<String>,
    pub paid_amount_cent: Option<i64>,
    pub subsidy_amount_cent: Option<i64>,
    pub receivable_amount_cent: Option<i64>,
    pub other_fee_cent: Option<i64>,
    pub estimated_postage_cent: Option<i64>,
    pub invoice_type: Option<String>,
    pub invoice_title: Option<String>,
    pub payment_status: Option<String>,
    pub buyer_message: Option<String>,
    pub seller_remark: Option<String>,
    pub freeze_reason: Option<String>,
    pub cancel_reason: Option<String>,
    pub exception_reason: Option<String>,
    pub extra_remark: Option<String>,
    pub items: Option<Vec<OrderEditItemInput>>,
    pub item_edits: Option<Vec<OrderItemEditInput>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderEditOutcome {
    Updated,
    NotFound,
    MissingExpectedVersion,
    VersionConflict,
    InvalidItemEdits,
}

#[derive(Debug, Clone)]
pub struct OrderEditState {
    pub order_month: NaiveDate,
    pub shop_id: ShopId,
    pub status: OrderStatus,
    pub oms_status: OmsOrderStatus,
    pub order_status: OrderMainStatus,
    pub review_status: OrderReviewStatus,
    pub version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderReviewState {
    pub oms_status: OmsOrderStatus,
    pub order_status: OrderMainStatus,
    pub review_status: OrderReviewStatus,
    pub fulfillment_status: OrderFulfillmentStatus,
    pub exception_status: OrderExceptionStatus,
    pub exception_flags: Vec<String>,
    pub version: i32,
}

#[derive(Debug, Clone)]
pub struct OrderReviewUpdate {
    pub oms_status: OmsOrderStatus,
    pub order_status: OrderMainStatus,
    pub review_status: OrderReviewStatus,
    pub fulfillment_status: OrderFulfillmentStatus,
    pub exception_status: OrderExceptionStatus,
    pub exception_flags: Vec<String>,
    pub expected_version: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct OrderReceiverUpdate {
    pub receiver_name: String,
    pub receiver_phone: String,
    pub receiver_address: String,
}

#[derive(Debug, Clone)]
pub struct OrderTagCreate {
    pub tag: String,
    pub created_by: Option<UserId>,
}

#[derive(Debug, Clone)]
pub struct OrderNoteCreate {
    pub note: String,
    pub created_by: Option<UserId>,
}

#[derive(Debug, Clone)]
pub struct OrderSplitPlanItem {
    pub line_id: String,
    pub qty: i32,
}

#[derive(Debug, Clone)]
pub struct OrderSplitPlanChild {
    pub items: Vec<OrderSplitPlanItem>,
}

#[derive(Debug, Clone)]
pub struct OrderSplitResultItem {
    pub line_id: String,
    pub sku_id: Option<String>,
    pub title: Option<String>,
    pub qty: i32,
    pub price_cent: i64,
}

#[derive(Debug, Clone)]
pub struct OrderSplitResultChild {
    pub order_id: OrderId,
    pub items: Vec<OrderSplitResultItem>,
}

#[derive(Debug, Clone)]
pub struct OrderSplitResult {
    pub source_order_id: OrderId,
    pub child_orders: Vec<OrderSplitResultChild>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderExceptionTicket {
    pub ticket_id: uuid::Uuid,
    pub order_id: OrderId,
    pub reason_code: String,
    pub status: String,
    pub handler_id: Option<UserId>,
    pub handled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OrderExceptionTicketUpdate {
    pub reason_code: String,
    pub status: String,
    pub handler_id: Option<UserId>,
    pub handled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct OrderExceptionReasonStat {
    pub reason_code: String,
    pub total: i64,
    pub open: i64,
    pub processed: i64,
}

#[derive(Debug, Clone)]
pub struct OrderExceptionStats {
    pub total: i64,
    pub open: i64,
    pub processed: i64,
    pub reasons: Vec<OrderExceptionReasonStat>,
}
