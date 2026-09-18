use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use chrono::{DateTime, TimeZone, Utc};
use clap::Parser;
use erp_adapter::PlatformAdapter;
use erp_common::feature_flags;
use erp_common::{http::header_get, settings::Settings, telemetry::init_tracing};
use erp_domain::InboxReceivedPayload;
use erp_repo::{
    db::Db, feature_flags::FeatureFlagsRepo, inbox::InboxRepo, outbox::OutboxRepo,
    shops::ShopsRepo, tenants::TenantsRepo, webhook_nonces::WebhookNoncesRepo,
};
use erp_types::{Platform, ShopId, TenantId};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{net::SocketAddr, str::FromStr, sync::Arc};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

#[derive(Parser, Debug)]
struct Args {}

#[derive(Clone)]
struct AppState {
    db: Db,
    adapters: Arc<AdapterRegistry>,
}

struct AdapterRegistry {
    taobao: erp_adapter_taobao::Adapter,
    jd: erp_adapter_jd::Adapter,
    pdd: erp_adapter_pdd::Adapter,
    douyin: erp_adapter_douyin::Adapter,
}

impl AdapterRegistry {
    fn new() -> Self {
        Self {
            taobao: erp_adapter_taobao::Adapter::new(),
            jd: erp_adapter_jd::Adapter::new(),
            pdd: erp_adapter_pdd::Adapter::new(),
            douyin: erp_adapter_douyin::Adapter::new(),
        }
    }

    fn platform_adapter(&self, platform: Platform) -> &dyn PlatformAdapter {
        match platform {
            Platform::Taobao => &self.taobao,
            Platform::Jd => &self.jd,
            Platform::Pdd => &self.pdd,
            Platform::Douyin => &self.douyin,
        }
    }
}

fn headers_to_json(headers: &HeaderMap) -> Value {
    let mut m = serde_json::Map::new();
    for (k, v) in headers.iter() {
        if let Ok(s) = v.to_str() {
            m.insert(k.to_string(), Value::String(s.to_string()));
        }
    }
    Value::Object(m)
}

fn header_lookup(
    headers: &Value,
    auth_meta: &Value,
    key: &str,
    defaults: &[&str],
) -> Option<String> {
    let custom = auth_meta
        .get("webhook")
        .and_then(|v| v.get(key))
        .and_then(|v| v.as_str());
    if let Some(name) = custom {
        return header_get(headers, name);
    }
    for name in defaults {
        if let Some(value) = header_get(headers, name) {
            return Some(value);
        }
    }
    None
}

fn webhook_tolerance_secs(auth_meta: &Value) -> i64 {
    auth_meta
        .get("webhook")
        .and_then(|v| v.get("tolerance_sec"))
        .and_then(|v| v.as_i64())
        .unwrap_or(300)
}

fn parse_webhook_timestamp(raw: &str) -> Option<DateTime<Utc>> {
    if let Ok(value) = raw.parse::<i64>() {
        if value > 1_000_000_000_000 {
            return Utc.timestamp_millis_opt(value).single();
        }
        return Utc.timestamp_opt(value, 0).single();
    }
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
}

fn compute_dedup_key(
    platform: &Platform,
    shop_id: &ShopId,
    headers: &Value,
    auth_meta: &Value,
    body: &[u8],
) -> String {
    // Prefer a platform-provided event id if present (supports custom header mapping).
    let mut base = format!("{}:{}", platform, shop_id);
    if let Some(event_id) = header_lookup(
        headers,
        auth_meta,
        "event_id_header",
        &[
            "x-event-id",
            "x-msg-id",
            "x-message-id",
            "event_id",
            "msg_id",
            "message_id",
        ],
    ) {
        if !event_id.trim().is_empty() {
            base.push(':');
            base.push_str(event_id.trim());
        }
    }
    let mut hasher = Sha256::new();
    hasher.update(base.as_bytes());
    hasher.update(b":");
    hasher.update(body);
    hex::encode(hasher.finalize())
}

