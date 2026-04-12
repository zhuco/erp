use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use erp_api::{build_app, AppState, Claims, JwtService};
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
    let claims = Claims {
        sub: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        perms: perms.into_iter().map(|perm| perm.to_string()).collect(),
        is_admin: false,
        iat: now,
        exp: now + 3600,
    };
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

fn sample_audit_reports_payload() -> serde_json::Value {
    serde_json::json!({
        "reports": [
            {
                "report_id": "REP-ORDER-99",
                "module": "订单",
                "period": "2026-03",
                "owner": "审计专员",
                "anomalies": 8,
                "status": "READY",
                "generated_at": "2026-04-08T10:00:00Z"
            }
        ],
        "rules": [
            {
                "rule_id": "RULE-9001",
                "module": "订单",
                "name": "超大额订单告警",
                "condition": "订单金额 > 20 万元",
                "enabled": true,
                "updated_at": "2026-04-08T10:02:00Z"
            }
        ]
    })
}

#[tokio::test]
async fn system_audit_reports_require_permissions() {
    let database_url = support::test_database_url();
    let db = Db::connect_lazy(&database_url).expect("lazy db should be constructible");
    let jwt = JwtService::new("test-secret".to_string());
    let app = build_app(AppState::new(db, Arc::new(jwt.clone())));

    let tenant_id = uuid::Uuid::new_v4();
    let user_id = uuid::Uuid::new_v4();
    let token = build_token(&jwt, tenant_id, user_id, vec![]);

    let get_request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/system/audit-reports"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let get_response = app.clone().oneshot(get_request).await.unwrap();
    assert_eq!(get_response.status(), StatusCode::FORBIDDEN);

    let put_request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/system/audit-reports"))
        .method("PUT")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(sample_audit_reports_payload().to_string()))
        .unwrap();
    let put_response = app.oneshot(put_request).await.unwrap();
    assert_eq!(put_response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn system_audit_reports_roundtrip_defaults_and_saved_catalog() -> anyhow::Result<()> {
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
        .uri(format!("/v1/tenants/{tenant_id}/system/audit-reports"))
        .method("GET")
        .header("authorization", format!("Bearer {read_token}"))
        .body(Body::empty())
        .unwrap();
    let get_response = app.clone().oneshot(get_request).await.unwrap();
    assert_eq!(get_response.status(), StatusCode::OK);
    let default_body = get_response.into_body().collect().await?.to_bytes();
    let default_payload: serde_json::Value = serde_json::from_slice(&default_body)?;
    let default_reports = default_payload
        .get("reports")
        .and_then(|value| value.as_array())
        .expect("default reports should be an array");
    let default_rules = default_payload
        .get("rules")
        .and_then(|value| value.as_array())
        .expect("default rules should be an array");
    assert_eq!(default_reports.len(), 3);
    assert_eq!(default_rules.len(), 2);
    assert_eq!(
        default_reports[0]
            .get("report_id")
            .and_then(|value| value.as_str()),
        Some("REP-ORDER-01")
    );
    assert_eq!(
        default_rules[0]
            .get("rule_id")
            .and_then(|value| value.as_str()),
        Some("RULE-001")
    );

    let expected_payload = sample_audit_reports_payload();
    let put_request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/system/audit-reports"))
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
        .uri(format!("/v1/tenants/{tenant_id}/system/audit-reports"))
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
        "select value_json from settings where tenant_id = $1 and scope_type = 'TENANT' and key = 'system.audit-reports.catalog'",
    )
    .bind(tenant_id)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(stored_value, expected_payload);

    Ok(())
}
