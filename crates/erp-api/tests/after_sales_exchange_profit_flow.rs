use anyhow::{Context, Result};
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use chrono::{Datelike, Utc};
use erp_api::{build_app, AppState, Claims, JwtService};
use erp_repo::after_sales::{AfterSalesRepo, ExchangeDocCreateInput};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use serde_json::{json, Value};
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
    let body_json = serde_json::from_slice::<Value>(&body_bytes)
        .unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&body_bytes) }));
    Ok((status, body_json))
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
          name text not null,
          status text not null default 'ACTIVE',
          pdd_only_mode boolean not null default false,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists after_sales (
          id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          order_id uuid not null,
          type text not null,
          status text not null,
          after_sale_status text not null,
          amount_cent bigint not null default 0,
          raw_json jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists shops (
          shop_id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          platform text not null,
          platform_shop_id text not null,
          shop_name text not null default '',
          status text not null default 'ACTIVE',
          credentials jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, platform, platform_shop_id)
        )
        "#,
        r#"
        create table if not exists order_index (
          tenant_id uuid not null references tenants(tenant_id),
          order_id uuid not null,
          order_month date not null,
          platform text not null,
          platform_order_id text not null,
          shop_id uuid not null references shops(shop_id),
          order_created_at timestamptz not null,
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_id)
        )
        "#,
        r#"
        create table if not exists orders (
          tenant_id uuid not null references tenants(tenant_id),
          order_month date not null,
          order_id uuid not null,
          platform text not null,
          shop_id uuid not null references shops(shop_id),
          platform_order_id text not null,
          status text not null,
          total_amount_cent bigint not null default 0,
          discount_amount_cent bigint not null default 0,
          order_created_at timestamptz not null,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_month, order_id)
        )
        "#,
        r#"
        create table if not exists order_items (
          tenant_id uuid not null references tenants(tenant_id),
          order_month date not null,
          order_id uuid not null,
          line_id text not null,
          sku_id text,
          qty bigint not null default 0,
          amount_cent bigint,
          pay_amount_cent bigint,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_month, order_id, line_id)
        )
        "#,
        r#"
        create table if not exists order_cogs (
          tenant_id uuid not null references tenants(tenant_id),
          order_id uuid not null,
          sku_id uuid not null,
          qty bigint not null default 0,
          unit_cost_cent bigint not null default 0,
          total_cost_cent bigint not null default 0,
          ref_type text not null default '',
          ref_id text not null default '',
          shipment_id uuid,
          outbound_doc_id uuid,
          order_item_id uuid,
          created_at timestamptz not null default now(),
          primary key (tenant_id, order_id, sku_id, ref_type, ref_id)
        )
        "#,
        r#"
        create table if not exists platform_statement_lines (
          tenant_id uuid not null references tenants(tenant_id),
          statement_id uuid not null default gen_random_uuid(),
          platform_order_id text not null,
          fee_cent bigint not null default 0,
          occurred_at timestamptz,
          primary key (tenant_id, statement_id, platform_order_id)
        )
        "#,
        r#"
        create table if not exists order_key_index (
          tenant_id uuid not null references tenants(tenant_id),
          platform text not null,
          platform_order_id text not null,
          order_id uuid not null,
          order_month date not null,
          shop_id uuid not null references shops(shop_id),
          order_created_at timestamptz not null,
          updated_at timestamptz not null default now(),
          primary key (tenant_id, platform, platform_order_id)
        )
        "#,
        r#"
        create table if not exists shipments (
          tenant_id uuid not null references tenants(tenant_id),
          shipment_id uuid not null default gen_random_uuid(),
          order_id uuid not null,
          tracking_no text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, shipment_id)
        )
        "#,
        r#"
        create table if not exists exchange_docs (
          exchange_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          after_sales_id uuid not null references after_sales(id) on delete cascade,
          exchange_status text not null default 'CREATED',
          outbound_doc_id uuid,
          shipment_id uuid,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists shipping_statement_lines (
          tenant_id uuid not null references tenants(tenant_id),
          line_id uuid not null default gen_random_uuid(),
          tracking_no text not null,
          fee_cent bigint not null default 0,
          occurred_at timestamptz,
          primary key (tenant_id, line_id)
        )
        "#,
        r#"
        create table if not exists logistics_exception_work_orders (
          ticket_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          order_id text not null,
          tracking_no text not null,
          carrier_code text not null,
          exception_type text not null,
          status text not null default 'OPEN',
          assignee text,
          due_at timestamptz,
          escalation_level int not null default 0,
          resolution text,
          resolved_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists profit_snapshots (
          tenant_id uuid not null references tenants(tenant_id),
          order_id uuid not null,
          revenue_cent bigint not null default 0,
          cost_cent bigint not null default 0,
          platform_fee_cent bigint not null default 0,
          shipping_fee_cent bigint not null default 0,
          gross_profit_cent bigint not null default 0,
          computed_at timestamptz not null,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_id)
        )
        "#,
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}