async fn is_pdd_only_mode(pool: &sqlx::PgPool, tenant_id: TenantId) -> anyhow::Result<bool> {
    if let Some(enabled) = feature_flags::pdd_only_mode_env() {
        return Ok(enabled);
    }
    let tenants_repo = TenantsRepo::new(pool);
    tenants_repo.is_pdd_only_mode(tenant_id).await
}

async fn read_only_enabled(pool: &sqlx::PgPool, tenant_id: TenantId) -> anyhow::Result<bool> {
    let repo = FeatureFlagsRepo::new(pool);
    repo.is_read_only(tenant_id, None).await
}

async fn healthz(State(state): State<AppState>) -> impl IntoResponse {
    let ok = sqlx::query_scalar::<_, i64>("select 1::bigint")
        .fetch_one(state.db.pool())
        .await
        .is_ok();
    if ok {
        (StatusCode::OK, "ok")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "db not ready")
    }
}

async fn webhook_ingest(
    State(state): State<AppState>,
    Path((platform_s, shop_id_s)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let platform = match Platform::from_str(&platform_s) {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, "bad platform").into_response(),
    };
    let shop_id = match ShopId::from_str(&shop_id_s) {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, "bad shop_id").into_response(),
    };

    let shops_repo = ShopsRepo::new(state.db.pool());
    let shop = match shops_repo.get_shop_auth(shop_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::NOT_FOUND, format!("shop not found: {e}")).into_response(),
    };

    let read_only = match read_only_enabled(state.db.pool(), shop.tenant_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if read_only {
        warn!(
            tenant_id = %shop.tenant_id,
            shop_id = %shop_id,
            platform = %platform,
            "read-only mode blocked webhook"
        );
        return (StatusCode::FORBIDDEN, "read-only mode active").into_response();
    }

    let pdd_only = match is_pdd_only_mode(state.db.pool(), shop.tenant_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if pdd_only && platform != Platform::Pdd {
        warn!(
            tenant_id = %shop.tenant_id,
            shop_id = %shop_id,
            platform = %platform,
            "pdd-only mode blocked webhook"
        );
        return (StatusCode::FORBIDDEN, "pdd-only mode: platform not allowed").into_response();
    }

    if shop.platform != platform {
        return (StatusCode::BAD_REQUEST, "platform mismatch").into_response();
    }

    let headers_json = headers_to_json(&headers);
    let received_at = Utc::now();
    let dedup_key = compute_dedup_key(&platform, &shop_id, &headers_json, &shop.auth_meta, &body);

    let tenant_id = shop.tenant_id;
    let record_failure = |reason: String| {
        let inbox_repo = InboxRepo::new(state.db.pool());
        let dedup_key = dedup_key.clone();
        let headers_json = headers_json.clone();
        let body = body.clone();
        let reason = reason.clone();
        async move {
            let _ = inbox_repo.claim_dedup_key(tenant_id, &dedup_key).await?;
            let raw_id = inbox_repo
                .insert_raw_payload(
                    tenant_id,
                    platform,
                    shop_id,
                    &headers_json,
                    body.as_ref(),
                    received_at,
                )
                .await?;
            let _ = inbox_repo
                .insert_inbox_event(
                    tenant_id,
                    platform,
                    shop_id,
                    &dedup_key,
                    raw_id,
                    received_at,
                    "FAILED",
                    "FAILED",
                    Some(reason.as_str()),
                )
                .await?;
            Ok::<(), anyhow::Error>(())
        }
    };

    let adapter = state.adapters.platform_adapter(platform);
    if let Err(err) = adapter
        .verify_signature(&shop, &headers_json, body.as_ref())
        .await
    {
        warn!(
            tenant_id = %shop.tenant_id,
            shop_id = %shop_id,
            platform = %platform,
            error = %err,
            "webhook signature verification failed"
        );
        if let Err(err) = record_failure(format!("signature_failed: {err}")).await {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
        }
        return (StatusCode::FORBIDDEN, "invalid signature").into_response();
    }

    let tolerance = webhook_tolerance_secs(&shop.auth_meta);
    if let Some(raw_ts) = header_lookup(
        &headers_json,
        &shop.auth_meta,
        "timestamp_header",
        &["x-timestamp", "x-event-timestamp", "timestamp"],
    ) {
        if let Some(ts) = parse_webhook_timestamp(&raw_ts) {
            let delta = (Utc::now() - ts).num_seconds().abs();
            if delta > tolerance {
                warn!(
                    tenant_id = %shop.tenant_id,
                    shop_id = %shop_id,
                    platform = %platform,
                    delta,
                    "webhook timestamp outside tolerance"
                );
                if let Err(err) = record_failure("timestamp_out_of_range".to_string()).await {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
                }
                return (StatusCode::UNAUTHORIZED, "timestamp out of range").into_response();
            }
        }
    }

    if let Some(nonce) = header_lookup(
        &headers_json,
        &shop.auth_meta,
        "nonce_header",
        &["x-nonce", "x-event-id", "nonce"],
    ) {
        let expires_at = Utc::now() + chrono::Duration::seconds(tolerance.max(0));
        let nonces_repo = WebhookNoncesRepo::new(state.db.pool());
        let inserted = match nonces_repo
            .claim_nonce(shop.tenant_id, platform, shop_id, &nonce, expires_at)
            .await
        {
            Ok(v) => v,
            Err(err) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
            }
        };
        if !inserted {
            warn!(
                tenant_id = %shop.tenant_id,
                shop_id = %shop_id,
                platform = %platform,
                "webhook replay detected"
            );
            if let Err(err) = record_failure("replay_detected".to_string()).await {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
            }
            return (StatusCode::CONFLICT, "replay detected").into_response();
        }
    }

    let inbox_repo = InboxRepo::new(state.db.pool());
    let is_first = match inbox_repo.claim_dedup_key(shop.tenant_id, &dedup_key).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let raw_id = match inbox_repo
        .insert_raw_payload(
            shop.tenant_id,
            platform,
            shop_id,
            &headers_json,
            &body,
            received_at,
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let status = if is_first { "RECEIVED" } else { "DUPLICATE" };
    let inbox_event_id = match inbox_repo
        .insert_inbox_event(
            shop.tenant_id,
            platform,
            shop_id,
            &dedup_key,
            raw_id,
            received_at,
            status,
            "PASSED",
            None,
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    if is_first {
        let outbox_repo = OutboxRepo::new(state.db.pool());
        let payload = serde_json::to_value(InboxReceivedPayload {
            tenant_id: shop.tenant_id,
            platform,
            shop_id,
            inbox_event_id,
        })
        .unwrap();

        if let Err(e) = outbox_repo
            .enqueue(shop.tenant_id, "INBOX_EVENT_RECEIVED", &payload)
            .await
        {
            warn!(error=%e, "failed to enqueue outbox");
            return (StatusCode::INTERNAL_SERVER_ERROR, "failed to enqueue").into_response();
        }
    }

    Json(json!({
        "ok": true,
        "duplicate": !is_first,
        "inbox_event_id": inbox_event_id,
        "dedup_key": dedup_key
    }))
    .into_response()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing("erp-channel");
    let _args = Args::parse();
    let settings = Settings::load()?;
    let db = Db::connect(&settings.database_url).await?;

    let adapters = Arc::new(AdapterRegistry::new());
    let state = AppState { db, adapters };

    let app = Router::new()
        .route("/healthz", get(healthz))
        // NOTE: Use shop-specific callback URL: /webhooks/{platform}/{shop_id}
        .route("/webhooks/:platform/:shop_id", post(webhook_ingest))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], settings.http_port_channel));
    info!(%addr, "erp-channel listening");
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}

