use anyhow::Result;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Datelike, Utc};
use erp_api::{build_app, AppState, Claims, JwtService};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

mod support;

fn build_state() -> (axum::Router, JwtService) {
    std::env::set_var("IDEMPOTENCY_DISABLED", "true");
    let database_url = support::test_database_url();
    let db = Db::connect_lazy(&database_url).expect("lazy db should be constructible");
    let jwt = JwtService::new("test-secret".to_string());
    let state = AppState::new(db, std::sync::Arc::new(jwt.clone()));
    let app = build_app(state);
    (app, jwt)
}

fn build_token(jwt: &JwtService, tenant_id: &str, perms: Vec<&str>) -> String {
    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: uuid::Uuid::new_v4().to_string(),
        tenant_id: tenant_id.to_string(),
        perms: perms.into_iter().map(|p| p.to_string()).collect(),
        is_admin: false,
        iat: now,
        exp: now + 3600,
    };
    jwt.encode(&claims).expect("token should encode")
}

fn tenant_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

async fn setup_schema(pool: &PgPool) -> Result<()> {
    let statements = [
        "create extension if not exists pgcrypto",
        r#"
        create table if not exists tenants (
          tenant_id uuid primary key,
          name text not null,
          status text not null default 'ACTIVE',
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
          tenant_id uuid not null,
          order_id uuid not null,
          order_month date not null,
          platform text not null,
          platform_order_id text not null,
          shop_id uuid not null,
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
          platform text not null,
          shop_id uuid not null,
          platform_order_id text not null,
          status text not null,
          oms_status text not null default 'NEW',
          order_status text not null default 'PENDING_REVIEW',
          fulfillment_status text not null default 'NOT_ALLOCATED',
          exception_status text not null default 'NONE',
          review_status text not null default 'NOT_REVIEWED',
          exception_flags jsonb not null default '[]'::jsonb,
          logistics_company text,
          tracking_no text,
          ship_warehouse_id uuid,
          inventory_shortage boolean not null default false,
          version int not null default 0,
          order_created_at timestamptz not null,
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_month, order_id)
        )
        "#,
        r#"
        create table if not exists warehouses (
          warehouse_id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          name text not null,
          is_default boolean not null default false,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists products (
          product_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          name text not null,
          allow_negative_stock boolean not null default false,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists product_warehouses (
          tenant_id uuid not null references tenants(tenant_id),
          product_id uuid not null references products(product_id),
          warehouse_id uuid not null references warehouses(warehouse_id),
          allow_negative_stock boolean not null default false,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, product_id, warehouse_id)
        )
        "#,
        r#"
        create table if not exists skus (
          sku_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          product_id uuid not null references products(product_id),
          sku_code text not null,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists order_items (
          tenant_id uuid not null,
          order_month date not null,
          order_id uuid not null,
          line_id text not null,
          sku_id text,
          qty int not null,
          primary key (tenant_id, order_month, order_id, line_id)
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
        create table if not exists order_exception_tickets (
          ticket_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          order_id uuid not null,
          reason_code text not null,
          status text not null default 'OPEN',
          handler_id uuid,
          handled_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, order_id, reason_code)
        )
        "#,
        r#"
        create table if not exists exception_reasons (
          tenant_id uuid not null references tenants(tenant_id),
          code text not null,
          label text not null,
          category text not null default '',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, code)
        )
        "#,
        r#"
        create table if not exists order_reviews (
          tenant_id uuid not null references tenants(tenant_id),
          order_id uuid not null,
          status text not null default 'PENDING',
          reviewed_by uuid,
          reviewed_at timestamptz,
          notes text not null default '',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_id)
        )
        "#,
        r#"
        create table if not exists settings (
          setting_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          scope_type text not null,
          scope_id uuid,
          key text not null,
          value_json jsonb not null default '{}'::jsonb,
          version int not null default 1,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create unique index if not exists idx_test_settings_scope_unique
          on settings (tenant_id, scope_type, scope_id, key)
        "#,
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}

#[tokio::test]
async fn oms_orders_forbidden_without_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();

    let token = build_token(&jwt, &tenant, vec![]);
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/orders"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value =
        serde_json::from_slice(&body).expect("error response should be json");
    assert_eq!(json.get("code").and_then(|v| v.as_str()), Some("forbidden"));
}

