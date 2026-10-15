use anyhow::{Context, Result};
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
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
    warehouse_id: uuid::Uuid,
    wave_id: uuid::Uuid,
    outbound_doc_id: uuid::Uuid,
}

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
          cost_cent bigint,
          batch_enabled boolean not null default false,
          serial_enabled boolean not null default false,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
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
        create table if not exists channel_sku_mappings (
          tenant_id uuid not null,
          sku_id uuid not null,
          status text not null default 'ACTIVE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, sku_id)
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
        create table if not exists waves (
          wave_id uuid primary key,
          tenant_id uuid not null,
          warehouse_id uuid not null,
          status text not null default 'CREATED',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists wave_docs (
          tenant_id uuid not null,
          wave_id uuid not null,
          outbound_doc_id uuid not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, wave_id, outbound_doc_id)
        )
        "#,
        r#"
        create table if not exists shipments (
          shipment_id uuid primary key,
          tenant_id uuid not null,
          shop_id uuid,
          order_id uuid,
          platform text,
          platform_order_id text,
          warehouse_id uuid,
          carrier_code text,
          tracking_no text,
          status text not null default 'REQUESTED',
          ship_status text not null default 'REQUESTED',
          last_push_error text not null default '',
          shipped_at timestamptz,
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        alter table shipments
          add column if not exists ship_status text not null default 'REQUESTED'
        "#,
        r#"
        alter table shipments
          add column if not exists last_push_error text not null default ''
        "#,
        r#"
        alter table shipments
          add column if not exists updated_at timestamptz not null default now()
        "#,
        r#"
        create table if not exists order_index (
          tenant_id uuid not null,
          order_id uuid not null,
          order_month date not null,
          platform text,
          platform_order_id text,
          shop_id uuid,
          order_created_at timestamptz not null,
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_id)
        )
        "#,
        r#"
        create table if not exists orders (
          tenant_id uuid not null,
          order_month date not null,
          order_id uuid not null,
          receiver_name text,
          receiver_phone text,
          receiver_address text,
          inventory_shortage boolean not null default false,
          exception_flags jsonb not null default '[]'::jsonb,
          status text not null default 'PENDING_SHIPMENT',
          ship_at timestamptz,
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_month, order_id)
        )
        "#,
        r#"
        alter table orders
          add column if not exists receiver_name text
        "#,
        r#"
        alter table orders
          add column if not exists receiver_phone text
        "#,
        r#"
        alter table orders
          add column if not exists receiver_address text
        "#,
        r#"
        alter table orders
          add column if not exists inventory_shortage boolean not null default false
        "#,
        r#"
        alter table orders
          add column if not exists exception_flags jsonb not null default '[]'::jsonb
        "#,
        r#"
        alter table orders
          add column if not exists status text not null default 'PENDING_SHIPMENT'
        "#,
        r#"
        alter table orders
          add column if not exists ship_at timestamptz
        "#,
        r#"
        alter table orders
          add column if not exists updated_at timestamptz not null default now()
        "#,
        r#"
        create table if not exists order_tags (
          tenant_id uuid not null,
          order_id uuid not null,
          tag text not null
        )
        "#,
        r#"
        create table if not exists outbound_docs (
          outbound_doc_id uuid primary key,
          tenant_id uuid not null,
          warehouse_id uuid not null,
          doc_type text not null,
          shipment_id uuid,
          doc_no text not null,
          status text not null default 'DRAFT',
          outbound_status text not null default 'DRAFT',
          pick_status text not null default 'NOT_PICKED',
          verify_status text not null default 'NOT_VERIFIED',
          pack_status text not null default 'NOT_PACKED',
          ship_status text not null default 'NOT_SHIPPED',
          picked_at timestamptz,
          packed_at timestamptz,
          shipped_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        alter table outbound_docs
          add column if not exists outbound_status text not null default 'DRAFT'
        "#,
        r#"
        alter table outbound_docs
          add column if not exists pick_status text not null default 'NOT_PICKED'
        "#,
        r#"
        alter table outbound_docs
          add column if not exists verify_status text not null default 'NOT_VERIFIED'
        "#,
        r#"
        alter table outbound_docs
          add column if not exists pack_status text not null default 'NOT_PACKED'
        "#,
        r#"
        alter table outbound_docs
          add column if not exists ship_status text not null default 'NOT_SHIPPED'
        "#,
        r#"
        alter table outbound_docs
          add column if not exists picked_at timestamptz
        "#,
        r#"
        alter table outbound_docs
          add column if not exists packed_at timestamptz
        "#,
        r#"
        alter table outbound_docs
          add column if not exists shipped_at timestamptz
        "#,
        r#"
        create table if not exists outbound_doc_items (
          tenant_id uuid not null,
          outbound_doc_id uuid not null,
          sku_id uuid not null,
          planned_qty bigint not null,
          picked_qty bigint not null default 0,
          packed_qty bigint not null default 0,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, outbound_doc_id, sku_id)
        )
        "#,
        r#"
        create table if not exists outbound_pick_tasks (
          pick_task_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          outbound_doc_id uuid not null,
          sku_id uuid not null,
          from_location_id uuid,
          container_id uuid,
          qty bigint not null,
          picked_qty bigint not null default 0,
          status text not null default 'NOT_PICKED',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        alter table outbound_pick_tasks
          add column if not exists container_id uuid
        "#,
        r#"
        create table if not exists pack_records (
          pack_record_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          outbound_doc_id uuid not null,
          shipment_id uuid,
          package_no text not null,
          weight_gram int,
          operator_id uuid,
          packed_at timestamptz not null default now(),
          pack_status text not null default 'PACKED',
          void_reason text,
          voided_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        alter table pack_records
          add column if not exists pack_status text not null default 'PACKED'
        "#,
        r#"
        alter table pack_records
          add column if not exists void_reason text
        "#,
        r#"
        alter table pack_records
          add column if not exists voided_at timestamptz
        "#,
        r#"
        alter table pack_records
          add column if not exists updated_at timestamptz not null default now()
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
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}

async fn seed_wave_ship(pool: &sqlx::PgPool) -> Result<SeedData> {
    let tenant_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();
    let product_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();
    let wave_id = uuid::Uuid::new_v4();
    let outbound_doc_id = uuid::Uuid::new_v4();

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .bind("wave-ship-tenant")
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
    .bind("Wave Warehouse")
    .bind("WH-WAVE-1")
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into products (product_id, tenant_id, name)
        values ($1, $2, $3)
        "#,
    )
    .bind(product_id)
    .bind(tenant_id)
    .bind("Wave Product")
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into skus (sku_id, tenant_id, product_id, sku_code, cost_cent)
        values ($1, $2, $3, $4, 120)
        "#,
    )
    .bind(sku_id)
    .bind(tenant_id)
    .bind(product_id)
    .bind("WAVE-SKU-1")
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into inventory_balances (
            tenant_id, warehouse_id, sku_id, on_hand, reserved, frozen, in_transit
        ) values ($1, $2, $3, 10, 0, 0, 0)
        "#,
    )
    .bind(tenant_id)
    .bind(warehouse_id)
    .bind(sku_id)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into waves (wave_id, tenant_id, warehouse_id, status)
        values ($1, $2, $3, 'PICKING')
        "#,
    )
    .bind(wave_id)
    .bind(tenant_id)
    .bind(warehouse_id)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into outbound_docs (
            outbound_doc_id, tenant_id, warehouse_id, doc_type, doc_no, status,
            outbound_status, pick_status, verify_status, pack_status, ship_status
        ) values ($1, $2, $3, 'SHIP', $4, 'PACKED', 'READY_TO_SHIP', 'PICKED',
                 'NOT_VERIFIED', 'PACKED', 'NOT_SHIPPED')
        "#,
    )
    .bind(outbound_doc_id)
    .bind(tenant_id)
    .bind(warehouse_id)
    .bind("OUT-WAVE-1")
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into outbound_doc_items (
            tenant_id, outbound_doc_id, sku_id, planned_qty, picked_qty, packed_qty
        ) values ($1, $2, $3, 2, 2, 2)
        "#,
    )
    .bind(tenant_id)
    .bind(outbound_doc_id)
    .bind(sku_id)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into wave_docs (tenant_id, wave_id, outbound_doc_id)
        values ($1, $2, $3)
        "#,
    )
    .bind(tenant_id)
    .bind(wave_id)
    .bind(outbound_doc_id)
    .execute(pool)
    .await?;

    Ok(SeedData {
        tenant_id,
        warehouse_id,
        wave_id,
        outbound_doc_id,
    })
}

