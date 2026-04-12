use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
};
use chrono::{Datelike, Utc};
use erp_api::{build_app, AppState, Claims, JwtService};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use tower::ServiceExt;

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
          shop_id uuid primary key,
          tenant_id uuid not null,
          platform text not null,
          platform_shop_id text not null,
          name text not null default '',
          shop_name text not null default '',
          status text not null default 'AUTHORIZED',
          default_carrier_code text,
          credentials jsonb not null default '{}'::jsonb,
          auth_meta_json jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, platform, platform_shop_id)
        )
        "#,
        r#"
        create table if not exists warehouses (
          warehouse_id uuid primary key,
          tenant_id uuid not null,
          name text not null,
          code text not null,
          is_default boolean not null default false,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, code)
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
          version int not null default 0,
          ship_at timestamptz,
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_month, order_id)
        )
        "#,
        r#"
        create table if not exists shipments (
          shipment_id uuid primary key,
          tenant_id uuid not null,
          shop_id uuid not null,
          order_id uuid not null,
          platform text not null,
          platform_order_id text not null,
          warehouse_id uuid,
          carrier_code text,
          tracking_no text,
          weight_gram int,
          status text not null default 'REQUESTED',
          shipment_status text not null default 'REQUESTED',
          status_version int not null default 0,
          requested_at timestamptz,
          shipped_at timestamptz,
          updated_at timestamptz not null default now(),
          unique (tenant_id, order_id)
        )
        "#,
        r#"
        create table if not exists shipment_tracking_events (
          event_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          shipment_id uuid not null,
          tracking_no text not null,
          status text not null,
          description text,
          occurred_at timestamptz not null default now(),
          created_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists shipment_packages (
          package_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          shipment_id uuid not null,
          package_no text not null,
          carrier_code text,
          tracking_no text,
          weight_gram int,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, shipment_id, package_no)
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
        create table if not exists idempotency_keys (
          tenant_id uuid not null,
          "key" text not null,
          route text not null,
          request_hash text not null,
          response_status int not null,
          response_body jsonb not null,
          created_at timestamptz not null default now(),
          unique (tenant_id, route, "key")
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
            "orders.write".to_string(),
            "shipping.write".to_string(),
            "shipping.read".to_string(),
        ],
        is_admin: false,
        iat: now,
        exp: now + 3600,
    };
    jwt.encode(&claims).expect("token should encode")
}