#[tokio::test]
async fn oms_orders_forbidden_on_tenant_mismatch() {
    let (app, jwt) = build_state();
    let tenant_a = tenant_id();
    let tenant_b = tenant_id();

    let token = build_token(&jwt, &tenant_b, vec!["orders.read"]);
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_a}/orders"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn review_order_forbidden_without_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let order_id = uuid::Uuid::new_v4();
    let body = serde_json::json!({
        "action": "PASS"
    });

    let token = build_token(&jwt, &tenant, vec!["orders.read"]);
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/orders/{order_id}/review"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn review_order_forbidden_on_tenant_mismatch() {
    let (app, jwt) = build_state();
    let tenant_a = tenant_id();
    let tenant_b = tenant_id();
    let order_id = uuid::Uuid::new_v4();
    let body = serde_json::json!({
        "action": "REJECT",
        "reason": "test"
    });

    let token = build_token(&jwt, &tenant_b, vec!["orders.write"]);
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_a}/orders/{order_id}/review"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn review_rules_forbidden_without_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();

    let token = build_token(&jwt, &tenant, vec![]);
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/oms/review-rules"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn blacklist_forbidden_without_write_perm() {
    let (app, jwt) = build_state();
    let tenant = tenant_id();
    let body = serde_json::json!({
        "key_type": "phone",
        "key_value": "13800001111",
        "reason": "test"
    });

    let token = build_token(&jwt, &tenant, vec!["orders.read"]);
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant}/oms/customer-blacklist"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn review_order_writes_audit_log() -> Result<()> {
    if let Err(err) = review_order_writes_audit_log_inner().await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping review_order_writes_audit_log: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn review_order_writes_audit_log_inner() -> Result<()> {
    let database_url = support::test_database_url();
    let db = Db::connect(&database_url).await?;
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let shop_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    let product_id = uuid::Uuid::new_v4();
    let _warehouse_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();
    let now = Utc::now();
    let order_month = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .bind("audit-tenant")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(shop_id)
    .bind(tenant_id)
    .bind("taobao")
    .bind("SHOP-1")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into products (product_id, tenant_id, name, allow_negative_stock)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(product_id)
    .bind(tenant_id)
    .bind("mock-product")
    .bind(false)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into skus (sku_id, tenant_id, product_id, sku_code)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(sku_id)
    .bind(tenant_id)
    .bind(product_id)
    .bind("SKU-1")
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
    .bind("PLAT-ORDER-1")
    .bind(shop_id)
    .bind(now)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into orders (
            tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status,
            oms_status, review_status, exception_flags, logistics_company, tracking_no,
            inventory_shortage, version, order_created_at
        ) values ($1, $2, $3, $4, $5, $6, $7, 'NEW', 'MANUAL_REVIEW', '[]'::jsonb, $8, $9, $10, 0, $11)
        "#,
    )
    .bind(tenant_id)
    .bind(order_month)
    .bind(order_id)
    .bind("taobao")
    .bind(shop_id)
    .bind("PLAT-ORDER-1")
    .bind("PENDING_SHIPMENT")
    .bind("SF")
    .bind("SF123456")
    .bind(false)
    .bind(now)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into order_items (
          tenant_id, order_month, order_id, line_id, sku_id, qty
        ) values ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(tenant_id)
    .bind(order_month)
    .bind(order_id)
    .bind("LINE-1")
    .bind(sku_id.to_string())
    .bind(1)
    .execute(db.pool())
    .await?;

    let (app, jwt) = build_state();
    let reviewer_id = uuid::Uuid::new_v4();
    let now_ts = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: reviewer_id.to_string(),
        tenant_id: tenant_id.to_string(),
        perms: vec!["orders.review".to_string()],
        is_admin: false,
        iat: now_ts,
        exp: now_ts + 3600,
    };
    let token = jwt.encode(&claims)?;
    let body = serde_json::json!({
        "action": "PASS",
        "version": 0
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/orders/{order_id}/review"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body)?))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let log_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from audit_logs
        where tenant_id = $1
          and entity_type = 'ORDER'
          and entity_id = $2
          and action = 'OMS.REVIEW_PASS'
        "#,
    )
    .bind(tenant_id)
    .bind(order_id.to_string())
    .fetch_one(db.pool())
    .await?;

    assert_eq!(log_count, 1);

    let review_row: (
        String,
        Option<uuid::Uuid>,
        Option<chrono::DateTime<Utc>>,
        String,
    ) = sqlx::query_as(
        r#"
            select status, reviewed_by, reviewed_at, notes
            from order_reviews
            where tenant_id = $1 and order_id = $2
            "#,
    )
    .bind(tenant_id)
    .bind(order_id)
    .fetch_one(db.pool())
    .await?;

    assert_eq!(review_row.0, "APPROVED");
    assert_eq!(review_row.1, Some(reviewer_id));
    assert!(review_row.2.is_some());
    assert!(review_row.3.is_empty());
    Ok(())
}

