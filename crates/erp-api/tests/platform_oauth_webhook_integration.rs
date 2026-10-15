use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use erp_api::{AppState, Claims, JwtService, build_app};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

mod support;

async fn setup_schema(pool: &sqlx::PgPool) -> anyhow::Result<()> {
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
          shop_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          platform text not null,
          platform_shop_id text not null,
          name text not null default '',
          shop_name text not null default '',
          status text not null default 'AUTHORIZED',
          credentials jsonb not null default '{}'::jsonb,
          auth_meta_json jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, platform, platform_shop_id)
        )
        "#,
        r#"
        create table if not exists platform_tokens (
          tenant_id uuid not null,
          shop_id uuid not null,
          token_status text not null default 'ACTIVE',
          access_token text,
          refresh_token text,
          expires_at timestamptz,
          scopes text,
          last_refreshed_at timestamptz,
          refresh_fail_count int not null default 0,
          last_error text not null default '',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, shop_id)
        )
        "#,
        r#"
        create table if not exists sync_jobs (
          job_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          shop_id uuid not null,
          job_type text not null,
          cursor text,
          watermark timestamptz,
          status text not null default 'PENDING',
          last_error text not null default '',
          last_success_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, shop_id, job_type)
        )
        "#,
        r#"
        create table if not exists settings (
          setting_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          scope_type text not null,
          scope_id uuid,
          key text not null,
          value_json jsonb not null default '{}'::jsonb,
          version int not null default 1,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, scope_type, scope_id, key)
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
          received_at timestamptz not null default now(),
          processed_at timestamptz,
          verify_status text not null default 'PASSED',
          failed_reason text not null default '',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, event_month, inbox_event_id),
          unique (tenant_id, event_month, dedup_key)
        )
        "#,
        r#"
        create table if not exists audit_logs (
          audit_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          user_id uuid,
          action text not null,
          entity_type text not null,
          entity_id text,
          before_json jsonb,
          after_json jsonb,
          ip text,
          ua text,
          created_at timestamptz not null default now()
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
        r#"
        create table if not exists dead_letters (
          dead_letter_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          outbox_id uuid not null,
          event_type text not null,
          payload jsonb not null,
          error text not null,
          attempts int not null,
          classification text not null,
          next_action text not null,
          status text not null default 'PENDING',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          replayed_at timestamptz
        )
        "#,
        r#"
        create unique index if not exists idx_dead_letters_outbox
          on dead_letters(tenant_id, outbox_id)
        "#,
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}

fn build_token(
    jwt: &JwtService,
    tenant_id: uuid::Uuid,
    user_id: uuid::Uuid,
    perms: Vec<&str>,
) -> String {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims::bearer(
        user_id.to_string(),
        tenant_id.to_string(),
        perms.into_iter().map(|perm| perm.to_string()).collect(),
        false,
        now,
        now + 3600,
    );
    jwt.encode(&claims).expect("token should encode")
}

async fn load_db() -> anyhow::Result<Option<Db>> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(None);
    };
    match Db::connect(&database_url).await {
        Ok(db) => Ok(Some(db)),
        Err(err) => {
            eprintln!("skipping db test: {err}");
            Ok(None)
        }
    }
}

fn month_start(dt: DateTime<Utc>) -> NaiveDate {
    NaiveDate::from_ymd_opt(dt.year(), dt.month(), 1).expect("month start should be valid")
}

