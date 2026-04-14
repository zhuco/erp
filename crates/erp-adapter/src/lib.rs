use async_trait::async_trait;
use chrono::{DateTime, Utc};
use erp_domain::OrderProjection;
use erp_types::{Platform, ShopId, TenantId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopAuthContext {
    pub tenant_id: TenantId,
    pub shop_id: ShopId,
    pub platform: Platform,
    pub platform_shop_id: String,
    pub credentials: Value,
    pub auth_meta: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryHint {
    NoRetry,
    RetryImmediately,
    RetryAfterMs(u64),
}

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("unsupported operation")]
    Unsupported,
    #[error("auth error: {0}")]
    Auth(String),
    #[error("rate limited")]
    RateLimited { retry_after_ms: u64 },
    #[error("temporary failure: {0}")]
    Temporary(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("unexpected: {0}")]
    Unexpected(String),
}

impl AdapterError {
    pub fn retry_hint(&self) -> RetryHint {
        match self {
            AdapterError::RateLimited { retry_after_ms } => {
                RetryHint::RetryAfterMs(*retry_after_ms)
            }
            AdapterError::Temporary(_) => RetryHint::RetryImmediately,
            AdapterError::Auth(_) => RetryHint::RetryAfterMs(60_000),
            AdapterError::BadRequest(_) => RetryHint::NoRetry,
            AdapterError::Unsupported => RetryHint::NoRetry,
            AdapterError::Unexpected(_) => RetryHint::RetryImmediately,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformTokenPayload {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub scopes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRefreshResult {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub scopes: Option<String>,
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncCursor {
    pub cursor: Option<String>,
    pub last_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullOrdersResult {
    pub orders: Vec<OrderProjection>,
    pub next_cursor: Option<String>,
    pub latest_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformSku {
    pub platform_sku_id: String,
    pub outer_sku_id: Option<String>,
    pub sku_code: Option<String>,
    pub barcode: Option<String>,
    pub name: Option<String>,
    pub spec: Option<Value>,
    pub tags: Vec<String>,
    pub image_urls: Vec<String>,
    pub weight_gram: Option<i32>,
    pub length_mm: Option<i32>,
    pub width_mm: Option<i32>,
    pub height_mm: Option<i32>,
    pub volume_cm3: Option<i32>,
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformProduct {
    pub platform_product_id: String,
    pub name: String,
    pub brand: Option<String>,
    pub category: Option<String>,
    pub attrs: Value,
    pub skus: Vec<PlatformSku>,
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullProductsResult {
    pub products: Vec<PlatformProduct>,
    pub next_cursor: Option<String>,
    pub latest_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRefundsResult {
    pub orders: Vec<OrderProjection>,
    pub next_cursor: Option<String>,
    pub latest_time: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait WebhookAdapter: Send + Sync {
    fn platform(&self) -> Platform;

    /// Parse webhook event to an order projection (if the webhook represents an order change).
    async fn parse_webhook_order(
        &self,
        shop: &ShopAuthContext,
        headers: &Value,
        body: &[u8],
    ) -> Result<Option<OrderProjection>, AdapterError>;

    /// Parse webhook event to **0..n** order projections.
    ///
    /// Some platforms batch multiple events/orders in one callback request.
    /// Default behavior wraps `parse_webhook_order`.
    async fn parse_webhook_orders(
        &self,
        shop: &ShopAuthContext,
        headers: &Value,
        body: &[u8],
    ) -> Result<Vec<OrderProjection>, AdapterError> {
        Ok(match self.parse_webhook_order(shop, headers, body).await? {
            Some(o) => vec![o],
            None => vec![],
        })
    }

    /// Verify signature if applicable. Default: accept (platform-specific adapters should override).
    async fn verify_webhook(
        &self,
        _shop: &ShopAuthContext,
        _headers: &Value,
        _body: &[u8],
    ) -> Result<(), AdapterError> {
        Ok(())
    }

    /// Ship an order on the platform. Default: unsupported.
    async fn ship_order(
        &self,
        _shop: &ShopAuthContext,
        _request: ShipOrderRequest,
    ) -> Result<ShipOrderResponse, AdapterError> {
        Err(AdapterError::Unsupported)
    }

    /// Push shipment/tracking info to platform. Default: reuse ship_order.
    async fn push_shipment(
        &self,
        shop: &ShopAuthContext,
        request: ShipOrderRequest,
    ) -> Result<ShipOrderResponse, AdapterError> {
        self.ship_order(shop, request).await
    }

    /// Sync stock quantity to platform. Default: unsupported.
    async fn sync_stock(
        &self,
        _shop: &ShopAuthContext,
        _request: SyncStockRequest,
    ) -> Result<SyncStockResponse, AdapterError> {
        Err(AdapterError::Unsupported)
    }
}

#[async_trait]
pub trait OrderAdapter: Send + Sync {
    fn platform(&self) -> Platform;

    /// Fetch a full order snapshot from platform by platform_order_id.
    async fn fetch_order(
        &self,
        _shop: &ShopAuthContext,
        _platform_order_id: &str,
    ) -> Result<OrderProjection, AdapterError> {
        Err(AdapterError::Unsupported)
    }
}

#[async_trait]
pub trait PlatformAdapter: Send + Sync {
    fn platform(&self) -> Platform;

    async fn verify_signature(
        &self,
        shop: &ShopAuthContext,
        headers: &Value,
        body: &[u8],
    ) -> Result<(), AdapterError>;

    async fn parse_webhook_orders(
        &self,
        shop: &ShopAuthContext,
        headers: &Value,
        body: &[u8],
    ) -> Result<Vec<OrderProjection>, AdapterError>;

    async fn ship_order(
        &self,
        shop: &ShopAuthContext,
        request: ShipOrderRequest,
    ) -> Result<ShipOrderResponse, AdapterError>;

    async fn sync_stock(
        &self,
        _shop: &ShopAuthContext,
        _request: SyncStockRequest,
    ) -> Result<SyncStockResponse, AdapterError> {
        Err(AdapterError::Unsupported)
    }

    async fn fetch_order(
        &self,
        shop: &ShopAuthContext,
        platform_order_id: &str,
    ) -> Result<OrderProjection, AdapterError>;

    async fn refresh_token(
        &self,
        _shop: &ShopAuthContext,
        _token: &PlatformTokenPayload,
    ) -> Result<TokenRefreshResult, AdapterError> {
        Err(AdapterError::Unsupported)
    }

    async fn pull_orders(
        &self,
        _shop: &ShopAuthContext,
        _cursor: SyncCursor,
    ) -> Result<PullOrdersResult, AdapterError> {
        Err(AdapterError::Unsupported)
    }

    async fn pull_products(
        &self,
        _shop: &ShopAuthContext,
        _cursor: SyncCursor,
    ) -> Result<PullProductsResult, AdapterError> {
        Err(AdapterError::Unsupported)
    }

    async fn pull_refunds(
        &self,
        _shop: &ShopAuthContext,
        _cursor: SyncCursor,
    ) -> Result<PullRefundsResult, AdapterError> {
        Err(AdapterError::Unsupported)
    }

    async fn exchange_authorization_code(
        &self,
        _shop: &ShopAuthContext,
        _code: &str,
        _redirect_uri: Option<String>,
    ) -> Result<TokenRefreshResult, AdapterError> {
        Err(AdapterError::Unsupported)
    }

    fn build_authorize_url(
        &self,
        _shop: &ShopAuthContext,
        _redirect_uri: Option<String>,
        _state: &str,
    ) -> Option<String> {
        None
    }
}

pub fn build_authorize_url_from_meta(
    auth_meta: &Value,
    redirect_uri: Option<String>,
    state: &str,
) -> Option<String> {
    let base = auth_meta.get("authorize_url")?.as_str()?.trim();
    if base.is_empty() {
        return None;
    }
    let mut url = base.to_string();
    let mut params = Vec::new();
    params.push(format!("state={state}"));
    if let Some(redirect_uri) = redirect_uri {
        if !redirect_uri.trim().is_empty() {
            params.push(format!("redirect_uri={redirect_uri}"));
        }
    }
    let sep = if url.contains('?') { "&" } else { "?" };
    url.push_str(sep);
    url.push_str(&params.join("&"));
    Some(url)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipOrderRequest {
    pub platform_order_id: String,
    pub warehouse_id: Uuid,
    pub carrier_code: Option<String>,
    pub tracking_no: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipOrderResponse {
    pub tracking_no: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStockRequest {
    pub platform_sku_id: String,
    pub warehouse_id: Uuid,
    pub target_qty: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStockResponse {
    pub target_qty: i64,
}
