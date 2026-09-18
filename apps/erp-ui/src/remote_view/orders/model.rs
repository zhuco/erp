use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub(super) struct OrderPage {
    pub(super) items: Vec<OrderSummary>,
    pub(super) total: i64,
}

#[derive(Deserialize)]
pub(super) struct OrderSummary {
    pub(super) id: String,
    pub(super) platform: String,
    pub(super) platform_order_id: String,
    pub(super) oms_status: String,
    pub(super) total_amount_cent: i64,
    pub(super) shop_id: String,
    pub(super) order_status: String,
    pub(super) review_status: String,
    pub(super) fulfillment_status: String,
    pub(super) exception_status: String,
    pub(super) payment_status: Option<String>,
    pub(super) receiver_name: Option<String>,
    pub(super) receiver_phone: Option<String>,
    pub(super) customer_name: Option<String>,
    pub(super) created_at: String,
    pub(super) updated_at: String,
    pub(super) exception_flags: Vec<String>,
    pub(super) tracking_no: Option<String>,
}

#[derive(Clone, Deserialize)]
pub(super) struct OrderItem {
    pub(super) id: String,
    pub(super) platform_sku_id: Option<String>,
    pub(super) outer_sku_id: Option<String>,
    pub(super) title: Option<String>,
    pub(super) quantity: i32,
    pub(super) price_cent: i64,
}

#[derive(Deserialize)]
pub(super) struct OrderHead {
    pub(super) id: String,
    pub(super) version: i32,
    pub(super) oms_status: String,
    pub(super) review_status: String,
}

#[derive(Deserialize)]
pub(super) struct OrderDetail {
    pub(super) order: OrderHead,
    pub(super) items: Vec<OrderItem>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(super) struct EditBody<'a> {
    pub(super) expected_version: i32,
    pub(super) item_edits: [ItemEdit<'a>; 1],
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(super) struct ItemEdit<'a> {
    pub(super) line_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) qty: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) price_cent: Option<i64>,
}