fn build_state(db: Db) -> (Router, JwtService) {
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
async fn wave_ship_success_persists_logs() -> Result<()> {
    let Some(db) = connect_test_db("wave_ship_success_persists_logs").await else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;
    let seed = seed_wave_ship(db.pool()).await?;

    let (app, jwt) = build_state(db.clone());
    let token = build_token(&jwt, &seed.tenant_id.to_string(), vec!["wms.wave_ship"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{}/wms/waves/{}/ship",
            seed.tenant_id, seed.wave_id
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&json!({
            "continue_on_error": true,
            "direct_ship": false
        }))?))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await?.to_bytes();
    let json: Value = serde_json::from_slice(&body_bytes)?;

    assert_eq!(
        json.get("wave_id").and_then(Value::as_str),
        Some(seed.wave_id.to_string().as_str())
    );
    assert_eq!(json.get("total").and_then(Value::as_i64), Some(1));
    assert_eq!(json.get("success").and_then(Value::as_i64), Some(1));
    assert_eq!(json.get("failed").and_then(Value::as_i64), Some(0));
    assert_eq!(
        json.get("direct_ship").and_then(Value::as_bool),
        Some(false)
    );

    let items = json
        .get("items")
        .and_then(Value::as_array)
        .context("response items missing")?;
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0].get("outbound_doc_id").and_then(Value::as_str),
        Some(seed.outbound_doc_id.to_string().as_str())
    );
    assert_eq!(
        items[0].get("status").and_then(Value::as_str),
        Some("SHIPPED")
    );

    let wave_status: String =
        sqlx::query_scalar("select status from waves where tenant_id = $1 and wave_id = $2")
            .bind(seed.tenant_id)
            .bind(seed.wave_id)
            .fetch_one(db.pool())
            .await?;
    assert_eq!(wave_status, "SHIPPED");

    let operation_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from operation_logs
        where tenant_id = $1 and action = 'WMS.WAVE_SHIPPED' and ref_id = $2
        "#,
    )
    .bind(seed.tenant_id)
    .bind(seed.wave_id.to_string())
    .fetch_one(db.pool())
    .await?;
    assert_eq!(operation_count, 1);

    let (op_success, op_failed, op_direct_ship): (i64, i64, bool) = sqlx::query_as(
        r#"
        select
          (metadata->>'success')::bigint,
          (metadata->>'failed')::bigint,
          (metadata->>'direct_ship')::boolean
        from operation_logs
        where tenant_id = $1 and action = 'WMS.WAVE_SHIPPED' and ref_id = $2
        "#,
    )
    .bind(seed.tenant_id)
    .bind(seed.wave_id.to_string())
    .fetch_one(db.pool())
    .await?;
    assert_eq!(op_success, 1);
    assert_eq!(op_failed, 0);
    assert!(!op_direct_ship);

    let audit_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from audit_logs
        where tenant_id = $1 and action = 'WMS.WAVE_SHIPPED' and entity_id = $2
        "#,
    )
    .bind(seed.tenant_id)
    .bind(seed.wave_id.to_string())
    .fetch_one(db.pool())
    .await?;
    assert_eq!(audit_count, 1);

    let (audit_status, audit_success, audit_failed, audit_direct_ship): (String, i64, i64, bool) =
        sqlx::query_as(
            r#"
            select
              (after_json->>'status')::text,
              (after_json->>'success')::bigint,
              (after_json->>'failed')::bigint,
              (after_json->>'direct_ship')::boolean
            from audit_logs
            where tenant_id = $1 and action = 'WMS.WAVE_SHIPPED' and entity_id = $2
            "#,
        )
        .bind(seed.tenant_id)
        .bind(seed.wave_id.to_string())
        .fetch_one(db.pool())
        .await?;
    assert_eq!(audit_status, "SHIPPED");
    assert_eq!(audit_success, 1);
    assert_eq!(audit_failed, 0);
    assert!(!audit_direct_ship);

    Ok(())
}

