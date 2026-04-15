use anyhow::{Context, Result};
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use chrono::{Duration, Utc};
use erp_api::{build_app, AppState, Claims, JwtService};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration as StdDuration;
use tower::ServiceExt;

mod support;

async fn setup_schema(pool: &sqlx::PgPool) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("select pg_advisory_xact_lock(9101001)")
        .execute(&mut *tx)
        .await?;

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
        create table if not exists warehouses (
          warehouse_id uuid primary key,
          tenant_id uuid not null,
          name text not null,
          code text,
          is_default boolean not null default false,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists products (
          product_id uuid primary key,
          tenant_id uuid not null,
          name text not null,
          status text not null default 'ACTIVE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists skus (
          sku_id uuid primary key,
          tenant_id uuid not null,
          product_id uuid not null,
          sku_code text not null,
          barcode text,
          name text,
          spec jsonb not null default '{}'::jsonb,
          tags text[] not null default '{}',
          image_urls text[] not null default '{}',
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
        create table if not exists inventory_balances (
          tenant_id uuid not null,
          warehouse_id uuid not null,
          sku_id uuid not null,
          on_hand bigint not null default 0,
          reserved bigint not null default 0,
          in_transit bigint not null default 0,
          frozen bigint not null default 0,
          updated_at timestamptz not null default now(),
          primary key (tenant_id, warehouse_id, sku_id)
        )
        "#,
        r#"
        create table if not exists sku_batches (
          batch_id uuid primary key,
          tenant_id uuid not null,
          sku_id uuid not null,
          batch_no text not null,
          production_date timestamptz,
          expire_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, sku_id, batch_no)
        )
        "#,
        r#"
        create table if not exists inventory_batch_balances (
          tenant_id uuid not null,
          warehouse_id uuid not null,
          sku_id uuid not null,
          batch_id uuid not null,
          on_hand bigint not null default 0,
          reserved bigint not null default 0,
          updated_at timestamptz not null default now(),
          primary key (tenant_id, warehouse_id, sku_id, batch_id)
        )
        "#,
        r#"
        create table if not exists product_packaging_specs (
          spec_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          sku_id uuid not null,
          level_code text not null,
          unit_name text not null,
          units_per_level bigint not null default 1,
          length_mm int,
          width_mm int,
          height_mm int,
          weight_gram int,
          is_default boolean not null default false,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, sku_id, level_code)
        )
        "#,
        r#"
        create table if not exists inventory_reconciliation_center (
          reconciliation_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          warehouse_id uuid not null,
          sku_id uuid not null,
          expected_on_hand bigint not null,
          actual_on_hand bigint not null,
          diff_qty bigint not null,
          status text not null,
          source_ref text,
          created_at timestamptz not null default now()
        )
        "#,
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(&mut *tx).await?;
    }

    tx.commit().await?;
    Ok(())
}

struct SeedData {
    tenant_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
}

async fn seed_data(pool: &sqlx::PgPool) -> Result<SeedData> {
    let tenant_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();
    let product_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .bind("inventory-atp-test-tenant")
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into warehouses (warehouse_id, tenant_id, name, code, is_default)
        values ($1, $2, 'Main', 'MAIN', true)
        "#,
    )
    .bind(warehouse_id)
    .bind(tenant_id)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into products (product_id, tenant_id, name, status)
        values ($1, $2, 'ATP Product', 'ACTIVE')
        "#,
    )
    .bind(product_id)
    .bind(tenant_id)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into skus (sku_id, tenant_id, product_id, sku_code, name, spec, sku_kind, batch_enabled, serial_enabled)
        values ($1, $2, $3, 'SKU-ATP-001', 'ATP SKU', '{}'::jsonb, 'NORMAL', true, false)
        "#,
    )
    .bind(sku_id)
    .bind(tenant_id)
    .bind(product_id)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into inventory_balances (tenant_id, warehouse_id, sku_id, on_hand, reserved, frozen, in_transit)
        values ($1, $2, $3, 50, 10, 5, 0)
        "#,
    )
    .bind(tenant_id)
    .bind(warehouse_id)
    .bind(sku_id)
    .execute(pool)
    .await?;

    let now = Utc::now();
    let batch_a = uuid::Uuid::new_v4();
    let batch_b = uuid::Uuid::new_v4();
    let batch_c = uuid::Uuid::new_v4();

    for (batch_id, batch_no, expire_at, on_hand, reserved) in [
        (
            batch_a,
            "BATCH-A",
            Some(now + Duration::days(7)),
            8_i64,
            0_i64,
        ),
        (
            batch_b,
            "BATCH-B",
            Some(now + Duration::days(20)),
            30_i64,
            5_i64,
        ),
        (batch_c, "BATCH-C", None, 10_i64, 0_i64),
    ] {
        sqlx::query(
            r#"
            insert into sku_batches (batch_id, tenant_id, sku_id, batch_no, expire_at)
            values ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(batch_id)
        .bind(tenant_id)
        .bind(sku_id)
        .bind(batch_no)
        .bind(expire_at)
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            insert into inventory_batch_balances (tenant_id, warehouse_id, sku_id, batch_id, on_hand, reserved)
            values ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(tenant_id)
        .bind(warehouse_id)
        .bind(sku_id)
        .bind(batch_id)
        .bind(on_hand)
        .bind(reserved)
        .execute(pool)
        .await?;
    }

    Ok(SeedData {
        tenant_id,
        warehouse_id,
        sku_id,
    })
}

