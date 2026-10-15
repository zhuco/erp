use chrono::{DateTime, NaiveDate, Utc};
use erp_types::{
    AfterSaleCaseId, AfterSaleState, AfterSalesType, OrderId, OrderStatus, Platform, RefundChannel,
    RefundState, ShopId, TenantId,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(test)]
mod contract_tests;

pub mod access;
pub mod aggregate;
pub mod accounting;
pub mod analytics;
pub mod billing;
pub mod company;
pub mod connector;
pub mod draft;
pub mod edit_save;
pub mod exit;
pub mod file_exchange;
pub mod grid_query;
pub mod group;
pub mod manufacturing;
pub mod oms;
pub mod onboarding;
pub mod platform;
pub mod quantity;
pub mod receipt;
pub mod search;
pub mod security;
pub mod service;
pub mod support;
pub mod trade;
pub mod usage;
pub mod workbench;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItemProjection {
    pub line_id: String,
    pub sku_id: Option<String>,
    pub title: Option<String>,
    pub qty: i32,
    pub price_cent: i64,
    pub amount_cent: i64,
    /// Line-level pay amount (optional; some platforms only provide order-level amount).
    pub pay_amount_cent: Option<i64>,
    /// Line-level discount amount (optional).
    pub discount_cent: Option<i64>,
    /// Sub-order refund state.
    pub refund_state: RefundState,
    /// Sub-order after-sale state (exchange/reship/repair, etc.).
    pub after_sale_state: AfterSaleState,
    /// Sub-order refund amount (if available).
    pub refund_amount_cent: Option<i64>,
    /// Platform-specific after-sale type code/desc.
    pub after_sale_type: Option<String>,
    /// Platform-specific after-sale/refund case id.
    pub after_sale_id: Option<String>,
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderProjection {
    pub tenant_id: TenantId,
    pub shop_id: ShopId,
    pub platform: Platform,
    pub platform_order_id: String,
    pub status: OrderStatus,
    pub total_amount_cent: i64,
    /// Sum of goods amount (optional).
    pub goods_amount_cent: Option<i64>,
    /// Shipping/postage amount (optional).
    pub postage_amount_cent: Option<i64>,
    /// Total discount amount (optional).
    pub discount_amount_cent: Option<i64>,
    /// Platform discount portion (optional).
    pub platform_discount_cent: Option<i64>,
    /// Seller/shop discount portion (optional).
    pub seller_discount_cent: Option<i64>,
    /// Coupon discount portion (optional).
    pub coupon_discount_cent: Option<i64>,
    pub currency: String,
    pub buyer_name: Option<String>,
    pub receiver_name: Option<String>,
    pub receiver_phone: Option<String>,
    pub receiver_address: Option<String>,
    /// Buyer paid time (optional).
    pub pay_at: Option<DateTime<Utc>>,
    /// Seller shipped time (optional).
    pub ship_at: Option<DateTime<Utc>>,
    /// Order finished time (optional).
    pub finish_at: Option<DateTime<Utc>>,
    /// Order cancelled time (optional).
    pub cancel_at: Option<DateTime<Utc>>,
    /// Buyer message / remarks (optional).
    pub buyer_message: Option<String>,
    /// Seller remark / memo (optional).
    pub seller_remark: Option<String>,
    /// Logistics company (optional).
    pub logistics_company: Option<String>,
    /// Tracking number (optional; if multi-package, store the first or joined string).
    pub tracking_no: Option<String>,
    pub order_created_at: DateTime<Utc>,
    pub platform_updated_at: Option<DateTime<Utc>>,
    pub raw: Value,
    pub items: Vec<OrderItemProjection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AfterSaleCase {
    pub tenant_id: TenantId,
    pub case_id: AfterSaleCaseId,
    pub platform: Platform,
    pub shop_id: ShopId,
    pub after_sale_id: String,
    pub after_sale_type: Option<String>,
    pub refund_state: RefundState,
    pub after_sale_state: AfterSaleState,
    pub refund_amount_cent: i64,
    pub item_count: i32,
    pub total_qty: i32,
    pub order_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AfterSaleCaseItem {
    pub tenant_id: TenantId,
    pub case_id: AfterSaleCaseId,
    pub order_id: OrderId,
    pub order_month: NaiveDate,
    pub line_id: String,
    pub sku_id: Option<String>,
    pub title: Option<String>,
    pub qty: i32,
    pub refund_state: RefundState,
    pub after_sale_state: AfterSaleState,
    pub refund_amount_cent: Option<i64>,
    pub after_sale_type: Option<String>,
    pub return_stocked: bool,
    pub return_warehouse_id: Option<uuid::Uuid>,
    pub return_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AfterSaleCaseDetail {
    pub case: AfterSaleCase,
    pub items: Vec<AfterSaleCaseItem>,
}

fn refund_state_rank(state: RefundState) -> i32 {
    match state {
        RefundState::None => 0,
        RefundState::Unknown => 1,
        RefundState::Closed => 2,
        RefundState::Processing => 3,
        RefundState::Success => 4,
    }
}

fn refund_state_from_rank(rank: i32) -> RefundState {
    match rank {
        4 => RefundState::Success,
        3 => RefundState::Processing,
        2 => RefundState::Closed,
        1 => RefundState::Unknown,
        _ => RefundState::None,
    }
}

fn after_sale_state_rank(state: AfterSaleState) -> i32 {
    match state {
        AfterSaleState::None => 0,
        AfterSaleState::Unknown => 1,
        AfterSaleState::Closed => 2,
        AfterSaleState::Processing => 3,
        AfterSaleState::Success => 4,
    }
}

fn after_sale_state_from_rank(rank: i32) -> AfterSaleState {
    match rank {
        4 => AfterSaleState::Success,
        3 => AfterSaleState::Processing,
        2 => AfterSaleState::Closed,
        1 => AfterSaleState::Unknown,
        _ => AfterSaleState::None,
    }
}

pub fn aggregate_refund_state<I>(states: I) -> RefundState
where
    I: IntoIterator<Item = RefundState>,
{
    let mut max_rank = 0;
    for state in states {
        max_rank = max_rank.max(refund_state_rank(state));
    }
    refund_state_from_rank(max_rank)
}

pub fn aggregate_after_sale_state<I>(states: I) -> AfterSaleState
where
    I: IntoIterator<Item = AfterSaleState>,
{
    let mut max_rank = 0;
    for state in states {
        max_rank = max_rank.max(after_sale_state_rank(state));
    }
    after_sale_state_from_rank(max_rank)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxReceivedPayload {
    pub tenant_id: TenantId,
    pub platform: Platform,
    pub shop_id: ShopId,
    pub inbox_event_id: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderUpsertedPayload {
    pub tenant_id: TenantId,
    pub platform: Platform,
    pub shop_id: ShopId,
    pub platform_order_id: String,
    pub order_id: OrderId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderReviewedPayload {
    pub tenant_id: TenantId,
    pub order_id: OrderId,
    pub shop_id: ShopId,
    pub platform: Platform,
    pub review_status: String,
    pub oms_status: String,
    pub exception_flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipmentRequestedPayload {
    pub tenant_id: TenantId,
    pub shipment_id: uuid::Uuid,
    pub order_id: OrderId,
    pub shop_id: ShopId,
    pub platform: Platform,
    pub platform_order_id: String,
    pub warehouse_id: uuid::Uuid,
    pub carrier_code: Option<String>,
    pub tracking_no: Option<String>,
    pub weight_gram: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipmentFailedPayload {
    pub tenant_id: TenantId,
    pub shipment_id: uuid::Uuid,
    pub order_id: OrderId,
    pub shop_id: ShopId,
    pub platform: Platform,
    pub platform_order_id: String,
    pub warehouse_id: uuid::Uuid,
    pub carrier_code: Option<String>,
    pub tracking_no: Option<String>,
    pub last_error: String,
}

pub const DLQ_REPLAY_REQUESTED_EVENT: &str = "DLQ_REPLAY_REQUESTED";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetterReplayPayload {
    pub tenant_id: TenantId,
    pub dead_letter_id: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitComputeRequestedPayload {
    pub tenant_id: TenantId,
    pub order_id: OrderId,
}

pub const PROFIT_COMPUTE_REQUESTED_EVENT: &str = "PROFIT_COMPUTE_REQUESTED";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryChangedPayload {
    pub tenant_id: TenantId,
    pub sku_id: uuid::Uuid,
    pub warehouse_id: uuid::Uuid,
    pub on_hand: i64,
    pub reserved: i64,
    pub frozen: i64,
    pub in_transit: i64,
    pub available: i64,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipmentShippedPayload {
    pub tenant_id: TenantId,
    pub shipment_id: uuid::Uuid,
    pub order_id: OrderId,
    pub shop_id: ShopId,
    pub platform: Platform,
    pub platform_order_id: String,
    pub warehouse_id: uuid::Uuid,
    pub carrier_code: Option<String>,
    pub tracking_no: Option<String>,
    pub shipped_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundRequestedPayload {
    pub tenant_id: TenantId,
    pub after_sales_id: uuid::Uuid,
    pub refund_doc_id: uuid::Uuid,
    pub amount_cent: i64,
    pub channel: RefundChannel,
    pub after_sales_type: AfterSalesType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRefreshFailedPayload {
    pub tenant_id: TenantId,
    pub shop_id: ShopId,
    pub platform: Platform,
    pub last_error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRefreshRequestedPayload {
    pub tenant_id: TenantId,
    pub shop_id: ShopId,
    pub platform: Platform,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregate_refund_state_prefers_success() {
        let states = vec![
            RefundState::Processing,
            RefundState::Success,
            RefundState::Closed,
        ];
        assert_eq!(aggregate_refund_state(states), RefundState::Success);
    }

    #[test]
    fn aggregate_refund_state_handles_unknown_only() {
        let states = vec![RefundState::Unknown];
        assert_eq!(aggregate_refund_state(states), RefundState::Unknown);
    }

    #[test]
    fn aggregate_after_sale_state_prefers_processing_over_closed() {
        let states = vec![AfterSaleState::Closed, AfterSaleState::Processing];
        assert_eq!(
            aggregate_after_sale_state(states),
            AfterSaleState::Processing
        );
    }

    #[test]
    fn aggregate_after_sale_state_defaults_none() {
        let states: Vec<AfterSaleState> = vec![];
        assert_eq!(aggregate_after_sale_state(states), AfterSaleState::None);
    }
}
