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
) -> Result<(StatusCode, serde_json::Value)> {
    let request = Request::builder()
        .uri(uri)
        .method(method)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())?;

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
        create table if not exists after_sales_sla_rules (
          rule_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          after_sales_type text not null,
          response_hours int not null default 4,
          resolution_hours int not null default 24,
          escalation_role text,
          status text not null default 'ACTIVE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
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
async fn after_sales_sla_escalation_marks_overdue_case() -> Result<()> {
    let Some(db) = connect_test_db("after_sales_sla_escalation_marks_overdue_case").await else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let after_sales_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into tenants(tenant_id, name, status)
        values ($1, 'sla-tenant', 'ACTIVE')
        on conflict (tenant_id) do nothing
        "#,
    )
    .bind(tenant_id)
    .execute(db.pool())
    .await?;
    sqlx::query(
        r#"
        insert into after_sales(
          id, tenant_id, order_id, type, status, after_sale_status, amount_cent, raw_json, created_at
        ) values ($1, $2, $3, 'EXCHANGE', 'OPEN', 'WAIT_SELLER_AGREE', 9900, '{}'::jsonb, now() - interval '48 hours')
        "#,
    )
    .bind(after_sales_id)
    .bind(tenant_id)
    .bind(order_id)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into after_sales_sla_rules(
          tenant_id, after_sales_type, response_hours, resolution_hours, escalation_role, status
        ) values ($1, 'EXCHANGE', 1, 2, 'SUPPORT_L2', 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .execute(db.pool())
    .await?;

    let (app, jwt) = build_state(db);
    let token = build_token(&jwt, &tenant_id.to_string(), vec!["aftersales.read"]);

    let (status, body) = request_json(
        &app,
        &token,
        Method::GET,
        format!("/v1/tenants/{tenant_id}/after-sales?limit=20&offset=0"),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "list failed: {body}");

    let items = body
        .get("items")
        .and_then(|v| v.as_array())
        .expect("items array");
    let target = items
        .iter()
        .find(|item| {
            item.get("id")
                .and_then(|v| v.as_str())
                .map(|v| v == after_sales_id.to_string())
                .unwrap_or(false)
        })
        .expect("target after sales item");
    assert_eq!(
        target.get("sla_status").and_then(|v| v.as_str()),
        Some("OVERDUE")
    );
    assert_eq!(
        target.get("escalation_role").and_then(|v| v.as_str()),
        Some("SUPPORT_L2")
    );
    assert!(target
        .get("sla_remaining_minutes")
        .and_then(|v| v.as_i64())
        .map(|v| v < 0)
        .unwrap_or(false));

    Ok(())
}