#[tokio::test]
async fn after_sales_exchange_docs_should_be_idempotent_for_same_shipment() -> Result<()> {
    let Some(db) =
        connect_test_db("after_sales_exchange_docs_should_be_idempotent_for_same_shipment").await
    else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let after_sales_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    let shipment_id = uuid::Uuid::new_v4();
    let outbound_doc_id = uuid::Uuid::new_v4();

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .bind("exchange-doc-tenant")
    .execute(db.pool())
    .await?;
    sqlx::query(
        r#"
        insert into after_sales(id, tenant_id, order_id, type, status, after_sale_status, amount_cent, raw_json)
        values ($1, $2, $3, 'EXCHANGE', 'APPROVED', 'WAIT_BUYER_RETURN_GOODS', 6800, '{}'::jsonb)
        "#,
    )
    .bind(after_sales_id)
    .bind(tenant_id)
    .bind(order_id)
    .execute(db.pool())
    .await?;

    let repo = AfterSalesRepo::new(db.pool());
    let first = repo
        .create_exchange_docs(
            erp_types::TenantId(tenant_id),
            after_sales_id,
            &[ExchangeDocCreateInput {
                outbound_doc_id: Some(outbound_doc_id),
                shipment_id: Some(shipment_id),
            }],
        )
        .await?;
    let second = repo
        .create_exchange_docs(
            erp_types::TenantId(tenant_id),
            after_sales_id,
            &[ExchangeDocCreateInput {
                outbound_doc_id: Some(outbound_doc_id),
                shipment_id: Some(shipment_id),
            }],
        )
        .await?;

    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    assert_eq!(first[0].exchange_id, second[0].exchange_id);

    let exchange_doc_count: i64 =
        sqlx::query_scalar("select count(*) from exchange_docs where tenant_id = $1 and after_sales_id = $2")
            .bind(tenant_id)
            .bind(after_sales_id)
            .fetch_one(db.pool())
            .await?;
    assert_eq!(exchange_doc_count, 1);

    Ok(())
}

#[tokio::test]
async fn task6_routes_validate_path_before_db_access() {
    let database_url = support::test_database_url();
    let db = Db::connect_lazy(&database_url).expect("lazy db should be constructible");
    let (app, jwt) = build_state(db);
    let token = build_token(
        &jwt,
        &uuid::Uuid::new_v4().to_string(),
        vec!["aftersales.write", "shipping.write", "finance.read"],
    );

    let cases = [
        (
            Method::POST,
            format!(
                "/v1/tenants/not-a-uuid/after-sales/{}/exchange",
                uuid::Uuid::new_v4(),
            ),
            Some(json!({})),
        ),
        (
            Method::POST,
            format!(
                "/v1/tenants/not-a-uuid/after-sales/{}/resend",
                uuid::Uuid::new_v4(),
            ),
            Some(json!({})),
        ),
        (
            Method::POST,
            format!(
                "/v1/tenants/not-a-uuid/logistics/exception-work-orders/{}/resolve",
                uuid::Uuid::new_v4(),
            ),
            Some(json!({ "resolution": "ok" })),
        ),
        (
            Method::GET,
            "/v1/tenants/not-a-uuid/fin/profit".to_string(),
            None,
        ),
        (
            Method::GET,
            format!("/v1/tenants/not-a-uuid/fin/profit/{}", uuid::Uuid::new_v4()),
            None,
        ),
    ];

    for (method, uri, body) in cases {
        let mut builder = Request::builder()
            .uri(uri.clone())
            .method(method.clone())
            .header("authorization", format!("Bearer {token}"));
        let request = if let Some(payload) = body {
            builder = builder.header("content-type", "application/json");
            builder
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap()
        } else {
            builder.body(Body::empty()).unwrap()
        };
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "route {method} {uri} should exist and validate tenant path before DB access"
        );
    }
}