async fn insert_inbox_event(
    pool: &sqlx::PgPool,
    tenant_id: uuid::Uuid,
    shop_id: uuid::Uuid,
    platform: &str,
    dedup_key: &str,
    status: &str,
    verify_status: &str,
    error: &str,
    failed_reason: &str,
    received_at: DateTime<Utc>,
    processed_at: Option<DateTime<Utc>>,
) -> anyhow::Result<uuid::Uuid> {
    let raw_id: i64 = sqlx::query_scalar(
        r#"
        insert into inbox_raw_payloads (tenant_id, platform, shop_id, headers, body, received_at)
        values ($1, $2, $3, $4, $5, $6)
        returning raw_id
        "#,
    )
    .bind(tenant_id)
    .bind(platform)
    .bind(shop_id)
    .bind(json!({
        "x-signature": "sig-test",
        "x-timestamp": received_at.timestamp(),
    }))
    .bind(Vec::from("{}"))
    .bind(received_at)
    .fetch_one(pool)
    .await?;

    let inbox_event_id: uuid::Uuid = sqlx::query_scalar(
        r#"
        insert into inbox_events (
          tenant_id,
          event_month,
          platform,
          shop_id,
          dedup_key,
          raw_id,
          status,
          error,
          received_at,
          processed_at,
          verify_status,
          failed_reason
        ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        returning inbox_event_id
        "#,
    )
    .bind(tenant_id)
    .bind(month_start(received_at))
    .bind(platform)
    .bind(shop_id)
    .bind(dedup_key)
    .bind(raw_id)
    .bind(status)
    .bind(error)
    .bind(received_at)
    .bind(processed_at)
    .bind(verify_status)
    .bind(failed_reason)
    .fetch_one(pool)
    .await?;

    Ok(inbox_event_id)
}

fn inbox_received_payload(
    tenant_id: uuid::Uuid,
    platform: &str,
    shop_id: uuid::Uuid,
    inbox_event_id: uuid::Uuid,
) -> Value {
    json!({
        "tenant_id": tenant_id,
        "platform": platform,
        "shop_id": shop_id,
        "inbox_event_id": inbox_event_id,
    })
}

#[allow(clippy::too_many_arguments)]
async fn insert_outbox_event(
    pool: &sqlx::PgPool,
    tenant_id: uuid::Uuid,
    outbox_id: uuid::Uuid,
    event_type: &str,
    payload: Value,
    status: &str,
    attempts: i32,
    next_retry_at: DateTime<Utc>,
    last_error: &str,
    created_at: DateTime<Utc>,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        insert into outbox_events (
          tenant_id,
          event_month,
          outbox_id,
          event_type,
          payload,
          status,
          attempts,
          next_retry_at,
          last_error,
          last_attempt_at,
          created_at,
          updated_at
        ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11)
        "#,
    )
    .bind(tenant_id)
    .bind(month_start(created_at))
    .bind(outbox_id)
    .bind(event_type)
    .bind(payload)
    .bind(status)
    .bind(attempts)
    .bind(next_retry_at)
    .bind(last_error)
    .bind(created_at)
    .bind(created_at)
    .execute(pool)
    .await?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn insert_dead_letter(
    pool: &sqlx::PgPool,
    tenant_id: uuid::Uuid,
    outbox_id: uuid::Uuid,
    payload: Value,
    error: &str,
    attempts: i32,
    classification: &str,
    next_action: &str,
    status: &str,
    created_at: DateTime<Utc>,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        insert into dead_letters (
          tenant_id,
          outbox_id,
          event_type,
          payload,
          error,
          attempts,
          classification,
          next_action,
          status,
          created_at,
          updated_at
        ) values ($1, $2, 'INBOX_EVENT_RECEIVED', $3, $4, $5, $6, $7, $8, $9, $9)
        "#,
    )
    .bind(tenant_id)
    .bind(outbox_id)
    .bind(payload)
    .bind(error)
    .bind(attempts)
    .bind(classification)
    .bind(next_action)
    .bind(status)
    .bind(created_at)
    .execute(pool)
    .await?;

    Ok(())
}

fn parse_alerts(item: &Value) -> Vec<String> {
    item.get("alert_messages")
        .and_then(Value::as_array)
        .expect("alert_messages should exist")
        .iter()
        .filter_map(|value| value.as_str().map(ToString::to_string))
        .collect()
}

fn assert_json_timestamp_eq(item: &Value, field: &str, expected: DateTime<Utc>) {
    let raw = item
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("{field} should exist"));
    let actual = DateTime::parse_from_rfc3339(raw)
        .expect("timestamp should be valid rfc3339")
        .with_timezone(&Utc);
    assert_eq!(actual, expected);
}

