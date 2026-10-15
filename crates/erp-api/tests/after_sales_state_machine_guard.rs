use anyhow::Result;
use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
};
use chrono::Utc;
use erp_api::{AppState, Claims, JwtService, build_app};
use erp_repo::after_sales::{AfterSalesRepo, ReturnReceiptCreateInput};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use std::time::Duration as StdDuration;
use tower::ServiceExt;

mod support;

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
        perms.into_iter().map(|v| v.to_string()).collect(),
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
    body: Option<serde_json::Value>,
) -> Result<(StatusCode, serde_json::Value)> {
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
    let bytes = response.into_body().collect().await?.to_bytes();
    let value = if bytes.is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_slice(&bytes).unwrap_or_else(|_| serde_json::json!({}))
    };
    Ok((status, value))
}

async fn connect_test_db(case_name: &str) -> Option<Db> {
    let database_url = support::test_database_url();
    let probe = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(StdDuration::from_secs(1))
        .connect(&database_url)
        .await;
    match probe {
        Ok(pool) => {
            drop(pool);
            match Db::connect(&database_url).await {
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

async fn setup_schema(pool: &sqlx::PgPool) -> Result<()> {
    let statements = [
        "create extension if not exists pgcrypto",
        r#"
        create table if not exists tenants (
          tenant_id uuid primary key,
          name text not null default 'test-tenant',
          status text not null default 'ACTIVE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists products (
          product_id uuid primary key,
          tenant_id uuid not null,
          name text not null,
          attrs jsonb not null default '{}'::jsonb,
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
        create table if not exists skus (
          sku_id uuid primary key,
          tenant_id uuid not null,
          product_id uuid not null,
          sku_code text not null,
          spec jsonb not null default '{}'::jsonb,
          is_active boolean not null default true,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists after_sales (
          id uuid primary key,
          tenant_id uuid not null,
          order_id uuid not null,
          type text not null,
          status text not null,
          after_sale_status text not null,
          reason_code text,
          amount_cent bigint not null default 0,
          raw_json jsonb not null default '{}'::jsonb,
          compensation_status text,
          evidence_status text,
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
        create table if not exists after_sales_items (
          tenant_id uuid not null,
          after_sales_id uuid not null,
          sku_id uuid not null,
          qty bigint not null default 0,
          refund_cent bigint not null default 0,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, after_sales_id, sku_id)
        )
        "#,
        r#"
        create table if not exists return_receipts (
          id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          after_sales_id uuid not null,
          warehouse_id uuid not null,
          status text not null,
          return_status text not null,
          received_at timestamptz,
          putaway_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists inbound_docs (
          doc_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          warehouse_id uuid not null,
          doc_type text not null,
          doc_no text not null,
          status text not null,
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
        "alter table after_sales add column if not exists after_sale_status text not null default 'WAIT_SELLER_AGREE'",
        "alter table after_sales add column if not exists amount_cent bigint not null default 0",
        "alter table after_sales add column if not exists raw_json jsonb not null default '{}'::jsonb",
        "alter table after_sales add column if not exists compensation_status text",
        "alter table after_sales add column if not exists evidence_status text",
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}

#[tokio::test]
async fn after_sales_state_machine_paths_validate_before_db_access() {
    let db =
        Db::connect_lazy(&support::test_database_url()).expect("lazy db should be constructible");
    let (app, jwt) = build_state(db);
    let token = build_token(
        &jwt,
        &uuid::Uuid::new_v4().to_string(),
        vec!["aftersales.write", "inventory.write", "wms.inbounds.write"],
    );
    let after_sales_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();

    let cases = [
        (
            Method::POST,
            format!("/v1/tenants/not-a-uuid/after-sales/{after_sales_id}/approve"),
            None,
        ),
        (
            Method::POST,
            format!("/v1/tenants/not-a-uuid/after-sales/{after_sales_id}/reject"),
            None,
        ),
        (
            Method::POST,
            format!("/v1/tenants/not-a-uuid/after-sales/{after_sales_id}/receive"),
            Some(serde_json::json!({
                "warehouse_id": warehouse_id,
                "putaway_done": true
            })),
        ),
    ];

    for (method, uri, body) in cases {
        let (status, _) = request_json(&app, &token, method, uri, body)
            .await
            .expect("request");
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn after_sales_state_machine_reject_then_approve_should_conflict() -> Result<()> {
    let Some(db) =
        connect_test_db("after_sales_state_machine_reject_then_approve_should_conflict").await
    else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let rejected_id = uuid::Uuid::new_v4();
    let open_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into tenants(tenant_id, name, status)
        values ($1, 'state-machine-tenant', 'ACTIVE')
        on conflict (tenant_id) do nothing
        "#,
    )
    .bind(tenant_id)
    .execute(db.pool())
    .await?;
    sqlx::query(
        r#"
        insert into after_sales(id, tenant_id, order_id, type, status, after_sale_status, amount_cent, raw_json)
        values ($1, $2, $3, 'RETURN_REFUND', 'REJECTED', 'REJECTED', 1200, '{}'::jsonb)
        "#,
    )
    .bind(rejected_id)
    .bind(tenant_id)
    .bind(order_id)
    .execute(db.pool())
    .await?;
    sqlx::query(
        r#"
        insert into after_sales(id, tenant_id, order_id, type, status, after_sale_status, amount_cent, raw_json)
        values ($1, $2, $3, 'RETURN_REFUND', 'OPEN', 'WAIT_SELLER_AGREE', 1200, '{}'::jsonb)
        "#,
    )
    .bind(open_id)
    .bind(tenant_id)
    .bind(order_id)
    .execute(db.pool())
    .await?;

    let (app, jwt) = build_state(db);
    let token = build_token(
        &jwt,
        &tenant_id.to_string(),
        vec!["aftersales.write", "inventory.write", "wms.inbounds.write"],
    );

    let (status, conflict_body) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant_id}/after-sales/{rejected_id}/approve"),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        conflict_body.get("code").and_then(|v| v.as_str()),
        Some("conflict")
    );

    let (status, first_reject) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant_id}/after-sales/{open_id}/reject"),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        first_reject.get("status").and_then(|v| v.as_str()),
        Some("REJECTED")
    );

    let (status, second_reject) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant_id}/after-sales/{open_id}/reject"),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        second_reject.get("status").and_then(|v| v.as_str()),
        Some("REJECTED")
    );

    let warehouse_id = uuid::Uuid::new_v4();
    let (status, receive_body) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant_id}/after-sales/{rejected_id}/receive"),
        Some(serde_json::json!({
            "warehouse_id": warehouse_id,
            "putaway_done": true
        })),
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        receive_body.get("code").and_then(|v| v.as_str()),
        Some("conflict")
    );

    Ok(())
}

#[tokio::test]
async fn after_sales_redo_receive_should_refresh_linked_inbound_doc_warehouse() -> Result<()> {
    let Some(db) =
        connect_test_db("after_sales_redo_receive_should_refresh_linked_inbound_doc_warehouse")
            .await
    else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let after_sales_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    let product_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();
    let warehouse_a = uuid::Uuid::new_v4();
    let warehouse_b = uuid::Uuid::new_v4();

    sqlx::query(
        r#"
        insert into tenants(tenant_id, name, status)
        values ($1, 'redo-receive-tenant', 'ACTIVE')
        on conflict (tenant_id) do nothing
        "#,
    )
    .bind(tenant_id)
    .execute(db.pool())
    .await?;
    sqlx::query(
        r#"
        insert into products(product_id, tenant_id, name, attrs)
        values ($1, $2, 'redo-receive-product', '{}'::jsonb)
        on conflict (product_id) do nothing
        "#,
    )
    .bind(product_id)
    .bind(tenant_id)
    .execute(db.pool())
    .await?;
    sqlx::query(
        r#"
        insert into warehouses(warehouse_id, tenant_id, name, code, is_default)
        values ($1, $2, 'redo-receive-wh-a', 'RR-A', true),
               ($3, $2, 'redo-receive-wh-b', 'RR-B', false)
        on conflict (warehouse_id) do nothing
        "#,
    )
    .bind(warehouse_a)
    .bind(tenant_id)
    .bind(warehouse_b)
    .execute(db.pool())
    .await?;
    sqlx::query(
        r#"
        insert into skus(sku_id, tenant_id, product_id, sku_code, spec, is_active)
        values ($1, $2, $3, 'redo-receive-sku', '{}'::jsonb, true)
        on conflict (sku_id) do nothing
        "#,
    )
    .bind(sku_id)
    .bind(tenant_id)
    .bind(product_id)
    .execute(db.pool())
    .await?;
    sqlx::query(
        r#"
        insert into after_sales(id, tenant_id, order_id, type, status, after_sale_status, amount_cent, raw_json)
        values ($1, $2, $3, 'RETURN_REFUND', 'APPROVED', 'WAIT_BUYER_RETURN_GOODS', 3600, '{}'::jsonb)
        "#,
    )
    .bind(after_sales_id)
    .bind(tenant_id)
    .bind(order_id)
    .execute(db.pool())
    .await?;
    sqlx::query(
        r#"
        insert into after_sales_items(tenant_id, after_sales_id, sku_id, qty, refund_cent)
        values ($1, $2, $3, 1, 3600)
        "#,
    )
    .bind(tenant_id)
    .bind(after_sales_id)
    .bind(sku_id)
    .execute(db.pool())
    .await?;

    let repo = AfterSalesRepo::new(db.pool());
    repo.receive_return(ReturnReceiptCreateInput {
        tenant_id: erp_types::TenantId(tenant_id),
        after_sales_id,
        warehouse_id: warehouse_a,
        putaway_done: false,
        received_at: Some(Utc::now()),
        link_inbound: true,
    })
    .await?;

    let original_doc_id: uuid::Uuid =
        sqlx::query_scalar("select doc_id from inbound_docs where tenant_id = $1 and doc_no = $2")
            .bind(tenant_id)
            .bind(format!("RETURN-{after_sales_id}"))
            .fetch_one(db.pool())
            .await?;

    repo.cancel_return_receipt(erp_types::TenantId(tenant_id), after_sales_id)
        .await?;

    repo.redo_return_receipt(ReturnReceiptCreateInput {
        tenant_id: erp_types::TenantId(tenant_id),
        after_sales_id,
        warehouse_id: warehouse_b,
        putaway_done: false,
        received_at: Some(Utc::now()),
        link_inbound: true,
    })
    .await?;

    #[derive(sqlx::FromRow)]
    struct InboundDocRow {
        doc_id: uuid::Uuid,
        warehouse_id: uuid::Uuid,
        status: String,
    }

    let inbound_doc: InboundDocRow = sqlx::query_as(
        "select doc_id, warehouse_id, status from inbound_docs where tenant_id = $1 and doc_no = $2",
    )
    .bind(tenant_id)
    .bind(format!("RETURN-{after_sales_id}"))
    .fetch_one(db.pool())
    .await?;

    assert_eq!(inbound_doc.doc_id, original_doc_id);
    assert_eq!(inbound_doc.warehouse_id, warehouse_b);
    assert_eq!(inbound_doc.status, "RECEIVING");

    let receipt_warehouse_id: uuid::Uuid = sqlx::query_scalar(
        "select warehouse_id from return_receipts where tenant_id = $1 and after_sales_id = $2",
    )
    .bind(tenant_id)
    .bind(after_sales_id)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(receipt_warehouse_id, warehouse_b);

    Ok(())
}
