use axum::{
    body::Body,
    http::{Request, StatusCode},
};
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
        create unique index if not exists idx_settings_scope_unique
        on settings (tenant_id, scope_type, scope_id, key)
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
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims::bearer(
        user_id.to_string(),
        tenant_id.to_string(),
        perms.into_iter().map(|perm| perm.to_string()).collect(),
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

fn sample_integrations_payload() -> serde_json::Value {
    serde_json::json!({
        "configs": [
            {
                "integration_id": "INT-OMS-99",
                "name": "Order Hub",
                "category": "Order",
                "provider": "OMS Adapter",
                "status": "CONNECTED",
                "success_rate": 99.2,
                "retry_policy": "retry 3 times / 15 minutes",
                "alert_channel": "wechat-integration-alert",
                "last_checked_at": "2026-04-08T10:20:00Z",
                "updated_at": "2026-04-08T10:21:00Z",
                "description": "Unified order ingestion"
            }
        ],
        "changes": [
            {
                "record_id": "CHG-9901",
                "integration_id": "INT-OMS-99",
                "action": "update_config",
                "operator": "system_admin",
                "comment": "raise retry policy",
                "created_at": "2026-04-08T10:22:00Z"
            }
        ]
    })
}

#[tokio::test]
async fn system_integrations_require_permissions() {
    let database_url = support::test_database_url();
    let db = Db::connect_lazy(&database_url).expect("lazy db should be constructible");
    let jwt = JwtService::new("test-secret".to_string());
    let app = build_app(AppState::new(db, Arc::new(jwt.clone())));

    let tenant_id = uuid::Uuid::new_v4();
    let user_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, tenant_id, user_id, vec![]);

    let get_request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/system/integrations"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let get_response = app.clone().oneshot(get_request).await.unwrap();
    assert_eq!(get_response.status(), StatusCode::FORBIDDEN);

    let put_request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/system/integrations"))
        .method("PUT")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(sample_integrations_payload().to_string()))
        .unwrap();
    let put_response = app.oneshot(put_request).await.unwrap();
    assert_eq!(put_response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn system_integrations_roundtrip_defaults_and_saved_catalog() -> anyhow::Result<()> {
    let Some(db) = load_db().await? else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    sqlx::query("insert into tenants (tenant_id, name, status) values ($1, 't-a', 'ACTIVE')")
        .bind(tenant_id)
        .execute(db.pool())
        .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let app = build_app(AppState::new(db.clone(), Arc::new(jwt.clone())));
    let read_token = build_token(
        &jwt,
        tenant_id,
        uuid::Uuid::new_v4(),
        vec!["SYS.SETTINGS.READ"],
    );
    let write_token = build_token(
        &jwt,
        tenant_id,
        uuid::Uuid::new_v4(),
        vec!["SYS.SETTINGS.WRITE"],
    );

    let get_request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/system/integrations"))
        .method("GET")
        .header("authorization", format!("Bearer {read_token}"))
        .body(Body::empty())
        .unwrap();
    let get_response = app.clone().oneshot(get_request).await.unwrap();
    assert_eq!(get_response.status(), StatusCode::OK);
    let default_body = get_response.into_body().collect().await?.to_bytes();
    let default_payload: serde_json::Value = serde_json::from_slice(&default_body)?;
    let default_configs = default_payload
        .get("configs")
        .and_then(|value| value.as_array())
        .expect("default configs should be an array");
    let default_changes = default_payload
        .get("changes")
        .and_then(|value| value.as_array())
        .expect("default changes should be an array");
    assert_eq!(default_configs.len(), 2);
    assert_eq!(default_changes.len(), 2);
    assert_eq!(
        default_configs[0]
            .get("integration_id")
            .and_then(|value| value.as_str()),
        Some("INT-EC-01")
    );
    assert_eq!(
        default_changes[0]
            .get("record_id")
            .and_then(|value| value.as_str()),
        Some("CHG-1001")
    );

    let expected_payload = sample_integrations_payload();
    let put_request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/system/integrations"))
        .method("PUT")
        .header("authorization", format!("Bearer {write_token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&expected_payload)?))
        .unwrap();
    let put_response = app.clone().oneshot(put_request).await.unwrap();
    assert_eq!(put_response.status(), StatusCode::OK);
    let put_body = put_response.into_body().collect().await?.to_bytes();
    let put_payload: serde_json::Value = serde_json::from_slice(&put_body)?;
    assert_eq!(put_payload, expected_payload);

    let get_request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/system/integrations"))
        .method("GET")
        .header("authorization", format!("Bearer {read_token}"))
        .body(Body::empty())
        .unwrap();
    let get_response = app.clone().oneshot(get_request).await.unwrap();
    assert_eq!(get_response.status(), StatusCode::OK);
    let body = get_response.into_body().collect().await?.to_bytes();
    let payload: serde_json::Value = serde_json::from_slice(&body)?;
    assert_eq!(payload, expected_payload);

    let stored_value: serde_json::Value = sqlx::query_scalar(
        "select value_json from settings where tenant_id = $1 and scope_type = 'TENANT' and key = 'system.integrations.catalog'",
    )
    .bind(tenant_id)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(stored_value, expected_payload);

    Ok(())
}
