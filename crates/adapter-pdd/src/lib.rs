#![recursion_limit = "256"]

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
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

const PDD_OAUTH_AUTHORIZE_URL: &str = "https://mms.pinduoduo.com/open.html";
const PDD_OAUTH_TOKEN_URL: &str = "https://open-api.pinduoduo.com/oauth/token";

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

    fn auth_meta_string(shop: &ShopAuthContext, pointers: &[&str]) -> Option<String> {
        for pointer in pointers {
            if let Some(value) = shop
                .auth_meta
                .pointer(pointer)
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                return Some(value.to_string());
            }
        }
        None
    }

    fn credential_string(shop: &ShopAuthContext, key: &str) -> Option<String> {
        shop.credentials
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
    }

    fn oauth_authorize_url(shop: &ShopAuthContext) -> String {
        Self::auth_meta_string(
            shop,
            &[
                "/oauth/authorize_url",
                "/authorize_url",
                "/oauth_authorize_url",
            ],
        )
        .or_else(|| Self::credential_string(shop, "authorize_url"))
        .unwrap_or_else(|| PDD_OAUTH_AUTHORIZE_URL.to_string())
    }

    fn oauth_token_url(shop: &ShopAuthContext) -> String {
        Self::auth_meta_string(
            shop,
            &["/oauth/token_url", "/token_url", "/oauth_token_url"],
        )
        .or_else(|| Self::credential_string(shop, "token_url"))
        .unwrap_or_else(|| PDD_OAUTH_TOKEN_URL.to_string())
    }

    fn default_redirect_uri(shop: &ShopAuthContext) -> Option<String> {
        Self::auth_meta_string(shop, &["/oauth/redirect_uri", "/redirect_uri"])
            .or_else(|| Self::credential_string(shop, "redirect_uri"))
    }

    fn parse_i64(value: &Value) -> Option<i64> {
        match value {
            Value::Number(number) => number.as_i64(),
            Value::String(text) => text.trim().parse::<i64>().ok(),
            _ => None,
        }
    }

    fn read_string_any(payload: &Value, pointers: &[&str]) -> Option<String> {
        for pointer in pointers {
            if let Some(value) = payload
                .pointer(pointer)
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                return Some(value.to_string());
            }
        }
        None
    }

    fn read_i64_any(payload: &Value, pointers: &[&str]) -> Option<i64> {
        for pointer in pointers {
            if let Some(value) = payload.pointer(pointer).and_then(Self::parse_i64) {
                return Some(value);
            }
        }
        None
    }

    fn parse_oauth_token_response(
        payload: &Value,
        now: DateTime<Utc>,
    ) -> Result<TokenRefreshResult, AdapterError> {
        let access_token = Self::read_string_any(
            payload,
            &[
                "/access_token",
                "/result/access_token",
                "/data/access_token",
            ],
        )
        .ok_or_else(|| AdapterError::Auth("oauth token response missing access_token".into()))?;
        let refresh_token = Self::read_string_any(
            payload,
            &[
                "/refresh_token",
                "/result/refresh_token",
                "/data/refresh_token",
            ],
        );
        let scopes = Self::read_string_any(
            payload,
            &[
                "/scope",
                "/scopes",
                "/result/scope",
                "/result/scopes",
                "/data/scope",
                "/data/scopes",
            ],
        );
        let expires_in = Self::read_i64_any(
            payload,
            &[
                "/expires_in",
                "/expire_in",
                "/result/expires_in",
                "/result/expire_in",
                "/data/expires_in",
                "/data/expire_in",
            ],
        );
        let expires_at = if let Some(seconds) = expires_in {
            Some(now + Duration::seconds(seconds.max(0)))
        } else {
            Self::read_string_any(
                payload,
                &["/expires_at", "/result/expires_at", "/data/expires_at"],
            )
            .and_then(|raw| DateTime::parse_from_rfc3339(raw.trim()).ok())
            .map(|value| value.with_timezone(&Utc))
        };

        Ok(TokenRefreshResult {
            access_token: Some(access_token),
            refresh_token,
            expires_at,
            scopes,
            raw: payload.clone(),
        })
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

    fn error_token(value: Option<&Value>) -> Option<String> {
        let value = value?;
        match value {
            Value::String(text) => {
                let trimmed = text.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            }
            Value::Number(number) => Some(number.to_string()),
            _ => None,
        }
    }

    fn classify_api_error(source: &str, code: Option<String>, message: String) -> AdapterError {
        let combined = format!(
            "{} {}",
            code.as_deref().unwrap_or_default(),
            message.as_str()
        )
        .to_lowercase();
        let auth_like = combined.contains("token")
            || combined.contains("auth")
            || combined.contains("sign")
            || combined.contains("forbidden")
            || combined.contains("unauthorized")
            || combined.contains("权限")
            || combined.contains("验签");
        if auth_like {
            return AdapterError::Auth(format!(
                "{source} auth error code={} message={message}",
                code.as_deref().unwrap_or("unknown")
            ));
        }

        let rate_limited = combined.contains("rate")
            || combined.contains("limit")
            || combined.contains("too many")
            || combined.contains("throttle")
            || combined.contains("频控")
            || combined.contains("限流");
        if rate_limited {
            return AdapterError::RateLimited {
                retry_after_ms: 60_000,
            };
        }

        let bad_request = combined.contains("invalid")
            || combined.contains("missing")
            || combined.contains("required")
            || combined.contains("param")
            || combined.contains("argument")
            || combined.contains("参数");
        if bad_request {
            return AdapterError::BadRequest(format!(
                "{source} bad request code={} message={message}",
                code.as_deref().unwrap_or("unknown")
            ));
        }

        AdapterError::Temporary(format!(
            "{source} api error code={} message={message}",
            code.as_deref().unwrap_or("unknown")
        ))
    }

    fn map_error_response(source: &str, error: &Value) -> AdapterError {
        let code = Self::error_token(error.get("error_code"))
            .or_else(|| Self::error_token(error.get("sub_code")))
            .or_else(|| Self::error_token(error.get("code")));
        let message = Self::error_token(error.get("sub_msg"))
            .or_else(|| Self::error_token(error.get("error_msg")))
            .or_else(|| Self::error_token(error.get("msg")))
            .or_else(|| Self::error_token(error.get("message")))
            .unwrap_or_else(|| error.to_string());

        Self::classify_api_error(source, code, message)
    }

    fn map_http_error(source: &str, status: reqwest::StatusCode, body: &[u8]) -> AdapterError {
        let body_text = String::from_utf8_lossy(body);
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return AdapterError::Auth(format!(
                "{source} gateway auth error status={} body={}...",
                status, body_text
            ));
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return AdapterError::RateLimited {
                retry_after_ms: 60_000,
            };
        }
        if status.is_client_error() {
            return AdapterError::BadRequest(format!(
                "{source} gateway bad request status={} body={}...",
                status, body_text
            ));
        }
        AdapterError::Temporary(format!(
            "{source} gateway status={} body={}...",
            status, body_text
        ))
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
        if let Some(as_code) = after_sales_status
            && let Some(ov) = Self::after_sales_override_status(as_code)
        {
            // Avoid overriding refund_status==1 (no after-sale) with None.
            return ov;
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
            return Err(Self::map_http_error("pdd", status, &body));
        }

        let v: Value = serde_json::from_slice(&body)
            .map_err(|e| AdapterError::Temporary(format!("json: {e}")))?;
        if let Some(error) = v.get("error_response") {
            return Err(Self::map_error_response("pdd", error));
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
        if let Some(v) = parse_json(body)
            && let Some(order_obj) = Self::find_first_order_object(&v, 12)
        {
            return Ok(vec![Self::map_order_to_projection(shop, order_obj)?]);
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

impl Adapter {
    fn is_refund_related_order(order: &OrderProjection) -> bool {
        if matches!(order.status, OrderStatus::Refunding | OrderStatus::Refunded) {
            return true;
        }
        order.items.iter().any(|item| {
            item.after_sale_id
                .as_deref()
                .map(str::trim)
                .map(|id| !id.is_empty())
                .unwrap_or(false)
                || item.refund_state != RefundState::None
                || item.after_sale_state != AfterSaleState::None
                || item.refund_amount_cent.unwrap_or(0) > 0
        })
    }

    fn value_to_text(v: &Value) -> Option<String> {
        if let Some(s) = v.as_str() {
            return Some(s.to_string());
        }
        if let Some(n) = v.as_i64() {
            return Some(n.to_string());
        }
        if let Some(n) = v.as_u64() {
            return Some(n.to_string());
        }
        if let Some(n) = v.as_f64() {
            return Some(n.to_string());
        }
        if let Some(b) = v.as_bool() {
            return Some(if b { "true" } else { "false" }.to_string());
        }
        None
    }

    fn credential_bool(shop: &ShopAuthContext, key: &str) -> Option<bool> {
        shop.credentials.get(key).and_then(|v| v.as_bool())
    }

    fn credential_i64(shop: &ShopAuthContext, key: &str) -> Option<i64> {
        shop.credentials.get(key).and_then(|v| v.as_i64())
    }

    fn credential_object(shop: &ShopAuthContext, key: &str) -> Option<Value> {
        shop.credentials.get(key).filter(|v| v.is_object()).cloned()
    }

    fn refund_api_enabled(shop: &ShopAuthContext) -> bool {
        Self::credential_bool(shop, "refund_api_enabled").unwrap_or(false)
    }

    fn refund_list_method(shop: &ShopAuthContext) -> Option<String> {
        Self::credential_string(shop, "refund_list_method")
    }

    fn refund_page_size(shop: &ShopAuthContext) -> i64 {
        Self::credential_i64(shop, "refund_page_size")
            .unwrap_or(50)
            .clamp(1, 200)
    }

    fn refund_list_biz(shop: &ShopAuthContext) -> Value {
        Self::credential_object(shop, "refund_list_biz").unwrap_or_else(|| serde_json::json!({}))
    }

    fn extract_values_by_pointer(v: &Value, pointer: Option<String>) -> Option<Vec<Value>> {
        let pointer = pointer.as_deref()?.trim();
        if pointer.is_empty() {
            return None;
        }
        let target = v.pointer(pointer)?;
        if let Some(arr) = target.as_array() {
            return Some(arr.clone());
        }
        Some(vec![target.clone()])
    }

    fn extract_text_by_pointer(v: &Value, pointer: Option<String>) -> Option<String> {
        let pointer = pointer.as_deref()?.trim();
        if pointer.is_empty() {
            return None;
        }
        v.pointer(pointer).and_then(Self::value_to_text)
    }

    fn extract_bool_by_pointer(v: &Value, pointer: Option<String>) -> Option<bool> {
        let pointer = pointer.as_deref()?.trim();
        if pointer.is_empty() {
            return None;
        }
        v.pointer(pointer).and_then(|x| x.as_bool())
    }

    fn extract_refund_list(
        shop: &ShopAuthContext,
        v: &Value,
    ) -> (Vec<Value>, Option<String>, bool) {
        let data = v.get("data").or_else(|| v.get("result")).unwrap_or(v);
        let mut items = data
            .get("list")
            .or_else(|| data.get("order_list"))
            .or_else(|| data.get("orders"))
            .or_else(|| data.get("items"))
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        if items.is_empty()
            && let Some(arr) = data.get("order_sn_list").and_then(|x| x.as_array())
        {
            items.extend(arr.iter().map(|x| serde_json::json!({ "order_sn": x })));
        }

        let mut next_cursor = data
            .get("cursor")
            .or_else(|| data.get("next_cursor"))
            .or_else(|| data.get("next"))
            .and_then(Self::value_to_text);
        let mut has_more = data
            .get("has_more")
            .or_else(|| data.get("has_next"))
            .or_else(|| data.get("more"))
            .and_then(|x| x.as_bool())
            .unwrap_or(false);

        let list_pointer = Self::credential_string(shop, "refund_list_pointer");
        if let Some(overridden) = Self::extract_values_by_pointer(v, list_pointer) {
            items = overridden;
        }
        if let Some(cursor) = Self::extract_text_by_pointer(
            v,
            Self::credential_string(shop, "refund_next_cursor_pointer"),
        ) {
            next_cursor = Some(cursor);
        }
        if let Some(flag) = Self::extract_bool_by_pointer(
            v,
            Self::credential_string(shop, "refund_has_more_pointer"),
        ) {
            has_more = flag;
        }

        (items, next_cursor, has_more)
    }

    fn first_text_from_keys(v: &Value, keys: &[&str]) -> Option<String> {
        for key in keys {
            if let Some(value) = v.get(*key).and_then(Self::value_to_text) {
                let value = value.trim().to_string();
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
        None
    }

    fn collect_urls(value: &Value) -> Vec<String> {
        match value {
            Value::String(s) => {
                let s = s.trim();
                if s.is_empty() {
                    vec![]
                } else {
                    vec![s.to_string()]
                }
            }
            Value::Array(arr) => arr
                .iter()
                .flat_map(Self::collect_urls)
                .collect::<Vec<String>>(),
            Value::Object(map) => {
                for key in ["url", "img_url", "image_url", "pic_url", "proof_url"] {
                    if let Some(value) = map.get(key).and_then(Self::value_to_text) {
                        let value = value.trim().to_string();
                        if !value.is_empty() {
                            return vec![value];
                        }
                    }
                }
                vec![]
            }
            _ => vec![],
        }
    }

    fn first_urls_from_keys(v: &Value, keys: &[&str]) -> Vec<String> {
        for key in keys {
            if let Some(value) = v.get(*key) {
                let urls = Self::collect_urls(value);
                if !urls.is_empty() {
                    return urls;
                }
            }
        }
        vec![]
    }

    fn first_text_from_sources(sources: &[&Value], keys: &[&str]) -> Option<String> {
        for source in sources {
            if let Some(value) = Self::first_text_from_keys(source, keys) {
                return Some(value);
            }
        }
        None
    }

    fn first_urls_from_sources(sources: &[&Value], keys: &[&str]) -> Vec<String> {
        for source in sources {
            let urls = Self::first_urls_from_keys(source, keys);
            if !urls.is_empty() {
                return urls;
            }
        }
        vec![]
    }

    fn first_rfc3339_from_sources(sources: &[&Value], keys: &[&str]) -> Option<String> {
        for source in sources {
            for key in keys {
                if let Some(value) = source.get(*key).and_then(parse_platform_time) {
                    return Some(value.to_rfc3339());
                }
            }
        }
        None
    }

    fn inject_refund_meta(order: &mut OrderProjection, source: &Value) {
        let order_raw = order.raw.clone();
        let default_updated_at = order
            .platform_updated_at
            .unwrap_or(order.order_created_at)
            .to_rfc3339();

        for item in &mut order.items {
            let sources = [&item.raw, source, &order_raw];
            let reason = Self::first_text_from_sources(
                &sources,
                &[
                    "refund_reason",
                    "reason",
                    "after_sale_reason",
                    "aftersale_reason",
                    "remark",
                ],
            );
            let responsibility = Self::first_text_from_sources(
                &sources,
                &[
                    "responsibility",
                    "responsible_party",
                    "refund_responsibility",
                    "responsibility_type",
                ],
            );
            let evidence_urls = Self::first_urls_from_sources(
                &sources,
                &[
                    "evidence_urls",
                    "proof_urls",
                    "voucher_urls",
                    "evidence",
                    "proof",
                    "proof_imgs",
                    "images",
                    "pics",
                ],
            );
            let updated_at = Self::first_rfc3339_from_sources(
                &sources,
                &[
                    "refund_updated_at",
                    "after_sale_updated_at",
                    "updated_at",
                    "update_time",
                    "modified",
                ],
            )
            .unwrap_or_else(|| default_updated_at.clone());
            let source_refund_id = Self::first_text_from_sources(
                &sources,
                &[
                    "after_sale_id",
                    "after_sales_id",
                    "aftersale_id",
                    "refund_id",
                    "refund_no",
                ],
            );
            let source_status = Self::first_text_from_sources(
                &sources,
                &[
                    "refund_status",
                    "after_sale_status",
                    "refund_state",
                    "after_sale_state",
                    "status",
                ],
            );

            if let Some(raw_obj) = item.raw.as_object_mut() {
                raw_obj.insert(
                    "_erp_refund_meta".to_string(),
                    serde_json::json!({
                        "reason": reason,
                        "responsibility": responsibility,
                        "evidence_urls": evidence_urls,
                        "updated_at": updated_at,
                        "source_refund_id": source_refund_id,
                        "source_status": source_status,
                    }),
                );
            }
        }
    }

    async fn pull_refunds_from_api(
        &self,
        shop: &ShopAuthContext,
        cursor: &SyncCursor,
    ) -> Result<Option<PullRefundsResult>, AdapterError> {
        if !Self::refund_api_enabled(shop) {
            return Ok(None);
        }
        let Some(api_type) = Self::refund_list_method(shop) else {
            return Ok(None);
        };

        let client_id = Self::creds_client_id(shop)?;
        let client_secret = Self::creds_client_secret(shop)?;
        let access_token = Self::creds_access_token(shop)
            .ok_or_else(|| AdapterError::Auth("missing credentials.access_token".into()))?;
        let gateway = Self::gateway_url(shop);
        let timestamp = Utc::now().timestamp().to_string();
        let page_size = Self::refund_page_size(shop).to_string();

        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("type".to_string(), api_type);
        params.insert("client_id".to_string(), client_id);
        params.insert("access_token".to_string(), access_token);
        params.insert("data_type".to_string(), "JSON".to_string());
        params.insert("timestamp".to_string(), timestamp);
        params.insert("page_size".to_string(), page_size);

        if let Some(v) = cursor.cursor.as_deref() {
            if v.parse::<i64>().is_ok() {
                params.insert("page".to_string(), v.to_string());
            } else {
                params.insert("cursor".to_string(), v.to_string());
            }
        }
        if let Some(last_time) = cursor.last_time {
            params.insert(
                "updated_time_start".to_string(),
                last_time.timestamp().to_string(),
            );
        }

        if let Some(map) = Self::refund_list_biz(shop).as_object() {
            for (k, v) in map {
                if let Some(text) = Self::value_to_text(v) {
                    params.insert(k.to_string(), text);
                }
            }
        }

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
            return Err(Self::map_http_error("pdd", status, &body));
        }

        let v: Value = serde_json::from_slice(&body)
            .map_err(|e| AdapterError::Temporary(format!("json: {e}")))?;
        if let Some(error) = v.get("error_response") {
            return Err(Self::map_error_response("pdd", error));
        }

        let (raw_list, mut next_cursor, has_more) = Self::extract_refund_list(shop, &v);
        let mut orders = Vec::new();
        let mut latest_time = cursor.last_time;
        let page = cursor
            .cursor
            .as_deref()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(1);
        let use_page = cursor
            .cursor
            .as_deref()
            .map(|v| v.parse::<i64>().is_ok())
            .unwrap_or(true);

        for raw in raw_list {
            let mut proj = if raw.get("item_list").is_some() || raw.get("order_sn").is_some() {
                Self::map_order_to_projection(shop, &raw)?
            } else if let Some(order_sn) = raw
                .get("order_sn")
                .or_else(|| raw.get("orderSn"))
                .or_else(|| raw.get("order_id"))
                .and_then(Self::value_to_text)
            {
                self.fetch_order_detail(shop, &order_sn).await?
            } else {
                continue;
            };

            let compare_time = proj.platform_updated_at.unwrap_or(proj.order_created_at);
            if let Some(last_time) = cursor.last_time
                && compare_time <= last_time
            {
                continue;
            }
            if Self::is_refund_related_order(&proj) {
                Self::inject_refund_meta(&mut proj, &raw);
                if latest_time.map(|t| compare_time > t).unwrap_or(true) {
                    latest_time = Some(compare_time);
                }
                orders.push(proj);
            }
        }

        if has_more {
            if next_cursor.is_none() && use_page {
                next_cursor = Some((page + 1).to_string());
            }
        } else {
            next_cursor = None;
        }

        Ok(Some(PullRefundsResult {
            orders,
            next_cursor,
            latest_time,
        }))
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

    async fn exchange_authorization_code(
        &self,
        shop: &ShopAuthContext,
        code: &str,
        redirect_uri: Option<String>,
    ) -> Result<TokenRefreshResult, AdapterError> {
        let code = code.trim();
        if code.is_empty() {
            return Err(AdapterError::BadRequest(
                "authorization code is required".into(),
            ));
        }

        let client_id = Self::creds_client_id(shop)?;
        let client_secret = Self::creds_client_secret(shop)?;
        let token_url = Self::oauth_token_url(shop);

        let mut form_params = vec![
            ("grant_type".to_string(), "authorization_code".to_string()),
            ("client_id".to_string(), client_id),
            ("client_secret".to_string(), client_secret),
            ("code".to_string(), code.to_string()),
        ];
        if let Some(uri) = redirect_uri.or_else(|| Self::default_redirect_uri(shop)) {
            form_params.push(("redirect_uri".to_string(), uri));
        }

        let response = self
            .client
            .post(&token_url)
            .form(&form_params)
            .send()
            .await
            .map_err(|err| AdapterError::Temporary(format!("oauth token request failed: {err}")))?;
        let status = response.status();
        let payload = response.json::<Value>().await.map_err(|err| {
            AdapterError::Unexpected(format!("oauth token response parse failed: {err}"))
        })?;
        if !status.is_success() {
            return Err(AdapterError::Auth(format!(
                "oauth token exchange failed: status={status}, payload={payload}"
            )));
        }

        Self::parse_oauth_token_response(&payload, Utc::now())
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
            if let Some(last_time) = cursor.last_time
                && compare_time <= last_time
            {
                continue;
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
        shop: &ShopAuthContext,
        cursor: SyncCursor,
    ) -> Result<PullRefundsResult, AdapterError> {
        let mock_refunds = shop
            .auth_meta
            .get("mock_refunds")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        if !mock_refunds.is_empty() {
            let mut orders = Vec::new();
            let mut latest_time = cursor.last_time;
            for raw in mock_refunds {
                let mut proj = Self::map_order_to_projection(shop, &raw)?;
                let compare_time = proj.platform_updated_at.unwrap_or(proj.order_created_at);
                if let Some(last_time) = cursor.last_time
                    && compare_time <= last_time
                {
                    continue;
                }
                if Self::is_refund_related_order(&proj) {
                    Self::inject_refund_meta(&mut proj, &raw);
                    if latest_time.map(|t| compare_time > t).unwrap_or(true) {
                        latest_time = Some(compare_time);
                    }
                    orders.push(proj);
                }
            }
            return Ok(PullRefundsResult {
                orders,
                next_cursor: None,
                latest_time,
            });
        }

        if Self::refund_api_enabled(shop) {
            match self.pull_refunds_from_api(shop, &cursor).await {
                Ok(Some(result)) => return Ok(result),
                Ok(None) => {}
                Err(_err) => {}
            }
        }

        let result = self.pull_orders(shop, cursor).await?;
        Ok(PullRefundsResult {
            orders: result
                .orders
                .into_iter()
                .filter(Self::is_refund_related_order)
                .collect(),
            next_cursor: result.next_cursor,
            latest_time: result.latest_time,
        })
    }

    fn build_authorize_url(
        &self,
        shop: &ShopAuthContext,
        redirect_uri: Option<String>,
        state: &str,
    ) -> Option<String> {
        let client_id = match Self::creds_client_id(shop) {
            Ok(value) => value,
            Err(_) => {
                return erp_adapter::build_authorize_url_from_meta(
                    &shop.auth_meta,
                    redirect_uri,
                    state,
                );
            }
        };

        let mut url = reqwest::Url::parse(&Self::oauth_authorize_url(shop)).ok()?;
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("response_type", "code");
            pairs.append_pair("client_id", &client_id);
            pairs.append_pair("state", state);
            if let Some(uri) = redirect_uri.or_else(|| Self::default_redirect_uri(shop)) {
                pairs.append_pair("redirect_uri", &uri);
            }
        }
        Some(url.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use erp_types::{Platform, ShopId, TenantId};
    use httpmock::Method::POST;
    use httpmock::MockServer;

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
    fn classify_pdd_error_response_as_auth() {
        let error = serde_json::json!({
            "error_code": 20001,
            "error_msg": "access_token expired"
        });
        let result = Adapter::map_error_response("pdd", &error);
        assert!(matches!(result, AdapterError::Auth(_)));
    }

    #[test]
    fn classify_pdd_error_response_as_rate_limited() {
        let error = serde_json::json!({
            "error_code": 70001,
            "sub_msg": "too many requests"
        });
        let result = Adapter::map_error_response("pdd", &error);
        assert!(matches!(result, AdapterError::RateLimited { .. }));
    }

    #[test]
    fn classify_pdd_http_client_error_as_bad_request() {
        let result = Adapter::map_http_error(
            "pdd",
            reqwest::StatusCode::BAD_REQUEST,
            br#"{"error":"invalid param"}"#,
        );
        assert!(matches!(result, AdapterError::BadRequest(_)));
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

    #[test]
    fn pdd_build_authorize_url_prefers_official_oauth_endpoint() {
        let adapter = Adapter::new();
        let shop = dummy_shop();

        let url = adapter
            .build_authorize_url(
                &shop,
                Some(
                    "https://erp.example.com/channels/shops?auth_callback=1&shop_id=SHOP-1"
                        .to_string(),
                ),
                "state-abc",
            )
            .expect("authorize url should exist");

        assert!(url.starts_with("https://mms.pinduoduo.com/open.html?"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=x"));
        assert!(url.contains("state=state-abc"));
        assert!(url.contains("redirect_uri=https%3A%2F%2Ferp.example.com%2Fchannels%2Fshops%3Fauth_callback%3D1%26shop_id%3DSHOP-1"));
    }

    #[test]
    fn pdd_parse_oauth_token_response_supports_top_level_and_nested_shape() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let top_level = serde_json::json!({
            "access_token": "access-a",
            "refresh_token": "refresh-a",
            "expires_in": 7200,
            "scope": "orders,shipping",
        });
        let parsed_top =
            Adapter::parse_oauth_token_response(&top_level, now).expect("top level parse");
        assert_eq!(parsed_top.access_token.as_deref(), Some("access-a"));
        assert_eq!(parsed_top.refresh_token.as_deref(), Some("refresh-a"));
        assert_eq!(parsed_top.scopes.as_deref(), Some("orders,shipping"));
        assert_eq!(
            parsed_top.expires_at,
            Some(now + chrono::Duration::seconds(7200))
        );

        let nested = serde_json::json!({
            "result": {
                "access_token": "access-b",
                "refresh_token": "refresh-b",
                "expires_in": 3600,
                "scope": "orders",
            }
        });
        let parsed_nested =
            Adapter::parse_oauth_token_response(&nested, now).expect("nested parse");
        assert_eq!(parsed_nested.access_token.as_deref(), Some("access-b"));
        assert_eq!(parsed_nested.refresh_token.as_deref(), Some("refresh-b"));
        assert_eq!(parsed_nested.scopes.as_deref(), Some("orders"));
        assert_eq!(
            parsed_nested.expires_at,
            Some(now + chrono::Duration::seconds(3600))
        );
    }

    #[tokio::test]
    async fn pull_refunds_should_return_mock_orders() {
        let adapter = Adapter::new();
        let mut shop = dummy_shop();
        shop.auth_meta = serde_json::json!({
            "mock_orders": [
                {
                    "order_sn": "PDD-ORDER-001",
                    "created_time": 1704067200,
                    "updated_at": 1704068200,
                    "confirm_status": 1,
                    "order_status": 2,
                    "refund_status": 2,
                    "after_sales_status": 2,
                    "pay_amount": 12.34
                }
            ]
        });

        let result = PlatformAdapter::pull_refunds(&adapter, &shop, SyncCursor::default())
            .await
            .expect("pull refunds");

        assert_eq!(result.orders.len(), 1);
        assert!(result.latest_time.is_some());
        assert_eq!(result.next_cursor, None);
    }

    #[tokio::test]
    async fn pull_refunds_should_filter_non_after_sales_orders() {
        let adapter = Adapter::new();
        let mut shop = dummy_shop();
        shop.auth_meta = serde_json::json!({
            "mock_orders": [
                {
                    "order_sn": "PDD-ORDER-NORMAL-001",
                    "created_time": 1704067200,
                    "updated_at": 1704068200,
                    "confirm_status": 1,
                    "order_status": 2,
                    "refund_status": 1,
                    "after_sales_status": 0,
                    "pay_amount": 12.34
                },
                {
                    "order_sn": "PDD-ORDER-REFUND-002",
                    "created_time": 1704069200,
                    "updated_at": 1704070200,
                    "confirm_status": 1,
                    "order_status": 2,
                    "refund_status": 2,
                    "after_sales_status": 2,
                    "pay_amount": 12.34
                }
            ]
        });

        let result = PlatformAdapter::pull_refunds(&adapter, &shop, SyncCursor::default())
            .await
            .expect("pull refunds");
        assert_eq!(result.orders.len(), 1);
        assert_eq!(result.orders[0].platform_order_id, "PDD-ORDER-REFUND-002");
    }

    #[tokio::test]
    async fn pull_refunds_should_use_refund_api_and_write_meta() {
        let server = MockServer::start();
        let refund_list = serde_json::json!({
            "data": {
                "list": [
                    {
                        "order_sn": "PDD-ORDER-API-001",
                        "created_time": 1700000000,
                        "updated_at": 1700000600,
                        "confirm_status": 1,
                        "order_status": 2,
                        "refund_status": 2,
                        "after_sales_status": 2,
                        "pay_amount": 12.34,
                        "refund_reason": "破损",
                        "responsibility": "seller",
                        "evidence_urls": ["https://example.com/pdd-proof.jpg"],
                        "refund_updated_at": 1700000600,
                        "after_sales_id": "PDD-AS-API-001",
                        "item_list": [
                            {
                                "goods_id": 1001,
                                "sku_id": 2001,
                                "goods_name": "退款商品",
                                "goods_count": 1,
                                "goods_price": 12.34,
                                "outer_id": "SKU-API-1",
                                "refund_status": 2,
                                "after_sales_status": 2,
                                "refund_amount": 12.34,
                                "after_sales_id": "PDD-AS-API-001"
                            }
                        ]
                    }
                ],
                "cursor": "2",
                "has_more": true
            }
        });
        let _refund = server.mock(|when, then| {
            when.method(POST).body_contains("pdd.refund.list");
            then.status(200).json_body(refund_list.clone());
        });

        let adapter = Adapter::new();
        let mut shop = dummy_shop();
        let mut creds = shop.credentials.as_object().cloned().unwrap_or_default();
        creds.insert(
            "gateway_url".to_string(),
            serde_json::Value::String(server.base_url()),
        );
        creds.insert(
            "refund_api_enabled".to_string(),
            serde_json::Value::Bool(true),
        );
        creds.insert(
            "refund_list_method".to_string(),
            serde_json::Value::String("pdd.refund.list".to_string()),
        );
        shop.credentials = serde_json::Value::Object(creds);
        shop.auth_meta = serde_json::json!({});

        let result = PlatformAdapter::pull_refunds(
            &adapter,
            &shop,
            SyncCursor {
                cursor: None,
                last_time: None,
            },
        )
        .await
        .expect("pull refunds via api");

        assert_eq!(result.orders.len(), 1);
        assert_eq!(result.next_cursor.as_deref(), Some("2"));
        assert!(result.latest_time.is_some());
        let meta = result.orders[0].items[0]
            .raw
            .get("_erp_refund_meta")
            .expect("refund meta exists");
        assert_eq!(meta.get("reason").and_then(|v| v.as_str()), Some("破损"));
        assert_eq!(
            meta.get("responsibility").and_then(|v| v.as_str()),
            Some("seller")
        );
        assert_eq!(
            meta.get("source_refund_id").and_then(|v| v.as_str()),
            Some("PDD-AS-API-001")
        );
        assert_eq!(
            meta.get("source_status").and_then(|v| v.as_str()),
            Some("2")
        );
    }

    #[tokio::test]
    async fn pull_refunds_should_fallback_when_refund_api_failed() {
        let server = MockServer::start();
        let _refund_fail = server.mock(|when, then| {
            when.method(POST).body_contains("pdd.refund.list");
            then.status(500).body("mock fail");
        });

        let adapter = Adapter::new();
        let mut shop = dummy_shop();
        let mut creds = shop.credentials.as_object().cloned().unwrap_or_default();
        creds.insert(
            "gateway_url".to_string(),
            serde_json::Value::String(server.base_url()),
        );
        creds.insert(
            "refund_api_enabled".to_string(),
            serde_json::Value::Bool(true),
        );
        creds.insert(
            "refund_list_method".to_string(),
            serde_json::Value::String("pdd.refund.list".to_string()),
        );
        shop.credentials = serde_json::Value::Object(creds);
        shop.auth_meta = serde_json::json!({
            "mock_orders": [
                {
                    "order_sn": "PDD-ORDER-FALLBACK-001",
                    "created_time": 1700000000,
                    "updated_at": 1700000600,
                    "confirm_status": 1,
                    "order_status": 2,
                    "refund_status": 2,
                    "after_sales_status": 2,
                    "pay_amount": 12.34
                }
            ]
        });

        let result = PlatformAdapter::pull_refunds(&adapter, &shop, SyncCursor::default())
            .await
            .expect("pull refunds with fallback");
        assert_eq!(result.orders.len(), 1);
        assert_eq!(result.orders[0].platform_order_id, "PDD-ORDER-FALLBACK-001");
    }

    #[tokio::test]
    async fn pull_refunds_should_keep_cursor_latest_time_consistent() {
        let server = MockServer::start();
        let updated_at = Utc::now().timestamp() + 3600;
        let refund_list = serde_json::json!({
            "data": {
                "list": [
                    {
                        "order_sn": "PDD-ORDER-CURSOR-001",
                        "created_time": updated_at - 3600,
                        "updated_at": updated_at,
                        "confirm_status": 1,
                        "order_status": 2,
                        "refund_status": 2,
                        "after_sales_status": 2,
                        "pay_amount": 12.34
                    }
                ],
                "cursor": "8",
                "has_more": true
            }
        });
        let _refund = server.mock(|when, then| {
            when.method(POST).body_contains("pdd.refund.list");
            then.status(200).json_body(refund_list.clone());
        });

        let adapter = Adapter::new();
        let mut shop = dummy_shop();
        let mut creds = shop.credentials.as_object().cloned().unwrap_or_default();
        creds.insert(
            "gateway_url".to_string(),
            serde_json::Value::String(server.base_url()),
        );
        creds.insert(
            "refund_api_enabled".to_string(),
            serde_json::Value::Bool(true),
        );
        creds.insert(
            "refund_list_method".to_string(),
            serde_json::Value::String("pdd.refund.list".to_string()),
        );
        shop.credentials = serde_json::Value::Object(creds);
        shop.auth_meta = serde_json::json!({});

        let last_time = Utc::now() - chrono::Duration::days(2);
        let result = PlatformAdapter::pull_refunds(
            &adapter,
            &shop,
            SyncCursor {
                cursor: Some("7".to_string()),
                last_time: Some(last_time),
            },
        )
        .await
        .expect("pull refunds via api");

        assert_eq!(result.next_cursor.as_deref(), Some("8"));
        assert!(result.latest_time.is_some());
        assert!(result.latest_time.expect("latest time") > last_time);
    }
}
