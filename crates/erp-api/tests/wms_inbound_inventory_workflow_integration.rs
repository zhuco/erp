use anyhow::{Context, Result};
use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
};
use chrono::Utc;
use erp_api::{AppState, Claims, JwtService, build_app};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceExt;

mod support;

struct SeedData {
    tenant_id: uuid::Uuid,
    supplier_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
}

async fn setup_schema(pool: &sqlx::PgPool) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("select pg_advisory_xact_lock(9101002)")
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
        create table if not exists suppliers (
          supplier_id uuid primary key,
          tenant_id uuid not null,
          name text not null,
          contact text,
          settlement_terms text,
          status text not null default 'ACTIVE',
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
          product_id uuid,
          sku_code text not null,
          barcode text,
          name text,
          cost_cent bigint,
          is_active boolean not null default true,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists purchase_orders (
          po_id uuid primary key,
          tenant_id uuid not null,
          supplier_id uuid not null,
          warehouse_id uuid,
          expected_at timestamptz,
          remark text,
          status text not null default 'DRAFT',
          purchase_order_status text not null default 'DRAFT',
          total_cent bigint not null default 0,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists purchase_order_items (
          tenant_id uuid not null,
          po_id uuid not null,
          sku_id uuid not null,
          qty bigint not null,
          price_cent bigint not null,
          received_qty bigint not null default 0,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, po_id, sku_id)
        )
        "#,
        r#"
        create table if not exists purchase_receipts (
          receipt_id uuid primary key,
          tenant_id uuid not null,
          po_id uuid not null,
          warehouse_id uuid not null,
          status text not null default 'CONFIRMED',
          receiving_status text not null default 'NOT_RECEIVED',
          confirmed_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists receipt_items (
          tenant_id uuid not null,
          receipt_id uuid not null,
          sku_id uuid not null,
          qty bigint not null,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, receipt_id, sku_id)
        )
        "#,
        r#"
        create table if not exists inbound_docs (
          doc_id uuid primary key,
          tenant_id uuid not null,
          warehouse_id uuid not null,
          doc_type text not null,
          doc_no text not null,
          status text not null default 'DRAFT',
          inbound_status text not null default 'DRAFT',
          receipt_status text not null default 'NOT_RECEIVED',
          inbound_qc_status text not null default 'NOT_REQUIRED',
          putaway_status text not null default 'NOT_PUTAWAY',
          ref_type text,
          ref_id text,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, doc_no)
        )
        "#,
        r#"
        create table if not exists inbound_doc_items (
          tenant_id uuid not null,
          doc_id uuid not null,
          sku_id uuid not null,
          planned_qty bigint not null default 0,
          received_qty bigint not null default 0,
          putaway_qty bigint not null default 0,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, doc_id, sku_id)
        )
        "#,
        r#"
        create table if not exists inbound_putaway_tasks (
          task_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          doc_id uuid not null,
          sku_id uuid not null,
          qty bigint not null,
          from_area text,
          to_location_id uuid,
          status text not null default 'DONE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists inbound_item_ops (
          tenant_id uuid not null,
          operation_id uuid not null,
          doc_id uuid not null,
          sku_id uuid not null,
          action text not null,
          qty bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, operation_id)
        )
        "#,
        r#"
        create table if not exists inbound_inspections (
          inspection_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          doc_id uuid not null,
          sku_id uuid not null,
          qty bigint not null,
          result text not null,
          note text,
          operator_id uuid,
          operation_id uuid not null,
          created_at timestamptz not null default now(),
          unique (tenant_id, operation_id)
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
        create table if not exists inventory_ledgers (
          ledger_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          warehouse_id uuid not null,
          sku_id uuid not null,
          delta_on_hand bigint not null,
          delta_reserved bigint not null,
          delta_in_transit bigint not null,
          delta_frozen bigint not null,
          reason_type text not null,
          ref_type text,
          ref_id text,
          occurred_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists purchase_suggestions (
          tenant_id uuid not null,
          sku_id uuid not null,
          suggested_qty bigint not null,
          reason text not null,
          computed_at timestamptz not null default now(),
          primary key (tenant_id, sku_id)
        )
        "#,
        r#"
        create table if not exists procurement_plans (
          plan_id uuid primary key,
          tenant_id uuid not null,
          request_id uuid,
          supplier_id uuid,
          status text not null default 'DRAFT',
          total_cent bigint not null default 0,
          planned_at timestamptz,
          expected_at timestamptz,
          remark text,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists procurement_plan_items (
          tenant_id uuid not null,
          plan_id uuid not null,
          sku_id uuid not null,
          qty bigint not null,
          price_cent bigint not null,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, plan_id, sku_id)
        )
        "#,
        r#"
        create table if not exists ap_docs (
          ap_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          supplier_id uuid not null,
          ref_type text not null,
          ref_id text not null,
          amount_cent bigint not null,
          status text not null default 'OPEN',
          settlement_status text not null default 'UNSETTLED',
          paid_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, ref_type, ref_id)
        )
        "#,
        r#"
        create table if not exists procurement_reconciliations (
          reconciliation_id uuid primary key,
          tenant_id uuid not null,
          supplier_id uuid not null,
          period_start date not null,
          period_end date not null,
          amount_cent bigint not null,
          status text not null default 'OPEN',
          remark text,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
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
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, scope, shop_id, key)
        )
        "#,
        r#"
        create table if not exists operation_logs (
          log_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          actor_type text not null,
          actor_user_id uuid,
          action text not null,
          ref_type text,
          ref_id text,
          metadata jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now()
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
        create table if not exists sku_cost_ledgers (
          ledger_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          sku_id uuid not null,
          previous_cost_cent bigint,
          new_cost_cent bigint not null,
          ref_type text,
          ref_id text,
          created_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists user_shop_access (
          tenant_id uuid not null,
          user_id uuid not null,
          shop_id uuid not null
        )
        "#,
        r#"
        create table if not exists user_warehouse_access (
          tenant_id uuid not null,
          user_id uuid not null,
          warehouse_id uuid not null
        )
        "#,
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(&mut *tx).await?;
    }

    // Shared integration DB may already have stricter SKU schema from other suites.
    sqlx::query("alter table skus add column if not exists product_id uuid")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "alter table purchase_orders add column if not exists purchase_order_status text not null default 'DRAFT'",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query("alter table purchase_orders add column if not exists supplier_name text")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "alter table purchase_receipts add column if not exists receiving_status text not null default 'NOT_RECEIVED'",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "alter table inbound_docs add column if not exists inbound_status text not null default 'DRAFT'",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "alter table inbound_docs add column if not exists receipt_status text not null default 'NOT_RECEIVED'",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "alter table inbound_docs add column if not exists inbound_qc_status text not null default 'NOT_REQUIRED'",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "alter table inbound_docs add column if not exists putaway_status text not null default 'NOT_PUTAWAY'",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "alter table ap_docs add column if not exists settlement_status text not null default 'UNSETTLED'",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query("alter table purchase_order_items alter column qty type bigint using qty::bigint")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "alter table purchase_order_items alter column price_cent type bigint using price_cent::bigint",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "alter table purchase_order_items alter column received_qty type bigint using received_qty::bigint",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "alter table inbound_doc_items alter column planned_qty type bigint using planned_qty::bigint",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "alter table inbound_doc_items alter column received_qty type bigint using received_qty::bigint",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "alter table inbound_doc_items alter column putaway_qty type bigint using putaway_qty::bigint",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query("alter table inbound_putaway_tasks alter column qty type bigint using qty::bigint")
        .execute(&mut *tx)
        .await?;
    sqlx::query("alter table receipt_items alter column qty type bigint using qty::bigint")
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

async fn seed_base(pool: &sqlx::PgPool) -> Result<SeedData> {
    let tenant_id = uuid::Uuid::new_v4();
    let supplier_id = uuid::Uuid::new_v4();
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
    .bind("integration-tenant")
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into suppliers (supplier_id, tenant_id, name, status)
        values ($1, $2, $3, 'ACTIVE')
        "#,
    )
    .bind(supplier_id)
    .bind(tenant_id)
    .bind("integration-supplier")
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into warehouses (warehouse_id, tenant_id, name, code)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(warehouse_id)
    .bind(tenant_id)
    .bind("integration-warehouse")
    .bind("INT-WH")
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into products (product_id, tenant_id, name, status)
        values ($1, $2, $3, 'ACTIVE')
        "#,
    )
    .bind(product_id)
    .bind(tenant_id)
    .bind("integration-product")
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into skus (sku_id, tenant_id, product_id, sku_code, name, is_active)
        values ($1, $2, $3, $4, $5, true)
        "#,
    )
    .bind(sku_id)
    .bind(tenant_id)
    .bind(product_id)
    .bind("INT-SKU-001")
    .bind("integration-sku")
    .execute(pool)
    .await?;

    Ok(SeedData {
        tenant_id,
        supplier_id,
        warehouse_id,
        sku_id,
    })
}

