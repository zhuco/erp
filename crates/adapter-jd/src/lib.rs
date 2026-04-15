use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use erp_adapter::{
    AdapterError, OrderAdapter, PlatformAdapter, PlatformTokenPayload, PullOrdersResult,
    PullProductsResult, PullRefundsResult, ShipOrderRequest, ShipOrderResponse, ShopAuthContext,
    SyncCursor, TokenRefreshResult, WebhookAdapter,
};
use erp_common::{
    crypto::md5_hex_upper,
    http::{parse_form_urlencoded, parse_json},
    time::{parse_gmt8_datetime_to_utc, parse_platform_time},
};
use erp_domain::{OrderItemProjection, OrderProjection};
use erp_types::{AfterSaleState, OrderStatus, Platform, RefundState};
use serde_json::Value;
use std::collections::BTreeMap;

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

    fn gateway_url(shop: &ShopAuthContext) -> String {
        shop.credentials
            .get("gateway_url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            // JOS router endpoint
            .unwrap_or_else(|| "https://api.jd.com/routerjson".to_string())
    }

    fn order_detail_method(shop: &ShopAuthContext) -> String {
        shop.credentials
            .get("order_detail_method")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            // Common POP order detail method name.
            .unwrap_or_else(|| "jingdong.pop.order.get".to_string())
    }

    fn jd_sign(params: &BTreeMap<String, String>, app_secret: &str) -> String {
        let mut to_sign = String::new();
        to_sign.push_str(app_secret);
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
        to_sign.push_str(app_secret);
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
            || combined.contains("invalid app")
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
        let code = Self::error_token(error.get("code"))
            .or_else(|| Self::error_token(error.get("error_code")))
            .or_else(|| Self::error_token(error.get("sub_code")));
        let message = Self::error_token(error.get("sub_msg"))
            .or_else(|| Self::error_token(error.get("msg")))
            .or_else(|| Self::error_token(error.get("message")))
            .or_else(|| Self::error_token(error.get("zh_desc")))
            .or_else(|| Self::error_token(error.get("en_desc")))
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
        None
    }

    fn parse_cent_like(v: &Value) -> Option<i64> {
        if let Some(n) = v.as_i64() {
            return Some(n);
        }
        if let Some(n) = v.as_u64() {
            return Some(n as i64);
        }
        if let Some(n) = v.as_f64() {
            return Some(n.round() as i64);
        }
        v.as_str().and_then(|s| s.trim().parse::<i64>().ok())
    }

    fn parse_yuan_like_to_cent(v: &Value) -> Option<i64> {
        if let Some(s) = v.as_str() {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return None;
            }
            if trimmed.contains('.') {
                return Self::yuan_str_to_cent(trimmed);
            }
            return trimmed.parse::<i64>().ok().map(|n| n * 100);
        }
        if let Some(n) = v.as_i64() {
            return Some(n * 100);
        }
        if let Some(n) = v.as_u64() {
            return Some((n as i64) * 100);
        }
        v.as_f64().map(|n| (n * 100.0).round() as i64)
    }

    fn parse_refund_state(value: Option<&Value>) -> RefundState {
        let Some(v) = value else {
            return RefundState::None;
        };
        let Some(raw) = Self::value_to_text(v) else {
            return RefundState::Unknown;
        };
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return RefundState::None;
        }
        if let Ok(state) = trimmed.parse::<RefundState>() {
            return state;
        }

        let normalized = trimmed.to_lowercase();
        match normalized.as_str() {
            "0" | "none" | "no_refund" | "no_after_sale" => RefundState::None,
            "1" | "2" | "processing" | "process" | "in_progress" | "pending" => {
                RefundState::Processing
            }
            "3" | "4" | "success" | "done" | "finished" | "complete" => RefundState::Success,
            "5" | "6" | "closed" | "cancelled" | "canceled" | "rejected" => RefundState::Closed,
            _ if normalized.contains("process")
                || normalized.contains("pending")
                || normalized.contains("refunding") =>
            {
                RefundState::Processing
            }
            _ if normalized.contains("success")
                || normalized.contains("done")
                || normalized.contains("finish") =>
            {
                RefundState::Success
            }
            _ if normalized.contains("close")
                || normalized.contains("cancel")
                || normalized.contains("reject") =>
            {
                RefundState::Closed
            }
            _ => RefundState::Unknown,
        }
    }

    fn parse_after_sale_state(value: Option<&Value>) -> AfterSaleState {
        let Some(v) = value else {
            return AfterSaleState::None;
        };
        let Some(raw) = Self::value_to_text(v) else {
            return AfterSaleState::Unknown;
        };
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return AfterSaleState::None;
        }
        if let Ok(state) = trimmed.parse::<AfterSaleState>() {
            return state;
        }

        let normalized = trimmed.to_lowercase();
        match normalized.as_str() {
            "0" | "none" | "no_after_sale" => AfterSaleState::None,
            "1" | "2" | "processing" | "process" | "in_progress" | "pending" => {
                AfterSaleState::Processing
            }
            "3" | "success" | "done" | "finished" | "complete" => AfterSaleState::Success,
            "4" | "5" | "closed" | "cancelled" | "canceled" | "rejected" => AfterSaleState::Closed,
            _ if normalized.contains("process") || normalized.contains("pending") => {
                AfterSaleState::Processing
            }
            _ if normalized.contains("success")
                || normalized.contains("done")
                || normalized.contains("finish") =>
            {
                AfterSaleState::Success
            }
            _ if normalized.contains("close")
                || normalized.contains("cancel")
                || normalized.contains("reject") =>
            {
                AfterSaleState::Closed
            }
            _ => AfterSaleState::Unknown,
        }
    }

    fn map_order_state(order_state: &str) -> OrderStatus {
        match order_state {
            "NOT_PAY" => OrderStatus::PendingPayment,
            "WAIT_SELLER_STOCK_OUT" | "WAIT_SELLER_DELIVERY" | "UN_DELIVERED" => {
                OrderStatus::PendingShipment
            }
            "DELIVERED" | "WAIT_GOODS_RECEIVE_CONFIRM" => OrderStatus::Shipped,
            "FINISHED_L" => OrderStatus::Completed,
            "TRADE_CANCELED" => OrderStatus::Cancelled,
            "PAUSE" | "POP_ORDER_PAUSE" | "LOCKED" => OrderStatus::OnHold,
            _ => OrderStatus::Unknown,
        }
    }

    fn find_first_order_object(v: &Value, depth: usize) -> Option<&Value> {
        if depth == 0 {
            return None;
        }
        if let Some(obj) = v.as_object() {
            if obj.contains_key("orderId") || obj.contains_key("order_id") {
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
            .get("orderId")
            .and_then(|v| {
                if v.is_string() {
                    v.as_str().map(|s| s.to_string())
                } else {
                    v.as_i64().map(|n| n.to_string())
                }
            })
            .or_else(|| {
                order
                    .get("order_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .ok_or_else(|| AdapterError::BadRequest("missing orderId".into()))?;

        let order_created_at = order
            .get("orderStartTime")
            .or_else(|| order.get("orderTime"))
            .or_else(|| order.get("created"))
            .and_then(|v| v.as_str())
            .and_then(parse_gmt8_datetime_to_utc)
            .unwrap_or_else(Utc::now);

        let platform_updated_at: Option<DateTime<Utc>> = order
            .get("modified")
            .or_else(|| order.get("updateTime"))
            .and_then(|v| v.as_str())
            .and_then(parse_gmt8_datetime_to_utc);

        let order_state = order
            .get("orderState")
            .or_else(|| order.get("order_state"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let status = Self::map_order_state(order_state);

        let total_amount_cent = order
            .get("orderPayment")
            .or_else(|| order.get("orderTotalPrice"))
            .or_else(|| order.get("order_price"))
            .and_then(|v| v.as_str())
            .and_then(Self::yuan_str_to_cent)
            .unwrap_or(0);

        let buyer_name = order
            .get("pin")
            .or_else(|| order.get("buyerPin"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let receiver_name = order
            .get("consignee")
            .or_else(|| order.get("receiver_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let receiver_phone = order
            .get("mobile")
            .or_else(|| order.get("phone"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let receiver_address = order
            .get("fullAddress")
            .or_else(|| order.get("address"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut items: Vec<OrderItemProjection> = Vec::new();
        let items_v = order
            .get("itemInfoList")
            .or_else(|| order.get("skuList"))
            .or_else(|| order.get("orderSku"))
            .or_else(|| order.get("orderSkuList"));
        if let Some(arr) = items_v.and_then(|v| v.as_array()) {
            for (idx, it) in arr.iter().enumerate() {
                let sku_id = it.get("skuId").or_else(|| it.get("sku_id")).and_then(|v| {
                    if v.is_string() {
                        v.as_str().map(|s| s.to_string())
                    } else {
                        v.as_i64().map(|n| n.to_string())
                    }
                });

                let qty = it
                    .get("skuNumber")
                    .or_else(|| it.get("num"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1) as i32;

                let price_cent = it
                    .get("jdPrice")
                    .or_else(|| it.get("price"))
                    .and_then(|v| v.as_str())
                    .and_then(Self::yuan_str_to_cent)
                    .unwrap_or(0);

                let amount_cent = price_cent * qty as i64;

                let title = it
                    .get("skuName")
                    .or_else(|| it.get("name"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let line_id = sku_id.clone().unwrap_or_else(|| format!("L{idx}"));
                let refund_state = Self::parse_refund_state(
                    it.get("refund_state")
                        .or_else(|| it.get("refundState"))
                        .or_else(|| it.get("refund_status"))
                        .or_else(|| it.get("refundStatus"))
                        .or_else(|| order.get("refund_state"))
                        .or_else(|| order.get("refundStatus")),
                );
                let after_sale_state = Self::parse_after_sale_state(
                    it.get("after_sale_state")
                        .or_else(|| it.get("afterSaleState"))
                        .or_else(|| it.get("after_sales_status"))
                        .or_else(|| it.get("afterSalesStatus"))
                        .or_else(|| order.get("after_sale_state"))
                        .or_else(|| order.get("afterSalesStatus")),
                );
                let refund_amount_cent = it
                    .get("refund_amount_cent")
                    .or_else(|| it.get("refundAmountCent"))
                    .and_then(Self::parse_cent_like)
                    .or_else(|| {
                        it.get("refund_amount")
                            .or_else(|| it.get("refundAmount"))
                            .or_else(|| it.get("refund_fee"))
                            .or_else(|| it.get("refundFee"))
                            .and_then(Self::parse_yuan_like_to_cent)
                    })
                    .or_else(|| {
                        order
                            .get("refund_amount_cent")
                            .or_else(|| order.get("refundAmountCent"))
                            .and_then(Self::parse_cent_like)
                    })
                    .or_else(|| {
                        order
                            .get("refund_amount")
                            .or_else(|| order.get("refundAmount"))
                            .and_then(Self::parse_yuan_like_to_cent)
                    });
                let after_sale_type = it
                    .get("after_sale_type")
                    .or_else(|| it.get("afterSaleType"))
                    .or_else(|| it.get("refund_type"))
                    .or_else(|| it.get("refundType"))
                    .and_then(Self::value_to_text)
                    .or_else(|| {
                        order
                            .get("after_sale_type")
                            .or_else(|| order.get("afterSaleType"))
                            .and_then(Self::value_to_text)
                    });
                let after_sale_id = it
                    .get("after_sale_id")
                    .or_else(|| it.get("afterSaleId"))
                    .or_else(|| it.get("refund_id"))
                    .or_else(|| it.get("refundId"))
                    .or_else(|| it.get("afsServiceId"))
                    .or_else(|| it.get("service_id"))
                    .and_then(Self::value_to_text)
                    .or_else(|| {
                        order
                            .get("after_sale_id")
                            .or_else(|| order.get("afterSaleId"))
                            .or_else(|| order.get("refund_id"))
                            .or_else(|| order.get("refundId"))
                            .and_then(Self::value_to_text)
                    });

                items.push(OrderItemProjection {
                    line_id,
                    sku_id,
                    title,
                    qty,
                    price_cent,
                    amount_cent,
                    pay_amount_cent: Some(amount_cent),
                    discount_cent: None,
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
            goods_amount_cent: None,
            postage_amount_cent: None,
            discount_amount_cent: None,
            platform_discount_cent: None,
            seller_discount_cent: None,
            coupon_discount_cent: None,
            currency: "CNY".to_string(),
            buyer_name,
            receiver_name,
            receiver_phone,
            receiver_address,
            pay_at: None,
            ship_at: None,
            finish_at: None,
            cancel_at: None,
            buyer_message: None,
            seller_remark: None,
            logistics_company: None,
            tracking_no: None,
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
        let app_key = Self::creds_app_key(shop)?;
        let app_secret = Self::creds_app_secret(shop)?;
        let access_token = Self::creds_access_token(shop)
            .ok_or_else(|| AdapterError::Auth("missing credentials.access_token".into()))?;
        let gateway = Self::gateway_url(shop);
        let method = Self::order_detail_method(shop);

        let timestamp = chrono::FixedOffset::east_opt(8 * 3600)
            .unwrap()
            .from_utc_datetime(&Utc::now().naive_utc())
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        // JOS business params are passed in 360buy_param_json
        let biz = serde_json::json!({
            "order_id": order_id,
            "orderId": order_id,
        });
        let biz_str = serde_json::to_string(&biz)
            .map_err(|e| AdapterError::BadRequest(format!("biz json: {e}")))?;

        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("method".to_string(), method);
        params.insert("app_key".to_string(), app_key);
        params.insert("access_token".to_string(), access_token);
        params.insert("timestamp".to_string(), timestamp);
        params.insert("v".to_string(), "2.0".to_string());
        params.insert("format".to_string(), "json".to_string());
        params.insert("sign_method".to_string(), "md5".to_string());
        params.insert("360buy_param_json".to_string(), biz_str);

        let sign = Self::jd_sign(&params, &app_secret);
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
            return Err(Self::map_http_error("jd", status, &body));
        }

        let v: Value = serde_json::from_slice(&body)
            .map_err(|e| AdapterError::Temporary(format!("json: {e}")))?;
        if let Some(error) = v.get("error_response") {
            return Err(Self::map_error_response("jd", error));
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

#[async_trait]
impl WebhookAdapter for Adapter {
    fn platform(&self) -> Platform {
        Platform::Jd
    }

    async fn verify_webhook(
        &self,
        shop: &ShopAuthContext,
        _headers: &Value,
        body: &[u8],
    ) -> Result<(), AdapterError> {
        // JD callbacks are usually form-urlencoded. If `sign` exists, verify using the same JOS signing rule.
        let params = parse_form_urlencoded(body);
        let Some(got_sign) = params.get("sign") else {
            return Ok(());
        };
        let secret = Self::creds_app_secret(shop)?;
        let computed = Self::jd_sign(&params, &secret);
        if !computed.eq_ignore_ascii_case(got_sign) {
            return Err(AdapterError::Auth("invalid jd signature".into()));
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

        // 2) form-urlencoded - often contains orderId/order_id only
        let params = parse_form_urlencoded(body);
        if let Some(order_id) = params.get("orderId").or_else(|| params.get("order_id")) {
            let o = self.fetch_order_detail(shop, order_id).await?;
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

    fn credential_bool(shop: &ShopAuthContext, key: &str) -> Option<bool> {
        shop.credentials.get(key).and_then(|v| v.as_bool())
    }

    fn credential_string(shop: &ShopAuthContext, key: &str) -> Option<String> {
        shop.credentials
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToString::to_string)
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
            .or_else(|| data.get("refund_list"))
            .or_else(|| data.get("items"))
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        if items.is_empty() {
            if let Some(order_obj) = Self::find_first_order_object(v, 12) {
                items.push(order_obj.clone());
            }
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

    async fn call_jd_api(
        &self,
        shop: &ShopAuthContext,
        method: &str,
        biz: Value,
    ) -> Result<Value, AdapterError> {
        let app_key = Self::creds_app_key(shop)?;
        let app_secret = Self::creds_app_secret(shop)?;
        let access_token = Self::creds_access_token(shop)
            .ok_or_else(|| AdapterError::Auth("missing credentials.access_token".into()))?;
        let gateway = Self::gateway_url(shop);

        let timestamp = chrono::FixedOffset::east_opt(8 * 3600)
            .unwrap()
            .from_utc_datetime(&Utc::now().naive_utc())
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        let biz_str = serde_json::to_string(&biz)
            .map_err(|e| AdapterError::BadRequest(format!("biz json: {e}")))?;

        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("method".to_string(), method.to_string());
        params.insert("app_key".to_string(), app_key);
        params.insert("access_token".to_string(), access_token);
        params.insert("timestamp".to_string(), timestamp);
        params.insert("v".to_string(), "2.0".to_string());
        params.insert("format".to_string(), "json".to_string());
        params.insert("sign_method".to_string(), "md5".to_string());
        params.insert("360buy_param_json".to_string(), biz_str);
        let sign = Self::jd_sign(&params, &app_secret);
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
            return Err(Self::map_http_error("jd", status, &body));
        }

        let v: Value = serde_json::from_slice(&body)
            .map_err(|e| AdapterError::Temporary(format!("json: {e}")))?;
        if let Some(error) = v.get("error_response") {
            return Err(Self::map_error_response("jd", error));
        }
        Ok(v)
    }

    async fn pull_refunds_from_api(
        &self,
        shop: &ShopAuthContext,
        cursor: &SyncCursor,
    ) -> Result<Option<PullRefundsResult>, AdapterError> {
        if !Self::refund_api_enabled(shop) {
            return Ok(None);
        }
        let Some(method) = Self::refund_list_method(shop) else {
            return Ok(None);
        };

        let mut biz = Self::refund_list_biz(shop);
        if !biz.is_object() {
            biz = serde_json::json!({});
        }
        if let Some(map) = biz.as_object_mut() {
            map.entry("page_size".to_string())
                .or_insert(Value::Number(Self::refund_page_size(shop).into()));
            if let Some(last_time) = cursor.last_time {
                map.entry("updated_time_start".to_string())
                    .or_insert(Value::Number(last_time.timestamp().into()));
            }
            if let Some(v) = cursor.cursor.as_deref() {
                if let Ok(page) = v.parse::<i64>() {
                    map.entry("page".to_string())
                        .or_insert(Value::Number(page.into()));
                } else {
                    map.entry("cursor".to_string())
                        .or_insert(Value::String(v.to_string()));
                }
            }
        }

        let response = self.call_jd_api(shop, &method, biz).await?;
        let (raw_list, mut next_cursor, has_more) = Self::extract_refund_list(shop, &response);

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
            let mut proj = if raw.get("itemInfoList").is_some() || raw.get("orderId").is_some() {
                Self::map_order_to_projection(shop, &raw)?
            } else if let Some(order_id) = raw
                .get("orderId")
                .or_else(|| raw.get("order_id"))
                .or_else(|| raw.get("id"))
                .and_then(Self::value_to_text)
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
        Platform::Jd
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
        Platform::Jd
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
                if let Some(last_time) = cursor.last_time {
                    if compare_time <= last_time {
                        continue;
                    }
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
        let mut orders: Vec<OrderProjection> = result
            .orders
            .into_iter()
            .filter(Self::is_refund_related_order)
            .collect();
        for order in &mut orders {
            let raw = order.raw.clone();
            Self::inject_refund_meta(order, &raw);
        }
        Ok(PullRefundsResult {
            orders,
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
        erp_adapter::build_authorize_url_from_meta(&shop.auth_meta, redirect_uri, state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use erp_adapter::PlatformAdapter;
    use erp_types::{AfterSaleState, Platform, RefundState, ShopId, TenantId};
    use httpmock::Method::POST;
    use httpmock::MockServer;

    fn dummy_shop_with_mock_orders() -> ShopAuthContext {
        ShopAuthContext {
            tenant_id: "00000000-0000-0000-0000-000000000000"
                .parse::<TenantId>()
                .unwrap(),
            shop_id: "00000000-0000-0000-0000-000000000000"
                .parse::<ShopId>()
                .unwrap(),
            platform: Platform::Jd,
            platform_shop_id: "jd-shop-001".to_string(),
            credentials: serde_json::json!({
                "app_key": "x",
                "app_secret": "y",
                "access_token": "z"
            }),
            auth_meta: serde_json::json!({
                "mock_orders": [
                    {
                        "orderId": "JD-ORDER-001",
                        "orderStartTime": "2026-04-12 10:00:00",
                        "modified": "2026-04-12 11:00:00",
                        "orderState": "DELIVERED",
                        "orderPayment": "12.34"
                    }
                ]
            }),
        }
    }

    #[test]
    fn classify_jd_error_response_as_auth() {
        let error = serde_json::json!({
            "code": "isv.token-invalid",
            "msg": "access token expired"
        });
        let result = Adapter::map_error_response("jd", &error);
        assert!(matches!(result, AdapterError::Auth(_)));
    }

    #[test]
    fn classify_jd_error_response_as_rate_limited() {
        let error = serde_json::json!({
            "code": 429,
            "sub_msg": "too many requests"
        });
        let result = Adapter::map_error_response("jd", &error);
        assert!(matches!(result, AdapterError::RateLimited { .. }));
    }

    #[test]
    fn classify_jd_http_client_error_as_bad_request() {
        let result = Adapter::map_http_error(
            "jd",
            reqwest::StatusCode::BAD_REQUEST,
            br#"{"msg":"invalid param"}"#,
        );
        assert!(matches!(result, AdapterError::BadRequest(_)));
    }

    #[tokio::test]
    async fn pull_refunds_should_return_mock_orders() {
        let adapter = Adapter::new();
        let mut shop = dummy_shop_with_mock_orders();
        shop.auth_meta = serde_json::json!({
            "mock_orders": [
                {
                    "orderId": "JD-ORDER-REFUND-001",
                    "orderStartTime": "2026-04-12 10:00:00",
                    "modified": "2026-04-12 11:00:00",
                    "orderState": "DELIVERED",
                    "orderPayment": "12.34",
                    "itemInfoList": [
                        {
                            "skuId": "SKU-1",
                            "skuNumber": 1,
                            "jdPrice": "12.34",
                            "skuName": "refund order",
                            "refund_state": "PROCESSING",
                            "after_sale_id": "JD-AS-001"
                        }
                    ]
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
    async fn pull_refunds_should_map_after_sales_item_fields() {
        let adapter = Adapter::new();
        let mut shop = dummy_shop_with_mock_orders();
        shop.auth_meta = serde_json::json!({
            "mock_orders": [
                {
                    "orderId": "JD-ORDER-REFUND-001",
                    "orderStartTime": "2026-04-12 10:00:00",
                    "modified": "2026-04-12 11:00:00",
                    "orderState": "DELIVERED",
                    "orderPayment": "12.34",
                    "itemInfoList": [
                        {
                            "skuId": "SKU-1",
                            "skuNumber": 1,
                            "jdPrice": "12.34",
                            "skuName": "测试商品",
                            "refund_state": "PROCESSING",
                            "after_sale_state": "PROCESSING",
                            "refund_amount": "12.34",
                            "after_sale_type": "return_refund",
                            "after_sale_id": "JD-AS-001"
                        }
                    ]
                }
            ]
        });

        let result = PlatformAdapter::pull_refunds(&adapter, &shop, SyncCursor::default())
            .await
            .expect("pull refunds");
        let item = &result.orders[0].items[0];
        assert_eq!(item.refund_state, RefundState::Processing);
        assert_eq!(item.after_sale_state, AfterSaleState::Processing);
        assert_eq!(item.refund_amount_cent, Some(1234));
        assert_eq!(item.after_sale_id.as_deref(), Some("JD-AS-001"));
    }

    #[tokio::test]
    async fn pull_refunds_should_filter_non_after_sales_orders() {
        let adapter = Adapter::new();
        let mut shop = dummy_shop_with_mock_orders();
        shop.auth_meta = serde_json::json!({
            "mock_orders": [
                {
                    "orderId": "JD-ORDER-NORMAL-001",
                    "orderStartTime": "2026-04-12 10:00:00",
                    "modified": "2026-04-12 11:00:00",
                    "orderState": "DELIVERED",
                    "orderPayment": "12.34",
                    "itemInfoList": [
                        { "skuId": "SKU-1", "skuNumber": 1, "jdPrice": "12.34", "skuName": "普通订单" }
                    ]
                },
                {
                    "orderId": "JD-ORDER-REFUND-002",
                    "orderStartTime": "2026-04-12 12:00:00",
                    "modified": "2026-04-12 13:00:00",
                    "orderState": "DELIVERED",
                    "orderPayment": "12.34",
                    "itemInfoList": [
                        {
                            "skuId": "SKU-2",
                            "skuNumber": 1,
                            "jdPrice": "12.34",
                            "skuName": "售后订单",
                            "refund_state": "PROCESSING",
                            "after_sale_id": "JD-AS-002"
                        }
                    ]
                }
            ]
        });

        let result = PlatformAdapter::pull_refunds(&adapter, &shop, SyncCursor::default())
            .await
            .expect("pull refunds");
        assert_eq!(result.orders.len(), 1);
        assert_eq!(result.orders[0].platform_order_id, "JD-ORDER-REFUND-002");
    }

    #[tokio::test]
    async fn pull_refunds_should_prefer_mock_refunds_over_mock_orders() {
        let adapter = Adapter::new();
        let mut shop = dummy_shop_with_mock_orders();
        shop.auth_meta = serde_json::json!({
            "mock_orders": [
                {
                    "orderId": "JD-ORDER-NORMAL-001",
                    "orderStartTime": "2026-04-12 10:00:00",
                    "modified": "2026-04-12 11:00:00",
                    "orderState": "DELIVERED",
                    "orderPayment": "12.34"
                }
            ],
            "mock_refunds": [
                {
                    "orderId": "JD-ORDER-REFUND-ONLY-001",
                    "orderStartTime": "2026-04-12 12:00:00",
                    "modified": "2026-04-12 13:00:00",
                    "orderState": "DELIVERED",
                    "orderPayment": "12.34",
                    "itemInfoList": [
                        {
                            "skuId": "SKU-R",
                            "skuNumber": 1,
                            "jdPrice": "12.34",
                            "skuName": "退款订单",
                            "refund_state": "PROCESSING",
                            "after_sale_id": "JD-AS-R1"
                        }
                    ]
                }
            ]
        });

        let result = PlatformAdapter::pull_refunds(&adapter, &shop, SyncCursor::default())
            .await
            .expect("pull refunds");
        assert_eq!(result.orders.len(), 1);
        assert_eq!(
            result.orders[0].platform_order_id,
            "JD-ORDER-REFUND-ONLY-001"
        );
    }

    #[tokio::test]
    async fn pull_refunds_should_use_refund_api_and_write_meta() {
        let server = MockServer::start();
        let now = Utc::now();
        let refund_list = serde_json::json!({
            "data": {
                "list": [
                    {
                        "orderId": "JD-ORDER-API-001",
                        "orderStartTime": now.timestamp() - 3600,
                        "modified": now.timestamp(),
                        "orderState": "DELIVERED",
                        "orderPayment": "12.34",
                        "refund_reason": "质量问题",
                        "responsibility": "seller",
                        "evidence_urls": ["https://example.com/jd-proof.jpg"],
                        "refund_updated_at": "2026-04-12 12:00:00",
                        "after_sale_id": "JD-AS-API-001",
                        "refund_status": 1,
                        "itemInfoList": [
                            {
                                "skuId": "SKU-API-1",
                                "skuNumber": 1,
                                "jdPrice": "12.34",
                                "skuName": "退款商品",
                                "refund_state": "PROCESSING",
                                "after_sale_state": "PROCESSING",
                                "refund_amount": "12.34",
                                "after_sale_id": "JD-AS-API-001"
                            }
                        ]
                    }
                ],
                "cursor": "2",
                "has_more": true
            }
        });
        let _refund = server.mock(|when, then| {
            when.method(POST);
            then.status(200).json_body(refund_list.clone());
        });

        let adapter = Adapter::new();
        let mut shop = dummy_shop_with_mock_orders();
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
            serde_json::Value::String("jd.refund.list".to_string()),
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
        assert_eq!(
            meta.get("reason").and_then(|v| v.as_str()),
            Some("质量问题")
        );
        assert_eq!(
            meta.get("responsibility").and_then(|v| v.as_str()),
            Some("seller")
        );
        assert_eq!(
            meta.get("source_refund_id").and_then(|v| v.as_str()),
            Some("JD-AS-API-001")
        );
        assert_eq!(
            meta.get("source_status").and_then(|v| v.as_str()),
            Some("PROCESSING")
        );
    }

    #[tokio::test]
    async fn pull_refunds_should_fallback_when_refund_api_failed() {
        let server = MockServer::start();
        let _refund_fail = server.mock(|when, then| {
            when.method(POST);
            then.status(500).body("mock fail");
        });

        let adapter = Adapter::new();
        let mut shop = dummy_shop_with_mock_orders();
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
            serde_json::Value::String("jd.refund.list".to_string()),
        );
        shop.credentials = serde_json::Value::Object(creds);
        shop.auth_meta = serde_json::json!({
            "mock_orders": [
                {
                    "orderId": "JD-ORDER-FALLBACK-001",
                    "orderStartTime": "2026-04-12 10:00:00",
                    "modified": "2026-04-12 12:00:00",
                    "orderState": "DELIVERED",
                    "orderPayment": "12.34",
                    "itemInfoList": [
                        {
                            "skuId": "SKU-F-1",
                            "skuNumber": 1,
                            "jdPrice": "12.34",
                            "skuName": "fallback order",
                            "refund_state": "PROCESSING",
                            "after_sale_id": "JD-AS-F-001"
                        }
                    ]
                }
            ]
        });

        let result = PlatformAdapter::pull_refunds(&adapter, &shop, SyncCursor::default())
            .await
            .expect("pull refunds with fallback");
        assert_eq!(result.orders.len(), 1);
        assert_eq!(result.orders[0].platform_order_id, "JD-ORDER-FALLBACK-001");
    }

    #[tokio::test]
    async fn pull_refunds_should_keep_cursor_latest_time_consistent() {
        let server = MockServer::start();
        let updated_at = Utc::now().timestamp() + 3600;
        let refund_list = serde_json::json!({
            "data": {
                "list": [
                    {
                        "orderId": "JD-ORDER-CURSOR-001",
                        "orderStartTime": updated_at - 3600,
                        "modified": updated_at,
                        "orderState": "DELIVERED",
                        "orderPayment": "12.34",
                        "itemInfoList": [
                            {
                                "skuId": "SKU-C-1",
                                "skuNumber": 1,
                                "jdPrice": "12.34",
                                "skuName": "cursor order",
                                "refund_state": "PROCESSING",
                                "after_sale_id": "JD-AS-C-001"
                            }
                        ]
                    }
                ],
                "cursor": "9",
                "has_more": true
            }
        });
        let _refund = server.mock(|when, then| {
            when.method(POST);
            then.status(200).json_body(refund_list.clone());
        });

        let adapter = Adapter::new();
        let mut shop = dummy_shop_with_mock_orders();
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
            serde_json::Value::String("jd.refund.list".to_string()),
        );
        shop.credentials = serde_json::Value::Object(creds);
        shop.auth_meta = serde_json::json!({});

        let last_time = Utc::now() - chrono::Duration::days(2);
        let result = PlatformAdapter::pull_refunds(
            &adapter,
            &shop,
            SyncCursor {
                cursor: Some("8".to_string()),
                last_time: Some(last_time),
            },
        )
        .await
        .expect("pull refunds via api");

        assert_eq!(result.next_cursor.as_deref(), Some("9"));
        assert!(result.latest_time.is_some());
        assert!(result.latest_time.expect("latest time") > last_time);
    }
}
