use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
};
use chrono::{Datelike, Utc};
use erp_api::{AppState, Claims, JwtService, build_app};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use std::sync::Arc;
use tower::ServiceExt;

mod support;

async fn setup_schema(pool: &sqlx::PgPool) -> anyhow::Result<()> {
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
        create table if not exists roles (
          role_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          name text not null,
          created_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists role_permissions (
          tenant_id uuid not null,
          role_id uuid not null,
          perm_code text not null
        )
        "#,
        r#"
        create table if not exists permissions (
          tenant_id uuid not null,
          code text not null,
          name text not null,
          module text not null,
          description text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, code)
        )
        "#,
        r#"
        create table if not exists users (
          user_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          email text not null,
          password_hash text not null,
          status text not null default 'ACTIVE',
          created_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists user_roles (
          tenant_id uuid not null,
          user_id uuid not null,
          role_id uuid not null
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
        create table if not exists settings (
          setting_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
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
        create table if not exists dead_letters (
          dead_letter_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          outbox_id uuid not null,
          event_type text not null,
          payload jsonb not null,
          error text not null,
          attempts int not null,
          classification text not null,
          next_action text not null,
          status text not null default 'PENDING',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          replayed_at timestamptz
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
          primary key (tenant_id, event_month, outbox_id),
          unique (tenant_id, event_month, dedup_key)
        )
        "#,
        r#"
        create table if not exists feature_flags (
          tenant_id uuid not null,
          scope text not null,
          shop_id uuid,
          key text not null,
          enabled boolean not null default false,
          updated_at timestamptz not null default now(),
          created_at timestamptz not null default now()
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
          inventory_shortage boolean not null default false,
          version int not null default 1,
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_month, order_id)
        )
        "#,
        r#"
        create table if not exists products (
          product_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          name text not null,
          allow_negative_stock boolean not null default false,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists skus (
          sku_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          product_id uuid not null,
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
        create table if not exists order_index (
          tenant_id uuid not null,
          order_id uuid not null,
          order_month date not null,
          platform text not null,
          platform_order_id text not null,
          shop_id uuid not null,
          primary key (tenant_id, order_id)
        )
        "#,
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}

fn build_token(
    jwt: &JwtService,
    tenant_id: uuid::Uuid,
    user_id: uuid::Uuid,
    perms: Vec<&str>,
) -> String {
    let now = Utc::now().timestamp() as usize;
    let claims = Claims::bearer(
        user_id.to_string(),
        tenant_id.to_string(),
        perms.into_iter().map(|p| p.to_string()).collect(),
        false,
        now,
        now + 3600,
    );
    jwt.encode(&claims).expect("token should encode")
}

async fn load_db() -> anyhow::Result<Option<Db>> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(None);
    };
    match Db::connect(&database_url).await {
        Ok(db) => Ok(Some(db)),
        Err(err) => {
            eprintln!("skipping db test: {err}");
            Ok(None)
        }
    }
}

#[tokio::test]
async fn roles_permission_forbidden() {
    std::env::set_var("IDEMPOTENCY_DISABLED", "true");
    let database_url = support::test_database_url();
    let db = Db::connect_lazy(&database_url).expect("lazy db should be constructible");
    let jwt = JwtService::new("test-secret".to_string());
    let app = build_app(AppState::new(db, Arc::new(jwt.clone())));

    let tenant_id = uuid::Uuid::new_v4();
    let user_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, tenant_id, user_id, vec![]);

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/roles"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response: Response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn roles_tenant_isolation() -> anyhow::Result<()> {
    let Some(db) = load_db().await? else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;

    let tenant_a = uuid::Uuid::new_v4();
    let tenant_b = uuid::Uuid::new_v4();
    sqlx::query("insert into tenants (tenant_id, name, status) values ($1, 't-a', 'ACTIVE')")
        .bind(tenant_a)
        .execute(db.pool())
        .await?;
    sqlx::query("insert into tenants (tenant_id, name, status) values ($1, 't-b', 'ACTIVE')")
        .bind(tenant_b)
        .execute(db.pool())
        .await?;

    let role_a = uuid::Uuid::new_v4();
    let role_b = uuid::Uuid::new_v4();
    sqlx::query("insert into roles (role_id, tenant_id, name) values ($1, $2, 'role-a')")
        .bind(role_a)
        .bind(tenant_a)
        .execute(db.pool())
        .await?;
    sqlx::query("insert into roles (role_id, tenant_id, name) values ($1, $2, 'role-b')")
        .bind(role_b)
        .bind(tenant_b)
        .execute(db.pool())
        .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let app = build_app(AppState::new(db.clone(), Arc::new(jwt.clone())));

    let token_a = build_token(&jwt, tenant_a, uuid::Uuid::new_v4(), vec!["SYS.ROLE.READ"]);
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_a}/roles"))
        .method("GET")
        .header("authorization", format!("Bearer {token_a}"))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await?.to_bytes();
    let roles: Vec<serde_json::Value> = serde_json::from_slice(&body_bytes)?;
    assert_eq!(roles.len(), 1);
    assert_eq!(
        roles[0].get("role_id").and_then(|v| v.as_str()),
        Some(role_a.to_string().as_str())
    );

    let token_b = build_token(&jwt, tenant_b, uuid::Uuid::new_v4(), vec!["SYS.ROLE.READ"]);
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_b}/roles"))
        .method("GET")
        .header("authorization", format!("Bearer {token_b}"))
        .body(Body::empty())
        .unwrap();
    let response: Response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await?.to_bytes();
    let roles: Vec<serde_json::Value> = serde_json::from_slice(&body_bytes)?;
    assert_eq!(roles.len(), 1);
    assert_eq!(
        roles[0].get("role_id").and_then(|v| v.as_str()),
        Some(role_b.to_string().as_str())
    );

    Ok(())
}