async fn seed_purchase_suggestion(
    pool: &sqlx::PgPool,
    tenant_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    qty: i64,
) -> Result<()> {
    sqlx::query(
        r#"
        insert into purchase_suggestions (
            tenant_id, sku_id, suggested_qty, reason, computed_at
        ) values ($1, $2, $3, $4, now())
        on conflict (tenant_id, sku_id)
        do update set
          suggested_qty = excluded.suggested_qty,
          reason = excluded.reason,
          computed_at = excluded.computed_at
        "#,
    )
    .bind(tenant_id)
    .bind(sku_id)
    .bind(qty)
    .bind("flow_test")
    .execute(pool)
    .await?;
    Ok(())
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
    let claims = Claims::bearer(
        uuid::Uuid::new_v4().to_string(),
        tenant_id.to_string(),
        perms.into_iter().map(|perm| perm.to_string()).collect(),
        false,
        now,
        now + 3600,
    );
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
        .acquire_timeout(Duration::from_secs(1))
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
async fn procurement_inbound_and_settlement_flow_integration() -> Result<()> {
    let Some(db) = connect_test_db("procurement_inbound_and_settlement_flow_integration").await
    else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;
    let seed = seed_base(db.pool()).await?;
    seed_purchase_suggestion(db.pool(), seed.tenant_id, seed.sku_id, 5).await?;

    let (app, jwt) = build_state(db.clone());
    let token = build_token(
        &jwt,
        &seed.tenant_id.to_string(),
        vec![
            "procurement_plans.write",
            "purchase_orders.write",
            "inventory.read",
            "procurement_reconciliations.read",
            "procurement_reconciliations.write",
        ],
    );

    let tenant = seed.tenant_id;

    let (status, generated_plan) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant}/procurement-plans/generate"),
        Some(json!({
            "supplier_id": seed.supplier_id,
            "max_items": 10,
            "min_suggested_qty": 1,
            "default_price_cent": 120
        })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "generate plan failed: {generated_plan}"
    );
    let generated_items = generated_plan
        .get("items")
        .and_then(|value| value.as_array())
        .context("generated plan items missing")?;
    assert_eq!(generated_items.len(), 1);
    assert_eq!(
        generated_items[0]
            .get("sku_id")
            .and_then(|value| value.as_str()),
        Some(seed.sku_id.to_string().as_str())
    );
    assert_eq!(
        generated_items[0]
            .get("qty")
            .and_then(|value| value.as_i64()),
        Some(5)
    );

    let (status, created_po) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant}/purchase-orders"),
        Some(json!({
            "supplier_id": seed.supplier_id,
            "warehouse_id": seed.warehouse_id,
            "remark": "integration-flow",
            "items": [{
                "sku_id": seed.sku_id,
                "qty": 5,
                "price_cent": 120
            }]
        })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "create purchase order failed: {created_po}"
    );
    let po_id = created_po
        .get("order")
        .and_then(|value| value.get("po_id"))
        .and_then(|value| value.as_str())
        .context("po_id missing in create response")?;

    // Keep this integration scoped to inbound/inventory domain:
    // procurement send/confirm response-shaping can drift across shared schemas.
    sqlx::query(
        r#"
        update purchase_orders
        set status = 'CONFIRMED',
            purchase_order_status = 'CONFIRMED',
            updated_at = now()
        where tenant_id = $1 and po_id = $2
        "#,
    )
    .bind(tenant)
    .bind(uuid::Uuid::parse_str(po_id)?)
    .execute(db.pool())
    .await?;

    let (status, inbound_result) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant}/purchase-orders/{po_id}/inbound"),
        Some(json!({
            "warehouse_id": seed.warehouse_id,
            "items": [{
                "sku_id": seed.sku_id,
                "qty": 5
            }],
            "update_cost": true
        })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "purchase inbound failed: {inbound_result}"
    );
    assert_eq!(
        inbound_result
            .get("order")
            .and_then(|value| value.get("status"))
            .and_then(|value| value.as_str()),
        Some("DONE")
    );
    let receipt_id = inbound_result
        .get("receipt")
        .and_then(|value| value.get("receipt_id"))
        .and_then(|value| value.as_str())
        .context("receipt_id missing in inbound response")?;
    let _inbound_doc_id = inbound_result
        .get("inbound_doc_id")
        .and_then(|value| value.as_str())
        .context("inbound_doc_id missing in inbound response")?;

    let (status, inout_records) = request_json(
        &app,
        &token,
        Method::GET,
        format!(
            "/v1/tenants/{tenant}/wms/inout-records?sku_id={}&limit=50",
            seed.sku_id
        ),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "query inout records failed: {inout_records}"
    );
    let inout_items = inout_records
        .get("items")
        .and_then(|value| value.as_array())
        .context("inout records items missing")?;
    assert!(
        inout_items.iter().any(|item| {
            item.get("reason_type").and_then(|value| value.as_str()) == Some("PURCHASE_RECEIPT")
                && item.get("delta_on_hand").and_then(|value| value.as_i64()) == Some(5)
        }),
        "missing PURCHASE_RECEIPT ledger in inout records: {inout_records}"
    );

    let (status, stock_change_details) = request_json(
        &app,
        &token,
        Method::GET,
        format!(
            "/v1/tenants/{tenant}/wms/stock-change-details?sku_id={}&limit=50",
            seed.sku_id
        ),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "query stock change details failed: {stock_change_details}"
    );
    let change_items = stock_change_details
        .get("items")
        .and_then(|value| value.as_array())
        .context("stock change detail items missing")?;
    assert!(
        change_items.iter().any(|item| {
            item.get("reason_type").and_then(|value| value.as_str()) == Some("PURCHASE_RECEIPT")
                && item.get("delta_on_hand").and_then(|value| value.as_i64()) == Some(5)
        }),
        "missing PURCHASE_RECEIPT ledger in stock change details: {stock_change_details}"
    );

    let (status, settlement) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant}/wms/inbound-settlements"),
        Some(json!({
            "supplier_id": seed.supplier_id,
            "period_start": "2026-04-01",
            "period_end": "2026-04-30",
            "amount_cent": 600,
            "status": "OPEN",
            "remark": format!("receipt={receipt_id}")
        })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "create inbound settlement failed: {settlement}"
    );
    assert_eq!(
        settlement
            .get("supplier_id")
            .and_then(|value| value.as_str())
            .map(|value| value.to_ascii_lowercase()),
        Some(seed.supplier_id.to_string())
    );

    let (status, settlement_list) = request_json(
        &app,
        &token,
        Method::GET,
        format!(
            "/v1/tenants/{tenant}/wms/inbound-settlements?supplier_id={}",
            seed.supplier_id
        ),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "list inbound settlements failed: {settlement_list}"
    );
    assert!(
        settlement_list
            .get("total")
            .and_then(|value| value.as_i64())
            .unwrap_or_default()
            >= 1,
        "settlement total should be >= 1: {settlement_list}"
    );

    Ok(())
}

