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
    time::parse_gmt8_datetime_to_utc,
};
use erp_domain::{OrderItemProjection, OrderProjection};
use erp_types::{OrderStatus, Platform};
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

                    items.push(OrderItemProjection {
                        line_id,
                        sku_id,
                        title,
                        qty,
                        price_cent,
                        amount_cent,
                        pay_amount_cent: Some(amount_cent),
                        discount_cent: Some((price_cent * qty as i64 - amount_cent).max(0)),
                        refund_state: Default::default(),
                        after_sale_state: Default::default(),
                        refund_amount_cent: None,
                        after_sale_type: None,
                        after_sale_id: None,
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
