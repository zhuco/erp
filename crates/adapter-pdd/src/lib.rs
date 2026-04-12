#![recursion_limit = "256"]

use async_trait::async_trait;
use chrono::Utc;
use erp_adapter::{
    AdapterError, OrderAdapter, PlatformAdapter, PlatformTokenPayload, PullOrdersResult,
    PullProductsResult, PullRefundsResult, ShipOrderRequest, ShipOrderResponse, ShopAuthContext,
    SyncCursor, TokenRefreshResult, WebhookAdapter,
};
use erp_common::{
    crypto::md5_hex_upper,
    http::{parse_form_urlencoded, parse_json},
    time::parse_platform_time,
};
use erp_domain::{OrderItemProjection, OrderProjection};
use erp_types::{AfterSaleState, OrderStatus, Platform, RefundState};
use serde_json::Value;
use std::collections::BTreeMap;

/// PDD order status codes (发货状态).
///
/// From `pdd.order.information.get` / `pdd.order.basic.list.get` docs:
/// - 1: 待发货
/// - 2: 已发货待签收
/// - 3: 已签收
#[allow(dead_code)]
const PDD_ORDER_STATUS_CODES: &[i64] = &[1, 2, 3];

/// PDD confirm status codes (成交状态).
/// - 0: 未成交
/// - 1: 已成交
/// - 2: 已取消
#[allow(dead_code)]
const PDD_CONFIRM_STATUS_CODES: &[i64] = &[0, 1, 2];

/// PDD refund status codes (退款状态).
/// - 1: 无售后或售后关闭
/// - 2: 售后处理中
/// - 3: 退款中
/// - 4: 退款成功
#[allow(dead_code)]
const PDD_REFUND_STATUS_CODES: &[i64] = &[1, 2, 3, 4];

/// PDD after-sales status codes (售后状态).
///
/// NOTE: This list is extracted from official docs enum list; we use it for test coverage.
#[allow(dead_code)]
const PDD_AFTER_SALES_STATUS_CODES: &[i64] = &[
    0, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 21, 22, 24, 25, 27, 31, 32, 33,
];

#[derive(Clone)]
pub struct Adapter {
    client: reqwest::Client,
}