#[tokio::test]
async fn review_order_blocks_shortage_without_negative_stock() -> Result<()> {
    if let Err(err) = review_order_shortage_case_inner(false, StatusCode::BAD_REQUEST).await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping review_order_blocks_shortage_without_negative_stock: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

#[tokio::test]
async fn review_order_allows_shortage_with_negative_stock() -> Result<()> {
    if let Err(err) = review_order_shortage_case_inner(true, StatusCode::OK).await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping review_order_allows_shortage_with_negative_stock: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn review_order_shortage_case_inner(
    allow_negative_stock: bool,
    expected_status: StatusCode,
) -> Result<()> {
    let database_url = support::test_database_url();
    let db = Db::connect(&database_url).await?;
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let shop_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    let product_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();
    let now = Utc::now();
    let order_month = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .bind("shortage-tenant")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(shop_id)
    .bind(tenant_id)
    .bind("taobao")
    .bind("SHOP-1")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into products (product_id, tenant_id, name, allow_negative_stock)
        values ($1, $2, $3, false)
        "#,
    )
    .bind(product_id)
    .bind(tenant_id)
    .bind("shortage-product")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into warehouses (warehouse_id, tenant_id, name)
        values ($1, $2, $3)
        "#,
    )
    .bind(warehouse_id)
    .bind(tenant_id)
    .bind("WH-1")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into product_warehouses (tenant_id, product_id, warehouse_id, allow_negative_stock)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(tenant_id)
    .bind(product_id)
    .bind(warehouse_id)
    .bind(allow_negative_stock)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into skus (sku_id, tenant_id, product_id, sku_code)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(sku_id)
    .bind(tenant_id)
    .bind(product_id)
    .bind("SKU-1")
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
    .bind("PLAT-ORDER-1")
    .bind(shop_id)
    .bind(now)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into orders (
            tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status,
            oms_status, review_status, exception_flags, logistics_company, tracking_no,
            ship_warehouse_id, inventory_shortage, version, order_created_at
        ) values ($1, $2, $3, $4, $5, $6, $7, 'NEW', 'MANUAL_REVIEW', '[]'::jsonb, $8, $9, $10, $11, 0, $12)
        "#,
    )
    .bind(tenant_id)
    .bind(order_month)
    .bind(order_id)
    .bind("taobao")
    .bind(shop_id)
    .bind("PLAT-ORDER-1")
    .bind("PENDING_SHIPMENT")
    .bind("SF")
    .bind("SF123456")
    .bind(warehouse_id)
    .bind(true)
    .bind(now)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into order_items (
          tenant_id, order_month, order_id, line_id, sku_id, qty
        ) values ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(tenant_id)
    .bind(order_month)
    .bind(order_id)
    .bind("LINE-1")
    .bind(sku_id.to_string())
    .bind(1)
    .execute(db.pool())
    .await?;

    let (app, jwt) = build_state();
    let token = build_token(&jwt, &tenant_id.to_string(), vec!["orders.review"]);
    let body = serde_json::json!({
        "action": "PASS",
        "version": 0
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/orders/{order_id}/review"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body)?))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), expected_status);

    Ok(())
}

#[tokio::test]
async fn review_order_blocks_invalid_goods() -> Result<()> {
    if let Err(err) = review_order_invalid_goods_case_inner(false).await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping review_order_blocks_invalid_goods: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

#[tokio::test]
async fn force_review_order_blocks_invalid_goods() -> Result<()> {
    if let Err(err) = review_order_invalid_goods_case_inner(true).await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping force_review_order_blocks_invalid_goods: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn review_order_invalid_goods_case_inner(force_review: bool) -> Result<()> {
    let database_url = support::test_database_url();
    let db = Db::connect(&database_url).await?;
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let shop_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    let product_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();
    let now = Utc::now();
    let order_month = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .bind("invalid-goods-tenant")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(shop_id)
    .bind(tenant_id)
    .bind("taobao")
    .bind("SHOP-1")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into products (product_id, tenant_id, name, allow_negative_stock)
        values ($1, $2, $3, false)
        "#,
    )
    .bind(product_id)
    .bind(tenant_id)
    .bind("invalid-goods-product")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into warehouses (warehouse_id, tenant_id, name)
        values ($1, $2, $3)
        "#,
    )
    .bind(warehouse_id)
    .bind(tenant_id)
    .bind("WH-1")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into product_warehouses (tenant_id, product_id, warehouse_id, allow_negative_stock)
        values ($1, $2, $3, true)
        "#,
    )
    .bind(tenant_id)
    .bind(product_id)
    .bind(warehouse_id)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into skus (sku_id, tenant_id, product_id, sku_code)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(sku_id)
    .bind(tenant_id)
    .bind(product_id)
    .bind("SKU-1")
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
    .bind("PLAT-ORDER-1")
    .bind(shop_id)
    .bind(now)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into orders (
            tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status,
            oms_status, review_status, exception_flags, logistics_company, tracking_no,
            ship_warehouse_id, inventory_shortage, version, order_created_at
        ) values ($1, $2, $3, $4, $5, $6, $7, 'NEW', 'MANUAL_REVIEW', $8, $9, $10, $11, false, 0, $12)
        "#,
    )
    .bind(tenant_id)
    .bind(order_month)
    .bind(order_id)
    .bind("taobao")
    .bind(shop_id)
    .bind("PLAT-ORDER-1")
    .bind("PENDING_SHIPMENT")
    .bind(serde_json::json!(["INVALID_GOODS"]))
    .bind("SF")
    .bind("SF123456")
    .bind(warehouse_id)
    .bind(now)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into order_items (
          tenant_id, order_month, order_id, line_id, sku_id, qty
        ) values ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(tenant_id)
    .bind(order_month)
    .bind(order_id)
    .bind("LINE-1")
    .bind(sku_id.to_string())
    .bind(1)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into settings (
          tenant_id, scope_type, scope_id, key, value_json
        ) values ($1, 'TENANT', null, $2, $3)
        "#,
    )
    .bind(tenant_id)
    .bind("orders.force_review_password")
    .bind(serde_json::json!("force-review"))
    .execute(db.pool())
    .await?;

    let (app, jwt) = build_state();
    let token = build_token(&jwt, &tenant_id.to_string(), vec!["orders.review"]);
    let (uri, body) = if force_review {
        (
            format!("/v1/tenants/{tenant_id}/orders/{order_id}/force-review"),
            serde_json::json!({
                "password": "force-review",
                "reason": "invalid goods",
                "version": 0
            }),
        )
    } else {
        (
            format!("/v1/tenants/{tenant_id}/orders/{order_id}/review"),
            serde_json::json!({
                "action": "PASS",
                "version": 0
            }),
        )
    };
    let request = Request::builder()
        .uri(uri)
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body)?))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response.into_body().collect().await?.to_bytes();
    let body_text = String::from_utf8_lossy(&body);
    assert!(
        body_text.contains("invalid goods"),
        "expected invalid goods error, got: {body_text}"
    );
    Ok(())
}

