use anyhow::{Context, Result};
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use chrono::Utc;
use erp_api::{build_app, AppState, Claims, JwtService};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceExt;

mod support;

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
        create table if not exists settings (
          setting_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          scope_type text not null,
          scope_id uuid,
          key text not null,
          value_json jsonb not null default '{}'::jsonb,
          version int not null default 1,
          updated_at timestamptz not null default now(),
          unique (tenant_id, scope_type, scope_id, key)
        )
        "#,
        r#"
        create table if not exists feature_flags (
          tenant_id uuid not null,
          scope text not null,
          shop_id uuid,
          key text not null,
          enabled boolean not null default true,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, scope, shop_id, key)
        )
        "#,
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}

async fn seed_tenant(pool: &sqlx::PgPool) -> Result<uuid::Uuid> {
    let tenant_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .bind("wms-execution-flow-tenant")
    .execute(pool)
    .await?;
    Ok(tenant_id)
}

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
    let json_body = serde_json::from_slice::<Value>(&body_bytes)
        .unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&body_bytes) }));
    Ok((status, json_body))
}

async fn connect_test_db(case_name: &str) -> Option<Db> {
    let database_url = support::test_database_url();
    let probe = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(1))
        .connect(&database_url)
        .await;
    match probe {
        Ok(pool) => {
            drop(pool);
            match Db::connect_lazy(&database_url) {
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

#[tokio::test]
async fn wms_site_execution_flow_integration() -> Result<()> {
    let Some(db) = connect_test_db("wms_site_execution_flow_integration").await else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;
    let tenant = seed_tenant(db.pool()).await?;

    let (app, jwt) = build_state(db);
    let token = build_token(&jwt, &tenant.to_string(), vec!["wms.write"]);

    let (status, labor_list) = request_json(
        &app,
        &token,
        Method::GET,
        format!("/v1/tenants/{tenant}/wms/labor/tasks"),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "list labor tasks failed: {labor_list}"
    );
    let labor_task_id = labor_list
        .get("items")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("task_id"))
        .and_then(Value::as_str)
        .context("labor task_id missing")?
        .to_string();

    let (status, labor_assigned) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant}/wms/labor/tasks/{labor_task_id}/assign"),
        Some(json!({ "assignee": "worker-a" })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "assign labor task failed: {labor_assigned}"
    );
    assert_eq!(
        labor_assigned.get("status").and_then(Value::as_str),
        Some("ASSIGNED")
    );
    assert_eq!(
        labor_assigned.get("assignee").and_then(Value::as_str),
        Some("worker-a")
    );

    let (status, labor_started) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant}/wms/labor/tasks/{labor_task_id}/start"),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "start labor task failed: {labor_started}"
    );
    assert_eq!(
        labor_started.get("status").and_then(Value::as_str),
        Some("IN_PROGRESS")
    );

    let (status, labor_completed) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant}/wms/labor/tasks/{labor_task_id}/complete"),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "complete labor task failed: {labor_completed}"
    );
    assert_eq!(
        labor_completed.get("status").and_then(Value::as_str),
        Some("DONE")
    );

    let (status, slotting_list) = request_json(
        &app,
        &token,
        Method::GET,
        format!("/v1/tenants/{tenant}/wms/slotting/suggestions"),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "list slotting suggestions failed: {slotting_list}"
    );
    let suggestion_id = slotting_list
        .get("items")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("suggestion_id"))
        .and_then(Value::as_str)
        .context("slotting suggestion_id missing")?
        .to_string();

    let (status, slotting_applied) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant}/wms/slotting/suggestions/{suggestion_id}/apply"),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "apply slotting suggestion failed: {slotting_applied}"
    );
    assert_eq!(
        slotting_applied.get("status").and_then(Value::as_str),
        Some("APPLIED")
    );

    let (status, replenishment_list) = request_json(
        &app,
        &token,
        Method::GET,
        format!("/v1/tenants/{tenant}/wms/replenishment/tasks"),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "list replenishment tasks failed: {replenishment_list}"
    );
    let replenishment_task_id = replenishment_list
        .get("items")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("task_id"))
        .and_then(Value::as_str)
        .context("replenishment task_id missing")?
        .to_string();

    let (status, replenishment_claimed) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant}/wms/replenishment/tasks/{replenishment_task_id}/claim"),
        Some(json!({ "assignee": "worker-b" })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "claim replenishment task failed: {replenishment_claimed}"
    );
    assert_eq!(
        replenishment_claimed.get("status").and_then(Value::as_str),
        Some("IN_PROGRESS")
    );
    assert_eq!(
        replenishment_claimed
            .get("assignee")
            .and_then(Value::as_str),
        Some("worker-b")
    );

    let (status, replenishment_completed) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant}/wms/replenishment/tasks/{replenishment_task_id}/complete"),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "complete replenishment task failed: {replenishment_completed}"
    );
    assert_eq!(
        replenishment_completed
            .get("status")
            .and_then(Value::as_str),
        Some("DONE")
    );

    let (status, dock_list) = request_json(
        &app,
        &token,
        Method::GET,
        format!("/v1/tenants/{tenant}/wms/dock/appointments"),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "list dock appointments failed: {dock_list}"
    );
    let appointment_id = dock_list
        .get("items")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("appointment_id"))
        .and_then(Value::as_str)
        .context("dock appointment_id missing")?
        .to_string();

    let (status, dock_checked_in) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant}/wms/dock/appointments/{appointment_id}/check-in"),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "check-in dock appointment failed: {dock_checked_in}"
    );
    assert_eq!(
        dock_checked_in.get("status").and_then(Value::as_str),
        Some("CHECKED_IN")
    );

    let (status, dock_loading) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant}/wms/dock/appointments/{appointment_id}/start-loading"),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "start-loading dock appointment failed: {dock_loading}"
    );
    assert_eq!(
        dock_loading.get("status").and_then(Value::as_str),
        Some("LOADING")
    );

    let (status, dock_departed) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant}/wms/dock/appointments/{appointment_id}/depart"),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "depart dock appointment failed: {dock_departed}"
    );
    assert_eq!(
        dock_departed.get("status").and_then(Value::as_str),
        Some("DEPARTED")
    );

    Ok(())
}