impl Adapter {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .expect("reqwest client");
        Self { client }
    }

    fn creds_client_id(shop: &ShopAuthContext) -> Result<String, AdapterError> {
        shop.credentials
            .get("client_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AdapterError::Auth("missing credentials.client_id".into()))
    }

    fn creds_client_secret(shop: &ShopAuthContext) -> Result<String, AdapterError> {
        shop.credentials
            .get("client_secret")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AdapterError::Auth("missing credentials.client_secret".into()))
    }

    fn creds_access_token(shop: &ShopAuthContext) -> Option<String> {
        shop.credentials
            .get("access_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    fn gateway_url(shop: &ShopAuthContext) -> String {
        shop.credentials
            .get("gateway_url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            // Common PDD router endpoint.
            .unwrap_or_else(|| "https://gw-api.pinduoduo.com/api/router".to_string())
    }

    fn order_detail_type(shop: &ShopAuthContext) -> String {
        shop.credentials
            .get("order_detail_type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "pdd.order.information.get".to_string())
    }

    fn pdd_sign(params: &BTreeMap<String, String>, client_secret: &str) -> String {
        // Official style: secret + (k1v1k2v2...) + secret, MD5, UPPERCASE.
        let mut to_sign = String::new();
        to_sign.push_str(client_secret);
        for (k, v) in params {
            if k == "sign" {
                continue;
            }
            if v.is_empty() {
                continue;
            }
            to_sign.push_str(k);
            to_sign.push_str(v);
        }
        to_sign.push_str(client_secret);
        md5_hex_upper(to_sign.as_bytes())
    }

    /// PDD amounts are documented in **yuan** (DOUBLE) for order APIs.
    /// We store money as cents (fen) in `OrderProjection`.
    fn parse_pdd_yuan_to_cent(v: &Value) -> Option<i64> {
        match v {
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    // treat as integer yuan
                    return Some(i.saturating_mul(100));
                }
                let f = n.as_f64()?;
                Some((f * 100.0).round() as i64)
            }
            Value::String(s) => {
                let s = s.trim();
                if s.is_empty() {
                    return None;
                }
                if s.contains('.') {
                    yuan_str_to_cent(s)
                } else {
                    // integer yuan
                    s.parse::<i64>().ok().map(|i| i.saturating_mul(100))
                }
            }
            _ => None,
        }
    }

    fn after_sales_override_status(code: i64) -> Option<OrderStatus> {
        // Official docs list includes exchange/repair statuses too; we only map those to ON_HOLD.
        // Refund-related: map to REFUNDING / REFUNDED.
        match code {
            0 => None,
            10 => Some(OrderStatus::Refunded),

            // refund / dispute in progress
            2 | 3 | 4 | 5 | 6 | 7 | 8 | 31 | 33 => Some(OrderStatus::Refunding),

            // exchange/resend in progress
            14 | 15 | 18 | 32 => Some(OrderStatus::OnHold),

            // repair in progress
            21 | 22 | 27 => Some(OrderStatus::OnHold),

            // closed / success / failure / cancelled
            9 | 11 | 12 | 13 | 16 | 17 | 24 | 25 => None,

            _ => None,
        }
    }

    fn map_order_status(
        confirm_status: Option<i64>,
        order_status: Option<i64>,
        refund_status: Option<i64>,
        after_sales_status: Option<i64>,
        risk_control_status: Option<i64>,
    ) -> OrderStatus {
        // Precedence rules (pragmatic for ERP ops):
        // 1) Refund success/in-progress overrides.
        // 2) On-hold (risk-control / non-refund after-sale).
        // 3) Cancelled.
        // 4) Payment/shipping states.

        if refund_status == Some(4) || after_sales_status == Some(10) {
            return OrderStatus::Refunded;
        }
        if matches!(refund_status, Some(2 | 3)) {
            return OrderStatus::Refunding;
        }
        if let Some(as_code) = after_sales_status {
            if let Some(ov) = Self::after_sales_override_status(as_code) {
                // Avoid overriding refund_status==1 (no after-sale) with None.
                return ov;
            }
        }
        if risk_control_status == Some(1) {
            return OrderStatus::OnHold;
        }
        if confirm_status == Some(2) {
            return OrderStatus::Cancelled;
        }
        if confirm_status == Some(0) {
            return OrderStatus::PendingPayment;
        }

        match order_status {
            Some(1) => OrderStatus::PendingShipment,
            Some(2) => OrderStatus::Shipped,
            Some(3) => OrderStatus::Completed,
            _ => OrderStatus::Unknown,
        }
    }

    fn map_refund_state(
        refund_status: Option<i64>,
        after_sales_status: Option<i64>,
    ) -> RefundState {
        // Prefer refund_status (coarse) + a few well-known after_sales_status overrides.
        if refund_status == Some(4) || after_sales_status == Some(10) {
            return RefundState::Success;
        }

        if matches!(refund_status, Some(2 | 3)) {
            return RefundState::Processing;
        }

        // After-sales status overrides (refund-related)
        if matches!(
            after_sales_status,
            Some(2 | 3 | 4 | 5 | 6 | 7 | 8 | 31 | 33)
        ) {
            return RefundState::Processing;
        }

        // Closed/failure/cancelled-like codes
        if matches!(
            after_sales_status,
            Some(9 | 11 | 12 | 13 | 16 | 17 | 24 | 25)
        ) {
            return RefundState::Closed;
        }

        match refund_status {
            Some(1) | None => RefundState::None,
            _ => RefundState::Unknown,
        }
    }

    fn map_after_sale_state(after_sales_status: Option<i64>) -> AfterSaleState {
        match after_sales_status {
            None | Some(0) => AfterSaleState::None,
            Some(10) => AfterSaleState::Success,
            // any after-sales code means "some after-sale exists"; detailed meaning stays in raw.
            Some(9 | 11 | 12 | 13 | 16 | 17 | 24 | 25) => AfterSaleState::Closed,
            Some(_) => AfterSaleState::Processing,
        }
    }

    fn find_first_order_object(v: &Value, depth: usize) -> Option<&Value> {
        if depth == 0 {
            return None;
        }
        if let Some(obj) = v.as_object() {
            if obj.contains_key("order_sn") {
                return Some(v);
            }
            for (_k, child) in obj {
                if let Some(found) = Self::find_first_order_object(child, depth - 1) {
                    return Some(found);
                }
            }
        } else if let Some(arr) = v.as_array() {
            for child in arr {
                if let Some(found) = Self::find_first_order_object(child, depth - 1) {
                    return Some(found);
                }
            }
        }
        None
    }

    fn map_order_to_projection(
        shop: &ShopAuthContext,
        order: &Value,
    ) -> Result<OrderProjection, AdapterError> {
        let platform_order_id = order
            .get("order_sn")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AdapterError::BadRequest("missing order_sn".into()))?;

        let confirm_status = order.get("confirm_status").and_then(|v| v.as_i64());
        let order_status_code = order.get("order_status").and_then(|v| v.as_i64());
        let refund_status = order.get("refund_status").and_then(|v| v.as_i64());
        let after_sales_status = order.get("after_sales_status").and_then(|v| v.as_i64());
        let risk_control_status = order.get("risk_control_status").and_then(|v| v.as_i64());

        let status = Self::map_order_status(
            confirm_status,
            order_status_code,
            refund_status,
            after_sales_status,
            risk_control_status,
        );

        let order_created_at = order
            .get("created_time")
            .or_else(|| order.get("order_create_time"))
            .and_then(parse_platform_time)
            .unwrap_or_else(Utc::now);

        let platform_updated_at = order
            .get("updated_at")
            .or_else(|| order.get("updated_time"))
            .or_else(|| order.get("update_time"))
            .and_then(parse_platform_time);

        let total_amount_cent = order
            .get("pay_amount")
            .or_else(|| order.get("order_amount"))
            .or_else(|| order.get("order_pay_amount"))
            .and_then(Self::parse_pdd_yuan_to_cent)
            .unwrap_or(0);

        let goods_amount_cent = order
            .get("goods_amount")
            .or_else(|| order.get("goods_amount_total"))
            .or_else(|| order.get("goods_sum_amount"))
            .and_then(Self::parse_pdd_yuan_to_cent);

        let postage_amount_cent = order
            .get("postage")
            .or_else(|| order.get("postage_amount"))
            .or_else(|| order.get("post_amount"))
            .or_else(|| order.get("shipping_amount"))
            .and_then(Self::parse_pdd_yuan_to_cent);

        let discount_amount_cent = order
            .get("discount_amount")
            .or_else(|| order.get("promotion_amount"))
            .or_else(|| order.get("discount_amount_total"))
            .and_then(Self::parse_pdd_yuan_to_cent);

        let platform_discount_cent = order
            .get("platform_discount")
            .or_else(|| order.get("platform_discount_amount"))
            .and_then(Self::parse_pdd_yuan_to_cent);

        let seller_discount_cent = order
            .get("seller_discount")
            .or_else(|| order.get("seller_discount_amount"))
            .or_else(|| order.get("shop_discount_amount"))
            .and_then(Self::parse_pdd_yuan_to_cent);

        let coupon_discount_cent = order
            .get("coupon_discount")
            .or_else(|| order.get("coupon_discount_amount"))
            .and_then(Self::parse_pdd_yuan_to_cent);

        let pay_at = order
            .get("pay_time")
            .or_else(|| order.get("payment_time"))
            .or_else(|| order.get("pay_time_str"))
            .and_then(parse_platform_time);

        let ship_at = order
            .get("shipping_time")
            .or_else(|| order.get("delivery_time"))
            .or_else(|| order.get("consign_time"))
            .and_then(parse_platform_time);

        let finish_at = order
            .get("confirm_time")
            .or_else(|| order.get("receive_time"))
            .or_else(|| order.get("success_time"))
            .and_then(parse_platform_time);

        let cancel_at = order
            .get("cancel_time")
            .or_else(|| order.get("canceled_time"))
            .and_then(parse_platform_time);

        let buyer_message = order
            .get("buyer_message")
            .or_else(|| order.get("buyer_memo"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let seller_remark = order
            .get("seller_remark")
            .or_else(|| order.get("seller_memo"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let logistics_company = order
            .get("shipping_company")
            .or_else(|| order.get("logistics_company"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let tracking_no = order
            .get("tracking_number")
            .or_else(|| order.get("tracking_no"))
            .or_else(|| order.get("express_no"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let buyer_name = order
            .get("buyer_name")
            .or_else(|| order.get("nickname"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // PDD encrypted receiver fields may be empty depending on order status / risk-control.
        // Prefer encrypted field if present; otherwise fall back to masked field.
        let receiver_name = order
            .get("receiver_name")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .or_else(|| {
                order
                    .get("receiver_name_mask")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
            });

        let receiver_phone = order
            .get("receiver_phone")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .or_else(|| {
                order
                    .get("receiver_phone_mask")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
            });

        let receiver_address = {
            let parts = [
                order.get("province").and_then(|v| v.as_str()),
                order.get("city").and_then(|v| v.as_str()),
                order.get("town").and_then(|v| v.as_str()),
                // prefer decrypted receiver_address; fallback to masked.
                order.get("receiver_address").and_then(|v| v.as_str()),
                order.get("receiver_address_mask").and_then(|v| v.as_str()),
                // some responses provide `address_mask` / `address` (detail-only)
                order.get("address_mask").and_then(|v| v.as_str()),
                order.get("address").and_then(|v| v.as_str()),
            ];
            let joined = parts
                .into_iter()
                .flatten()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            if joined.is_empty() {
                None
            } else {
                Some(joined)
            }
        };

        let mut items: Vec<OrderItemProjection> = Vec::new();
        let items_v = order
            .get("item_list")
            .or_else(|| order.get("goods_list"))
            .or_else(|| order.get("order_goods_list"))
            .or_else(|| order.get("goods_info"));
        if let Some(arr) = items_v.and_then(|v| v.as_array()) {
            for (idx, it) in arr.iter().enumerate() {
                let goods_id = it.get("goods_id").and_then(|v| {
                    if v.is_string() {
                        v.as_str().map(|s| s.to_string())
                    } else {
                        v.as_i64().map(|n| n.to_string())
                    }
                });

                let sku_id = it.get("sku_id").and_then(|v| {
                    if v.is_string() {
                        v.as_str().map(|s| s.to_string())
                    } else {
                        v.as_i64().map(|n| n.to_string())
                    }
                });

                let qty = it
                    .get("goods_count")
                    .or_else(|| it.get("quantity"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1) as i32;

                let price_cent = it
                    .get("goods_price")
                    .or_else(|| it.get("price"))
                    .and_then(Self::parse_pdd_yuan_to_cent)
                    .unwrap_or(0);

                // PDD item_list usually doesn't include per-line amount, compute from price*qty.
                let amount_cent = price_cent.saturating_mul(qty as i64);

                let pay_amount_cent = it
                    .get("pay_amount")
                    .or_else(|| it.get("goods_amount"))
                    .or_else(|| it.get("item_amount"))
                    .and_then(Self::parse_pdd_yuan_to_cent);

                let discount_cent = it
                    .get("discount_amount")
                    .or_else(|| it.get("promotion_amount"))
                    .and_then(Self::parse_pdd_yuan_to_cent)
                    .or_else(|| pay_amount_cent.map(|p| (amount_cent - p).max(0)));

                // Sub-order after-sale/refund info (best-effort).
                let it_refund_status = it
                    .get("refund_status")
                    .and_then(|v| v.as_i64())
                    .or(refund_status);

                let it_after_sales_status = it
                    .get("after_sales_status")
                    .and_then(|v| v.as_i64())
                    .or(after_sales_status);

                let refund_state = Self::map_refund_state(it_refund_status, it_after_sales_status);
                let after_sale_state = Self::map_after_sale_state(it_after_sales_status);

                let refund_amount_cent = it
                    .get("refund_amount")
                    .or_else(|| it.get("after_sale_amount"))
                    .or_else(|| order.get("refund_amount"))
                    .and_then(Self::parse_pdd_yuan_to_cent);

                let after_sale_type = it
                    .get("after_sales_type")
                    .or_else(|| it.get("after_sale_type"))
                    .and_then(|v| {
                        if v.is_string() {
                            v.as_str().map(|s| s.to_string())
                        } else {
                            v.as_i64().map(|n| n.to_string())
                        }
                    });

                let after_sale_id = it
                    .get("after_sales_id")
                    .or_else(|| it.get("after_sale_id"))
                    .or_else(|| it.get("after_sales_sn"))
                    .and_then(|v| {
                        if v.is_string() {
                            v.as_str().map(|s| s.to_string())
                        } else {
                            v.as_i64().map(|n| n.to_string())
                        }
                    });

                let title = it
                    .get("goods_name")
                    .or_else(|| it.get("title"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Stable line_id: prefer outer_id, else goods_id:sku_id, else fallback.
                let line_id = it
                    .get("outer_id")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .or_else(|| match (goods_id.as_ref(), sku_id.as_ref()) {
                        (Some(g), Some(s)) => Some(format!("{g}:{s}")),
                        (Some(g), None) => Some(g.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| format!("L{idx}"));

                items.push(OrderItemProjection {
                    line_id,
                    sku_id,
                    title,
                    qty,
                    price_cent,
                    amount_cent,
                    pay_amount_cent,
                    discount_cent,
                    refund_state,
                    after_sale_state,
                    refund_amount_cent,
                    after_sale_type,
                    after_sale_id,
                    raw: it.clone(),
                });
            }
        }

        Ok(OrderProjection {
            tenant_id: shop.tenant_id,
            shop_id: shop.shop_id,
            platform: shop.platform,
            platform_order_id,
            status,
            total_amount_cent,
            goods_amount_cent,
            postage_amount_cent,
            discount_amount_cent,
            platform_discount_cent,
            seller_discount_cent,
            coupon_discount_cent,
            currency: "CNY".to_string(),
            buyer_name,
            receiver_name,
            receiver_phone,
            receiver_address,
            pay_at,
            ship_at,
            finish_at,
            cancel_at,
            buyer_message,
            seller_remark,
            logistics_company,
            tracking_no,
            order_created_at,
            platform_updated_at,
            raw: order.clone(),
            items,
        })
    }

    async fn fetch_order_detail(
        &self,
        shop: &ShopAuthContext,
        order_sn: &str,
    ) -> Result<OrderProjection, AdapterError> {
        let client_id = Self::creds_client_id(shop)?;
        let client_secret = Self::creds_client_secret(shop)?;
        let access_token = Self::creds_access_token(shop)
            .ok_or_else(|| AdapterError::Auth("missing credentials.access_token".into()))?;
        let gateway = Self::gateway_url(shop);
        let api_type = Self::order_detail_type(shop);

        let timestamp = Utc::now().timestamp().to_string();

        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("type".to_string(), api_type);
        params.insert("client_id".to_string(), client_id);
        params.insert("access_token".to_string(), access_token);
        params.insert("data_type".to_string(), "JSON".to_string());
        params.insert("timestamp".to_string(), timestamp);

        // Business param
        params.insert("order_sn".to_string(), order_sn.to_string());

        let sign = Self::pdd_sign(&params, &client_secret);
        params.insert("sign".to_string(), sign);

        let resp = self
            .client
            .post(gateway)
            .form(&params)
            .send()
            .await
            .map_err(|e| AdapterError::Temporary(format!("http: {e}")))?;

        let status = resp.status();
        let body = resp
            .bytes()
            .await
            .map_err(|e| AdapterError::Temporary(format!("read: {e}")))?;
        if !status.is_success() {
            return Err(AdapterError::Temporary(format!(
                "pdd gateway status={} body={}...",
                status,
                String::from_utf8_lossy(&body)
            )));
        }

        let v: Value = serde_json::from_slice(&body)
            .map_err(|e| AdapterError::Temporary(format!("json: {e}")))?;
        if v.get("error_response").is_some() {
            return Err(AdapterError::Temporary(format!("pdd error_response: {v}")));
        }

        let order_obj = Self::find_first_order_object(&v, 12).ok_or_else(|| {
            AdapterError::Temporary("cannot find order object in response".into())
        })?;
        Self::map_order_to_projection(shop, order_obj)
    }
}

impl Default for Adapter {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert `12.34` yuan string to `1234` cents.
fn yuan_str_to_cent(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let neg = s.starts_with('-');
    let s = if neg { &s[1..] } else { s };
    let mut it = s.split('.');
    let int_part = it.next().unwrap_or("0");
    let frac_part = it.next().unwrap_or("");
    if it.next().is_some() {
        return None;
    }
    let int_val: i64 = int_part.parse().ok()?;
    let frac = match frac_part.len() {
        0 => 0,
        1 => frac_part.parse::<i64>().ok()? * 10,
        _ => frac_part[..2].parse::<i64>().ok()?,
    };
    let mut cent = int_val * 100 + frac;
    if neg {
        cent = -cent;
    }
    Some(cent)
}

#[async_trait]
impl WebhookAdapter for Adapter {
    fn platform(&self) -> Platform {
        Platform::Pdd
    }

    async fn verify_webhook(
        &self,
        shop: &ShopAuthContext,
        _headers: &Value,
        body: &[u8],
    ) -> Result<(), AdapterError> {
        // PDD callbacks are usually form-urlencoded. If `sign` exists, verify using the router signing rule.
        let params = parse_form_urlencoded(body);
        let Some(got_sign) = params.get("sign") else {
            return Ok(());
        };
        let secret = Self::creds_client_secret(shop)?;
        let computed = Self::pdd_sign(&params, &secret);
        if !computed.eq_ignore_ascii_case(got_sign) {
            return Err(AdapterError::Auth("invalid pdd signature".into()));
        }
        Ok(())
    }

    async fn parse_webhook_order(
        &self,
        shop: &ShopAuthContext,
        headers: &Value,
        body: &[u8],
    ) -> Result<Option<OrderProjection>, AdapterError> {
        let mut orders =
            erp_adapter::WebhookAdapter::parse_webhook_orders(self, shop, headers, body).await?;
        Ok(orders.pop())
    }

    async fn parse_webhook_orders(
        &self,
        shop: &ShopAuthContext,
        _headers: &Value,
        body: &[u8],
    ) -> Result<Vec<OrderProjection>, AdapterError> {
        // 1) direct JSON snapshot
        if let Some(v) = parse_json(body) {
            if let Some(order_obj) = Self::find_first_order_object(&v, 12) {
                return Ok(vec![Self::map_order_to_projection(shop, order_obj)?]);
            }
        }

        // 2) form-urlencoded - typically contains order_sn only
        let params = parse_form_urlencoded(body);
        if let Some(order_sn) = params.get("order_sn") {
            let o = self.fetch_order_detail(shop, order_sn).await?;
            return Ok(vec![o]);
        }

        Ok(vec![])
    }
}

#[async_trait]
impl OrderAdapter for Adapter {
    fn platform(&self) -> Platform {
        Platform::Pdd
    }

    async fn fetch_order(
        &self,
        shop: &ShopAuthContext,
        platform_order_id: &str,
    ) -> Result<OrderProjection, AdapterError> {
        self.fetch_order_detail(shop, platform_order_id).await
    }
}

#[async_trait]
impl PlatformAdapter for Adapter {
    fn platform(&self) -> Platform {
        Platform::Pdd
    }

    async fn verify_signature(
        &self,
        shop: &ShopAuthContext,
        headers: &Value,
        body: &[u8],
    ) -> Result<(), AdapterError> {
        self.verify_webhook(shop, headers, body).await
    }

    async fn parse_webhook_orders(
        &self,
        shop: &ShopAuthContext,
        headers: &Value,
        body: &[u8],
    ) -> Result<Vec<OrderProjection>, AdapterError> {
        WebhookAdapter::parse_webhook_orders(self, shop, headers, body).await
    }

    async fn ship_order(
        &self,
        shop: &ShopAuthContext,
        request: ShipOrderRequest,
    ) -> Result<ShipOrderResponse, AdapterError> {
        WebhookAdapter::ship_order(self, shop, request).await
    }

    async fn fetch_order(
        &self,
        shop: &ShopAuthContext,
        platform_order_id: &str,
    ) -> Result<OrderProjection, AdapterError> {
        OrderAdapter::fetch_order(self, shop, platform_order_id).await
    }

    async fn refresh_token(
        &self,
        shop: &ShopAuthContext,
        token: &PlatformTokenPayload,
    ) -> Result<TokenRefreshResult, AdapterError> {
        if shop
            .auth_meta
            .get("refresh_fail")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            return Err(AdapterError::Auth("mock refresh failure".into()));
        }
        let refresh_token = token
            .refresh_token
            .clone()
            .ok_or_else(|| AdapterError::Auth("missing refresh_token".into()))?;
        let now = Utc::now();
        let access_token = format!("{refresh_token}-{}", now.timestamp());
        Ok(TokenRefreshResult {
            access_token: Some(access_token),
            refresh_token: Some(refresh_token),
            expires_at: Some(now + chrono::Duration::hours(2)),
            scopes: token.scopes.clone(),
            raw: serde_json::json!({"mock": true}),
        })
    }

    async fn pull_orders(
        &self,
        shop: &ShopAuthContext,
        cursor: SyncCursor,
    ) -> Result<PullOrdersResult, AdapterError> {
        let mock_orders = shop
            .auth_meta
            .get("mock_orders")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut orders = Vec::new();
        let mut latest_time = cursor.last_time;
        for raw in mock_orders {
            let proj = Self::map_order_to_projection(shop, &raw)?;
            let compare_time = proj.platform_updated_at.unwrap_or(proj.order_created_at);
            if let Some(last_time) = cursor.last_time {
                if compare_time <= last_time {
                    continue;
                }
            }
            if latest_time.map(|t| compare_time > t).unwrap_or(true) {
                latest_time = Some(compare_time);
            }
            orders.push(proj);
        }
        Ok(PullOrdersResult {
            orders,
            next_cursor: None,
            latest_time,
        })
    }

    async fn pull_products(
        &self,
        _shop: &ShopAuthContext,
        cursor: SyncCursor,
    ) -> Result<PullProductsResult, AdapterError> {
        Ok(PullProductsResult {
            products: Vec::new(),
            next_cursor: None,
            latest_time: cursor.last_time,
        })
    }

    async fn pull_refunds(
        &self,
        _shop: &ShopAuthContext,
        cursor: SyncCursor,
    ) -> Result<PullRefundsResult, AdapterError> {
        Ok(PullRefundsResult {
            orders: Vec::new(),
            next_cursor: None,
            latest_time: cursor.last_time,
        })
    }

    fn build_authorize_url(
        &self,
        shop: &ShopAuthContext,
        redirect_uri: Option<String>,
        state: &str,
    ) -> Option<String> {
        erp_adapter::build_authorize_url_from_meta(&shop.auth_meta, redirect_uri, state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use erp_types::{Platform, ShopId, TenantId};

    fn dummy_shop() -> ShopAuthContext {
        ShopAuthContext {
            tenant_id: "00000000-0000-0000-0000-000000000000"
                .parse::<TenantId>()
                .unwrap(),
            shop_id: "00000000-0000-0000-0000-000000000000"
                .parse::<ShopId>()
                .unwrap(),
            platform: Platform::Pdd,
            platform_shop_id: "pdd-shop-001".to_string(),
            credentials: serde_json::json!({
                "client_id": "x",
                "client_secret": "y",
                "access_token": "z"
            }),
            auth_meta: serde_json::json!({}),
        }
    }

    #[test]
    fn pdd_yuan_to_cent_parsing() {
        assert_eq!(
            Adapter::parse_pdd_yuan_to_cent(&serde_json::json!(1.23)),
            Some(123)
        );
        assert_eq!(
            Adapter::parse_pdd_yuan_to_cent(&serde_json::json!(1)),
            Some(100)
        );
        assert_eq!(
            Adapter::parse_pdd_yuan_to_cent(&serde_json::json!("1.23")),
            Some(123)
        );
        assert_eq!(
            Adapter::parse_pdd_yuan_to_cent(&serde_json::json!("1")),
            Some(100)
        );
    }

    #[test]
    fn pdd_status_mapping_basic_coverage() {
        // confirm_status coverage
        assert!(PDD_CONFIRM_STATUS_CODES.contains(&0));
        assert!(PDD_CONFIRM_STATUS_CODES.contains(&1));
        assert!(PDD_CONFIRM_STATUS_CODES.contains(&2));

        // order_status coverage
        for code in PDD_ORDER_STATUS_CODES {
            let st = Adapter::map_order_status(Some(1), Some(*code), Some(1), Some(0), Some(0));
            match code {
                1 => assert_eq!(st, OrderStatus::PendingShipment),
                2 => assert_eq!(st, OrderStatus::Shipped),
                3 => assert_eq!(st, OrderStatus::Completed),
                _ => unreachable!(),
            }
        }

        // unpaid / cancelled
        assert_eq!(
            Adapter::map_order_status(Some(0), Some(1), Some(1), Some(0), Some(0)),
            OrderStatus::PendingPayment
        );
        assert_eq!(
            Adapter::map_order_status(Some(2), Some(1), Some(1), Some(0), Some(0)),
            OrderStatus::Cancelled
        );

        // refund_status coverage
        for code in PDD_REFUND_STATUS_CODES {
            let st = Adapter::map_order_status(Some(1), Some(1), Some(*code), Some(0), Some(0));
            match code {
                1 => assert_eq!(st, OrderStatus::PendingShipment),
                2 | 3 => assert_eq!(st, OrderStatus::Refunding),
                4 => assert_eq!(st, OrderStatus::Refunded),
                _ => unreachable!(),
            }
        }

        // risk-control
        assert_eq!(
            Adapter::map_order_status(Some(1), Some(1), Some(1), Some(0), Some(1)),
            OrderStatus::OnHold
        );

        // after_sales_status coverage
        for code in PDD_AFTER_SALES_STATUS_CODES {
            let ov = Adapter::after_sales_override_status(*code);
            match *code {
                0 => assert_eq!(ov, None),
                10 => assert_eq!(ov, Some(OrderStatus::Refunded)),
                2 | 3 | 4 | 5 | 6 | 7 | 8 | 31 | 33 => assert_eq!(ov, Some(OrderStatus::Refunding)),
                14 | 15 | 18 | 21 | 22 | 27 | 32 => assert_eq!(ov, Some(OrderStatus::OnHold)),
                9 | 11 | 12 | 13 | 16 | 17 | 24 | 25 => assert_eq!(ov, None),
                _ => {
                    // ensure we don't silently introduce new enum values
                    panic!("unexpected after_sales_status code in coverage list: {code}");
                }
            }
        }

        // after_sales refund success overrides refund_status=1
        assert_eq!(
            Adapter::map_order_status(Some(1), Some(3), Some(1), Some(10), Some(0)),
            OrderStatus::Refunded
        );

        // after_sales refunding overrides order status
        assert_eq!(
            Adapter::map_order_status(Some(1), Some(3), Some(1), Some(2), Some(0)),
            OrderStatus::Refunding
        );
    }

    #[test]
    fn pdd_order_projection_core_fields() {
        let shop = dummy_shop();
        let order = serde_json::json!({
            "order_sn": "202601010001",
            "confirm_status": 1,
            "order_status": 2,
            "refund_status": 1,
            "after_sales_status": 0,
            "risk_control_status": 0,
            "created_time": 1704067200, // 2024-01-01 00:00:00 UTC
            "updated_at": 1704068200,
            "pay_amount": 12.34,
            "province": "浙江省",
            "city": "杭州市",
            "town": "西湖区",
            "receiver_address_mask": "翠苑路***号",
            "receiver_name_mask": "张**",
            "receiver_phone_mask": "138****0000",
            "item_list": [
                {"goods_id": 1001, "sku_id": 2001, "goods_name": "测试商品", "goods_count": 2, "goods_price": 6.17, "outer_id": "SKU-OUT-1"}
            ]
        });

        let proj = Adapter::map_order_to_projection(&shop, &order).expect("map");
        assert_eq!(proj.platform_order_id, "202601010001");
        assert_eq!(proj.status, OrderStatus::Shipped);
        assert_eq!(proj.total_amount_cent, 1234);
        assert_eq!(proj.receiver_name.as_deref(), Some("张**"));
        assert_eq!(proj.receiver_phone.as_deref(), Some("138****0000"));
        assert!(proj.receiver_address.as_ref().unwrap().contains("浙江省"));
        assert_eq!(proj.items.len(), 1);
        assert_eq!(proj.items[0].line_id, "SKU-OUT-1");
        assert_eq!(proj.items[0].qty, 2);
        assert_eq!(proj.items[0].price_cent, 617);
        assert_eq!(proj.items[0].amount_cent, 1234);
    }

    #[test]
    fn pdd_order_projection_enrichment_fields_and_item_aftersales() {
        let shop = dummy_shop();
        let order = serde_json::json!({
            "order_sn": "202601010002",
            "confirm_status": 1,
            "order_status": 1,
            "refund_status": 1,
            "after_sales_status": 0,
            "risk_control_status": 0,
            "created_time": 1704067200,
            "updated_at": 1704068200,
            "pay_amount": "12.34",
            "goods_amount": "10.00",
            "postage": 2.34,
            "discount_amount": 1.00,
            "platform_discount": 0.20,
            "seller_discount": 0.30,
            "coupon_discount": 0.50,
            "pay_time": 1704067300,
            "shipping_time": 1704067400,
            "confirm_time": 1704067500,
            "buyer_message": "请尽快发货",
            "seller_remark": "VIP客户",
            "shipping_company": "中通快递",
            "tracking_number": "YT123",
            "province": "浙江省",
            "city": "杭州市",
            "town": "西湖区",
            "receiver_address_mask": "翠苑路***号",
            "receiver_name_mask": "张**",
            "receiver_phone_mask": "138****0000",
            "item_list": [
                {
                    "goods_id": 1001,
                    "sku_id": 2001,
                    "goods_name": "测试商品A",
                    "goods_count": 1,
                    "goods_price": 5.00,
                    "outer_id": "SKU-OUT-A",
                    "pay_amount": 4.50,
                    "discount_amount": 0.50,
                    "refund_status": 1,
                    "after_sales_status": 14,
                    "refund_amount": 0,
                    "after_sales_type": "exchange",
                    "after_sales_id": "AS-001"
                },
                {
                    "goods_id": 1002,
                    "sku_id": 2002,
                    "goods_name": "测试商品B",
                    "goods_count": 1,
                    "goods_price": 5.00,
                    "outer_id": "SKU-OUT-B",
                    "refund_status": 2,
                    "after_sales_status": 2,
                    "refund_amount": 5.00,
                    "after_sales_type": 1,
                    "after_sales_id": 9001
                }
            ]
        });

        let proj = Adapter::map_order_to_projection(&shop, &order).expect("map");
        assert_eq!(proj.total_amount_cent, 1234);
        assert_eq!(proj.goods_amount_cent, Some(1000));
        assert_eq!(proj.postage_amount_cent, Some(234));
        assert_eq!(proj.discount_amount_cent, Some(100));
        assert_eq!(proj.platform_discount_cent, Some(20));
        assert_eq!(proj.seller_discount_cent, Some(30));
        assert_eq!(proj.coupon_discount_cent, Some(50));
        assert!(proj.pay_at.is_some());
        assert!(proj.ship_at.is_some());
        assert!(proj.finish_at.is_some());
        assert_eq!(proj.buyer_message.as_deref(), Some("请尽快发货"));
        assert_eq!(proj.seller_remark.as_deref(), Some("VIP客户"));
        assert_eq!(proj.logistics_company.as_deref(), Some("中通快递"));
        assert_eq!(proj.tracking_no.as_deref(), Some("YT123"));

        assert_eq!(proj.items.len(), 2);
        // item A: exchange-like after-sales => after_sale_state Processing, refund_state None
        assert_eq!(proj.items[0].line_id, "SKU-OUT-A");
        assert_eq!(proj.items[0].pay_amount_cent, Some(450));
        assert_eq!(proj.items[0].discount_cent, Some(50));
        assert_eq!(proj.items[0].refund_state, RefundState::None);
        assert_eq!(proj.items[0].after_sale_state, AfterSaleState::Processing);
        assert_eq!(proj.items[0].refund_amount_cent, Some(0));
        assert_eq!(proj.items[0].after_sale_type.as_deref(), Some("exchange"));
        assert_eq!(proj.items[0].after_sale_id.as_deref(), Some("AS-001"));

        // item B: refund processing
        assert_eq!(proj.items[1].line_id, "SKU-OUT-B");
        assert_eq!(proj.items[1].refund_state, RefundState::Processing);
        assert_eq!(proj.items[1].after_sale_state, AfterSaleState::Processing);
        assert_eq!(proj.items[1].refund_amount_cent, Some(500));
        assert_eq!(proj.items[1].after_sale_type.as_deref(), Some("1"));
        assert_eq!(proj.items[1].after_sale_id.as_deref(), Some("9001"));
    }
}