#[tokio::test]
async fn wave_ship_continue_on_error_reports_partial_failure() -> Result<()> {
    let Some(db) = connect_test_db("wave_ship_continue_on_error_reports_partial_failure").await
    else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;
    let seed = seed_wave_ship(db.pool()).await?;

    let failed_outbound_doc_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into outbound_docs (
            outbound_doc_id, tenant_id, warehouse_id, doc_type, doc_no, status,
            outbound_status, pick_status, verify_status, pack_status, ship_status
        ) values ($1, $2, $3, 'SHIP', $4, 'DRAFT', 'DRAFT', 'NOT_PICKED',
                  'NOT_VERIFIED', 'NOT_PACKED', 'NOT_SHIPPED')
        "#,
    )
    .bind(failed_outbound_doc_id)
    .bind(seed.tenant_id)
    .bind(seed.warehouse_id)
    .bind("OUT-WAVE-FAIL-1")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into wave_docs (tenant_id, wave_id, outbound_doc_id)
        values ($1, $2, $3)
        "#,
    )
    .bind(seed.tenant_id)
    .bind(seed.wave_id)
    .bind(failed_outbound_doc_id)
    .execute(db.pool())
    .await?;

    let (app, jwt) = build_state(db.clone());
    let token = build_token(&jwt, &seed.tenant_id.to_string(), vec!["wms.wave_ship"]);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{}/wms/waves/{}/ship",
            seed.tenant_id, seed.wave_id
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&json!({
            "continue_on_error": true,
            "direct_ship": false
        }))?))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await?.to_bytes();
    let json: Value = serde_json::from_slice(&body_bytes)?;

    assert_eq!(json.get("total").and_then(Value::as_i64), Some(2));
    let success = json
        .get("success")
        .and_then(Value::as_i64)
        .context("success missing")?;
    let failed = json
        .get("failed")
        .and_then(Value::as_i64)
        .context("failed missing")?;
    assert!(success > 0, "expected at least one shipped doc: {json}");
    assert!(failed > 0, "expected at least one failed doc: {json}");
    assert_eq!(success + failed, 2);

    let items = json
        .get("items")
        .and_then(Value::as_array)
        .context("response items missing")?;
    assert_eq!(items.len(), 2);
    assert!(
        items
            .iter()
            .any(|item| item.get("status").and_then(Value::as_str) != Some("SHIPPED"))
    );

    let (op_success, op_failed): (i64, i64) = sqlx::query_as(
        r#"
        select
          (metadata->>'success')::bigint,
          (metadata->>'failed')::bigint
        from operation_logs
        where tenant_id = $1 and action = 'WMS.WAVE_SHIPPED' and ref_id = $2
        order by created_at desc
        limit 1
        "#,
    )
    .bind(seed.tenant_id)
    .bind(seed.wave_id.to_string())
    .fetch_one(db.pool())
    .await?;
    assert_eq!(op_success, success);
    assert_eq!(op_failed, failed);

    Ok(())
}

#[tokio::test]
async fn wave_ship_forbidden_without_wave_ship_perm() -> Result<()> {
    let database_url = support::test_database_url();
    let db = Db::connect_lazy(&database_url)?;
    let (app, jwt) = build_state(db);

    let tenant_id = uuid::Uuid::new_v4().to_string();
    let wave_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, &tenant_id, vec!["wms.write"]);

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/wms/waves/{wave_id}/ship"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&json!({
            "continue_on_error": true
        }))?))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = response.into_body().collect().await?.to_bytes();
    let payload: Value = serde_json::from_slice(&body)?;
    assert_eq!(
        payload.get("code").and_then(Value::as_str),
        Some("forbidden")
    );

    Ok(())
}