fn build_state(db: Db) -> (Router, JwtService) {
    std::env::set_var("IDEMPOTENCY_DISABLED", "true");
    let jwt = JwtService::new("test-secret".to_string());
    let state = AppState::new(db, Arc::new(jwt.clone()));
    let app = build_app(state);
    (app, jwt)
}

fn build_token(jwt: &JwtService, tenant_id: &str, perms: Vec<&str>) -> String {
    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: uuid::Uuid::new_v4().to_string(),
        tenant_id: tenant_id.to_string(),
        perms: perms.into_iter().map(|perm| perm.to_string()).collect(),
        is_admin: false,
        iat: now,
        exp: now + 3600,
    };
    jwt.encode(&claims).expect("token should encode")
}

async fn request_json(
    app: &Router,
    token: &str,
    method: Method,
    uri: String,
    body: Option<Value>,
) -> Result<(StatusCode, Value)> {
    let mut builder = Request::builder()
        .uri(uri)
        .method(method)
        .header("authorization", format!("Bearer {token}"));
    let request = if let Some(payload) = body {
        builder = builder.header("content-type", "application/json");
        builder.body(Body::from(serde_json::to_vec(&payload)?))?
    } else {
        builder.body(Body::empty())?
    };

    let response = app.clone().oneshot(request).await?;
    let status = response.status();
    let body_bytes = response.into_body().collect().await?.to_bytes();
    if body_bytes.is_empty() {
        return Ok((status, json!({})));
    }
    let json_body = serde_json::from_slice::<Value>(&body_bytes)
        .unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&body_bytes) }));
    Ok((status, json_body))
}

async fn connect_test_db(case_name: &str) -> Option<Db> {
    let database_url = support::test_database_url();
    let probe = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(StdDuration::from_secs(1))
        .connect(&database_url)
        .await;
    match probe {
        Ok(pool) => {
            drop(pool);
            match Db::connect_lazy(&database_url) {
                Ok(db) => Some(db),
                Err(err) => {
                    eprintln!("skipping {case_name}: {err}");
                    None
                }
            }
        }
        Err(err) => {
            eprintln!("skipping {case_name}: {err}");
            None
        }
    }
}

