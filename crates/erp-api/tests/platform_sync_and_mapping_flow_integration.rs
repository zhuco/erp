use anyhow::Result;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use erp_api::{build_app, AppState, Claims, JwtService};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

async fn setup_schema(pool: &sqlx::PgPool) -> Result<()> {
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
        create table if not exists sync_job_runs (
          run_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          job_id uuid not null,
          shop_id uuid not null,
          job_type text not null,
          status text not null,
          item_count int not null default 0,
          duration_ms bigint not null default 0,
          error text not null default '',
          started_at timestamptz not null default now(),
          finished_at timestamptz not null default now()
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
          updated_at timestamptz not null default now(),
          unique (tenant_id, scope_type, scope_id, key)
        )
        "#,
        r#"
        create table if not exists feature_flags (
          tenant_id uuid not null,
          scope text not null,
          shop_id uuid,
          key text not null,
          enabled boolean not null default true,
          updated_at timestamptz not null default now(),
          unique (tenant_id, scope, shop_id, key)
        )
        "#,
        r#"
        create table if not exists stock_sync_jobs (
          job_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          shop_id uuid not null,
          sku_id uuid not null,
          warehouse_id uuid not null,
          target_qty bigint not null,
          status text not null default 'PENDING',
          attempts int not null default 0,
          last_error text,
          success_count int not null default 0,
          failure_count int not null default 0,
          last_duration_ms int not null default 0,
          last_success_at timestamptz,
          next_retry_at timestamptz not null default now(),
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
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
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}

fn build_token(jwt: &JwtService, tenant_id: &str) -> String {
    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: uuid::Uuid::new_v4().to_string(),
        tenant_id: tenant_id.to_string(),
        perms: vec![
            "settings.read".to_string(),
            "orders.download".to_string(),
            "inventory.write".to_string(),
            "aftersales.write".to_string(),
        ],
        is_admin: false,
        iat: now,
        exp: now + 3600,
    };
    jwt.encode(&claims).expect("token should encode")
}

async fn response_json(response: axum::response::Response) -> Result<Value> {
    let status = response.status();
    let body = response.into_body().collect().await?.to_bytes();
    if status != StatusCode::OK {
        let text = String::from_utf8_lossy(&body);
        anyhow::bail!("unexpected status {status}: {text}");
    }
    Ok(serde_json::from_slice(&body)?)
}