#[tokio::test]
async fn ship_endpoint_idempotent() -> anyhow::Result<()> {
    if let Err(err) = ship_endpoint_idempotent_inner().await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping ship_endpoint_idempotent: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

#[tokio::test]
async fn ship_endpoint_order_id_idempotent() -> anyhow::Result<()> {
    if let Err(err) = ship_endpoint_order_id_idempotent_inner().await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping ship_endpoint_order_id_idempotent: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

#[tokio::test]
async fn ship_endpoint_idempotent_conflict() -> anyhow::Result<()> {
    if let Err(err) = ship_endpoint_idempotent_conflict_inner().await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping ship_endpoint_idempotent_conflict: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn ship_endpoint_idempotent_inner() -> anyhow::Result<()> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(());
    };

    let db = Db::connect(&database_url).await?;
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let shop_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    let platform_order_id = "PLAT-ORDER-1";

    let now = Utc::now();
    let order_month =
        chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).expect("valid date");

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .bind("test-tenant")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
        values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED')
        "#,
    )
    .bind(shop_id)
    .bind(tenant_id)
    .bind("taobao")
    .bind("SHOP-1")
    .bind("shop")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into warehouses (warehouse_id, tenant_id, name, code)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(warehouse_id)
    .bind(tenant_id)
    .bind("Main")
    .bind("WH-1")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into orders (tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status)
        values ($1, $2, $3, $4, $5, $6, 'PENDING_SHIPMENT')
        "#,
    )
    .bind(tenant_id)
    .bind(order_month)
    .bind(order_id)
    .bind("taobao")
    .bind(shop_id)
    .bind(platform_order_id)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into order_index (
            tenant_id, order_id, order_month, platform, platform_order_id, shop_id, order_created_at
        ) values ($1,$2,$3,$4,$5,$6,$7)
        "#,
    )
    .bind(tenant_id)
    .bind(order_id)
    .bind(order_month)
    .bind("taobao")
    .bind(platform_order_id)
    .bind(shop_id)
    .bind(now)
    .execute(db.pool())
    .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let token = build_token(&jwt, &tenant_id.to_string());
    let state = AppState::new(db.clone(), std::sync::Arc::new(jwt));
    let app = build_app(state);

    let body = serde_json::json!({
        "order_id": order_id,
        "warehouse_id": warehouse_id,
        "carrier_code": "ZTO",
        "tracking_no": "TRACK-1"
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/shipments"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("Idempotency-Key", "idempo-1")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body)?))
        .unwrap();

    let response: Response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    let shipment_id = json
        .get("shipment_id")
        .and_then(|v| v.as_str())
        .expect("shipment_id");

    sqlx::query(
        r#"
        update orders
        set status = 'SHIPPED'
        where tenant_id = $1 and order_id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(order_id)
    .execute(db.pool())
    .await?;

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/shipments"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("Idempotency-Key", "idempo-1")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body)?))
        .unwrap();

    let response: Response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    let shipment_id_again = json
        .get("shipment_id")
        .and_then(|v| v.as_str())
        .expect("shipment_id");

    assert_eq!(shipment_id_again, shipment_id);

    let outbox_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from outbox_events
        where tenant_id = $1 and event_type = 'SHIPMENT_REQUESTED'
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(outbox_count, 1);

    Ok(())
}

async fn ship_endpoint_order_id_idempotent_inner() -> anyhow::Result<()> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(());
    };

    let db = Db::connect(&database_url).await?;
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let shop_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    let platform_order_id = "PLAT-ORDER-2";

    let now = Utc::now();
    let order_month =
        chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).expect("valid date");

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .bind("test-tenant")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
        values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED')
        "#,
    )
    .bind(shop_id)
    .bind(tenant_id)
    .bind("taobao")
    .bind("SHOP-2")
    .bind("shop")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into warehouses (warehouse_id, tenant_id, name, code)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(warehouse_id)
    .bind(tenant_id)
    .bind("Main")
    .bind("WH-2")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into orders (tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status)
        values ($1, $2, $3, $4, $5, $6, 'PENDING_SHIPMENT')
        "#,
    )
    .bind(tenant_id)
    .bind(order_month)
    .bind(order_id)
    .bind("taobao")
    .bind(shop_id)
    .bind(platform_order_id)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into order_index (
            tenant_id, order_id, order_month, platform, platform_order_id, shop_id, order_created_at
        ) values ($1,$2,$3,$4,$5,$6,$7)
        "#,
    )
    .bind(tenant_id)
    .bind(order_id)
    .bind(order_month)
    .bind("taobao")
    .bind(platform_order_id)
    .bind(shop_id)
    .bind(now)
    .execute(db.pool())
    .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let token = build_token(&jwt, &tenant_id.to_string());
    let state = AppState::new(db.clone(), std::sync::Arc::new(jwt));
    let app = build_app(state);

    let body = serde_json::json!({
        "order_id": order_id,
        "warehouse_id": warehouse_id,
        "carrier_code": "ZTO",
        "tracking_no": "TRACK-2"
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/shipments"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("Idempotency-Key", "idempo-2")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body)?))
        .unwrap();

    let response: Response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    let shipment_id = json
        .get("shipment_id")
        .and_then(|v| v.as_str())
        .expect("shipment_id");

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/shipments"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("Idempotency-Key", "idempo-3")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body)?))
        .unwrap();

    let response: Response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    let shipment_id_again = json
        .get("shipment_id")
        .and_then(|v| v.as_str())
        .expect("shipment_id");

    assert_eq!(shipment_id_again, shipment_id);

    let outbox_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from outbox_events
        where tenant_id = $1 and event_type = 'SHIPMENT_REQUESTED'
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(outbox_count, 1);

    let shipment_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from shipments
        where tenant_id = $1 and order_id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(order_id)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(shipment_count, 1);

    Ok(())
}

#[tokio::test]
async fn ship_endpoint_feature_disabled() -> anyhow::Result<()> {
    if let Err(err) = ship_endpoint_feature_disabled_inner().await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping ship_endpoint_feature_disabled: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn ship_endpoint_feature_disabled_inner() -> anyhow::Result<()> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(());
    };

    let db = Db::connect(&database_url).await?;
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let shop_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    let platform_order_id = "PLAT-ORDER-FLAG";

    let now = Utc::now();
    let order_month =
        chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).expect("valid date");

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .bind("test-tenant")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
        values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED')
        "#,
    )
    .bind(shop_id)
    .bind(tenant_id)
    .bind("taobao")
    .bind("SHOP-FLAG")
    .bind("shop")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into warehouses (warehouse_id, tenant_id, name, code)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(warehouse_id)
    .bind(tenant_id)
    .bind("Main")
    .bind("WH-FLAG")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into orders (tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status)
        values ($1, $2, $3, $4, $5, $6, 'PENDING_SHIPMENT')
        "#,
    )
    .bind(tenant_id)
    .bind(order_month)
    .bind(order_id)
    .bind("taobao")
    .bind(shop_id)
    .bind(platform_order_id)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into order_index (
            tenant_id, order_id, order_month, platform, platform_order_id, shop_id, order_created_at
        ) values ($1,$2,$3,$4,$5,$6,$7)
        "#,
    )
    .bind(tenant_id)
    .bind(order_id)
    .bind(order_month)
    .bind("taobao")
    .bind(platform_order_id)
    .bind(shop_id)
    .bind(now)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into feature_flags (tenant_id, scope, shop_id, key, enabled)
        values ($1, 'SHOP', $2, 'SHIPPING_WRITE', false)
        "#,
    )
    .bind(tenant_id)
    .bind(shop_id)
    .execute(db.pool())
    .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let token = build_token(&jwt, &tenant_id.to_string());
    let state = AppState::new(db.clone(), std::sync::Arc::new(jwt));
    let app = build_app(state);

    let body = serde_json::json!({
        "order_id": order_id,
        "warehouse_id": warehouse_id,
        "carrier_code": "ZTO",
        "tracking_no": "TRACK-FLAG"
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/shipments"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("Idempotency-Key", "idempo-flag")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body)?))
        .unwrap();

    let response: Response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    assert_eq!(
        json.get("code").and_then(|v| v.as_str()),
        Some("feature_disabled")
    );
    assert!(json
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .contains("SHIPPING_WRITE"));

    Ok(())
}

async fn ship_endpoint_idempotent_conflict_inner() -> anyhow::Result<()> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(());
    };

    let db = Db::connect(&database_url).await?;
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let shop_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    let platform_order_id = "PLAT-ORDER-3";

    let now = Utc::now();
    let order_month =
        chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).expect("valid date");

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .bind("test-tenant")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
        values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED')
        "#,
    )
    .bind(shop_id)
    .bind(tenant_id)
    .bind("taobao")
    .bind("SHOP-3")
    .bind("shop")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into warehouses (warehouse_id, tenant_id, name, code)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(warehouse_id)
    .bind(tenant_id)
    .bind("Main")
    .bind("WH-3")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into orders (tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status)
        values ($1, $2, $3, $4, $5, $6, 'PENDING_SHIPMENT')
        "#,
    )
    .bind(tenant_id)
    .bind(order_month)
    .bind(order_id)
    .bind("taobao")
    .bind(shop_id)
    .bind(platform_order_id)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into order_index (
            tenant_id, order_id, order_month, platform, platform_order_id, shop_id, order_created_at
        ) values ($1,$2,$3,$4,$5,$6,$7)
        "#,
    )
    .bind(tenant_id)
    .bind(order_id)
    .bind(order_month)
    .bind("taobao")
    .bind(platform_order_id)
    .bind(shop_id)
    .bind(now)
    .execute(db.pool())
    .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let token = build_token(&jwt, &tenant_id.to_string());
    let state = AppState::new(db.clone(), std::sync::Arc::new(jwt));
    let app = build_app(state);

    let body = serde_json::json!( {
        "order_id": order_id,
        "warehouse_id": warehouse_id,
        "carrier_code": "ZTO",
        "tracking_no": "TRACK-3"
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/shipments"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("Idempotency-Key", "idempo-conflict")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body)?))
        .unwrap();

    let response: Response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = serde_json::json!( {
        "order_id": order_id,
        "warehouse_id": warehouse_id,
        "carrier_code": "ZTO",
        "tracking_no": "TRACK-3-CHANGED"
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/shipments"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("Idempotency-Key", "idempo-conflict")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body)?))
        .unwrap();

    let response: Response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);

    Ok(())
}

#[tokio::test]
async fn logistics_tracking_placeholder() -> anyhow::Result<()> {
    if let Err(err) = logistics_tracking_placeholder_inner().await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping logistics_tracking_placeholder: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn logistics_tracking_placeholder_inner() -> anyhow::Result<()> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(());
    };

    let db = Db::connect(&database_url).await?;
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let shop_id = uuid::Uuid::new_v4();
    let shipment_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    let tracking_no = "TRACK-LOG-1";

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .bind("test-tenant")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shipments (
            shipment_id, tenant_id, shop_id, order_id, platform, platform_order_id,
            carrier_code, tracking_no, status, updated_at
        )
        values ($1, $2, $3, $4, 'taobao', 'PLAT-LOG-1', 'SF', $5, 'REQUESTED', now())
        "#,
    )
    .bind(shipment_id)
    .bind(tenant_id)
    .bind(shop_id)
    .bind(order_id)
    .bind(tracking_no)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shipment_tracking_events (
            tenant_id, shipment_id, tracking_no, status, description, occurred_at
        )
        values ($1, $2, $3, 'IN_TRANSIT', '到达分拨', now())
        "#,
    )
    .bind(tenant_id)
    .bind(shipment_id)
    .bind(tracking_no)
    .execute(db.pool())
    .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let token = build_token(&jwt, &tenant_id.to_string());
    let state = AppState::new(db.clone(), std::sync::Arc::new(jwt));
    let app = build_app(state);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/logistics/shipments?tracking_no={tracking_no}"
        ))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response: Response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    let items = json.get("items").and_then(|v| v.as_array()).expect("items");
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0].get("tracking_no").and_then(|v| v.as_str()),
        Some(tracking_no)
    );

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/logistics/shipments/{shipment_id}/tracking"
        ))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response: Response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    assert_eq!(
        json.get("latest_status").and_then(|v| v.as_str()),
        Some("IN_TRANSIT")
    );
    assert_eq!(
        json.get("tracking_no").and_then(|v| v.as_str()),
        Some(tracking_no)
    );

    Ok(())
}