#[tokio::test]
async fn review_order_writes_audit_log() -> anyhow::Result<()> {
    let Some(db) = load_db().await? else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    sqlx::query("insert into tenants (tenant_id, name, status) values ($1, 't-a', 'ACTIVE')")
        .bind(tenant_id)
        .execute(db.pool())
        .await?;

    let order_id = uuid::Uuid::new_v4();
    let product_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();
    let order_month =
        chrono::NaiveDate::from_ymd_opt(Utc::now().year(), Utc::now().month(), 1).unwrap();
    let shop_id = uuid::Uuid::new_v4();

    sqlx::query(
        r#"
        insert into products (product_id, tenant_id, name, allow_negative_stock)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(product_id)
    .bind(tenant_id)
    .bind("system-product")
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
        insert into orders
          (tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status, oms_status, review_status, exception_flags, logistics_company, tracking_no, inventory_shortage, version)
        values
          ($1, $2, $3, 'PDD', $4, 'PO-1', 'NEW', 'NEW', 'MANUAL_REVIEW', '[]'::jsonb, $5, $6, $7, 1)
        "#,
    )
    .bind(tenant_id)
    .bind(order_month)
    .bind(order_id)
    .bind(shop_id)
    .bind("SF")
    .bind("SF123456")
    .bind(false)
    .execute(db.pool())
    .await?;
    sqlx::query(
        r#"
        insert into order_index (tenant_id, order_id, order_month, platform, platform_order_id, shop_id)
        values ($1, $2, $3, 'PDD', 'PO-1', $4)
        "#,
    )
    .bind(tenant_id)
    .bind(order_id)
    .bind(order_month)
    .bind(shop_id)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into order_items
          (tenant_id, order_month, order_id, line_id, sku_id, qty)
        values
          ($1, $2, $3, $4, $5, $6)
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

    let jwt = JwtService::new("test-secret".to_string());
    let app = build_app(AppState::new(db.clone(), Arc::new(jwt.clone())));
    let token = build_token(
        &jwt,
        tenant_id,
        uuid::Uuid::new_v4(),
        vec!["OMS.ORDER.REVIEW"],
    );

    let body = serde_json::json!({ "action": "PASS" });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/orders/{order_id}/review"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body)?))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let action: Option<String> =
        sqlx::query_scalar("select action from audit_logs where tenant_id = $1 and entity_id = $2")
            .bind(tenant_id)
            .bind(order_id.to_string())
            .fetch_optional(db.pool())
            .await?;
    assert_eq!(action.as_deref(), Some("OMS.REVIEW_PASS"));

    Ok(())
}