#[tokio::test]
async fn platform_sync_and_mapping_flow_smoke() -> Result<()> {
    if let Err(err) = platform_sync_and_mapping_flow_smoke_inner().await {
        let message = err.to_string();
        if message.contains("pool timed out")
            || message.contains("Connection refused")
            || message.contains("DATABASE_URL")
        {
            eprintln!("skipping platform_sync_and_mapping_flow_smoke: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn platform_sync_and_mapping_flow_smoke_inner() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("TEST_DATABASE_URL"))
        .map_err(|_| anyhow::anyhow!("DATABASE_URL not set"))?;

    let db = Db::connect_lazy(&database_url)?;
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let shop_id = uuid::Uuid::new_v4();
    let stock_job_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();

    sqlx::query("insert into tenants (tenant_id, name, status) values ($1,$2,$3)")
        .bind(tenant_id)
        .bind("test-tenant")
        .bind("ACTIVE")
        .execute(db.pool())
        .await?;

    sqlx::query(
        r#"
        insert into shops (
          shop_id, tenant_id, platform, platform_shop_id, name, status, credentials, auth_meta_json
        ) values ($1,$2,$3,$4,$5,$6,$7,$8)
        "#,
    )
    .bind(shop_id)
    .bind(tenant_id)
    .bind("TAOBAO")
    .bind(format!("TB-{shop_id}"))
    .bind("test-shop")
    .bind("AUTHORIZED")
    .bind(json!({}))
    .bind(json!({}))
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into stock_sync_jobs (
          job_id, tenant_id, shop_id, sku_id, warehouse_id, target_qty, status, next_retry_at
        ) values ($1,$2,$3,$4,$5,$6,$7,$8)
        "#,
    )
    .bind(stock_job_id)
    .bind(tenant_id)
    .bind(shop_id)
    .bind(sku_id)
    .bind(warehouse_id)
    .bind(42_i64)
    .bind("FAILED")
    .bind(Utc::now() + Duration::minutes(10))
    .execute(db.pool())
    .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let state = AppState::new(db.clone(), Arc::new(jwt.clone()));
    let app = build_app(state);
    let token = build_token(&jwt, &tenant_id.to_string());

    let update_order_settings_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/v1/tenants/{tenant_id}/shops/{shop_id}/sync-settings/orders"
                ))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "auto_download_orders": true,
                        "download_interval_minutes": 15
                    })
                    .to_string(),
                ))?,
        )
        .await?;
    let update_order_settings = response_json(update_order_settings_resp).await?;
    assert_eq!(
        update_order_settings
            .get("download_interval_minutes")
            .and_then(Value::as_i64),
        Some(15)
    );

    let get_order_settings_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/v1/tenants/{tenant_id}/shops/{shop_id}/sync-settings/orders"
                ))
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())?,
        )
        .await?;
    let get_order_settings = response_json(get_order_settings_resp).await?;
    assert_eq!(
        get_order_settings
            .get("auto_download_orders")
            .and_then(Value::as_bool),
        Some(true)
    );

    let manual_download_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/v1/tenants/{tenant_id}/shops/{shop_id}/platform-orders/download/manual"
                ))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "cursor": "cursor-1"
                    })
                    .to_string(),
                ))?,
        )
        .await?;
    let manual_download = response_json(manual_download_resp).await?;
    assert_eq!(
        manual_download
            .get("job")
            .and_then(|v| v.get("job_type"))
            .and_then(Value::as_str),
        Some("order_sync")
    );

    let time_range_download_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/v1/tenants/{tenant_id}/shops/{shop_id}/platform-orders/download/by-time-range"
                ))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "start_time": "2026-01-01T00:00:00Z",
                        "end_time": "2026-01-01T01:00:00Z",
                        "cursor": "cursor-2"
                    })
                    .to_string(),
                ))?,
        )
        .await?;
    let time_range_download = response_json(time_range_download_resp).await?;
    assert_eq!(
        time_range_download
            .get("job")
            .and_then(|v| v.get("job_type"))
            .and_then(Value::as_str),
        Some("order_sync")
    );

    let update_inventory_settings_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/v1/tenants/{tenant_id}/shops/{shop_id}/sync-settings/inventory"
                ))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "enable_stock_sync": true
                    })
                    .to_string(),
                ))?,
        )
        .await?;
    let update_inventory_settings = response_json(update_inventory_settings_resp).await?;
    assert_eq!(
        update_inventory_settings
            .get("enable_stock_sync")
            .and_then(Value::as_bool),
        Some(true)
    );

    let inventory_sync_run_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/v1/tenants/{tenant_id}/shops/{shop_id}/platform-sync/inventory/run"
                ))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "status": "FAILED",
                        "limit": 20
                    })
                    .to_string(),
                ))?,
        )
        .await?;
    let inventory_sync_run = response_json(inventory_sync_run_resp).await?;
    assert_eq!(
        inventory_sync_run.get("requested").and_then(Value::as_i64),
        Some(1)
    );

    let update_after_sales_settings_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/v1/tenants/{tenant_id}/shops/{shop_id}/sync-settings/after-sales"
                ))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "enable_after_sales_download": true
                    })
                    .to_string(),
                ))?,
        )
        .await?;
    let update_after_sales_settings = response_json(update_after_sales_settings_resp).await?;
    assert_eq!(
        update_after_sales_settings
            .get("enable_after_sales_download")
            .and_then(Value::as_bool),
        Some(true)
    );

    let after_sales_sync_run_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/v1/tenants/{tenant_id}/shops/{shop_id}/platform-sync/after-sales/run"
                ))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from("{}"))?,
        )
        .await?;
    let after_sales_sync_run = response_json(after_sales_sync_run_resp).await?;
    assert_eq!(
        after_sales_sync_run.get("job_type").and_then(Value::as_str),
        Some("refund_sync")
    );

    let return_exchange_sync_run_resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/v1/tenants/{tenant_id}/shops/{shop_id}/platform-sync/returns-exchanges/run"
                ))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from("{}"))?,
        )
        .await?;
    let return_exchange_sync_run = response_json(return_exchange_sync_run_resp).await?;
    assert_eq!(
        return_exchange_sync_run
            .get("job_type")
            .and_then(Value::as_str),
        Some("refund_sync")
    );

    let sync_job_count: i64 =
        sqlx::query_scalar("select count(*) from sync_jobs where tenant_id = $1 and shop_id = $2")
            .bind(tenant_id)
            .bind(shop_id)
            .fetch_one(db.pool())
            .await?;
    assert_eq!(sync_job_count, 2);

    let sync_run_count: i64 = sqlx::query_scalar(
        "select count(*) from sync_job_runs where tenant_id = $1 and shop_id = $2",
    )
    .bind(tenant_id)
    .bind(shop_id)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(sync_run_count, 4);

    let settings_count: i64 = sqlx::query_scalar(
        "select count(*) from settings where tenant_id = $1 and scope_type = 'SHOP' and scope_id = $2",
    )
    .bind(tenant_id)
    .bind(shop_id)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(settings_count, 8);

    let stock_status: String = sqlx::query_scalar(
        "select status from stock_sync_jobs where tenant_id = $1 and job_id = $2",
    )
    .bind(tenant_id)
    .bind(stock_job_id)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(stock_status, "PENDING");

    Ok(())
}