#[tokio::test]
async fn wms_manual_inbound_audit_receive_putaway_flow_integration() -> Result<()> {
    let Some(db) =
        connect_test_db("wms_manual_inbound_audit_receive_putaway_flow_integration").await
    else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;
    let seed = seed_base(db.pool()).await?;

    let (app, jwt) = build_state(db.clone());
    let token = build_token(
        &jwt,
        &seed.tenant_id.to_string(),
        vec!["wms.inbounds.read", "wms.inbounds.write", "inventory.read"],
    );
    let tenant = seed.tenant_id;

    let (status, updated_settings) = request_json(
        &app,
        &token,
        Method::PUT,
        format!("/v1/tenants/{tenant}/wms/inbound-settings"),
        Some(json!({
            "require_audit": true
        })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "update inbound settings failed: {updated_settings}"
    );
    assert_eq!(
        updated_settings
            .get("require_audit")
            .and_then(|value| value.as_bool()),
        Some(true)
    );

    let (status, loaded_settings) = request_json(
        &app,
        &token,
        Method::GET,
        format!("/v1/tenants/{tenant}/wms/inbound-settings"),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "load inbound settings failed: {loaded_settings}"
    );
    assert_eq!(
        loaded_settings
            .get("require_audit")
            .and_then(|value| value.as_bool()),
        Some(true)
    );

    let doc_no = format!("INB-{}", uuid::Uuid::new_v4());
    let (status, inbound_doc) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant}/wms/inbounds"),
        Some(json!({
            "warehouse_id": seed.warehouse_id,
            "doc_type": "PURCHASE",
            "doc_no": doc_no,
            "items": [{
                "sku_id": seed.sku_id,
                "planned_qty": 3
            }]
        })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "create inbound doc failed: {inbound_doc}"
    );
    let doc_id = inbound_doc
        .get("doc")
        .and_then(|value| value.get("doc_id"))
        .and_then(|value| value.as_str())
        .context("doc_id missing in create inbound response")?;

    let (status, receive_before_audit) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant}/wms/inbounds/{doc_id}/receive"),
        Some(json!({
            "items": [{
                "sku_id": seed.sku_id,
                "qty": 3,
                "operation_id": uuid::Uuid::new_v4()
            }]
        })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "receive before audit should fail: {receive_before_audit}"
    );
    assert!(
        receive_before_audit
            .get("message")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .contains("not audited"),
        "unexpected receive-before-audit message: {receive_before_audit}"
    );

    let (status, audited_doc) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant}/wms/inbounds/{doc_id}/audit"),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "audit inbound failed: {audited_doc}"
    );
    assert_eq!(
        audited_doc
            .get("doc")
            .and_then(|value| value.get("status"))
            .and_then(|value| value.as_str()),
        Some("AUDITED")
    );

    let (status, received_doc) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant}/wms/inbounds/{doc_id}/receive"),
        Some(json!({
            "items": [{
                "sku_id": seed.sku_id,
                "qty": 3,
                "operation_id": uuid::Uuid::new_v4()
            }]
        })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "receive inbound after audit failed: {received_doc}"
    );
    assert_eq!(
        received_doc
            .get("doc")
            .and_then(|value| value.get("receipt_status"))
            .and_then(|value| value.as_str()),
        Some("RECEIVED")
    );

    let (status, putaway_doc) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant}/wms/inbounds/{doc_id}/putaway"),
        Some(json!({
            "items": [{
                "sku_id": seed.sku_id,
                "qty": 3,
                "operation_id": uuid::Uuid::new_v4(),
                "from_area": "RECEIVING"
            }]
        })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "putaway inbound failed: {putaway_doc}"
    );
    assert_eq!(
        putaway_doc
            .get("doc")
            .and_then(|value| value.get("status"))
            .and_then(|value| value.as_str()),
        Some("PUTAWAY_DONE")
    );

    let (status, inout_records) = request_json(
        &app,
        &token,
        Method::GET,
        format!(
            "/v1/tenants/{tenant}/wms/inout-records?sku_id={}&limit=50",
            seed.sku_id
        ),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "query inout records after putaway failed: {inout_records}"
    );
    let inout_items = inout_records
        .get("items")
        .and_then(|value| value.as_array())
        .context("inout records items missing after putaway")?;
    assert!(
        inout_items.iter().any(|item| {
            item.get("reason_type").and_then(|value| value.as_str()) == Some("PURCHASE_RECEIPT")
                && item.get("ref_type").and_then(|value| value.as_str()) == Some("INBOUND_DOC")
                && item.get("ref_id").and_then(|value| value.as_str()) == Some(doc_id)
                && item.get("delta_on_hand").and_then(|value| value.as_i64()) == Some(3)
        }),
        "missing inbound ledger after putaway: {inout_records}"
    );

    Ok(())
}
