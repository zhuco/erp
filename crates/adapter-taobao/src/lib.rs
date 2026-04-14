use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use erp_adapter::{
    AdapterError, OrderAdapter, PlatformAdapter, PlatformTokenPayload, PullOrdersResult,
    PullProductsResult, PullRefundsResult, ShipOrderRequest, ShipOrderResponse, ShopAuthContext,
    SyncCursor, TokenRefreshResult, WebhookAdapter,
};
use erp_common::{
    crypto::{hmac_md5_hex_upper, hmac_sha256_hex_upper, md5_hex_upper},
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

    fn creds_session(shop: &ShopAuthContext) -> Option<String> {
        shop.credentials
            .get("session")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    fn gateway_url(shop: &ShopAuthContext) -> String {
        shop.credentials
            .get("gateway_url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            // TOP official https gateway (trade.fullinfo.get doc)
            .unwrap_or_else(|| "https://eco.taobao.com/router/rest".to_string())
    }

    fn sign_method(shop: &ShopAuthContext) -> String {
        shop.credentials
            .get("sign_method")
            .and_then(|v| v.as_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_else(|| "md5".to_string())
    }

    /// Compute TOP signature.
    ///
    /// Algorithm (high-level):
    /// - sort params by key
    /// - concat key+value (skip empty values)
    /// - for md5: md5(secret + concat + secret) => UPPER hex
    /// - for hmac: hmac-md5(concat, secret) => UPPER hex
    /// - for hmac_sha256 / hmac-sha256: hmac-sha256(concat, secret) => UPPER hex
    fn top_sign(params: &BTreeMap<String, String>, secret: &str, sign_method: &str) -> String {
        let mut to_sign = String::new();
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

        match sign_method.to_lowercase().as_str() {
            "hmac" => hmac_md5_hex_upper(secret.as_bytes(), to_sign.as_bytes()),
            "hmac_sha256" | "hmac-sha256" => {
                hmac_sha256_hex_upper(secret.as_bytes(), to_sign.as_bytes())
            }
            _ => {
                let mut bytes = Vec::with_capacity(secret.len() * 2 + to_sign.len());
                bytes.extend_from_slice(secret.as_bytes());
                bytes.extend_from_slice(to_sign.as_bytes());
                bytes.extend_from_slice(secret.as_bytes());
                md5_hex_upper(&bytes)
            }
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
            // more than 1 dot
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
            "4" | "5" | "closed" | "cancelled" | "canceled" | "rejected" => {
                AfterSaleState::Closed
            }
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

    fn map_trade_status(status: &str, trade: &Value) -> OrderStatus {
        // Official trade status list: see taobao.trade.fullinfo.get
        match status {
            "TRADE_NO_CREATE_PAY" | "WAIT_BUYER_PAY" | "PAY_PENDING" => OrderStatus::PendingPayment,
            "WAIT_SELLER_SEND_GOODS" | "WAIT_PRE_AUTH_CONFIRM" | "PAID_FORBID_CONSIGN" => {
                // buyer paid (or equivalent), waiting seller
                OrderStatus::PendingShipment
            }
            "SELLER_CONSIGNED_PART" => OrderStatus::PartiallyShipped,
            "WAIT_BUYER_CONFIRM_GOODS" | "TRADE_BUYER_SIGNED" => OrderStatus::Shipped,
            "TRADE_FINISHED" => OrderStatus::Completed,
            "TRADE_CLOSED_BY_TAOBAO" => OrderStatus::Cancelled,
            "TRADE_CLOSED" => {
                // Paid then refunded -> closed
                // Prefer to surface refund completion, but keep raw for detail.
                if trade.get("pay_time").is_some() {
                    OrderStatus::Refunded
                } else {
                    OrderStatus::Closed
                }
            }
            _ => OrderStatus::Unknown,
        }
    }

    fn map_trade_to_projection(
        shop: &ShopAuthContext,
        trade: &Value,
    ) -> Result<OrderProjection, AdapterError> {
        let tid = trade
            .get("tid")
            .and_then(|v| {
                if v.is_string() {
                    v.as_str().map(|s| s.to_string())
                } else {
                    v.as_i64().map(|n| n.to_string())
                }
            })
            .ok_or_else(|| AdapterError::BadRequest("missing trade.tid".into()))?;

        let created_str = trade
            .get("created")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AdapterError::BadRequest("missing trade.created".into()))?;
        let order_created_at: DateTime<Utc> = parse_gmt8_datetime_to_utc(created_str)
            .ok_or_else(|| AdapterError::BadRequest("bad trade.created".into()))?;

        let platform_updated_at = trade
            .get("modified")
            .and_then(|v| v.as_str())
            .and_then(parse_gmt8_datetime_to_utc);

        let status_str = trade.get("status").and_then(|v| v.as_str()).unwrap_or("");
        let status = Self::map_trade_status(status_str, trade);

        let total_amount_cent = trade
            .get("payment")
            .and_then(|v| v.as_str())
            .and_then(Self::yuan_str_to_cent)
            .or_else(|| {
                trade
                    .get("total_fee")
                    .and_then(|v| v.as_str())
                    .and_then(Self::yuan_str_to_cent)
            })
            .unwrap_or(0);

        // Receiver
        let receiver_name = trade
            .get("receiver_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let receiver_phone = trade
            .get("receiver_mobile")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                trade
                    .get("receiver_phone")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            });

        let receiver_address = {
            let parts = [
                trade.get("receiver_state").and_then(|v| v.as_str()),
                trade.get("receiver_city").and_then(|v| v.as_str()),
                trade.get("receiver_district").and_then(|v| v.as_str()),
                trade.get("receiver_town").and_then(|v| v.as_str()),
                trade.get("receiver_address").and_then(|v| v.as_str()),
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

        let buyer_name = trade
            .get("buyer_nick")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                trade
                    .get("buyer_open_uid")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            });

        let mut items: Vec<OrderItemProjection> = Vec::new();
        if let Some(orders) = trade.get("orders") {
            let arr = orders
                .get("order")
                .and_then(|v| v.as_array())
                .or_else(|| orders.as_array());
            if let Some(arr) = arr {
                for (idx, it) in arr.iter().enumerate() {
                    let line_id = it
                        .get("oid")
                        .and_then(|v| {
                            if v.is_string() {
                                v.as_str().map(|s| s.to_string())
                            } else {
                                v.as_i64().map(|n| n.to_string())
                            }
                        })
                        .unwrap_or_else(|| format!("L{idx}"));

                    let qty = it.get("num").and_then(|v| v.as_i64()).unwrap_or(1) as i32;

                    let price_cent = it
                        .get("price")
                        .and_then(|v| v.as_str())
                        .and_then(Self::yuan_str_to_cent)
                        .unwrap_or(0);

                    let amount_cent = it
                        .get("payment")
                        .and_then(|v| v.as_str())
                        .and_then(Self::yuan_str_to_cent)
                        .unwrap_or(price_cent * qty as i64);

                    let sku_id = it
                        .get("sku_id")
                        .and_then(|v| {
                            if v.is_string() {
                                v.as_str().map(|s| s.to_string())
                            } else {
                                v.as_i64().map(|n| n.to_string())
                            }
                        })
                        .or_else(|| {
                            it.get("outer_sku_id")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        });

                    let title = it
                        .get("title")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let refund_state = Self::parse_refund_state(
                        it.get("refund_state")
                            .or_else(|| it.get("refundState"))
                            .or_else(|| it.get("refund_status"))
                            .or_else(|| it.get("refundStatus"))
                            .or_else(|| trade.get("refund_state"))
                            .or_else(|| trade.get("refundStatus")),
                    );
                    let after_sale_state = Self::parse_after_sale_state(
                        it.get("after_sale_state")
                            .or_else(|| it.get("afterSaleState"))
                            .or_else(|| it.get("after_sales_status"))
                            .or_else(|| it.get("afterSalesStatus"))
                            .or_else(|| trade.get("after_sale_state"))
                            .or_else(|| trade.get("afterSalesStatus")),
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
                            trade
                                .get("refund_amount_cent")
                                .or_else(|| trade.get("refundAmountCent"))
                                .and_then(Self::parse_cent_like)
                        })
                        .or_else(|| {
                            trade
                                .get("refund_amount")
                                .or_else(|| trade.get("refundAmount"))
                                .and_then(Self::parse_yuan_like_to_cent)
                        });
                    let after_sale_type = it
                        .get("after_sale_type")
                        .or_else(|| it.get("afterSaleType"))
                        .or_else(|| it.get("refund_type"))
                        .or_else(|| it.get("refundType"))
                        .and_then(Self::value_to_text)
                        .or_else(|| {
                            trade
                                .get("after_sale_type")
                                .or_else(|| trade.get("afterSaleType"))
                                .and_then(Self::value_to_text)
                        });
                    let after_sale_id = it
                        .get("after_sale_id")
                        .or_else(|| it.get("afterSaleId"))
                        .or_else(|| it.get("refund_id"))
                        .or_else(|| it.get("refundId"))
                        .or_else(|| it.get("refund_phase"))
                        .or_else(|| it.get("refundPhase"))
                        .and_then(Self::value_to_text)
                        .or_else(|| {
                            trade
                                .get("after_sale_id")
                                .or_else(|| trade.get("afterSaleId"))
                                .or_else(|| trade.get("refund_id"))
                                .or_else(|| trade.get("refundId"))
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
                        discount_cent: Some((price_cent * qty as i64 - amount_cent).max(0)),
                        refund_state,
                        after_sale_state,
                        refund_amount_cent,
                        after_sale_type,
                        after_sale_id,
                        raw: it.clone(),
                    });
                }
            }
        }

        Ok(OrderProjection {
            tenant_id: shop.tenant_id,
            shop_id: shop.shop_id,
            platform: shop.platform,
            platform_order_id: tid,
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
            raw: trade.clone(),
            items,
        })
    }

    /// TOP: taobao.trade.fullinfo.get
    async fn fetch_trade_fullinfo(
        &self,
        shop: &ShopAuthContext,
        tid: &str,
    ) -> Result<OrderProjection, AdapterError> {
        let app_key = Self::creds_app_key(shop)?;
        let secret = Self::creds_app_secret(shop)?;
        let session = Self::creds_session(shop)
            .ok_or_else(|| AdapterError::Auth("missing credentials.session".into()))?;
        let gateway = Self::gateway_url(shop);
        let sign_method = Self::sign_method(shop);

        let timestamp = chrono::FixedOffset::east_opt(8 * 3600)
            .unwrap()
            .from_utc_datetime(&Utc::now().naive_utc())
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        // Field list: keep to commonly used + receiver + items.
        let fields = [
            "tid,status,created,modified,pay_time,end_time,consign_time",
            "payment,total_fee,post_fee,discount_fee,adjust_fee",
            "buyer_nick,buyer_open_uid",
            "receiver_name,receiver_mobile,receiver_phone,receiver_state,receiver_city,receiver_district,receiver_town,receiver_address",
            "orders",
        ]
        .join(",");

        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert(
            "method".to_string(),
            "taobao.trade.fullinfo.get".to_string(),
        );
        params.insert("app_key".to_string(), app_key);
        params.insert("session".to_string(), session);
        params.insert("timestamp".to_string(), timestamp);
        params.insert("v".to_string(), "2.0".to_string());
        params.insert("format".to_string(), "json".to_string());
        params.insert("sign_method".to_string(), sign_method.clone());
        params.insert("fields".to_string(), fields);
        params.insert("tid".to_string(), tid.to_string());

        let sign = Self::top_sign(&params, &secret, &sign_method);
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
                "taobao gateway status={} body={}...",
                status,
                String::from_utf8_lossy(&body)
            )));
        }

        let v: Value = serde_json::from_slice(&body)
            .map_err(|e| AdapterError::Temporary(format!("json: {e}")))?;

        if let Some(err) = v.get("error_response") {
            return Err(AdapterError::Temporary(format!(
                "taobao error_response: {err}"
            )));
        }

        let trade = v
            .get("trade_fullinfo_get_response")
            .and_then(|x| x.get("trade"))
            .ok_or_else(|| {
                AdapterError::Temporary("missing trade_fullinfo_get_response.trade".into())
            })?;

        Self::map_trade_to_projection(shop, trade)
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
        Platform::Taobao
    }

    async fn verify_webhook(
        &self,
        shop: &ShopAuthContext,
        _headers: &Value,
        body: &[u8],
    ) -> Result<(), AdapterError> {
        // Taobao webhooks are commonly delivered as x-www-form-urlencoded.
        let params = parse_form_urlencoded(body);
        let Some(got_sign) = params.get("sign") else {
            // If no sign present, accept (some internal testing payloads).
            return Ok(());
        };

        let secret = Self::creds_app_secret(shop)?;
        let sign_method = params
            .get("sign_method")
            .map(|s| s.to_lowercase())
            .unwrap_or_else(|| "md5".to_string());

        let computed = Self::top_sign(&params, &secret, &sign_method);
        if !computed.eq_ignore_ascii_case(got_sign) {
            return Err(AdapterError::Auth("invalid taobao signature".into()));
        }
        Ok(())
    }

    async fn parse_webhook_order(
        &self,
        shop: &ShopAuthContext,
        _headers: &Value,
        body: &[u8],
    ) -> Result<Option<OrderProjection>, AdapterError> {
        // Legacy single-order API; for real traffic, use parse_webhook_orders.
        let mut orders =
            erp_adapter::WebhookAdapter::parse_webhook_orders(self, shop, _headers, body).await?;
        Ok(orders.pop())
    }

    async fn parse_webhook_orders(
        &self,
        shop: &ShopAuthContext,
        _headers: &Value,
        body: &[u8],
    ) -> Result<Vec<OrderProjection>, AdapterError> {
        // 1) JSON payload that already contains Trade snapshot
        if let Some(v) = parse_json(body) {
            if let Some(trade) = v
                .get("trade_fullinfo_get_response")
                .and_then(|x| x.get("trade"))
                .or_else(|| v.get("trade"))
            {
                return Ok(vec![Self::map_trade_to_projection(shop, trade)?]);
            }
        }

        // 2) Form-urlencoded: may contain tid directly or contain a message with tid.
        let params = parse_form_urlencoded(body);
        if let Some(tid) = params.get("tid") {
            let o = self.fetch_trade_fullinfo(shop, tid).await?;
            return Ok(vec![o]);
        }

        // TOP message service sometimes puts message JSON in `message`.
        if let Some(msg) = params.get("message") {
            if let Ok(v) = serde_json::from_str::<Value>(msg) {
                if let Some(tid) = v.get("tid").and_then(|x| x.as_str()) {
                    let o = self.fetch_trade_fullinfo(shop, tid).await?;
                    return Ok(vec![o]);
                }
            }
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
            .or_else(|| data.get("trade_list"))
            .or_else(|| data.get("orders"))
            .or_else(|| data.get("refund_list"))
            .or_else(|| data.get("items"))
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        if items.is_empty() {
            if let Some(trade) = v
                .get("trade_fullinfo_get_response")
                .and_then(|x| x.get("trade"))
                .or_else(|| v.get("trade"))
            {
                items.push(trade.clone());
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
        if let Some(cursor) =
            Self::extract_text_by_pointer(v, Self::credential_string(shop, "refund_next_cursor_pointer"))
        {
            next_cursor = Some(cursor);
        }
        if let Some(flag) =
            Self::extract_bool_by_pointer(v, Self::credential_string(shop, "refund_has_more_pointer"))
        {
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

    async fn call_top_api(
        &self,
        shop: &ShopAuthContext,
        method: &str,
        biz: Value,
    ) -> Result<Value, AdapterError> {
        let app_key = Self::creds_app_key(shop)?;
        let secret = Self::creds_app_secret(shop)?;
        let session = Self::creds_session(shop)
            .ok_or_else(|| AdapterError::Auth("missing credentials.session".into()))?;
        let gateway = Self::gateway_url(shop);
        let sign_method = Self::sign_method(shop);
        let timestamp = chrono::FixedOffset::east_opt(8 * 3600)
            .unwrap()
            .from_utc_datetime(&Utc::now().naive_utc())
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("method".to_string(), method.to_string());
        params.insert("app_key".to_string(), app_key);
        params.insert("session".to_string(), session);
        params.insert("timestamp".to_string(), timestamp);
        params.insert("v".to_string(), "2.0".to_string());
        params.insert("format".to_string(), "json".to_string());
        params.insert("sign_method".to_string(), sign_method.clone());
        if let Some(map) = biz.as_object() {
            for (k, v) in map {
                if let Some(text) = Self::value_to_text(v) {
                    params.insert(k.to_string(), text);
                }
            }
        }

        let sign = Self::top_sign(&params, &secret, &sign_method);
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
                "taobao gateway status={} body={}...",
                status,
                String::from_utf8_lossy(&body)
            )));
        }

        let v: Value = serde_json::from_slice(&body)
            .map_err(|e| AdapterError::Temporary(format!("json: {e}")))?;
        if let Some(err) = v.get("error_response") {
            return Err(AdapterError::Temporary(format!(
                "taobao error_response: {err}"
            )));
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
                map.entry("start_modified".to_string())
                    .or_insert(Value::String(last_time.with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap()).format("%Y-%m-%d %H:%M:%S").to_string()));
            }
            if let Some(v) = cursor.cursor.as_deref() {
                if let Ok(page) = v.parse::<i64>() {
                    map.entry("page_no".to_string())
                        .or_insert(Value::Number(page.into()));
                } else {
                    map.entry("cursor".to_string())
                        .or_insert(Value::String(v.to_string()));
                }
            }
        }

        let response = self.call_top_api(shop, &method, biz).await?;
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
            let mut proj = if raw.get("orders").is_some() || raw.get("tid").is_some() {
                Self::map_trade_to_projection(shop, &raw)?
            } else if let Some(tid) = raw
                .get("tid")
                .or_else(|| raw.get("order_id"))
                .or_else(|| raw.get("id"))
                .and_then(Self::value_to_text)
            {
                self.fetch_trade_fullinfo(shop, &tid).await?
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
        Platform::Taobao
    }

    async fn fetch_order(
        &self,
        shop: &ShopAuthContext,
        platform_order_id: &str,
    ) -> Result<OrderProjection, AdapterError> {
        self.fetch_trade_fullinfo(shop, platform_order_id).await
    }
}