#[tokio::test]
async fn exception_pool_resolve_updates_review_state() -> Result<()> {
    if let Err(err) = exception_pool_resolve_updates_review_state_inner().await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping exception_pool_resolve_updates_review_state: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn exception_pool_resolve_updates_review_state_inner() -> Result<()> {
    let database_url = support::test_database_url();
    let db = Db::connect(&database_url).await?;
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let shop_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    let now = Utc::now();
    let order_month = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .bind("exception-tenant")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(shop_id)
    .bind(tenant_id)
    .bind("taobao")
    .bind("SHOP-EX-1")
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
    .bind("PLAT-EX-1")
    .bind(shop_id)
    .bind(now)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into orders (
            tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status,
            oms_status, order_status, fulfillment_status, exception_status, review_status,
            exception_flags, inventory_shortage, version, order_created_at
        ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)
        "#,
    )
    .bind(tenant_id)
    .bind(order_month)
    .bind(order_id)
    .bind("taobao")
    .bind(shop_id)
    .bind("PLAT-EX-1")
    .bind("PENDING_SHIPMENT")
    .bind("REVIEW_PENDING")
    .bind("PENDING_REVIEW")
    .bind("NOT_ALLOCATED")
    .bind("MANUAL_HOLD")
    .bind("MANUAL_REVIEW")
    .bind(serde_json::json!(["REVIEW_FLAGGED"]))
    .bind(false)
    .bind(0)
    .bind(now)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into order_exception_tickets (
            tenant_id, order_id, reason_code, status
        ) values ($1, $2, $3, 'OPEN')
        "#,
    )
    .bind(tenant_id)
    .bind(order_id)
    .bind("REVIEW_FLAGGED")
    .execute(db.pool())
    .await?;

    let (app, jwt) = build_state();
    let token = build_token(&jwt, &tenant_id.to_string(), vec!["orders.write"]);
    let body = serde_json::json!({
        "action": "RESOLVE",
        "flags": ["REVIEW_FLAGGED"]
    });
    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/orders/{order_id}/exceptions/handle"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body)?))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let row: (String, String, String, String, serde_json::Value) = sqlx::query_as(
        r#"
        select oms_status, review_status, order_status, exception_status, exception_flags
        from orders
        where tenant_id = $1 and order_id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(order_id)
    .fetch_one(db.pool())
    .await?;

    assert_eq!(row.0, "PENDING_SHIPMENT");
    assert_eq!(row.1, "APPROVED");
    assert_eq!(row.2, "PENDING_ALLOCATE");
    assert_eq!(row.3, "RELEASED");
    let flag_count = row
        .4
        .as_array()
        .map(|value| value.len())
        .unwrap_or_default();
    assert_eq!(flag_count, 0);

    let ticket_row: (String, Option<chrono::DateTime<Utc>>) = sqlx::query_as(
        r#"
        select status, handled_at
        from order_exception_tickets
        where tenant_id = $1 and order_id = $2 and reason_code = 'REVIEW_FLAGGED'
        "#,
    )
    .bind(tenant_id)
    .bind(order_id)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(ticket_row.0, "RESOLVED");
    assert!(ticket_row.1.is_some());

    let review_status: String = sqlx::query_scalar(
        r#"
        select status
        from order_reviews
        where tenant_id = $1 and order_id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(order_id)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(review_status, "APPROVED");

    Ok(())
}