#[tokio::test]
async fn logistics_resolve_triggers_profit_snapshot_write_and_read() -> Result<()> {
    let Some(db) =
        connect_test_db("logistics_resolve_triggers_profit_snapshot_write_and_read").await
    else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let shop_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    let order_month = chrono::NaiveDate::from_ymd_opt(Utc::now().year(), Utc::now().month(), 1)
        .expect("valid month date");
    let order_created_at = Utc::now();

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .bind("task6-profit-tenant")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, shop_name, status)
        values ($1, $2, $3, $4, $5, 'ACTIVE')
        "#,
    )
    .bind(shop_id)
    .bind(tenant_id)
    .bind("taobao")
    .bind("TASK6-SHOP-1")
    .bind("task6-profit-shop")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into order_index (
          tenant_id, order_id, order_month, platform, platform_order_id, shop_id, order_created_at
        ) values ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(tenant_id)
    .bind(order_id)
    .bind(order_month)
    .bind("taobao")
    .bind("TASK6-PLAT-ORDER-1")
    .bind(shop_id)
    .bind(order_created_at)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into order_key_index (
          tenant_id, platform, platform_order_id, order_id, order_month, shop_id, order_created_at
        ) values ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(tenant_id)
    .bind("taobao")
    .bind("TASK6-PLAT-ORDER-1")
    .bind(order_id)
    .bind(order_month)
    .bind(shop_id)
    .bind(order_created_at)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into orders (
          tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status,
          total_amount_cent, discount_amount_cent, order_created_at
        ) values ($1, $2, $3, $4, $5, $6, 'PENDING', $7, $8, $9)
        "#,
    )
    .bind(tenant_id)
    .bind(order_month)
    .bind(order_id)
    .bind("taobao")
    .bind(shop_id)
    .bind("TASK6-PLAT-ORDER-1")
    .bind(5_000_i64)
    .bind(500_i64)
    .bind(order_created_at)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into order_items (
          tenant_id, order_month, order_id, line_id, sku_id, qty, amount_cent, pay_amount_cent
        ) values ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(tenant_id)
    .bind(order_month)
    .bind(order_id)
    .bind("LINE-1")
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(1_i64)
    .bind(4_500_i64)
    .bind(4_300_i64)
    .execute(db.pool())
    .await?;

    let (app, jwt) = build_state(db);
    let token = build_token(
        &jwt,
        &tenant_id.to_string(),
        vec!["shipping.write", "shipping.read", "finance.read"],
    );

    let (status, create_work_order) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant_id}/logistics/exception-work-orders"),
        Some(json!({
          "order_id": order_id.to_string(),
          "tracking_no": "TASK6-TRACK-001",
          "carrier_code": "YTO",
          "exception_type": "DELAYED"
        })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "create logistics work order failed: {create_work_order}"
    );
    let ticket_id = create_work_order
        .get("ticket_id")
        .and_then(Value::as_str)
        .context("ticket_id missing")?
        .to_string();

    let (status, resolved_work_order) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant_id}/logistics/exception-work-orders/{ticket_id}/resolve"),
        Some(json!({
          "resolution": "rescheduled delivery and customer notified"
        })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "resolve logistics work order failed: {resolved_work_order}"
    );
    assert_eq!(
        resolved_work_order.get("status").and_then(Value::as_str),
        Some("RESOLVED")
    );

    let (status, profit_list) = request_json(
        &app,
        &token,
        Method::GET,
        format!("/v1/tenants/{tenant_id}/fin/profit?shop_id={shop_id}&limit=20&offset=0"),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "list profit snapshots failed: {profit_list}"
    );
    let items = profit_list
        .get("items")
        .and_then(Value::as_array)
        .context("profit list items missing")?;
    let snapshot = items
        .iter()
        .find(|item| {
            item.get("order_id")
                .and_then(Value::as_str)
                .map(|value| value == order_id.to_string())
                .unwrap_or(false)
        })
        .context("profit snapshot for resolved logistics work order not found")?;
    assert_eq!(
        snapshot.get("revenue_cent").and_then(Value::as_i64),
        Some(4_300)
    );
    assert_eq!(snapshot.get("cost_cent").and_then(Value::as_i64), Some(0));
    assert_eq!(
        snapshot.get("platform_fee_cent").and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        snapshot.get("shipping_fee_cent").and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        snapshot.get("gross_profit_cent").and_then(Value::as_i64),
        Some(4_300)
    );

    let (status, profit_detail) = request_json(
        &app,
        &token,
        Method::GET,
        format!("/v1/tenants/{tenant_id}/fin/profit/{order_id}"),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "get profit snapshot detail failed: {profit_detail}"
    );
    assert_eq!(
        profit_detail
            .get("snapshot")
            .and_then(|v| v.get("order_id"))
            .and_then(Value::as_str),
        Some(order_id.to_string().as_str())
    );

    Ok(())
}