#[tokio::test]
async fn shipment_packages_update_order_status() -> anyhow::Result<()> {
    if let Err(err) = shipment_packages_update_order_status_inner().await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping shipment_packages_update_order_status: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn shipment_packages_update_order_status_inner() -> anyhow::Result<()> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(());
    };

    let db = Db::connect(&database_url).await?;
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let shop_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    let shipment_id = uuid::Uuid::new_v4();
    let now = Utc::now();
    let order_month =
        chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).expect("valid date");

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .bind("package-tenant")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
        values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED')
        "#,
    )
    .bind(shop_id)
    .bind(tenant_id)
    .bind("taobao")
    .bind("SHOP-PKG")
    .bind("shop")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into warehouses (warehouse_id, tenant_id, name, code)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(warehouse_id)
    .bind(tenant_id)
    .bind("Main")
    .bind("WH-PKG")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into orders (
            tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status,
            oms_status, order_status, fulfillment_status, exception_status, review_status,
            exception_flags
        ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,'[]'::jsonb)
        "#,
    )
    .bind(tenant_id)
    .bind(order_month)
    .bind(order_id)
    .bind("taobao")
    .bind(shop_id)
    .bind("PLAT-PKG-1")
    .bind("PENDING_SHIPMENT")
    .bind("PENDING_SHIPMENT")
    .bind("PENDING_ALLOCATE")
    .bind("NOT_ALLOCATED")
    .bind("NONE")
    .bind("APPROVED")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into order_index (
            tenant_id, order_id, order_month, platform, platform_order_id, shop_id, order_created_at
        ) values ($1,$2,$3,$4,$5,$6,$7)
        "#,
    )
    .bind(tenant_id)
    .bind(order_id)
    .bind(order_month)
    .bind("taobao")
    .bind("PLAT-PKG-1")
    .bind(shop_id)
    .bind(now)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shipments (
            shipment_id, tenant_id, shop_id, order_id, platform, platform_order_id, warehouse_id,
            carrier_code, tracking_no, status
        ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,'REQUESTED')
        "#,
    )
    .bind(shipment_id)
    .bind(tenant_id)
    .bind(shop_id)
    .bind(order_id)
    .bind("taobao")
    .bind("PLAT-PKG-1")
    .bind(warehouse_id)
    .bind("ZTO")
    .bind("TRACK-PKG-1")
    .execute(db.pool())
    .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let token = build_token(&jwt, &tenant_id.to_string());
    let state = AppState::new(db.clone(), std::sync::Arc::new(jwt));
    let app = build_app(state);

    let body = serde_json::json!({
        "packages": [{
            "package_no": "PKG-1",
            "carrier_code": "ZTO",
            "tracking_no": "TRACK-PKG-1",
            "weight_gram": 1200
        }]
    });
    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/shipments/{shipment_id}/packages"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body)?))
        .unwrap();

    let response: Response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let fulfillment_status: String = sqlx::query_scalar(
        r#"
        select fulfillment_status
        from orders
        where tenant_id = $1 and order_id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(order_id)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(fulfillment_status, "READY_TO_SHIP");

    let order_status: String = sqlx::query_scalar(
        r#"
        select order_status
        from orders
        where tenant_id = $1 and order_id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(order_id)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(order_status, "PICKING");

    let package_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from shipment_packages
        where tenant_id = $1 and shipment_id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(shipment_id)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(package_count, 1);

    Ok(())
}