#[tokio::test]
async fn exception_stats_reports_counts() -> Result<()> {
    if let Err(err) = exception_stats_reports_counts_inner().await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping exception_stats_reports_counts: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn exception_stats_reports_counts_inner() -> Result<()> {
    let database_url = support::test_database_url();
    let db = Db::connect(&database_url).await?;
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .bind("exception-stats")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into exception_reasons (tenant_id, code, label, category)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(tenant_id)
    .bind("REVIEW_FLAGGED")
    .bind("Review flagged")
    .bind("REVIEW")
    .execute(db.pool())
    .await?;

    let order_id_a = uuid::Uuid::new_v4();
    let order_id_b = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into order_exception_tickets (
            tenant_id, order_id, reason_code, status
        ) values
          ($1, $2, $3, 'OPEN'),
          ($1, $4, $3, 'RESOLVED')
        "#,
    )
    .bind(tenant_id)
    .bind(order_id_a)
    .bind("REVIEW_FLAGGED")
    .bind(order_id_b)
    .execute(db.pool())
    .await?;

    let (app, jwt) = build_state();
    let token = build_token(&jwt, &tenant_id.to_string(), vec!["orders.read"]);
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/orders/exceptions/stats"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body)?;
    assert_eq!(json.get("total").and_then(|v| v.as_i64()), Some(2));
    assert_eq!(json.get("open").and_then(|v| v.as_i64()), Some(1));
    assert_eq!(json.get("processed").and_then(|v| v.as_i64()), Some(1));
    let reasons = json
        .get("reasons")
        .and_then(|v| v.as_array())
        .expect("reasons");
    assert_eq!(reasons.len(), 1);
    let reason = &reasons[0];
    assert_eq!(
        reason.get("reason_code").and_then(|v| v.as_str()),
        Some("REVIEW_FLAGGED")
    );
    assert_eq!(
        reason.get("label").and_then(|v| v.as_str()),
        Some("Review flagged")
    );
    assert_eq!(reason.get("total").and_then(|v| v.as_i64()), Some(2));
    assert_eq!(reason.get("open").and_then(|v| v.as_i64()), Some(1));
    assert_eq!(reason.get("processed").and_then(|v| v.as_i64()), Some(1));

    Ok(())
}
