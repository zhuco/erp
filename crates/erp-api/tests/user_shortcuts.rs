use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use erp_api::{AppState, Claims, JwtService, build_app};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use tower::ServiceExt;

mod support;

async fn setup_schema(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    let statements = [
        "create extension if not exists pgcrypto",
        r#"
        create table if not exists user_shortcuts (
          tenant_id uuid not null,
          user_id uuid not null,
          shortcuts_json jsonb not null default '[]'::jsonb,
          updated_at timestamptz not null default now(),
          created_at timestamptz not null default now(),
          primary key (tenant_id, user_id)
        )
        "#,
    ];
    for statement in statements {
        sqlx::query(statement).execute(pool).await?;
    }
    Ok(())
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

fn build_token(jwt: &JwtService, tenant_id: uuid::Uuid, user_id: uuid::Uuid) -> String {
    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        perms: Vec::new(),
        is_admin: true,
        iat: now,
        exp: now + 3600,
    };
    jwt.encode(&claims).expect("token should encode")
}

#[tokio::test]
async fn user_shortcuts_roundtrip() -> anyhow::Result<()> {
    let Some(db) = load_db().await? else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let user_id = uuid::Uuid::new_v4();
    let jwt = JwtService::new("test-secret".to_string());
    let app = build_app(AppState::new(db, std::sync::Arc::new(jwt.clone())));

    let token = build_token(&jwt, tenant_id, user_id);
    let payload = serde_json::json!({
        "shortcuts": [
            { "path": "/orders", "name": "订单" },
            { "path": "/shipments", "name": "发货/包裹" }
        ]
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/me/shortcuts"))
        .method("PUT")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await?.to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body)?;
    assert_eq!(json["shortcuts"].as_array().map(|v| v.len()), Some(2));

    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/me/shortcuts"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await?.to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body)?;
    assert_eq!(json["shortcuts"].as_array().map(|v| v.len()), Some(2));

    Ok(())
}
