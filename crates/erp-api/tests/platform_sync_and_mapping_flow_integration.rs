use anyhow::Result;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use erp_api::{AppState, Claims, JwtService, build_app};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use serde_json::{Value, json};
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
        r#"
        create table if not exists products (
          product_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          name text not null,
          brand text,
          category text,
          attrs jsonb not null default '{}'::jsonb,
          status text not null default 'ACTIVE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists skus (
          sku_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          product_id uuid not null references products(product_id),
          sku_code text not null,
          barcode text,
          name text,
          note text,
          spec jsonb not null default '{}'::jsonb,
          tags text[] not null default '{}'::text[],
          image_urls text[] not null default '{}'::text[],
          weight_gram int,
          length_mm int,
          width_mm int,
          height_mm int,
          volume_cm3 int,
          sku_kind text not null default 'NORMAL',
          cost_cent bigint,
          is_active boolean not null default true,
          status text not null default 'ACTIVE',
          batch_enabled boolean not null default false,
          serial_enabled boolean not null default false,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, sku_code)
        )
        "#,
        r#"
        create table if not exists sku_boms (
          tenant_id uuid not null references tenants(tenant_id),
          bundle_sku_id uuid not null references skus(sku_id),
          component_sku_id uuid not null references skus(sku_id),
          qty bigint not null default 1,
          bom_version int not null default 1,
          expire_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, bundle_sku_id, component_sku_id, bom_version)
        )
        "#,
        r#"
        create table if not exists channel_sku_mappings (
          mapping_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          shop_id uuid not null references shops(shop_id),
          platform text not null,
          platform_sku_id text not null,
          platform_outer_sku_id text,
          sku_id uuid not null references skus(sku_id),
          status text not null default 'ACTIVE',
          conflict_reason text not null default '',
          confidence int not null default 0,
          match_source text not null default 'AUTO',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, shop_id, platform, platform_sku_id)
        )
        "#,
        r#"
        create table if not exists shop_products (
          product_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          shop_id uuid not null references shops(shop_id),
          platform text not null,
          platform_product_id text not null,
          merchant_code text,
          title text not null,
          product_type text not null default 'STANDARD',
          status text not null default 'ACTIVE',
          platform_updated_at timestamptz,
          raw_json jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, shop_id, platform, platform_product_id)
        )
        "#,
        r#"
        create table if not exists shop_product_skus (
          sku_record_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          shop_id uuid not null references shops(shop_id),
          platform text not null,
          platform_product_id text not null,
          platform_sku_id text not null,
          merchant_sku_code text,
          title text,
          spec_text text,
          price_cent bigint,
          stock_qty bigint,
          status text not null default 'ACTIVE',
          barcode text,
          image_url text,
          platform_updated_at timestamptz,
          raw_json jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, shop_id, platform, platform_sku_id)
        )
        "#,
        r#"
        create table if not exists shop_product_tasks (
          task_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          task_type text not null,
          status text not null default 'PENDING',
          shop_id uuid references shops(shop_id),
          platform text,
          total_count int not null default 0,
          success_count int not null default 0,
          failure_count int not null default 0,
          error text not null default '',
          payload_json jsonb not null default '{}'::jsonb,
          result_json jsonb not null default '{}'::jsonb,
          started_at timestamptz not null default now(),
          finished_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists operation_logs (
          log_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          actor_type text not null,
          actor_user_id uuid,
          action text not null,
          ref_type text,
          ref_id text,
          metadata jsonb not null default '{}'::jsonb,
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
            "shop_products.read".to_string(),
            "shop_products.write".to_string(),
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

#[tokio::test]
async fn shop_product_match_writes_operation_logs() -> Result<()> {
    if let Err(err) = shop_product_match_writes_operation_logs_inner().await {
        let message = err.to_string();
        if message.contains("pool timed out")
            || message.contains("Connection refused")
            || message.contains("DATABASE_URL")
        {
            eprintln!("skipping shop_product_match_writes_operation_logs: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn shop_product_match_writes_operation_logs_inner() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("TEST_DATABASE_URL"))
        .map_err(|_| anyhow::anyhow!("DATABASE_URL not set"))?;

    let db = Db::connect_lazy(&database_url)?;
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let shop_id = uuid::Uuid::new_v4();
    let product_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();
    let auto_sku_record_id = uuid::Uuid::new_v4();
    let manual_sku_record_id = uuid::Uuid::new_v4();

    sqlx::query("insert into tenants (tenant_id, name, status) values ($1,$2,$3)")
        .bind(tenant_id)
        .bind("match-tenant")
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
    .bind(format!("TB-MATCH-{shop_id}"))
    .bind("match-shop")
    .bind("AUTHORIZED")
    .bind(json!({}))
    .bind(json!({}))
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into products (product_id, tenant_id, name, status)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(product_id)
    .bind(tenant_id)
    .bind("mock-product")
    .bind("ACTIVE")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into skus (
          sku_id, tenant_id, product_id, sku_code, barcode, name, spec, tags, image_urls,
          sku_kind, cost_cent, is_active, status, batch_enabled, serial_enabled
        ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
        "#,
    )
    .bind(sku_id)
    .bind(tenant_id)
    .bind(product_id)
    .bind("SKU-MATCH-001")
    .bind("BC-MATCH-001")
    .bind("mock-sku")
    .bind(json!({"spec_text":"default"}))
    .bind(Vec::<String>::new())
    .bind(Vec::<String>::new())
    .bind("NORMAL")
    .bind(Some(100_i64))
    .bind(true)
    .bind("ACTIVE")
    .bind(false)
    .bind(false)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shop_products (
          tenant_id, shop_id, platform, platform_product_id, merchant_code, title,
          product_type, status, raw_json
        ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9)
        "#,
    )
    .bind(tenant_id)
    .bind(shop_id)
    .bind("TAOBAO")
    .bind("P-MATCH-1")
    .bind("MRC-1")
    .bind("match-product")
    .bind("STANDARD")
    .bind("ACTIVE")
    .bind(json!({}))
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shop_product_skus (
          sku_record_id, tenant_id, shop_id, platform, platform_product_id, platform_sku_id,
          merchant_sku_code, title, spec_text, status, raw_json
        ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
        "#,
    )
    .bind(auto_sku_record_id)
    .bind(tenant_id)
    .bind(shop_id)
    .bind("TAOBAO")
    .bind("P-MATCH-1")
    .bind("P-SKU-AUTO-1")
    .bind("SKU-MATCH-001")
    .bind("auto-sku")
    .bind("default")
    .bind("ACTIVE")
    .bind(json!({}))
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shop_product_skus (
          sku_record_id, tenant_id, shop_id, platform, platform_product_id, platform_sku_id,
          merchant_sku_code, title, spec_text, status, raw_json
        ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
        "#,
    )
    .bind(manual_sku_record_id)
    .bind(tenant_id)
    .bind(shop_id)
    .bind("TAOBAO")
    .bind("P-MATCH-1")
    .bind("P-SKU-MANUAL-1")
    .bind("SKU-MATCH-MANUAL")
    .bind("manual-sku")
    .bind("default")
    .bind("ACTIVE")
    .bind(json!({}))
    .execute(db.pool())
    .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let state = AppState::new(db.clone(), Arc::new(jwt.clone()));
    let app = build_app(state);
    let token = build_token(&jwt, &tenant_id.to_string());

    let auto_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/tenants/{tenant_id}/shop-products/auto-match"))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "sku_record_ids": [auto_sku_record_id.to_string()]
                    })
                    .to_string(),
                ))?,
        )
        .await?;
    let auto_json = response_json(auto_resp).await?;
    assert_eq!(
        auto_json
            .get("task")
            .and_then(|v| v.get("task_type"))
            .and_then(Value::as_str),
        Some("AUTO_MATCH")
    );

    let manual_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/v1/tenants/{tenant_id}/shop-products/manual-match"
                ))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "sku_record_ids": [manual_sku_record_id.to_string()],
                        "sku_id": sku_id.to_string()
                    })
                    .to_string(),
                ))?,
        )
        .await?;
    let manual_json = response_json(manual_resp).await?;
    assert_eq!(
        manual_json
            .get("task")
            .and_then(|v| v.get("task_type"))
            .and_then(Value::as_str),
        Some("MANUAL_MATCH")
    );

    let delete_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/v1/tenants/{tenant_id}/shop-products/delete-match"
                ))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "sku_record_ids": [manual_sku_record_id.to_string()]
                    })
                    .to_string(),
                ))?,
        )
        .await?;
    let delete_json = response_json(delete_resp).await?;
    let delete_task = delete_json
        .get("task")
        .expect("delete match response should include task");
    assert_eq!(
        delete_task.get("task_type").and_then(Value::as_str),
        Some("DELETE_MATCH")
    );
    assert_eq!(
        delete_task.get("status").and_then(Value::as_str),
        Some("SUCCESS")
    );
    assert_eq!(
        delete_task.get("total_count").and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        delete_task.get("success_count").and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        delete_task.get("failure_count").and_then(Value::as_i64),
        Some(0)
    );

    let list_manual_sku_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/v1/tenants/{tenant_id}/shop-products/skus?keyword=P-SKU-MANUAL-1&limit=10"
                ))
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())?,
        )
        .await?;
    let list_manual_sku_json = response_json(list_manual_sku_resp).await?;
    let manual_items = list_manual_sku_json
        .get("items")
        .and_then(Value::as_array)
        .expect("shop-product sku list should include items");
    assert_eq!(manual_items.len(), 1);
    let manual_item = &manual_items[0];
    assert_eq!(
        manual_item.get("platform_sku_id").and_then(Value::as_str),
        Some("P-SKU-MANUAL-1")
    );
    assert!(
        manual_item
            .get("mapping_id")
            .map(Value::is_null)
            .unwrap_or(false)
    );
    assert!(
        manual_item
            .get("mapping_status")
            .map(Value::is_null)
            .unwrap_or(false)
    );
    assert!(
        manual_item
            .get("match_source")
            .map(Value::is_null)
            .unwrap_or(false)
    );

    let manual_mapping_count_after_delete: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from channel_sku_mappings
        where tenant_id = $1
          and shop_id = $2
          and platform = 'TAOBAO'
          and platform_sku_id = 'P-SKU-MANUAL-1'
        "#,
    )
    .bind(tenant_id)
    .bind(shop_id)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(manual_mapping_count_after_delete, 0);

    let auto_log_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from operation_logs
        where tenant_id = $1
          and action = 'SHOP_PRODUCTS.AUTO_MATCH'
          and actor_user_id is not null
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(auto_log_count, 1);

    let manual_log_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from operation_logs
        where tenant_id = $1
          and action = 'SHOP_PRODUCTS.MANUAL_MATCH'
          and actor_user_id is not null
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(manual_log_count, 1);

    let delete_log_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from operation_logs
        where tenant_id = $1
          and action = 'SHOP_PRODUCTS.DELETE_MATCH'
          and actor_user_id is not null
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(delete_log_count, 1);

    let auto_log_metadata: Value = sqlx::query_scalar(
        r#"
        select metadata
        from operation_logs
        where tenant_id = $1 and action = 'SHOP_PRODUCTS.AUTO_MATCH'
        order by created_at desc
        limit 1
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db.pool())
    .await?;
    assert!(auto_log_metadata.get("task_id").is_some());
    assert_eq!(
        auto_log_metadata
            .get("success_count")
            .and_then(Value::as_i64),
        Some(1)
    );

    let manual_log_metadata: Value = sqlx::query_scalar(
        r#"
        select metadata
        from operation_logs
        where tenant_id = $1 and action = 'SHOP_PRODUCTS.MANUAL_MATCH'
        order by created_at desc
        limit 1
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db.pool())
    .await?;
    let expected_sku_id = sku_id.to_string();
    assert!(manual_log_metadata.get("task_id").is_some());
    assert_eq!(
        manual_log_metadata
            .get("requested_sku_id")
            .and_then(Value::as_str),
        Some(expected_sku_id.as_str())
    );

    let delete_log_metadata: Value = sqlx::query_scalar(
        r#"
        select metadata
        from operation_logs
        where tenant_id = $1 and action = 'SHOP_PRODUCTS.DELETE_MATCH'
        order by created_at desc
        limit 1
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db.pool())
    .await?;
    let delete_task_id = delete_task
        .get("task_id")
        .and_then(Value::as_str)
        .expect("delete task should include task_id");
    let expected_deleted_record_id = manual_sku_record_id.to_string();
    assert_eq!(
        delete_log_metadata.get("task_id").and_then(Value::as_str),
        Some(delete_task_id)
    );
    assert_eq!(
        delete_log_metadata.get("task_type").and_then(Value::as_str),
        Some("DELETE_MATCH")
    );
    assert_eq!(
        delete_log_metadata
            .get("task_status")
            .and_then(Value::as_str),
        Some("SUCCESS")
    );
    assert_eq!(
        delete_log_metadata
            .get("total_count")
            .and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        delete_log_metadata
            .get("success_count")
            .and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        delete_log_metadata
            .get("failure_count")
            .and_then(Value::as_i64),
        Some(0)
    );
    let requested_delete_ids = delete_log_metadata
        .get("requested_sku_record_ids")
        .and_then(Value::as_array)
        .expect("delete operation log should include requested_sku_record_ids");
    assert_eq!(requested_delete_ids.len(), 1);
    assert_eq!(
        requested_delete_ids
            .first()
            .and_then(Value::as_str)
            .unwrap_or_default(),
        expected_deleted_record_id
    );

    Ok(())
}
