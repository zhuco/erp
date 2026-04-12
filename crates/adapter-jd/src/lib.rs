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

                items.push(OrderItemProjection {
                    line_id,
                    sku_id,
                    title,
                    qty,
                    price_cent,
                    amount_cent,
                    pay_amount_cent: Some(amount_cent),
                    discount_cent: None,
                    refund_state: Default::default(),
                    after_sale_state: Default::default(),
                    refund_amount_cent: None,
                    after_sale_type: None,
                    after_sale_id: None,
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
            return Err(AdapterError::Temporary(format!(
                "jd gateway status={} body={}...",
                status,
                String::from_utf8_lossy(&body)
            )));
        }

        let v: Value = serde_json::from_slice(&body)
            .map_err(|e| AdapterError::Temporary(format!("json: {e}")))?;
        if v.get("error_response").is_some() {
            return Err(AdapterError::Temporary(format!("jd error_response: {v}")));
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