#[tokio::test]
async fn platform_integration_shop_overview_requires_permissions() {
    let database_url = support::test_database_url();
    let db = Db::connect_lazy(&database_url).expect("lazy db should be constructible");
    let jwt = JwtService::new("test-secret".to_string());
    let app = build_app(AppState::new(db, Arc::new(jwt.clone())));

    let tenant_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, tenant_id, uuid::Uuid::new_v4(), vec![]);

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/system/integrations/shops"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn platform_integration_shop_overview_reflects_oauth_webhook_and_sync_health()
-> anyhow::Result<()> {
    let Some(db) = load_db().await? else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let healthy_shop_id = uuid::Uuid::new_v4();
    let disconnected_shop_id = uuid::Uuid::new_v4();
    let webhook_warning_shop_id = uuid::Uuid::new_v4();
    let webhook_dead_shop_id = uuid::Uuid::new_v4();
    let healthy_webhook_received_at = Utc::now() - Duration::minutes(3);
    let healthy_webhook_processed_at = healthy_webhook_received_at + Duration::seconds(20);
    let webhook_warning_received_at = Utc::now() - Duration::minutes(9);
    let webhook_dead_received_at = Utc::now() - Duration::minutes(16);
    let webhook_retry_at = Utc::now() + Duration::minutes(4);

    sqlx::query("insert into tenants (tenant_id, name, status) values ($1, 'tenant-a', 'ACTIVE')")
        .bind(tenant_id)
        .execute(db.pool())
        .await?;

    sqlx::query(
        r#"
        insert into shops (
          shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status, credentials, auth_meta_json
        ) values ($1, $2, 'taobao', 'TB-HEALTHY', '鍋ュ悍店铺', '鍋ュ悍店铺', 'AUTHORIZED', $3, $4)
        "#,
    )
    .bind(healthy_shop_id)
    .bind(tenant_id)
    .bind(json!({
        "webhook_verification": {
            "mode": "STRICT"
        }
    }))
    .bind(json!({
        "authorize_url": "https://auth.example.com/connect"
    }))
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shops (
          shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status, credentials, auth_meta_json
        ) values ($1, $2, 'jd', 'JD-DISCONNECTED', '异常店铺', '异常店铺', 'AUTHORIZED', $3, $4)
        "#,
    )
    .bind(disconnected_shop_id)
    .bind(tenant_id)
    .bind(json!({
        "webhook_verification": {
            "mode": "STRICT"
        }
    }))
    .bind(json!({
        "oauth_status": "FAILED",
        "oauth_error": "state_mismatch",
        "oauth_updated_at": "2026-04-08T09:00:00Z"
    }))
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shops (
          shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status, credentials, auth_meta_json
        ) values ($1, $2, 'douyin', 'DY-WEBHOOK-WARN', 'Webhook 棰勮店铺', 'Webhook 棰勮店铺', 'AUTHORIZED', $3, '{}'::jsonb)
        "#,
    )
    .bind(webhook_warning_shop_id)
    .bind(tenant_id)
    .bind(json!({
        "webhook_verification": {
            "mode": "STRICT"
        }
    }))
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shops (
          shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status, credentials, auth_meta_json
        ) values ($1, $2, 'pinduoduo', 'PDD-WEBHOOK-DEAD', 'Webhook Dead Shop', 'Webhook Dead Shop', 'AUTHORIZED', $3, '{}'::jsonb)
        "#,
    )
    .bind(webhook_dead_shop_id)
    .bind(tenant_id)
    .bind(json!({
        "webhook_verification": {
            "mode": "STRICT"
        }
    }))
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into platform_tokens (
          tenant_id,
          shop_id,
          token_status,
          access_token,
          refresh_token,
          expires_at,
          scopes,
          last_refreshed_at,
          refresh_fail_count,
          last_error
        ) values ($1, $2, 'ACTIVE', 'dy-access-token', 'dy-refresh-token', $3, 'orders,inventory', $4, 0, '')
        "#,
    )
    .bind(tenant_id)
    .bind(webhook_warning_shop_id)
    .bind(Utc::now() + Duration::hours(4))
    .bind(Utc::now() - Duration::minutes(30))
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into platform_tokens (
          tenant_id,
          shop_id,
          token_status,
          access_token,
          refresh_token,
          expires_at,
          scopes,
          last_refreshed_at,
          refresh_fail_count,
          last_error
        ) values ($1, $2, 'ACTIVE', 'pdd-access-token', 'pdd-refresh-token', $3, 'orders,inventory', $4, 0, '')
        "#,
    )
    .bind(tenant_id)
    .bind(webhook_dead_shop_id)
    .bind(Utc::now() + Duration::hours(4))
    .bind(Utc::now() - Duration::minutes(40))
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into sync_jobs (tenant_id, shop_id, job_type, status, last_error, last_success_at)
        values
          ($1, $2, 'order_sync', 'PENDING', '', $3),
          ($1, $2, 'refund_sync', 'PENDING', '', $4),
          ($1, $5, 'order_sync', 'FAILED', 'platform timeout', $6),
          ($1, $5, 'shipping_sync', 'PENDING', '', $7),
          ($1, $8, 'order_sync', 'PENDING', '', $9),
          ($1, $10, 'order_sync', 'PENDING', '', $11)
        "#,
    )
    .bind(tenant_id)
    .bind(healthy_shop_id)
    .bind(Utc::now() - Duration::minutes(5))
    .bind(Utc::now() - Duration::minutes(8))
    .bind(disconnected_shop_id)
    .bind(Utc::now() - Duration::hours(6))
    .bind(Utc::now() - Duration::hours(5))
    .bind(webhook_warning_shop_id)
    .bind(Utc::now() - Duration::minutes(7))
    .bind(webhook_dead_shop_id)
    .bind(Utc::now() - Duration::minutes(18))
    .execute(db.pool())
    .await?;

    let _ = insert_inbox_event(
        db.pool(),
        tenant_id,
        healthy_shop_id,
        "taobao",
        "tb-healthy-webhook-1",
        "PROCESSED",
        "PASSED",
        "",
        "",
        healthy_webhook_received_at,
        Some(healthy_webhook_processed_at),
    )
    .await?;

    let webhook_warning_inbox_event_id = insert_inbox_event(
        db.pool(),
        tenant_id,
        webhook_warning_shop_id,
        "douyin",
        "dy-webhook-warning-1",
        "FAILED",
        "FAILED",
        "signature mismatch",
        "signature mismatch",
        webhook_warning_received_at,
        None,
    )
    .await?;

    let webhook_dead_inbox_event_id = insert_inbox_event(
        db.pool(),
        tenant_id,
        webhook_dead_shop_id,
        "pinduoduo",
        "pdd-webhook-dead-1",
        "FAILED",
        "PASSED",
        "worker timeout",
        "worker timeout",
        webhook_dead_received_at,
        None,
    )
    .await?;

    insert_outbox_event(
        db.pool(),
        tenant_id,
        uuid::Uuid::new_v4(),
        "INBOX_EVENT_RECEIVED",
        inbox_received_payload(
            tenant_id,
            "douyin",
            webhook_warning_shop_id,
            webhook_warning_inbox_event_id,
        ),
        "PENDING",
        0,
        webhook_warning_received_at,
        "",
        webhook_warning_received_at,
    )
    .await?;
    insert_outbox_event(
        db.pool(),
        tenant_id,
        uuid::Uuid::new_v4(),
        "INBOX_EVENT_RECEIVED",
        inbox_received_payload(
            tenant_id,
            "douyin",
            webhook_warning_shop_id,
            webhook_warning_inbox_event_id,
        ),
        "FAILED",
        2,
        webhook_retry_at,
        "signature mismatch",
        webhook_warning_received_at + Duration::seconds(15),
    )
    .await?;

    let webhook_dead_outbox_id = uuid::Uuid::new_v4();
    insert_outbox_event(
        db.pool(),
        tenant_id,
        webhook_dead_outbox_id,
        "INBOX_EVENT_RECEIVED",
        inbox_received_payload(
            tenant_id,
            "pinduoduo",
            webhook_dead_shop_id,
            webhook_dead_inbox_event_id,
        ),
        "DEAD",
        5,
        webhook_dead_received_at,
        "worker timeout",
        webhook_dead_received_at + Duration::seconds(10),
    )
    .await?;
    insert_dead_letter(
        db.pool(),
        tenant_id,
        webhook_dead_outbox_id,
        inbox_received_payload(
            tenant_id,
            "pinduoduo",
            webhook_dead_shop_id,
            webhook_dead_inbox_event_id,
        ),
        "worker timeout",
        5,
        "TRANSIENT",
        "MANUAL_REPLAY",
        "PENDING",
        webhook_dead_received_at + Duration::seconds(10),
    )
    .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let app = build_app(AppState::new(db.clone(), Arc::new(jwt.clone())));
    let write_token = build_token(
        &jwt,
        tenant_id,
        uuid::Uuid::new_v4(),
        vec!["settings.read", "channels.write"],
    );
    let read_token = build_token(&jwt, tenant_id, uuid::Uuid::new_v4(), vec!["settings.read"]);

    let authorize_request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/shops/{healthy_shop_id}/auth/authorize"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {write_token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "redirect_uri": "https://erp.example.com/callback"
            })
            .to_string(),
        ))
        .unwrap();
    let authorize_response = app.clone().oneshot(authorize_request).await.unwrap();
    assert_eq!(authorize_response.status(), StatusCode::OK);
    let authorize_body = authorize_response.into_body().collect().await?.to_bytes();
    let authorize_payload: Value = serde_json::from_slice(&authorize_body)?;
    let state = authorize_payload
        .get("state")
        .and_then(Value::as_str)
        .expect("authorize state should exist")
        .to_string();

    let callback_request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/shops/{healthy_shop_id}/auth/callback"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {write_token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "access_token": "access-token",
                "refresh_token": "refresh-token",
                "expires_at": (Utc::now() + Duration::hours(2)).to_rfc3339(),
                "scopes": "orders,inventory",
                "state": state
            })
            .to_string(),
        ))
        .unwrap();
    let callback_response = app.clone().oneshot(callback_request).await.unwrap();
    assert_eq!(callback_response.status(), StatusCode::OK);

    let overview_request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/system/integrations/shops"))
        .method("GET")
        .header("authorization", format!("Bearer {read_token}"))
        .body(Body::empty())
        .unwrap();
    let overview_response = app.clone().oneshot(overview_request).await.unwrap();
    assert_eq!(overview_response.status(), StatusCode::OK);
    let overview_body = overview_response.into_body().collect().await?.to_bytes();
    let overview_payload: Value = serde_json::from_slice(&overview_body)?;
    let items = overview_payload
        .get("items")
        .and_then(Value::as_array)
        .expect("overview items should exist");
    assert_eq!(items.len(), 4);

    let healthy = items
        .iter()
        .find(|item| {
            item.get("shop_id").and_then(Value::as_str) == Some(&healthy_shop_id.to_string())
        })
        .expect("healthy shop should exist");
    assert_eq!(
        healthy.get("oauth_status").and_then(Value::as_str),
        Some("SUCCESS")
    );
    assert_eq!(
        healthy.get("integration_status").and_then(Value::as_str),
        Some("CONNECTED")
    );
    assert_eq!(
        healthy.get("webhook_mode").and_then(Value::as_str),
        Some("STRICT")
    );
    assert_eq!(
        healthy.get("webhook_status").and_then(Value::as_str),
        Some("HEALTHY")
    );
    assert_eq!(
        healthy.get("webhook_sla_status").and_then(Value::as_str),
        Some("HEALTHY")
    );
    assert_eq!(
        healthy.get("webhook_failed_count").and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        healthy
            .get("webhook_verify_failed_count")
            .and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        healthy.get("webhook_pending_count").and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        healthy
            .get("webhook_retrying_count")
            .and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        healthy.get("webhook_dlq_count").and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        healthy.get("failed_job_count").and_then(Value::as_i64),
        Some(0)
    );
    assert_json_timestamp_eq(
        healthy,
        "last_webhook_received_at",
        healthy_webhook_received_at,
    );
    assert_json_timestamp_eq(
        healthy,
        "last_webhook_processed_at",
        healthy_webhook_processed_at,
    );
    assert!(
        healthy
            .get("webhook_next_retry_at")
            .is_some_and(Value::is_null)
    );
    assert!(parse_alerts(healthy).is_empty());

    let disconnected = items
        .iter()
        .find(|item| {
            item.get("shop_id").and_then(Value::as_str) == Some(&disconnected_shop_id.to_string())
        })
        .expect("disconnected shop should exist");
    assert_eq!(
        disconnected.get("oauth_status").and_then(Value::as_str),
        Some("FAILED")
    );
    assert_eq!(
        disconnected
            .get("integration_status")
            .and_then(Value::as_str),
        Some("DISCONNECTED")
    );
    assert_eq!(
        disconnected.get("webhook_status").and_then(Value::as_str),
        Some("IDLE")
    );
    assert_eq!(
        disconnected
            .get("webhook_sla_status")
            .and_then(Value::as_str),
        Some("IDLE")
    );
    assert!(
        disconnected
            .get("last_webhook_received_at")
            .is_some_and(Value::is_null)
    );
    assert!(
        disconnected
            .get("last_webhook_processed_at")
            .is_some_and(Value::is_null)
    );
    assert!(
        disconnected
            .get("webhook_next_retry_at")
            .is_some_and(Value::is_null)
    );
    assert_eq!(
        disconnected
            .get("webhook_failed_count")
            .and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        disconnected
            .get("webhook_verify_failed_count")
            .and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        disconnected
            .get("webhook_pending_count")
            .and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        disconnected
            .get("webhook_retrying_count")
            .and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        disconnected
            .get("webhook_dlq_count")
            .and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        disconnected.get("failed_job_count").and_then(Value::as_i64),
        Some(1)
    );
    let disconnected_alerts = parse_alerts(disconnected);
    assert!(
        disconnected_alerts
            .iter()
            .any(|value| value.contains("\u{6388}\u{6743}"))
    );
    assert!(
        disconnected_alerts
            .iter()
            .any(|value| value.contains("order_sync"))
    );

    let webhook_warning = items
        .iter()
        .find(|item| {
            item.get("shop_id").and_then(Value::as_str)
                == Some(&webhook_warning_shop_id.to_string())
        })
        .expect("webhook warning shop should exist");
    assert_eq!(
        webhook_warning.get("oauth_status").and_then(Value::as_str),
        Some("SUCCESS")
    );
    assert_eq!(
        webhook_warning
            .get("integration_status")
            .and_then(Value::as_str),
        Some("WARNING")
    );
    assert_eq!(
        webhook_warning
            .get("webhook_status")
            .and_then(Value::as_str),
        Some("WARNING")
    );
    assert_eq!(
        webhook_warning
            .get("webhook_sla_status")
            .and_then(Value::as_str),
        Some("WARNING")
    );
    assert_eq!(
        webhook_warning
            .get("failed_job_count")
            .and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        webhook_warning
            .get("webhook_failed_count")
            .and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        webhook_warning
            .get("webhook_verify_failed_count")
            .and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        webhook_warning
            .get("webhook_pending_count")
            .and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        webhook_warning
            .get("webhook_retrying_count")
            .and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        webhook_warning
            .get("webhook_dlq_count")
            .and_then(Value::as_i64),
        Some(0)
    );
    assert_json_timestamp_eq(
        webhook_warning,
        "last_webhook_received_at",
        webhook_warning_received_at,
    );
    assert!(
        webhook_warning
            .get("last_webhook_processed_at")
            .is_some_and(Value::is_null)
    );
    assert_json_timestamp_eq(webhook_warning, "webhook_next_retry_at", webhook_retry_at);
    let webhook_alerts = parse_alerts(webhook_warning);
    let webhook_dead = items
        .iter()
        .find(|item| {
            item.get("shop_id").and_then(Value::as_str) == Some(&webhook_dead_shop_id.to_string())
        })
        .expect("webhook dead-letter shop should exist");
    assert_eq!(
        webhook_dead.get("oauth_status").and_then(Value::as_str),
        Some("SUCCESS")
    );
    assert_eq!(
        webhook_dead
            .get("integration_status")
            .and_then(Value::as_str),
        Some("WARNING")
    );
    assert_eq!(
        webhook_dead.get("webhook_status").and_then(Value::as_str),
        Some("WARNING")
    );
    assert_eq!(
        webhook_dead
            .get("webhook_sla_status")
            .and_then(Value::as_str),
        Some("DEAD")
    );
    assert_eq!(
        webhook_dead
            .get("webhook_pending_count")
            .and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        webhook_dead
            .get("webhook_retrying_count")
            .and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        webhook_dead
            .get("webhook_dlq_count")
            .and_then(Value::as_i64),
        Some(1)
    );
    assert_json_timestamp_eq(
        webhook_dead,
        "last_webhook_received_at",
        webhook_dead_received_at,
    );
    assert!(
        webhook_dead
            .get("last_webhook_processed_at")
            .is_some_and(Value::is_null)
    );
    assert!(
        webhook_dead
            .get("webhook_next_retry_at")
            .is_some_and(Value::is_null)
    );
    let webhook_dead_alerts = parse_alerts(webhook_dead);
    assert!(
        webhook_dead_alerts
            .iter()
            .any(|value| value.contains("\u{6b7b}\u{4fe1}"))
    );
    assert!(
        webhook_alerts
            .iter()
            .any(|value| value.contains("\u{91cd}\u{8bd5}"))
    );
    assert!(
        webhook_alerts
            .iter()
            .any(|value| value.contains("\u{9a8c}\u{7b7e}"))
    );

    Ok(())
}