#[cfg(test)]
mod webhook_tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use sqlx::{PgPool, postgres::PgPoolOptions};
    use std::time::Duration;
    use tower::ServiceExt;

    struct EnvGuard {
        previous: Option<String>,
    }

    impl EnvGuard {
        fn set(value: &str) -> Self {
            let previous = std::env::var("PDD_ONLY_MODE").ok();
            std::env::set_var("PDD_ONLY_MODE", value);
            Self { previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var("PDD_ONLY_MODE", value),
                None => std::env::remove_var("PDD_ONLY_MODE"),
            }
        }
    }

    async fn try_pool() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
        PgPoolOptions::new()
            .acquire_timeout(Duration::from_secs(2))
            .connect(&url)
            .await
            .ok()
    }

    async fn setup_schema(pool: &PgPool) -> anyhow::Result<()> {
        let statements = [
            "create extension if not exists pgcrypto",
            r#"
            create table if not exists tenants (
              tenant_id uuid primary key,
              name text not null,
              status text not null default 'ACTIVE',
              pdd_only_mode boolean not null default false,
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now()
            )
            "#,
            r#"
            create table if not exists shops (
              shop_id uuid primary key,
              tenant_id uuid not null,
              platform text not null,
              platform_shop_id text not null,
              name text not null default '',
              shop_name text not null default '',
              status text not null default 'AUTHORIZED',
              default_carrier_code text,
              credentials jsonb not null default '{}'::jsonb,
              auth_meta_json jsonb not null default '{}'::jsonb,
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              unique (tenant_id, platform, platform_shop_id)
            )
            "#,
            r#"
            create table if not exists inbox_raw_payloads (
              raw_id bigserial primary key,
              tenant_id uuid not null,
              platform text not null,
              shop_id uuid not null,
              headers jsonb not null default '{}'::jsonb,
              body bytea not null,
              received_at timestamptz not null default now()
            )
            "#,
            r#"
            create table if not exists inbox_dedup (
              tenant_id uuid not null,
              dedup_key text not null,
              first_seen_at timestamptz not null default now(),
              last_seen_at timestamptz not null default now(),
              seen_count int not null default 1,
              primary key (tenant_id, dedup_key)
            )
            "#,
            r#"
            create table if not exists inbox_events (
              tenant_id uuid not null,
              event_month date not null,
              inbox_event_id uuid not null default gen_random_uuid(),
              platform text not null,
              shop_id uuid not null,
              dedup_key text not null,
              raw_id bigint not null,
              status text not null default 'RECEIVED',
              error text not null default '',
              verify_status text not null default 'PASSED',
              failed_reason text not null default '',
              received_at timestamptz not null default now(),
              processed_at timestamptz,
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              primary key (tenant_id, event_month, inbox_event_id),
              unique (tenant_id, event_month, dedup_key)
            )
            "#,
            r#"
            create table if not exists outbox_events (
              tenant_id uuid not null,
              event_month date not null,
              outbox_id uuid not null default gen_random_uuid(),
              event_type text not null,
              payload jsonb not null,
              trace_id text,
              dedup_key text,
              status text not null default 'PENDING',
              attempts int not null default 0,
              next_retry_at timestamptz not null default now(),
              last_error text not null default '',
              last_attempt_at timestamptz not null default now(),
              published_at timestamptz,
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              primary key (tenant_id, event_month, outbox_id)
            )
            "#,
        ];

        for stmt in statements {
            sqlx::query(stmt).execute(pool).await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn pdd_only_blocks_non_pdd_webhooks() -> anyhow::Result<()> {
        let Some(pool) = try_pool().await else {
            eprintln!("skipping pdd_only_blocks_non_pdd_webhooks: db unavailable");
            return Ok(());
        };
        setup_schema(&pool).await?;

        let _guard = EnvGuard::set("true");

        let tenant_id = uuid::Uuid::new_v4();
        let shops = vec![
            (Platform::Taobao, uuid::Uuid::new_v4()),
            (Platform::Jd, uuid::Uuid::new_v4()),
            (Platform::Douyin, uuid::Uuid::new_v4()),
            (Platform::Pdd, uuid::Uuid::new_v4()),
        ];

        sqlx::query(
            r#"
            insert into tenants (tenant_id, name, status)
            values ($1, $2, 'ACTIVE')
            "#,
        )
        .bind(tenant_id)
        .bind("test-tenant")
        .execute(&pool)
        .await?;

        for (platform, shop_id) in &shops {
            sqlx::query(
                r#"
                insert into shops (
                  shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status, credentials, auth_meta_json
                ) values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED', '{}'::jsonb, '{}'::jsonb)
                "#,
            )
            .bind(*shop_id)
            .bind(tenant_id)
            .bind(platform.to_string())
            .bind(format!("{}-shop", platform))
            .bind(format!("{}-shop", platform))
            .execute(&pool)
            .await?;
        }

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
        let db = Db::connect_lazy(&database_url)?;
        let state = AppState {
            db,
            adapters: Arc::new(AdapterRegistry::new()),
        };
        let app = Router::new()
            .route("/webhooks/:platform/:shop_id", post(webhook_ingest))
            .with_state(state);

        for (platform, shop_id, expected) in [
            (Platform::Taobao, shops[0].1, StatusCode::FORBIDDEN),
            (Platform::Jd, shops[1].1, StatusCode::FORBIDDEN),
            (Platform::Douyin, shops[2].1, StatusCode::FORBIDDEN),
            (Platform::Pdd, shops[3].1, StatusCode::OK),
        ] {
            let request = Request::builder()
                .uri(format!("/webhooks/{platform}/{shop_id}"))
                .method("POST")
                .body(Body::from("payload"))
                .unwrap();
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(response.status(), expected);
            let _ = response.into_body().collect().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn webhook_dedup_does_not_duplicate_events() -> anyhow::Result<()> {
        let Some(pool) = try_pool().await else {
            eprintln!("skipping webhook_dedup_does_not_duplicate_events: db unavailable");
            return Ok(());
        };
        setup_schema(&pool).await?;

        let tenant_id = uuid::Uuid::new_v4();
        let shop_id = uuid::Uuid::new_v4();

        sqlx::query(
            r#"
            insert into tenants (tenant_id, name, status)
            values ($1, 'test-tenant', 'ACTIVE')
            "#,
        )
        .bind(tenant_id)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
            values ($1, $2, 'taobao', 'SHOP-1', 'shop', 'shop', 'AUTHORIZED')
            "#,
        )
        .bind(shop_id)
        .bind(tenant_id)
        .execute(&pool)
        .await?;

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
        let db = Db::connect_lazy(&database_url)?;
        let state = AppState {
            db,
            adapters: Arc::new(AdapterRegistry::new()),
        };
        let app = Router::new()
            .route("/webhooks/:platform/:shop_id", post(webhook_ingest))
            .with_state(state);

        let request = Request::builder()
            .uri(format!("/webhooks/taobao/{shop_id}"))
            .method("POST")
            .header("x-event-id", "evt-1")
            .body(Body::from("payload"))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let request = Request::builder()
            .uri(format!("/webhooks/taobao/{shop_id}"))
            .method("POST")
            .header("x-event-id", "evt-1")
            .body(Body::from("payload"))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = response.into_body().collect().await?.to_bytes();
        let json: Value = serde_json::from_slice(&body_bytes)?;
        assert_eq!(json.get("duplicate").and_then(|v| v.as_bool()), Some(true));

        let inbox_count: i64 = sqlx::query_scalar("select count(*) from inbox_events")
            .fetch_one(&pool)
            .await?;
        let outbox_count: i64 = sqlx::query_scalar(
            "select count(*) from outbox_events where event_type = 'INBOX_EVENT_RECEIVED'",
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(inbox_count, 1);
        assert_eq!(outbox_count, 1);
        Ok(())
    }

    #[tokio::test]
    async fn webhook_dedup_uses_custom_event_id_header() -> anyhow::Result<()> {
        let Some(pool) = try_pool().await else {
            eprintln!("skipping webhook_dedup_uses_custom_event_id_header: db unavailable");
            return Ok(());
        };
        setup_schema(&pool).await?;

        let tenant_id = uuid::Uuid::new_v4();
        let shop_id = uuid::Uuid::new_v4();

        sqlx::query(
            r#"
            insert into tenants (tenant_id, name, status)
            values ($1, 'test-tenant', 'ACTIVE')
            "#,
        )
        .bind(tenant_id)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into shops (
              shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status, auth_meta_json
            ) values ($1, $2, 'taobao', 'SHOP-1', 'shop', 'shop', 'AUTHORIZED', $3)
            "#,
        )
        .bind(shop_id)
        .bind(tenant_id)
        .bind(serde_json::json!({
            "webhook": {
                "event_id_header": "x-custom-id"
            }
        }))
        .execute(&pool)
        .await?;

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
        let db = Db::connect_lazy(&database_url)?;
        let state = AppState {
            db,
            adapters: Arc::new(AdapterRegistry::new()),
        };
        let app = Router::new()
            .route("/webhooks/:platform/:shop_id", post(webhook_ingest))
            .with_state(state);

        let request = Request::builder()
            .uri(format!("/webhooks/taobao/{shop_id}"))
            .method("POST")
            .header("x-custom-id", "evt-42")
            .body(Body::from("payload"))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let request = Request::builder()
            .uri(format!("/webhooks/taobao/{shop_id}"))
            .method("POST")
            .header("x-custom-id", "evt-42")
            .body(Body::from("payload"))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = response.into_body().collect().await?.to_bytes();
        let json: Value = serde_json::from_slice(&body_bytes)?;
        assert_eq!(json.get("duplicate").and_then(|v| v.as_bool()), Some(true));

        let inbox_count: i64 = sqlx::query_scalar("select count(*) from inbox_events")
            .fetch_one(&pool)
            .await?;
        assert_eq!(inbox_count, 1);
        Ok(())
    }

    #[tokio::test]
    async fn webhook_rejects_invalid_signature() -> anyhow::Result<()> {
        let Some(pool) = try_pool().await else {
            eprintln!("skipping webhook_rejects_invalid_signature: db unavailable");
            return Ok(());
        };
        setup_schema(&pool).await?;

        let tenant_id = uuid::Uuid::new_v4();
        let shop_id = uuid::Uuid::new_v4();

        sqlx::query(
            r#"
            insert into tenants (tenant_id, name, status)
            values ($1, 'test-tenant', 'ACTIVE')
            "#,
        )
        .bind(tenant_id)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into shops (
              shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status, credentials, auth_meta_json
            ) values ($1, $2, 'pdd', 'PDD-SHOP', 'shop', 'shop', 'AUTHORIZED', '{}'::jsonb, '{}'::jsonb)
            "#,
        )
        .bind(shop_id)
        .bind(tenant_id)
        .execute(&pool)
        .await?;

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
        let db = Db::connect_lazy(&database_url)?;
        let state = AppState {
            db,
            adapters: Arc::new(AdapterRegistry::new()),
        };
        let app = Router::new()
            .route("/webhooks/:platform/:shop_id", post(webhook_ingest))
            .with_state(state);

        let request = Request::builder()
            .uri(format!("/webhooks/pdd/{shop_id}"))
            .method("POST")
            .body(Body::from("order_sn=ORDER-1&sign=BAD"))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let _ = response.into_body().collect().await?;

        let raw_count: i64 =
            sqlx::query_scalar("select count(*) from inbox_raw_payloads where tenant_id = $1")
                .bind(tenant_id)
                .fetch_one(&pool)
                .await?;
        assert_eq!(raw_count, 1);

        let verify_status: Option<String> = sqlx::query_scalar(
            "select verify_status from inbox_events where tenant_id = $1 limit 1",
        )
        .bind(tenant_id)
        .fetch_optional(&pool)
        .await?;
        assert_eq!(verify_status.as_deref(), Some("FAILED"));

        Ok(())
    }
}
