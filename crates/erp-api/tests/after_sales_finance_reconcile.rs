use anyhow::Result;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use chrono::Utc;
use erp_api::{build_app, AppState, Claims, JwtService};
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
    let claims = Claims {
        sub: uuid::Uuid::new_v4().to_string(),
        tenant_id: tenant_id.to_string(),
        perms: perms.into_iter().map(|v| v.to_string()).collect(),
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
        serde_json::from_slice(&bytes).unwrap_or_else(|_| {
            serde_json::json!({
                "raw": String::from_utf8_lossy(&bytes)
            })
        })
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
        create table if not exists refund_docs (
          id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          after_sales_id uuid not null,
          amount_cent bigint not null default 0,
          status text not null,
          refund_status text not null,
          channel text not null,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, after_sales_id)
        )
        "#,
        r#"
        create table if not exists after_sales_shipments (
          tenant_id uuid not null,
          after_sales_id uuid not null,
          shipment_id uuid not null,
          shipment_type text not null,
          origin_shipment_id uuid,
          created_at timestamptz not null default now(),
          primary key (tenant_id, after_sales_id, shipment_id)
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
        "alter table after_sales add column if not exists after_sale_status text not null default 'WAIT_SELLER_AGREE'",
        "alter table after_sales add column if not exists amount_cent bigint not null default 0",
        "alter table after_sales add column if not exists raw_json jsonb not null default '{}'::jsonb",
        "alter table after_sales add column if not exists compensation_status text",
        "alter table after_sales add column if not exists evidence_status text",
        "alter table return_receipts add column if not exists return_status text not null default 'WAIT_RECEIVE'",
        "alter table return_receipts add column if not exists received_at timestamptz",
        "alter table return_receipts add column if not exists putaway_at timestamptz",
        "alter table refund_docs add column if not exists refund_status text not null default 'PENDING_APPROVAL'",
        "alter table refund_docs add column if not exists channel text not null default 'MANUAL'",
        "alter table refund_docs add column if not exists amount_cent bigint not null default 0",
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}

#[tokio::test]
async fn after_sales_finance_reconcile_refund_request_should_be_idempotent() -> Result<()> {
    let Some(db) =
        connect_test_db("after_sales_finance_reconcile_refund_request_should_be_idempotent").await
    else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let after_sales_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into tenants(tenant_id, name, status)
        values ($1, 'finance-tenant', 'ACTIVE')
        on conflict (tenant_id) do nothing
        "#,
    )
    .bind(tenant_id)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into after_sales(id, tenant_id, order_id, type, status, after_sale_status, amount_cent, raw_json)
        values ($1, $2, $3, 'RETURN_REFUND', 'APPROVED', 'WAIT_BUYER_RETURN_GOODS', 8800, '{}'::jsonb)
        "#,
    )
    .bind(after_sales_id)
    .bind(tenant_id)
    .bind(order_id)
    .execute(db.pool())
    .await?;

    let pool = db.pool().clone();
    let (app, jwt) = build_state(db);
    let token = build_token(&jwt, &tenant_id.to_string(), vec!["aftersales.write"]);

    let uri = format!("/v1/tenants/{tenant_id}/after-sales/{after_sales_id}/refund");
    let payload = serde_json::json!({
        "amount_cent": 8800,
        "channel": "MANUAL"
    });

    let (status, first) = request_json(&app, &token, Method::POST, uri.clone(), Some(payload.clone())).await?;
    assert_eq!(status, StatusCode::OK, "first refund request failed: {first}");
    let first_refund_id = first
        .get("id")
        .and_then(|v| v.as_str())
        .expect("refund id")
        .to_string();

    let (status, second) = request_json(&app, &token, Method::POST, uri, Some(payload)).await?;
    assert_eq!(status, StatusCode::OK, "second refund request failed: {second}");
    let second_refund_id = second
        .get("id")
        .and_then(|v| v.as_str())
        .expect("refund id")
        .to_string();

    assert_eq!(first_refund_id, second_refund_id);

    let refund_doc_count: i64 = sqlx::query_scalar(
        "select count(*) from refund_docs where tenant_id = $1 and after_sales_id = $2",
    )
    .bind(tenant_id)
    .bind(after_sales_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(refund_doc_count, 1);

    let outbox_count: i64 = sqlx::query_scalar(
        "select count(*) from outbox_events where tenant_id = $1 and event_type = 'REFUND_REQUESTED'",
    )
    .bind(tenant_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(outbox_count, 1);

    let audit_count: i64 = sqlx::query_scalar(
        "select count(*) from audit_logs where tenant_id = $1 and action = 'AFTERSALES.REFUND_REQUESTED'",
    )
    .bind(tenant_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(audit_count, 1);

    let audit_after_json: serde_json::Value = sqlx::query_scalar(
        r#"
        select coalesce(after_json, '{}'::jsonb)
        from audit_logs
        where tenant_id = $1
          and action = 'AFTERSALES.REFUND_REQUESTED'
        limit 1
        "#,
    )
    .bind(tenant_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        audit_after_json.get("refund_doc_id").and_then(|v| v.as_str()),
        Some(first_refund_id.as_str())
    );
    assert_eq!(
        audit_after_json.get("after_sales_id").and_then(|v| v.as_str()),
        Some(after_sales_id.to_string().as_str())
    );
    assert_eq!(
        audit_after_json.get("amount_cent").and_then(|v| v.as_i64()),
        Some(8800)
    );
    assert_eq!(
        audit_after_json.get("channel").and_then(|v| v.as_str()),
        Some("MANUAL")
    );

    Ok(())
}