#[tokio::test]
async fn dead_letter_replay_enqueues_outbox() -> anyhow::Result<()> {
    let Some(db) = load_db().await? else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    sqlx::query("insert into tenants (tenant_id, name, status) values ($1, 't-a', 'ACTIVE')")
        .bind(tenant_id)
        .execute(db.pool())
        .await?;

    let dead_letter_id = uuid::Uuid::new_v4();
    let outbox_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into dead_letters
          (dead_letter_id, tenant_id, outbox_id, event_type, payload, error, attempts, classification, next_action, status)
        values
          ($1, $2, $3, 'ORDER_IMPORTED', '{}'::jsonb, 'failed', 3, 'RETRYABLE', 'REPLAY', 'PENDING')
        "#,
    )
    .bind(dead_letter_id)
    .bind(tenant_id)
    .bind(outbox_id)
    .execute(db.pool())
    .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let app = build_app(AppState::new(db.clone(), Arc::new(jwt.clone())));
    let token = build_token(
        &jwt,
        tenant_id,
        uuid::Uuid::new_v4(),
        vec!["SYS.DLQ.REPLAY"],
    );
    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/dead-letters/{dead_letter_id}/replay"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await?.to_bytes();
    let payload: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    let outbox_id = payload
        .get("outbox_id")
        .and_then(|v| v.as_str())
        .expect("outbox_id");

    let count: i64 = sqlx::query_scalar(
        "select count(*) from outbox_events where tenant_id = $1 and outbox_id::text = $2",
    )
    .bind(tenant_id)
    .bind(outbox_id)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(count, 1);

    Ok(())
}

#[tokio::test]
async fn settings_version_conflict_returns_409() -> anyhow::Result<()> {
    let Some(db) = load_db().await? else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    sqlx::query("insert into tenants (tenant_id, name, status) values ($1, 't-a', 'ACTIVE')")
        .bind(tenant_id)
        .execute(db.pool())
        .await?;

    sqlx::query(
        r#"
        insert into settings (tenant_id, scope_type, key, value_json, version)
        values ($1, 'TENANT', 'review.rules', '{}'::jsonb, 2)
        "#,
    )
    .bind(tenant_id)
    .execute(db.pool())
    .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let app = build_app(AppState::new(db.clone(), Arc::new(jwt.clone())));
    let token = build_token(
        &jwt,
        tenant_id,
        uuid::Uuid::new_v4(),
        vec!["SYS.SETTINGS.WRITE"],
    );
    let body = serde_json::json!({
        "scope_type": "TENANT",
        "key": "review.rules",
        "value": {},
        "expected_version": 1
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/settings"))
        .method("PUT")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body)?))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    Ok(())
}

#[tokio::test]
async fn user_data_scopes_roundtrip() -> anyhow::Result<()> {
    let Some(db) = load_db().await? else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    sqlx::query("insert into tenants (tenant_id, name, status) values ($1, 't-a', 'ACTIVE')")
        .bind(tenant_id)
        .execute(db.pool())
        .await?;

    let user_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into users (user_id, tenant_id, email, password_hash, status)
        values ($1, $2, 'user@erp', 'hash', 'ACTIVE')
        "#,
    )
    .bind(user_id)
    .bind(tenant_id)
    .execute(db.pool())
    .await?;

    let shop_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();
    sqlx::query("insert into user_shop_access (tenant_id, user_id, shop_id) values ($1, $2, $3)")
        .bind(tenant_id)
        .bind(user_id)
        .bind(shop_id)
        .execute(db.pool())
        .await?;
    sqlx::query(
        "insert into user_warehouse_access (tenant_id, user_id, warehouse_id) values ($1, $2, $3)",
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(warehouse_id)
    .execute(db.pool())
    .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let app = build_app(AppState::new(db.clone(), Arc::new(jwt.clone())));
    let token = build_token(
        &jwt,
        tenant_id,
        user_id,
        vec!["SYS.USER.READ", "SYS.USER.WRITE"],
    );

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/users/{user_id}/data-scopes"
        ))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response: Response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await?.to_bytes();
    let payload: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    let shops = payload
        .get("shop_ids")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        shops
            .iter()
            .any(|v| v.as_str() == Some(&shop_id.to_string()))
    );

    let body = serde_json::json!({
        "shop_ids": [shop_id.to_string()],
        "warehouse_ids": []
    });
    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/users/{user_id}/data-scopes"
        ))
        .method("PUT")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body)?))
        .unwrap();
    let response: Response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    Ok(())
}
