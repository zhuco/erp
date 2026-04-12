use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
};
use chrono::{Datelike, Utc};
use erp_api::{build_app, AppState, Claims, JwtService};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use serde_json::Value;
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
        create table if not exists platform_tokens (
          tenant_id uuid not null,
          shop_id uuid not null,
          token_status text not null default 'ACTIVE',
          access_token text,
          refresh_token text,
          expires_at timestamptz,
          scopes text,
          last_refreshed_at timestamptz,
          refresh_fail_count int not null default 0,
          last_error text not null default '',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, shop_id)
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
        create table if not exists outbox_events (
          tenant_id uuid not null,
          event_month date not null,
          outbox_id uuid not null default gen_random_uuid(),
          event_type text not null,
          payload jsonb not null,
          status text not null default 'PENDING',
          attempts int not null default 0,
          next_retry_at timestamptz not null default now(),
          last_error text not null default '',
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
        perms: vec!["settings.read".to_string()],
        is_admin: true,
        iat: now,
        exp: now + 3600,
    };
    jwt.encode(&claims).expect("token should encode")
}

#[tokio::test]
async fn shop_auth_flow_smoke() -> anyhow::Result<()> {
    if let Err(err) = shop_auth_flow_smoke_inner().await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping shop_auth_flow_smoke: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn shop_auth_flow_smoke_inner() -> anyhow::Result<()> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(());
    };
    let db = Db::connect_lazy(&database_url)?;
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    sqlx::query("insert into tenants (tenant_id, name, status) values ($1,$2,$3)")
        .bind(tenant_id)
        .bind("test-tenant")
        .bind("ACTIVE")
        .execute(db.pool())
        .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let state = AppState::new(db.clone(), std::sync::Arc::new(jwt.clone()));
    let app = build_app(state);
    let token = build_token(&jwt, &tenant_id.to_string());

    let body = serde_json::json!({
        "platform": "taobao",
        "platform_shop_id": "TB-001",
        "name": "测试店铺"
    });
    let response: Response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/tenants/{tenant_id}/shops"))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string()))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let payload = response.into_body().collect().await?.to_bytes();
    let shop: Value = serde_json::from_slice(&payload)?;
    let shop_id = shop
        .get("shop_id")
        .and_then(Value::as_str)
        .expect("shop_id should exist")
        .to_string();

    let response: Response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/v1/tenants/{tenant_id}/sync-jobs"))
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    let auth_body = serde_json::json!({
        "access_token": "access",
        "refresh_token": "refresh",
        "expires_at": Utc::now().checked_add_signed(chrono::Duration::hours(1)).unwrap().to_rfc3339(),
        "scopes": "orders,shipping"
    });
    let response: Response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/v1/tenants/{tenant_id}/shops/{shop_id}/auth/callback"
                ))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(auth_body.to_string()))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let payload = response.into_body().collect().await?.to_bytes();
    let shop: Value = serde_json::from_slice(&payload)?;
    let token_status = shop
        .get("token_status")
        .and_then(Value::as_object)
        .expect("token_status should exist");
    assert!(token_status
        .get("has_access_token")
        .and_then(Value::as_bool)
        .unwrap_or(false));

    let response: Response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/v1/tenants/{tenant_id}/shops/{shop_id}/tokens/refresh"
                ))
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let payload = response.into_body().collect().await?.to_bytes();
    let resp: Value = serde_json::from_slice(&payload)?;
    assert!(resp.get("outbox_id").is_some());

    let event_month = chrono::NaiveDate::from_ymd_opt(Utc::now().year(), Utc::now().month(), 1)
        .expect("valid date");
    let outbox_count: i64 = sqlx::query_scalar(
        "select count(*) from outbox_events where tenant_id = $1 and event_month = $2",
    )
    .bind(tenant_id)
    .bind(event_month)
    .fetch_one(db.pool())
    .await?;
    assert!(outbox_count >= 1);

    Ok(())
}