#[async_trait]
impl PlatformAdapter for Adapter {
    fn platform(&self) -> Platform {
        Platform::Taobao
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
            let proj = Self::map_trade_to_projection(shop, &raw)?;
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
                let mut proj = Self::map_trade_to_projection(shop, &raw)?;
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
            platform: Platform::Taobao,
            platform_shop_id: "taobao-shop-001".to_string(),
            credentials: serde_json::json!({
                "app_key": "x",
                "app_secret": "y",
                "session": "z"
            }),
            auth_meta: serde_json::json!({
                "mock_orders": [
                    {
                        "tid": "TB-ORDER-001",
                        "created": "2026-04-12 10:00:00",
                        "modified": "2026-04-12 11:00:00",
                        "status": "WAIT_SELLER_SEND_GOODS",
                        "payment": "12.34",
                        "orders": {
                            "order": [
                                {
                                    "oid": "1",
                                    "num": 1,
                                    "price": "12.34",
                                    "payment": "12.34",
                                    "title": "测试商品"
                                }
                            ]
                        }
                    }
                ]
            }),
        }
    }

    #[tokio::test]
    async fn pull_refunds_should_return_mock_orders() {
        let adapter = Adapter::new();
        let mut shop = dummy_shop_with_mock_orders();
        shop.auth_meta = serde_json::json!({
            "mock_orders": [
                {
                    "tid": "TB-ORDER-REFUND-001",
                    "created": "2026-04-12 10:00:00",
                    "modified": "2026-04-12 11:00:00",
                    "status": "WAIT_SELLER_SEND_GOODS",
                    "payment": "12.34",
                    "orders": {
                        "order": [
                            {
                                "oid": "1",
                                "num": 1,
                                "price": "12.34",
                                "payment": "12.34",
                                "title": "refund order",
                                "refund_state": "PROCESSING",
                                "after_sale_id": "TB-AS-001"
                            }
                        ]
                    }
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
                    "tid": "TB-ORDER-REFUND-001",
                    "created": "2026-04-12 10:00:00",
                    "modified": "2026-04-12 11:00:00",
                    "status": "WAIT_SELLER_SEND_GOODS",
                    "payment": "12.34",
                    "orders": {
                        "order": [
                            {
                                "oid": "1",
                                "num": 1,
                                "price": "12.34",
                                "payment": "12.34",
                                "title": "测试商品",
                                "refund_state": "PROCESSING",
                                "after_sale_state": "PROCESSING",
                                "refund_amount": "12.34",
                                "after_sale_type": "return_refund",
                                "after_sale_id": "TB-AS-001"
                            }
                        ]
                    }
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
        assert_eq!(item.after_sale_id.as_deref(), Some("TB-AS-001"));
    }

    #[tokio::test]
    async fn pull_refunds_should_filter_non_after_sales_orders() {
        let adapter = Adapter::new();
        let mut shop = dummy_shop_with_mock_orders();
        shop.auth_meta = serde_json::json!({
            "mock_orders": [
                {
                    "tid": "TB-ORDER-NORMAL-001",
                    "created": "2026-04-12 10:00:00",
                    "modified": "2026-04-12 11:00:00",
                    "status": "WAIT_SELLER_SEND_GOODS",
                    "payment": "12.34",
                    "orders": {
                        "order": [
                            { "oid": "1", "num": 1, "price": "12.34", "payment": "12.34", "title": "普通订单" }
                        ]
                    }
                },
                {
                    "tid": "TB-ORDER-REFUND-002",
                    "created": "2026-04-12 12:00:00",
                    "modified": "2026-04-12 13:00:00",
                    "status": "WAIT_SELLER_SEND_GOODS",
                    "payment": "12.34",
                    "orders": {
                        "order": [
                            {
                                "oid": "2",
                                "num": 1,
                                "price": "12.34",
                                "payment": "12.34",
                                "title": "售后订单",
                                "refund_state": "PROCESSING",
                                "after_sale_id": "TB-AS-002"
                            }
                        ]
                    }
                }
            ]
        });

        let result = PlatformAdapter::pull_refunds(&adapter, &shop, SyncCursor::default())
            .await
            .expect("pull refunds");
        assert_eq!(result.orders.len(), 1);
        assert_eq!(result.orders[0].platform_order_id, "TB-ORDER-REFUND-002");
    }

    #[tokio::test]
    async fn pull_refunds_should_use_refund_api_and_write_meta() {
        let server = MockServer::start();
        let refund_list = serde_json::json!({
            "data": {
                "list": [
                    {
                        "tid": "TB-ORDER-API-001",
                        "created": "2026-04-12 10:00:00",
                        "modified": "2026-04-12 12:00:00",
                        "status": "WAIT_SELLER_SEND_GOODS",
                        "payment": "12.34",
                        "refund_reason": "尺寸不符",
                        "responsibility": "seller",
                        "evidence_urls": ["https://example.com/tb-proof.jpg"],
                        "refund_updated_at": "2026-04-12 12:00:00",
                        "after_sale_id": "TB-AS-API-001",
                        "refund_status": "PROCESSING",
                        "orders": {
                            "order": [
                                {
                                    "oid": "OI-API-1",
                                    "num": 1,
                                    "price": "12.34",
                                    "payment": "12.34",
                                    "title": "退款商品",
                                    "refund_state": "PROCESSING",
                                    "after_sale_state": "PROCESSING",
                                    "refund_amount": "12.34",
                                    "after_sale_id": "TB-AS-API-001"
                                }
                            ]
                        }
                    }
                ],
                "cursor": "2",
                "has_more": true
            }
        });
        let _refund = server.mock(|when, then| {
            when.method(POST).body_contains("taobao.refund.list");
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
            serde_json::Value::String("taobao.refund.list".to_string()),
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
        assert_eq!(meta.get("reason").and_then(|v| v.as_str()), Some("尺寸不符"));
        assert_eq!(
            meta.get("responsibility").and_then(|v| v.as_str()),
            Some("seller")
        );
        assert_eq!(
            meta.get("source_refund_id").and_then(|v| v.as_str()),
            Some("TB-AS-API-001")
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
            when.method(POST).body_contains("taobao.refund.list");
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
            serde_json::Value::String("taobao.refund.list".to_string()),
        );
        shop.credentials = serde_json::Value::Object(creds);
        shop.auth_meta = serde_json::json!({
            "mock_orders": [
                {
                    "tid": "TB-ORDER-FALLBACK-001",
                    "created": "2026-04-12 10:00:00",
                    "modified": "2026-04-12 11:00:00",
                    "status": "WAIT_SELLER_SEND_GOODS",
                    "payment": "12.34",
                    "orders": {
                        "order": [
                            {
                                "oid": "OI-F-1",
                                "num": 1,
                                "price": "12.34",
                                "payment": "12.34",
                                "title": "fallback order",
                                "refund_state": "PROCESSING",
                                "after_sale_id": "TB-AS-F-001"
                            }
                        ]
                    }
                }
            ]
        });

        let result = PlatformAdapter::pull_refunds(&adapter, &shop, SyncCursor::default())
            .await
            .expect("pull refunds with fallback");
        assert_eq!(result.orders.len(), 1);
        assert_eq!(result.orders[0].platform_order_id, "TB-ORDER-FALLBACK-001");
    }

    #[tokio::test]
    async fn pull_refunds_should_keep_cursor_latest_time_consistent() {
        let server = MockServer::start();
        let modified = (Utc::now() + chrono::Duration::hours(1))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        let created = (Utc::now() - chrono::Duration::hours(1))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        let refund_list = serde_json::json!({
            "data": {
                "list": [
                    {
                        "tid": "TB-ORDER-CURSOR-001",
                        "created": created,
                        "modified": modified,
                        "status": "WAIT_SELLER_SEND_GOODS",
                        "payment": "12.34",
                        "orders": {
                            "order": [
                                {
                                    "oid": "OI-C-1",
                                    "num": 1,
                                    "price": "12.34",
                                    "payment": "12.34",
                                    "title": "cursor order",
                                    "refund_state": "PROCESSING",
                                    "after_sale_id": "TB-AS-C-001"
                                }
                            ]
                        }
                    }
                ],
                "cursor": "5",
                "has_more": true
            }
        });
        let _refund = server.mock(|when, then| {
            when.method(POST).body_contains("taobao.refund.list");
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
            serde_json::Value::String("taobao.refund.list".to_string()),
        );
        shop.credentials = serde_json::Value::Object(creds);
        shop.auth_meta = serde_json::json!({});

        let last_time = Utc::now() - chrono::Duration::days(2);
        let result = PlatformAdapter::pull_refunds(
            &adapter,
            &shop,
            SyncCursor {
                cursor: Some("4".to_string()),
                last_time: Some(last_time),
            },
        )
        .await
        .expect("pull refunds via api");

        assert_eq!(result.next_cursor.as_deref(), Some("5"));
        assert!(result.latest_time.is_some());
        assert!(result.latest_time.expect("latest time") > last_time);
    }
}
