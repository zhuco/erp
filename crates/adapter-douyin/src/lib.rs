use async_trait::async_trait;
use chrono::Utc;
use erp_adapter::{
    AdapterError, OrderAdapter, PlatformAdapter, PlatformTokenPayload, PullOrdersResult,
    PullProductsResult, PullRefundsResult, ShipOrderRequest, ShipOrderResponse, ShopAuthContext,
    SyncCursor, SyncStockRequest, SyncStockResponse, TokenRefreshResult, WebhookAdapter,
};
use erp_common::{
    crypto::{hmac_sha256_hex_lower, md5_hex_lower},
    http::{header_get, parse_json},
    time::parse_platform_time,
};
use erp_domain::{OrderItemProjection, OrderProjection};
use erp_types::{AfterSaleState, OrderStatus, Platform, RefundState};
use reqwest::header::{HeaderMap, RETRY_AFTER};
use reqwest::StatusCode;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// Douyin shop (抖店) `order_status` enums.
///
/// From official FAQ / order sync guide:
/// - 1: 待确认/待支付
/// - 103: 部分支付
/// - 105: 已支付
/// - 2: 备货中(待发货)
/// - 101: 部分发货
/// - 3: 已发货(全部发货)
/// - 4: 已取消
/// - 5: 已完成(已收货)
/// - 21: 发货前退款完结
/// - 22: 发货后退款完结
/// - 39: 收货后退款完结
#[allow(dead_code)]
const DY_ORDER_STATUS_CODES: &[i64] = &[1, 103, 105, 2, 101, 3, 4, 5, 21, 22, 39];

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

    fn creds_app_key(shop: &ShopAuthContext) -> Result<String, AdapterError> {
        shop.credentials
            .get("app_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AdapterError::Auth("missing credentials.app_key".into()))
    }

    fn creds_app_secret(shop: &ShopAuthContext) -> Result<String, AdapterError> {
        shop.credentials
            .get("app_secret")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AdapterError::Auth("missing credentials.app_secret".into()))
    }

    fn creds_access_token(shop: &ShopAuthContext) -> Option<String> {
        shop.credentials
            .get("access_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    fn gateway_base(shop: &ShopAuthContext) -> String {
        shop.credentials
            .get("gateway_base")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            // NOTE: some environments use https://openapi-fxg.jinritemai.com
            .unwrap_or_else(|| "https://openapi.jinritemai.com".to_string())
    }

    fn order_detail_path(shop: &ShopAuthContext) -> String {
        shop.credentials
            .get("order_detail_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "/order/orderDetail".to_string())
    }

    fn sign_method(shop: &ShopAuthContext) -> String {
        shop.credentials
            .get("sign_method")
            .and_then(|v| v.as_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_else(|| "md5".to_string())
    }

    fn webhook_secret(shop: &ShopAuthContext) -> Option<String> {
        shop.credentials
            .get("webhook_secret")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    fn order_list_path(shop: &ShopAuthContext) -> String {
        shop.credentials
            .get("order_list_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "/order/searchList".to_string())
    }

    fn order_list_method(shop: &ShopAuthContext) -> String {
        shop.credentials
            .get("order_list_method")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "order.searchList".to_string())
    }

    fn order_list_page_size(shop: &ShopAuthContext) -> i64 {
        shop.credentials
            .get("order_list_page_size")
            .and_then(|v| v.as_i64())
            .filter(|v| *v > 0)
            .unwrap_or(50)
    }

    fn ship_path(shop: &ShopAuthContext) -> String {
        shop.credentials
            .get("ship_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "/order/logisticsAdd".to_string())
    }

    fn ship_method(shop: &ShopAuthContext) -> String {
        shop.credentials
            .get("ship_method")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "order.logisticsAdd".to_string())
    }

    fn ship_carrier_field(shop: &ShopAuthContext) -> String {
        shop.credentials
            .get("ship_carrier_field")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "logistics_code".to_string())
    }

    fn ship_tracking_field(shop: &ShopAuthContext) -> String {
        shop.credentials
            .get("ship_tracking_field")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "tracking_no".to_string())
    }

    fn stock_path(shop: &ShopAuthContext) -> String {
        shop.credentials
            .get("stock_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "/sku/syncStock".to_string())
    }

    fn stock_method(shop: &ShopAuthContext) -> String {
        shop.credentials
            .get("stock_method")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "sku.syncStock".to_string())
    }

    fn stock_sku_field(shop: &ShopAuthContext) -> String {
        shop.credentials
            .get("stock_sku_field")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "sku_id".to_string())
    }

    fn stock_qty_field(shop: &ShopAuthContext) -> String {
        shop.credentials
            .get("stock_qty_field")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "stock_num".to_string())
    }

    fn stock_warehouse_field(shop: &ShopAuthContext) -> Option<String> {
        shop.credentials
            .get("stock_warehouse_field")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
    }

    fn token_refresh_url(shop: &ShopAuthContext) -> String {
        if let Some(url) = shop
            .credentials
            .get("token_refresh_url")
            .and_then(|v| v.as_str())
        {
            return url.to_string();
        }
        let base = shop
            .credentials
            .get("token_base")
            .or_else(|| shop.credentials.get("oauth_base"))
            .and_then(|v| v.as_str())
            .unwrap_or("https://openapi.jinritemai.com");
        let path = shop
            .credentials
            .get("token_refresh_path")
            .and_then(|v| v.as_str())
            .unwrap_or("/oauth2/refresh_token");
        format!("{base}{path}")
    }

    fn token_app_key_field(shop: &ShopAuthContext) -> String {
        shop.credentials
            .get("token_app_key_field")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "app_id".to_string())
    }

    fn token_refresh_method(shop: &ShopAuthContext) -> String {
        shop.credentials
            .get("token_refresh_method")
            .and_then(|v| v.as_str())
            .map(|s| s.to_uppercase())
            .unwrap_or_else(|| "POST".to_string())
    }

    fn token_refresh_secret_field(shop: &ShopAuthContext) -> Option<String> {
        shop.credentials
            .get("token_refresh_secret_field")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Canonicalize JSON by sorting object keys recursively.
    fn sort_json(v: &Value) -> Value {
        match v {
            Value::Object(map) => {
                let mut keys: Vec<String> = map.keys().cloned().collect();
                keys.sort();
                let mut out = Map::new();
                for k in keys {
                    if let Some(child) = map.get(&k) {
                        out.insert(k, Self::sort_json(child));
                    }
                }
                Value::Object(out)
            }
            Value::Array(arr) => Value::Array(arr.iter().map(Self::sort_json).collect()),
            other => other.clone(),
        }
    }

    /// Douyin shop API signature.
    ///
    /// Open platform convention:
    /// - sort params by key (exclude `sign`)
    /// - concat key+value into a single string
    /// - md5: md5(secret + concat + secret)
    /// - hmac-sha256: hmac_sha256(concat, secret)
    /// - hex is LOWERCASE
    fn douyin_sign(params: &BTreeMap<String, String>, secret: &str, sign_method: &str) -> String {
        let mut concat = String::new();
        for (k, v) in params {
            if k == "sign" {
                continue;
            }
            if v.is_empty() {
                continue;
            }
            concat.push_str(k);
            concat.push_str(v);
        }

        match sign_method.to_lowercase().as_str() {
            "hmac-sha256" | "hmac_sha256" => {
                hmac_sha256_hex_lower(secret.as_bytes(), concat.as_bytes())
            }
            _ => {
                let mut bytes = Vec::with_capacity(secret.len() * 2 + concat.len());
                bytes.extend_from_slice(secret.as_bytes());
                bytes.extend_from_slice(concat.as_bytes());
                bytes.extend_from_slice(secret.as_bytes());
                md5_hex_lower(&bytes)
            }
        }
    }

    fn retry_after_ms(headers: &HeaderMap) -> Option<u64> {
        headers
            .get(RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|raw| raw.parse::<u64>().ok())
            .map(|secs| secs.saturating_mul(1000))
    }

    fn api_message(v: &Value) -> String {
        v.get("message")
            .or_else(|| v.get("msg"))
            .or_else(|| v.get("err_msg"))
            .or_else(|| v.get("err_tips"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown douyin error")
            .to_string()
    }

    fn extract_api_error(v: &Value) -> Option<(i64, String)> {
        if let Some(code) = v.get("code").and_then(|x| x.as_i64()) {
            if code != 0 {
                return Some((code, Self::api_message(v)));
            }
        }
        if let Some(code) = v
            .get("err_no")
            .or_else(|| v.get("err_code"))
            .and_then(|x| x.as_i64())
        {
            if code != 0 {
                return Some((code, Self::api_message(v)));
            }
        }
        None
    }

    fn map_http_error(status: StatusCode, body: &[u8], headers: &HeaderMap) -> AdapterError {
        let body_text = String::from_utf8_lossy(body);
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return AdapterError::Auth(format!(
                "douyin http auth error status={} body={body_text}",
                status
            ));
        }
        if status == StatusCode::TOO_MANY_REQUESTS {
            return AdapterError::RateLimited {
                retry_after_ms: Self::retry_after_ms(headers).unwrap_or(60_000),
            };
        }
        if status.is_client_error() {
            return AdapterError::BadRequest(format!(
                "douyin http bad request status={} body={body_text}",
                status
            ));
        }
        AdapterError::Temporary(format!(
            "douyin http error status={} body={body_text}",
            status
        ))
    }

    fn map_api_error(code: i64, message: &str, retry_after_ms: Option<u64>) -> AdapterError {
        let lower = message.to_lowercase();
        let is_auth = matches!(code, 1001 | 1002 | 1003 | 1004 | 1005 | 2001 | 2002)
            || lower.contains("access_token")
            || lower.contains("refresh_token")
            || lower.contains("app_key")
            || lower.contains("app_id")
            || message.contains("授权")
            || message.contains("权限");
        if is_auth {
            return AdapterError::Auth(format!("douyin auth error {code}: {message}"));
        }

        let is_rate_limited = matches!(code, 1009 | 1010 | 429 | 2005 | 2006)
            || lower.contains("rate")
            || message.contains("频繁")
            || message.contains("限流");
        if is_rate_limited {
            return AdapterError::RateLimited {
                retry_after_ms: retry_after_ms.unwrap_or(60_000),
            };
        }

        let is_bad_request = matches!(code, 1000 | 1008 | 400 | 2003 | 2004)
            || lower.contains("param")
            || lower.contains("invalid")
            || message.contains("参数")
            || message.contains("非法");
        if is_bad_request {
            return AdapterError::BadRequest(format!("douyin bad request {code}: {message}"));
        }

        AdapterError::Temporary(format!("douyin api error {code}: {message}"))
    }

    async fn call_openapi(
        &self,
        shop: &ShopAuthContext,
        method: &str,
        path: &str,
        biz: Value,
    ) -> Result<Value, AdapterError> {
        let app_key = Self::creds_app_key(shop)?;
        let app_secret = Self::creds_app_secret(shop)?;
        let access_token = Self::creds_access_token(shop)
            .ok_or_else(|| AdapterError::Auth("missing credentials.access_token".into()))?;
        let base = Self::gateway_base(shop);
        let sign_method = Self::sign_method(shop);

        let timestamp = Utc::now().timestamp().to_string();
        let v = "2".to_string();
        let biz_sorted = Self::sort_json(&biz);
        let param_json = serde_json::to_string(&biz_sorted)
            .map_err(|e| AdapterError::BadRequest(format!("param_json: {e}")))?;

        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("app_key".to_string(), app_key);
        params.insert("access_token".to_string(), access_token);
        params.insert("method".to_string(), method.to_string());
        params.insert("param_json".to_string(), param_json);
        params.insert("timestamp".to_string(), timestamp);
        params.insert("v".to_string(), v);
        params.insert("sign_method".to_string(), sign_method.clone());

        let sign = Self::douyin_sign(&params, &app_secret, &sign_method);
        params.insert("sign".to_string(), sign);

        let url = format!("{base}{path}");
        let resp = self
            .client
            .post(url)
            .form(&params)
            .send()
            .await
            .map_err(|e| AdapterError::Temporary(format!("http: {e}")))?;

        let status = resp.status();
        let headers = resp.headers().clone();
        let body = resp
            .bytes()
            .await
            .map_err(|e| AdapterError::Temporary(format!("read: {e}")))?;

        if !status.is_success() {
            return Err(Self::map_http_error(status, &body, &headers));
        }

        let v: Value = serde_json::from_slice(&body)
            .map_err(|e| AdapterError::Temporary(format!("json: {e}")))?;
        if let Some((code, message)) = Self::extract_api_error(&v) {
            return Err(Self::map_api_error(
                code,
                &message,
                Self::retry_after_ms(&headers),
            ));
        }

        Ok(v)
    }

    fn map_order_status(code: i64) -> OrderStatus {
        // Source: official FAQ / order sync guide.
        match code {
            1 | 103 => OrderStatus::PendingPayment,
            105 | 2 => OrderStatus::PendingShipment,
            101 => OrderStatus::PartiallyShipped,
            3 => OrderStatus::Shipped,
            4 => OrderStatus::Cancelled,
            5 => OrderStatus::Completed,
            21 | 22 | 39 => OrderStatus::Refunded,
            _ => OrderStatus::Unknown,
        }
    }

    fn parse_amount_to_cent(v: &Value) -> Option<i64> {
        match v {
            Value::Number(n) => n.as_i64(),
            Value::String(s) => {
                // Douyin shop usually uses integer cents string.
                // Tolerate yuan decimals when some gateways return floats.
                if s.contains('.') {
                    yuan_str_to_cent(s)
                } else {
                    s.parse::<i64>().ok()
                }
            }
            _ => None,
        }
    }

    fn map_refund_state(refund_status: Option<i64>, order_status_code: Option<i64>) -> RefundState {
        // Douyin shop returns multiple possible fields; we keep the mapping deliberately coarse.
        // Order-status based fallback: 21/22/39 are "refund completed" states.
        if matches!(order_status_code, Some(21 | 22 | 39)) {
            return RefundState::Success;
        }
        match refund_status {
            None | Some(0) => RefundState::None,
            Some(1) => RefundState::Processing,
            Some(2) => RefundState::Success,
            Some(3) => RefundState::Closed,
            Some(_) => RefundState::Unknown,
        }
    }

    fn map_after_sale_state(after_sale_status: Option<i64>) -> AfterSaleState {
        match after_sale_status {
            None | Some(0) => AfterSaleState::None,
            Some(1) | Some(2) => AfterSaleState::Processing,
            Some(3) => AfterSaleState::Success,
            Some(4) => AfterSaleState::Closed,
            Some(_) => AfterSaleState::Unknown,
        }
    }

    fn find_first_order_object(v: &Value, depth: usize) -> Option<&Value> {
        if depth == 0 {
            return None;
        }
        if let Some(obj) = v.as_object() {
            if obj.contains_key("order_id") || obj.contains_key("orderId") {
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

    fn is_risk_processing(order: &Value) -> bool {
        // Official guide: during risk-control, order query returns `sku_order_tag_ui.key=risk_processing`.
        fn tag_is_risk(tag: &Value) -> bool {
            tag.get("key")
                .and_then(|v| v.as_str())
                .map(|s| s == "risk_processing")
                .unwrap_or(false)
        }

        if let Some(tag) = order.get("sku_order_tag_ui") {
            if tag_is_risk(tag) {
                return true;
            }
        }
        if let Some(arr) = order.get("sku_order_list").and_then(|v| v.as_array()) {
            for sku in arr {
                if let Some(tag) = sku.get("sku_order_tag_ui") {
                    if tag_is_risk(tag) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn map_order_to_projection(
        shop: &ShopAuthContext,
        order: &Value,
    ) -> Result<OrderProjection, AdapterError> {
        let platform_order_id = order
            .get("order_id")
            .and_then(|v| {
                if v.is_string() {
                    v.as_str().map(|s| s.to_string())
                } else {
                    v.as_i64().map(|n| n.to_string())
                }
            })
            .or_else(|| {
                order
                    .get("orderId")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .ok_or_else(|| AdapterError::BadRequest("missing order_id".into()))?;

        let order_created_at = order
            .get("create_time")
            .or_else(|| order.get("create_time_sec"))
            .or_else(|| order.get("created"))
            .and_then(parse_platform_time)
            .unwrap_or_else(Utc::now);

        let platform_updated_at = order
            .get("update_time")
            .or_else(|| order.get("modified"))
            .and_then(parse_platform_time);

        let status_code = order
            .get("order_status")
            .or_else(|| order.get("main_status"))
            .and_then(|v| v.as_i64());
        let mut status = status_code
            .map(Self::map_order_status)
            .unwrap_or(OrderStatus::Unknown);

        // risk-control override
        if Self::is_risk_processing(order) {
            status = OrderStatus::OnHold;
        }

        let total_amount_cent = order
            .get("pay_amount")
            .or_else(|| order.get("order_amount"))
            .or_else(|| order.get("total_pay_amount"))
            .and_then(Self::parse_amount_to_cent)
            .unwrap_or(0);

        // Best-effort money breakdown (fields vary by API version & permissions).
        let goods_amount_cent = order
            .get("order_amount")
            .or_else(|| order.get("goods_amount"))
            .or_else(|| order.get("total_amount"))
            .and_then(Self::parse_amount_to_cent);

        let postage_amount_cent = order
            .get("post_amount")
            .or_else(|| order.get("postage"))
            .or_else(|| order.get("shipping_amount"))
            .and_then(Self::parse_amount_to_cent);

        let discount_amount_cent = order
            .get("promotion_amount")
            .or_else(|| order.get("discount_amount"))
            .or_else(|| order.get("coupon_amount"))
            .and_then(Self::parse_amount_to_cent);

        let platform_discount_cent = order
            .get("platform_discount_amount")
            .or_else(|| order.get("platform_discount"))
            .and_then(Self::parse_amount_to_cent);

        let seller_discount_cent = order
            .get("shop_discount_amount")
            .or_else(|| order.get("seller_discount_amount"))
            .or_else(|| order.get("seller_discount"))
            .and_then(Self::parse_amount_to_cent);

        let coupon_discount_cent = order
            .get("coupon_amount")
            .or_else(|| order.get("coupon_discount_amount"))
            .and_then(Self::parse_amount_to_cent);

        // Time fields.
        let pay_at = order
            .get("pay_time")
            .or_else(|| order.get("pay_time_sec"))
            .or_else(|| order.get("payment_time"))
            .and_then(parse_platform_time);

        let ship_at = order
            .get("ship_time")
            .or_else(|| order.get("shipping_time"))
            .or_else(|| order.get("consign_time"))
            .and_then(parse_platform_time);

        let finish_at = order
            .get("finish_time")
            .or_else(|| order.get("confirm_time"))
            .or_else(|| order.get("receive_time"))
            .and_then(parse_platform_time);

        let cancel_at = order
            .get("cancel_time")
            .or_else(|| order.get("canceled_time"))
            .and_then(parse_platform_time);

        let buyer_message = order
            .get("buyer_words")
            .or_else(|| order.get("buyer_message"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let seller_remark = order
            .get("seller_words")
            .or_else(|| order.get("seller_remark"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        // Logistics (optional; may require additional permissions/APIs).
        let (logistics_company, tracking_no) =
            if let Some(arr) = order.get("logistics_info").and_then(|v| v.as_array()) {
                let first = arr.first().cloned().unwrap_or(Value::Null);
                let company = first
                    .get("company")
                    .or_else(|| first.get("company_name"))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                let track = first
                    .get("tracking_no")
                    .or_else(|| first.get("tracking_number"))
                    .or_else(|| first.get("waybill_no"))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                (company, track)
            } else {
                (
                    order
                        .get("logistics_company")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    order
                        .get("tracking_no")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                )
            };

        let buyer_name = order
            .get("open_id")
            .or_else(|| order.get("buyer_id"))
            .or_else(|| order.get("user_id"))
            .and_then(|v| {
                if v.is_string() {
                    v.as_str().map(|s| s.to_string())
                } else {
                    v.as_i64().map(|n| n.to_string())
                }
            });

        // Receiver fields: use masked fields by default.
        let receiver_name = order
            .get("mask_post_receiver")
            .or_else(|| order.get("post_receiver"))
            .or_else(|| order.get("receiver_name"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let receiver_phone = order
            .get("mask_post_tel")
            .or_else(|| order.get("post_tel"))
            .or_else(|| order.get("receiver_phone"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let receiver_address = {
            // Official orderDetail returns mask_post_addr as an object.
            if let Some(addr) = order.get("mask_post_addr") {
                let parts = [
                    addr.get("province").and_then(|v| v.as_str()),
                    addr.get("city").and_then(|v| v.as_str()),
                    addr.get("district").and_then(|v| v.as_str()),
                    addr.get("street").and_then(|v| v.as_str()),
                    addr.get("detail").and_then(|v| v.as_str()),
                ];
                let joined = parts
                    .into_iter()
                    .flatten()
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
                if !joined.is_empty() {
                    Some(joined)
                } else {
                    None
                }
            } else {
                None
            }
        };

        let mut items: Vec<OrderItemProjection> = Vec::new();
        let items_v = order
            .get("sku_order_list")
            .or_else(|| order.get("product_order_list"))
            .or_else(|| order.get("item_list"));
        if let Some(arr) = items_v.and_then(|v| v.as_array()) {
            for (idx, it) in arr.iter().enumerate() {
                let sku_order_id = it.get("sku_order_id").and_then(|v| {
                    if v.is_string() {
                        v.as_str().map(|s| s.to_string())
                    } else {
                        v.as_i64().map(|n| n.to_string())
                    }
                });

                let out_sku_id = it
                    .get("out_sku_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let sku_id = it.get("sku_id").and_then(|v| {
                    if v.is_string() {
                        v.as_str().map(|s| s.to_string())
                    } else {
                        v.as_i64().map(|n| n.to_string())
                    }
                });

                let qty = it
                    .get("item_num")
                    .or_else(|| it.get("num"))
                    .or_else(|| it.get("quantity"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1) as i32;

                let price_cent = it
                    .get("goods_price")
                    .or_else(|| it.get("sale_price"))
                    .or_else(|| it.get("price"))
                    .and_then(Self::parse_amount_to_cent)
                    .unwrap_or(0);

                let amount_cent = it
                    .get("order_amount")
                    .or_else(|| it.get("total_price"))
                    .or_else(|| it.get("amount"))
                    .and_then(Self::parse_amount_to_cent)
                    .unwrap_or(price_cent.saturating_mul(qty as i64));

                let pay_amount_cent = it
                    .get("pay_amount")
                    .or_else(|| it.get("real_pay_amount"))
                    .or_else(|| it.get("total_pay_amount"))
                    .and_then(Self::parse_amount_to_cent);

                let discount_cent = it
                    .get("promotion_amount")
                    .or_else(|| it.get("discount_amount"))
                    .or_else(|| it.get("coupon_amount"))
                    .and_then(Self::parse_amount_to_cent)
                    .or_else(|| pay_amount_cent.map(|p| (amount_cent - p).max(0)));

                // After-sale / refund info (sub-order level), best-effort.
                let after_sale_info = it
                    .get("after_sale_info")
                    .or_else(|| it.get("aftersale_info"));

                let refund_status = after_sale_info
                    .and_then(|a| a.get("refund_status").or_else(|| a.get("refundStatus")))
                    .or_else(|| it.get("refund_status").or_else(|| it.get("refundStatus")))
                    .and_then(|v| v.as_i64());

                let after_sale_status = after_sale_info
                    .and_then(|a| {
                        a.get("after_sale_status")
                            .or_else(|| a.get("afterSaleStatus"))
                    })
                    .or_else(|| {
                        it.get("after_sale_status")
                            .or_else(|| it.get("afterSaleStatus"))
                    })
                    .and_then(|v| v.as_i64());

                let refund_state = Self::map_refund_state(refund_status, status_code);
                let after_sale_state = Self::map_after_sale_state(after_sale_status);

                let refund_amount_cent = after_sale_info
                    .and_then(|a| a.get("refund_amount").or_else(|| a.get("refundAmount")))
                    .or_else(|| it.get("refund_amount"))
                    .and_then(Self::parse_amount_to_cent);

                let after_sale_type = after_sale_info
                    .and_then(|a| {
                        a.get("after_sale_type_desc")
                            .or_else(|| a.get("after_sale_type"))
                    })
                    .or_else(|| {
                        it.get("after_sale_type_desc")
                            .or_else(|| it.get("after_sale_type"))
                    })
                    .and_then(|v| {
                        if v.is_string() {
                            v.as_str().map(|s| s.to_string())
                        } else {
                            v.as_i64().map(|n| n.to_string())
                        }
                    });

                let after_sale_id = after_sale_info
                    .and_then(|a| a.get("after_sale_id").or_else(|| a.get("aftersale_id")))
                    .or_else(|| it.get("after_sale_id").or_else(|| it.get("aftersale_id")))
                    .and_then(|v| {
                        if v.is_string() {
                            v.as_str().map(|s| s.to_string())
                        } else {
                            v.as_i64().map(|n| n.to_string())
                        }
                    });

                let title = it
                    .get("product_name")
                    .or_else(|| it.get("sku_name"))
                    .or_else(|| it.get("title"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Stable line_id: sku_order_id > out_sku_id > sku_id > fallback.
                let line_id = sku_order_id
                    .clone()
                    .or(out_sku_id.clone())
                    .or(sku_id.clone())
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
        order_id: &str,
    ) -> Result<OrderProjection, AdapterError> {
        let path = Self::order_detail_path(shop);
        let method = "order.orderDetail";
        let biz = serde_json::json!({
            "order_id": order_id,
        });
        let v = self.call_openapi(shop, method, &path, biz).await?;
        let order_obj = Self::find_first_order_object(&v, 12)
            .ok_or_else(|| AdapterError::Temporary("cannot find order object".into()))?;
        Self::map_order_to_projection(shop, order_obj)
    }

    fn value_to_string(v: &Value) -> Option<String> {
        if let Some(s) = v.as_str() {
            return Some(s.to_string());
        }
        if let Some(n) = v.as_i64() {
            return Some(n.to_string());
        }
        None
    }

    fn extract_order_list(v: &Value) -> (Vec<Value>, Option<String>, bool) {
        let data = v.get("data").unwrap_or(v);
        let list = data
            .get("shop_order_list")
            .or_else(|| data.get("order_list"))
            .or_else(|| data.get("list"))
            .or_else(|| data.get("orders"))
            .or_else(|| data.get("items"));

        let mut items = Vec::new();
        if let Some(arr) = list.and_then(|v| v.as_array()) {
            items.extend(arr.iter().cloned());
        } else if let Some(arr) = data.get("order_id_list").and_then(|v| v.as_array()) {
            items.extend(arr.iter().map(|id| serde_json::json!({ "order_id": id })));
        }

        let mut has_more = data
            .get("has_more")
            .or_else(|| data.get("has_next"))
            .or_else(|| data.get("has_next_page"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut next_cursor = data
            .get("cursor")
            .or_else(|| data.get("next_cursor"))
            .or_else(|| data.get("next"))
            .and_then(Self::value_to_string);

        let total = data.get("total").and_then(|v| v.as_i64());
        let page = data
            .get("page")
            .or_else(|| data.get("page_no"))
            .and_then(|v| v.as_i64());
        let size = data
            .get("size")
            .or_else(|| data.get("page_size"))
            .and_then(|v| v.as_i64());
        if !has_more {
            if let (Some(total), Some(page), Some(size)) = (total, page, size) {
                has_more = page * size < total;
                if has_more && next_cursor.is_none() {
                    next_cursor = Some((page + 1).to_string());
                }
            }
        }

        (items, next_cursor, has_more)
    }

    async fn push_ship_order(
        &self,
        shop: &ShopAuthContext,
        request: ShipOrderRequest,
    ) -> Result<ShipOrderResponse, AdapterError> {
        let carrier_code = request
            .carrier_code
            .clone()
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| AdapterError::BadRequest("carrier_code required".into()))?;
        let tracking_no = request
            .tracking_no
            .clone()
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| AdapterError::BadRequest("tracking_no required".into()))?;

        let mut biz = serde_json::json!({
            "order_id": request.platform_order_id,
        });
        if let Some(map) = biz.as_object_mut() {
            map.insert(
                Self::ship_carrier_field(shop),
                Value::String(carrier_code.clone()),
            );
            map.insert(
                Self::ship_tracking_field(shop),
                Value::String(tracking_no.clone()),
            );
        }

        let method = Self::ship_method(shop);
        let path = Self::ship_path(shop);
        let _ = self.call_openapi(shop, &method, &path, biz).await?;

        Ok(ShipOrderResponse {
            tracking_no: Some(tracking_no),
        })
    }

    async fn push_sync_stock(
        &self,
        shop: &ShopAuthContext,
        request: SyncStockRequest,
    ) -> Result<SyncStockResponse, AdapterError> {
        if request.target_qty < 0 {
            return Err(AdapterError::BadRequest(
                "target_qty must be non-negative".into(),
            ));
        }

        let mut biz = serde_json::json!({});
        if let Some(map) = biz.as_object_mut() {
            map.insert(
                Self::stock_sku_field(shop),
                Value::String(request.platform_sku_id.clone()),
            );
            map.insert(
                Self::stock_qty_field(shop),
                Value::Number(request.target_qty.into()),
            );
            if let Some(field) = Self::stock_warehouse_field(shop) {
                map.insert(field, Value::String(request.warehouse_id.to_string()));
            }
        }

        let method = Self::stock_method(shop);
        let path = Self::stock_path(shop);
        let _ = self.call_openapi(shop, &method, &path, biz).await?;

        Ok(SyncStockResponse {
            target_qty: request.target_qty,
        })
    }
}

impl Default for Adapter {
    fn default() -> Self {
        Self::new()
    }
}

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
        Platform::Douyin
    }

    async fn verify_webhook(
        &self,
        shop: &ShopAuthContext,
        headers: &Value,
        body: &[u8],
    ) -> Result<(), AdapterError> {
        // Douyin shop event push: headers contain app-id and event-sign.
        let Some(got_sign) =
            header_get(headers, "event-sign").or_else(|| header_get(headers, "event_sign"))
        else {
            return Ok(());
        };

        let app_id = header_get(headers, "app-id")
            .or_else(|| header_get(headers, "app_id"))
            .unwrap_or_else(|| Self::creds_app_key(shop).unwrap_or_default());

        let secret = Self::webhook_secret(shop)
            .unwrap_or_else(|| Self::creds_app_secret(shop).unwrap_or_default());

        let mut bytes = Vec::with_capacity(app_id.len() + body.len() + secret.len());
        bytes.extend_from_slice(app_id.as_bytes());
        bytes.extend_from_slice(body);
        bytes.extend_from_slice(secret.as_bytes());

        let computed = md5_hex_lower(&bytes);
        if !computed.eq_ignore_ascii_case(&got_sign) {
            return Err(AdapterError::Auth("invalid douyin event-sign".into()));
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
        // Douyin event body is commonly a JSON array.
        let Some(v) = parse_json(body) else {
            return Ok(vec![]);
        };

        let mut out: Vec<OrderProjection> = Vec::new();

        if let Some(arr) = v.as_array() {
            for msg in arr {
                // If message already contains full order detail, map directly.
                if let Some(order_obj) = Self::find_first_order_object(msg, 8) {
                    // Heuristic: if it contains items list, treat as order snapshot.
                    if order_obj.get("sku_order_list").is_some()
                        || order_obj.get("product_order_list").is_some()
                    {
                        out.push(Self::map_order_to_projection(shop, order_obj)?);
                        continue;
                    }
                }

                // Otherwise, try to extract order_id and fetch.
                let order_id = msg
                    .get("order_id")
                    .or_else(|| msg.get("biz_id"))
                    .or_else(|| msg.pointer("/data/order_id"))
                    .and_then(|x| {
                        if x.is_string() {
                            x.as_str().map(|s| s.to_string())
                        } else {
                            x.as_i64().map(|n| n.to_string())
                        }
                    });

                if let Some(order_id) = order_id {
                    let o = self.fetch_order_detail(shop, &order_id).await?;
                    out.push(o);
                }
            }
            return Ok(out);
        }

        // If it's a single object, try map as order snapshot.
        if let Some(order_obj) = Self::find_first_order_object(&v, 12) {
            out.push(Self::map_order_to_projection(shop, order_obj)?);
        }

        Ok(out)
    }

    async fn ship_order(
        &self,
        shop: &ShopAuthContext,
        request: ShipOrderRequest,
    ) -> Result<ShipOrderResponse, AdapterError> {
        self.push_ship_order(shop, request).await
    }

    async fn sync_stock(
        &self,
        shop: &ShopAuthContext,
        request: SyncStockRequest,
    ) -> Result<SyncStockResponse, AdapterError> {
        self.push_sync_stock(shop, request).await
    }
}

#[async_trait]
impl OrderAdapter for Adapter {
    fn platform(&self) -> Platform {
        Platform::Douyin
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
        Platform::Douyin
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

    async fn sync_stock(
        &self,
        shop: &ShopAuthContext,
        request: SyncStockRequest,
    ) -> Result<SyncStockResponse, AdapterError> {
        WebhookAdapter::sync_stock(self, shop, request).await
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
        let refresh_token = token
            .refresh_token
            .clone()
            .ok_or_else(|| AdapterError::Auth("missing refresh_token".into()))?;
        let app_key = Self::creds_app_key(shop)?;
        let app_secret = Self::creds_app_secret(shop)?;
        let url = Self::token_refresh_url(shop);
        let app_key_field = Self::token_app_key_field(shop);
        let refresh_method = Self::token_refresh_method(shop);

        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert(app_key_field, app_key);
        params.insert("refresh_token".to_string(), refresh_token.clone());
        params.insert("grant_type".to_string(), "refresh_token".to_string());
        if let Some(secret_field) = Self::token_refresh_secret_field(shop) {
            params.insert(secret_field, app_secret);
        }

        let request = if refresh_method == "GET" {
            self.client.get(url).query(&params)
        } else {
            self.client.post(url).form(&params)
        };

        let resp = request
            .send()
            .await
            .map_err(|e| AdapterError::Temporary(format!("http: {e}")))?;
        let status = resp.status();
        let headers = resp.headers().clone();
        let body = resp
            .bytes()
            .await
            .map_err(|e| AdapterError::Temporary(format!("read: {e}")))?;
        if !status.is_success() {
            return Err(Self::map_http_error(status, &body, &headers));
        }

        let v: Value = serde_json::from_slice(&body)
            .map_err(|e| AdapterError::Temporary(format!("json: {e}")))?;
        if let Some((code, message)) = Self::extract_api_error(&v) {
            return Err(Self::map_api_error(
                code,
                &message,
                Self::retry_after_ms(&headers),
            ));
        }

        let data = v.get("data").unwrap_or(&v);
        let access_token = data
            .get("access_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AdapterError::Unexpected("missing access_token".into()))?;
        let refresh_token = data
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| token.refresh_token.clone());
        let expires_in = data.get("expires_in").and_then(|v| v.as_i64());
        let expires_at = expires_in.map(|secs| Utc::now() + chrono::Duration::seconds(secs));
        let scopes = data
            .get("scope")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| token.scopes.clone());

        Ok(TokenRefreshResult {
            access_token: Some(access_token),
            refresh_token,
            expires_at,
            scopes,
            raw: v,
        })
    }

    async fn pull_orders(
        &self,
        shop: &ShopAuthContext,
        cursor: SyncCursor,
    ) -> Result<PullOrdersResult, AdapterError> {
        let method = Self::order_list_method(shop);
        let path = Self::order_list_path(shop);
        let page_size = Self::order_list_page_size(shop);
        let now = Utc::now();
        let start_time = cursor
            .last_time
            .map(|t| t.timestamp())
            .unwrap_or_else(|| (now - chrono::Duration::hours(1)).timestamp());
        let end_time = now.timestamp();

        let mut biz = serde_json::json!({
            "update_time_start": start_time,
            "update_time_end": end_time,
            "start_time": start_time,
            "end_time": end_time,
            "size": page_size,
            "page_size": page_size,
        });

        let page = cursor
            .cursor
            .as_deref()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(1);
        let cursor_str = cursor
            .cursor
            .as_deref()
            .filter(|v| v.parse::<i64>().is_err())
            .map(|v| v.to_string());
        let use_page = cursor_str.is_none();

        if let Some(map) = biz.as_object_mut() {
            if use_page {
                map.insert("page".to_string(), Value::Number(page.into()));
            }
            if let Some(cursor_str) = cursor_str {
                map.insert("cursor".to_string(), Value::String(cursor_str));
            }
        }

        let response = self.call_openapi(shop, &method, &path, biz).await?;
        let (raw_list, mut next_cursor, has_more) = Self::extract_order_list(&response);

        let mut orders = Vec::new();
        let mut latest_time = cursor.last_time;
        for raw in raw_list {
            let proj =
                if raw.get("sku_order_list").is_some() || raw.get("product_order_list").is_some() {
                    Self::map_order_to_projection(shop, &raw)?
                } else if let Some(order_id) = raw
                    .get("order_id")
                    .or_else(|| raw.get("orderId"))
                    .and_then(Self::value_to_string)
                {
                    self.fetch_order_detail(shop, &order_id).await?
                } else {
                    continue;
                };

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

        if has_more {
            if next_cursor.is_none() && use_page {
                next_cursor = Some((page + 1).to_string());
            }
        } else {
            next_cursor = None;
        }

        Ok(PullOrdersResult {
            orders,
            next_cursor,
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
    use erp_common::crypto::md5_hex_lower;
    use erp_types::{Platform, ShopId, TenantId};

    fn dummy_shop() -> ShopAuthContext {
        ShopAuthContext {
            tenant_id: "00000000-0000-0000-0000-000000000000"
                .parse::<TenantId>()
                .unwrap(),
            shop_id: "00000000-0000-0000-0000-000000000000"
                .parse::<ShopId>()
                .unwrap(),
            platform: Platform::Douyin,
            platform_shop_id: "douyin-shop-001".to_string(),
            credentials: serde_json::json!({
                "app_key": "x",
                "app_secret": "y",
                "access_token": "z"
            }),
            auth_meta: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn douyin_verify_webhook_signature_ok() {
        let mut shop = dummy_shop();
        shop.credentials = serde_json::json!({
            "app_key": "app-123",
            "app_secret": "secret-456",
            "webhook_secret": "hook-789",
        });
        let body = br#"{"event":"order_update"}"#;
        let mut bytes = Vec::new();
        bytes.extend_from_slice("app-123".as_bytes());
        bytes.extend_from_slice(body);
        bytes.extend_from_slice("hook-789".as_bytes());
        let sign = md5_hex_lower(&bytes);
        let headers = serde_json::json!({
            "app-id": "app-123",
            "event-sign": sign,
        });

        Adapter::new()
            .verify_webhook(&shop, &headers, body)
            .await
            .expect("verify webhook");
    }

    #[test]
    fn douyin_order_projection_maps_string_amounts() {
        let shop = dummy_shop();
        let order = serde_json::json!({
            "order_id": "6919756900423669520",
            "order_status": 2,
            "create_time": 1704067200,
            "pay_amount": "12.34",
            "order_amount": "12.34",
            "sku_order_list": [
                {"out_sku_id": "SKU-OUT-2", "sku_id": 2002, "product_name": "测试商品", "item_num": 1, "goods_price": "12.34", "order_amount": "12.34"}
            ]
        });

        let proj = Adapter::map_order_to_projection(&shop, &order).expect("map");
        assert_eq!(proj.total_amount_cent, 1234);
        assert_eq!(proj.goods_amount_cent, Some(1234));
        assert_eq!(proj.items[0].price_cent, 1234);
        assert_eq!(proj.items[0].amount_cent, 1234);
    }

    #[test]
    fn douyin_order_status_enum_full_coverage() {
        for code in DY_ORDER_STATUS_CODES {
            let st = Adapter::map_order_status(*code);
            match *code {
                1 | 103 => assert_eq!(st, OrderStatus::PendingPayment),
                105 | 2 => assert_eq!(st, OrderStatus::PendingShipment),
                101 => assert_eq!(st, OrderStatus::PartiallyShipped),
                3 => assert_eq!(st, OrderStatus::Shipped),
                4 => assert_eq!(st, OrderStatus::Cancelled),
                5 => assert_eq!(st, OrderStatus::Completed),
                21 | 22 | 39 => assert_eq!(st, OrderStatus::Refunded),
                _ => panic!("unexpected code in coverage list: {code}"),
            }
        }
    }

    #[test]
    fn douyin_risk_processing_overrides_to_on_hold() {
        let shop = dummy_shop();
        let order = serde_json::json!({
            "order_id": "6919756900423669517",
            "order_status": 2,
            "create_time": 1704067200,
            "pay_amount": 100,
            "sku_order_tag_ui": {"key": "risk_processing"},
            "mask_post_receiver": "张**",
            "mask_post_tel": "138****0000",
            "mask_post_addr": {"province": "浙江省", "city": "杭州市", "district": "西湖区", "detail": "翠苑路***号"},
            "sku_order_list": [
                {"out_sku_id": "SKU-OUT-1", "sku_id": 2001, "product_name": "测试商品", "item_num": 1, "goods_price": 100, "order_amount": 100}
            ]
        });

        let proj = Adapter::map_order_to_projection(&shop, &order).expect("map");
        assert_eq!(proj.status, OrderStatus::OnHold);
    }

    #[test]
    fn douyin_order_projection_core_fields() {
        let shop = dummy_shop();
        let order = serde_json::json!({
            "order_id": "6919756900423669517",
            "order_status": 3,
            "create_time": 1704067200,
            "update_time": 1704068200,
            "pay_amount": 1234,
            "open_id": "buyer_open_id",
            "mask_post_receiver": "张**",
            "mask_post_tel": "138****0000",
            "mask_post_addr": {"province": "浙江省", "city": "杭州市", "district": "西湖区", "street": "翠苑街道", "detail": "翠苑路***号"},
            "sku_order_list": [
                {"out_sku_id": "SKU-OUT-1", "sku_id": 2001, "product_name": "测试商品", "item_num": 2, "goods_price": 617, "order_amount": 1234}
            ]
        });

        let proj = Adapter::map_order_to_projection(&shop, &order).expect("map");
        assert_eq!(proj.platform_order_id, "6919756900423669517");
        assert_eq!(proj.status, OrderStatus::Shipped);
        assert_eq!(proj.total_amount_cent, 1234);
        assert_eq!(proj.buyer_name.as_deref(), Some("buyer_open_id"));
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
    fn douyin_order_projection_enrichment_fields_and_item_aftersale() {
        let shop = dummy_shop();
        let order = serde_json::json!({
            "order_id": "6919756900423669999",
            "order_status": 3,
            "create_time": 1704067200,
            "update_time": 1704068200,
            "pay_amount": 1234,
            "order_amount": 1500,
            "post_amount": 100,
            "promotion_amount": 200,
            "platform_discount_amount": 50,
            "shop_discount_amount": 80,
            "coupon_amount": 70,
            "pay_time": 1704067300,
            "ship_time": 1704067400,
            "finish_time": 1704067500,
            "buyer_words": "留言",
            "seller_words": "备注",
            "logistics_info": [{"company": "顺丰", "tracking_no": "SF123"}],
            "open_id": "buyer_open_id",
            "mask_post_receiver": "张**",
            "mask_post_tel": "138****0000",
            "mask_post_addr": {"province": "浙江省", "city": "杭州市", "district": "西湖区", "street": "翠苑街道", "detail": "翠苑路***号"},
            "sku_order_list": [
                {
                    "sku_order_id": "SO1",
                    "out_sku_id": "SKU-OUT-1",
                    "sku_id": 2001,
                    "product_name": "测试商品",
                    "item_num": 1,
                    "goods_price": 1500,
                    "order_amount": 1500,
                    "pay_amount": 1234,
                    "promotion_amount": 266,
                    "after_sale_info": {
                        "refund_status": 1,
                        "after_sale_status": 2,
                        "refund_amount": 100,
                        "after_sale_type_desc": "仅退款",
                        "after_sale_id": "AS-001"
                    }
                }
            ]
        });

        let proj = Adapter::map_order_to_projection(&shop, &order).expect("map");
        assert_eq!(proj.total_amount_cent, 1234);
        assert_eq!(proj.goods_amount_cent, Some(1500));
        assert_eq!(proj.postage_amount_cent, Some(100));
        assert_eq!(proj.discount_amount_cent, Some(200));
        assert_eq!(proj.platform_discount_cent, Some(50));
        assert_eq!(proj.seller_discount_cent, Some(80));
        assert_eq!(proj.coupon_discount_cent, Some(70));
        assert!(proj.pay_at.is_some());
        assert!(proj.ship_at.is_some());
        assert!(proj.finish_at.is_some());
        assert_eq!(proj.buyer_message.as_deref(), Some("留言"));
        assert_eq!(proj.seller_remark.as_deref(), Some("备注"));
        assert_eq!(proj.logistics_company.as_deref(), Some("顺丰"));
        assert_eq!(proj.tracking_no.as_deref(), Some("SF123"));

        assert_eq!(proj.items.len(), 1);
        let it = &proj.items[0];
        assert_eq!(it.pay_amount_cent, Some(1234));
        assert_eq!(it.discount_cent, Some(266));
        assert_eq!(it.refund_state, RefundState::Processing);
        assert_eq!(it.after_sale_state, AfterSaleState::Processing);
        assert_eq!(it.refund_amount_cent, Some(100));
        assert_eq!(it.after_sale_type.as_deref(), Some("仅退款"));
        assert_eq!(it.after_sale_id.as_deref(), Some("AS-001"));
    }
}