#[tokio::test]
async fn inventory_atp_fefo_packaging_and_reconciliation_flow() -> Result<()> {
    let Some(db) = connect_test_db("inventory_atp_fefo_packaging_and_reconciliation_flow").await
    else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;
    let seeded = seed_data(db.pool()).await?;

    let (app, jwt) = build_state(db);
    let token = build_token(
        &jwt,
        &seeded.tenant_id.to_string(),
        vec![
            "inventory.read",
            "inventory.write",
            "skus.read",
            "skus.write",
        ],
    );

    let (status, upsert_packaging) = request_json(
        &app,
        &token,
        Method::PUT,
        format!(
            "/v1/tenants/{}/skus/{}/packaging-specs/default",
            seeded.tenant_id, seeded.sku_id
        ),
        Some(json!({
          "level_code": "CASE",
          "unit_name": "箱",
          "units_per_level": 12,
          "length_mm": 320,
          "width_mm": 240,
          "height_mm": 180,
          "weight_gram": 5600
        })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "upsert packaging spec failed: {upsert_packaging}"
    );

    let (status, list_packaging) = request_json(
        &app,
        &token,
        Method::GET,
        format!(
            "/v1/tenants/{}/skus/{}/packaging-specs",
            seeded.tenant_id, seeded.sku_id
        ),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "list packaging specs failed: {list_packaging}"
    );
    let first_pack = list_packaging
        .get("items")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .context("packaging spec item missing")?;
    assert_eq!(
        first_pack.get("level_code").and_then(Value::as_str),
        Some("CASE")
    );
    assert_eq!(
        first_pack.get("units_per_level").and_then(Value::as_i64),
        Some(12)
    );

    let (status, atp_resp) = request_json(
        &app,
        &token,
        Method::GET,
        format!(
            "/v1/tenants/{}/inventory/atp?warehouse_id={}&sku_id={}&request_qty=20",
            seeded.tenant_id, seeded.warehouse_id, seeded.sku_id
        ),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "inventory atp failed: {atp_resp}");
    assert_eq!(atp_resp.get("available").and_then(Value::as_i64), Some(35));
    assert_eq!(atp_resp.get("atp_qty").and_then(Value::as_i64), Some(20));
    assert_eq!(
        atp_resp.get("shortage_qty").and_then(Value::as_i64),
        Some(0)
    );
    let allocations = atp_resp
        .get("allocations")
        .and_then(Value::as_array)
        .context("allocation result missing")?;
    assert_eq!(allocations.len(), 2);
    assert_eq!(
        allocations[0].get("batch_no").and_then(Value::as_str),
        Some("BATCH-A")
    );
    assert_eq!(
        allocations[0].get("allocated_qty").and_then(Value::as_i64),
        Some(8)
    );
    assert_eq!(
        allocations[1].get("batch_no").and_then(Value::as_str),
        Some("BATCH-B")
    );
    assert_eq!(
        allocations[1].get("allocated_qty").and_then(Value::as_i64),
        Some(12)
    );

    let (status, reconciliation) = request_json(
        &app,
        &token,
        Method::POST,
        format!(
            "/v1/tenants/{}/inventory/reconciliation-center/check",
            seeded.tenant_id
        ),
        Some(json!({
          "warehouse_id": seeded.warehouse_id,
          "sku_id": seeded.sku_id,
          "expected_on_hand": 45,
          "source_ref": "WMS-CYCLE-COUNT-20260409"
        })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "reconciliation center check failed: {reconciliation}"
    );
    assert_eq!(
        reconciliation.get("actual_on_hand").and_then(Value::as_i64),
        Some(50)
    );
    assert_eq!(
        reconciliation.get("diff_qty").and_then(Value::as_i64),
        Some(5)
    );
    assert_eq!(
        reconciliation.get("status").and_then(Value::as_str),
        Some("MISMATCH")
    );
    assert!(
        reconciliation
            .get("reconciliation_id")
            .and_then(Value::as_str)
            .is_some(),
        "reconciliation id missing"
    );

    Ok(())
}

#[tokio::test]
async fn inventory_atp_and_packaging_validation_failures() -> Result<()> {
    let Some(db) = connect_test_db("inventory_atp_and_packaging_validation_failures").await else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;
    let seeded = seed_data(db.pool()).await?;

    let (app, jwt) = build_state(db);
    let token = build_token(
        &jwt,
        &seeded.tenant_id.to_string(),
        vec!["inventory.read", "skus.write"],
    );

    let (status, atp_resp) = request_json(
        &app,
        &token,
        Method::GET,
        format!(
            "/v1/tenants/{}/inventory/atp?warehouse_id=bad-warehouse&sku_id={}&request_qty=20",
            seeded.tenant_id, seeded.sku_id
        ),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "invalid warehouse_id should be rejected: {atp_resp}"
    );

    let (status, upsert_resp) = request_json(
        &app,
        &token,
        Method::PUT,
        format!(
            "/v1/tenants/{}/skus/{}/packaging-specs/default",
            seeded.tenant_id, seeded.sku_id
        ),
        Some(json!({
          "level_code": "CASE",
          "unit_name": "Case",
          "units_per_level": 0
        })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "invalid units_per_level should be rejected: {upsert_resp}"
    );

    Ok(())
}

#[tokio::test]
async fn inventory_reconciliation_permission_and_validation_failures() -> Result<()> {
    let Some(db) =
        connect_test_db("inventory_reconciliation_permission_and_validation_failures").await
    else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;
    let seeded = seed_data(db.pool()).await?;

    let (app, jwt) = build_state(db);
    let readonly_token = build_token(
        &jwt,
        &seeded.tenant_id.to_string(),
        vec!["inventory.read", "skus.read"],
    );

    let (status, forbidden_resp) = request_json(
        &app,
        &readonly_token,
        Method::POST,
        format!(
            "/v1/tenants/{}/inventory/reconciliation-center/check",
            seeded.tenant_id
        ),
        Some(json!({
          "warehouse_id": seeded.warehouse_id,
          "sku_id": seeded.sku_id,
          "expected_on_hand": 50
        })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "missing inventory.write should be forbidden: {forbidden_resp}"
    );

    let write_token = build_token(
        &jwt,
        &seeded.tenant_id.to_string(),
        vec!["inventory.write", "inventory.read"],
    );
    let (status, invalid_sku_resp) = request_json(
        &app,
        &write_token,
        Method::POST,
        format!(
            "/v1/tenants/{}/inventory/reconciliation-center/check",
            seeded.tenant_id
        ),
        Some(json!({
          "warehouse_id": seeded.warehouse_id,
          "sku_id": "bad-sku-id",
          "expected_on_hand": 50
        })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "invalid sku_id should be rejected: {invalid_sku_resp}"
    );

    Ok(())
}
