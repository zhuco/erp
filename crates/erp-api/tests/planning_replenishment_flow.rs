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
use std::time::Duration as StdDuration;
use tower::ServiceExt;

mod support;

async fn setup_schema(pool: &sqlx::PgPool) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        r#"
        select pg_advisory_xact_lock(hashtext('planning_replenishment_flow_setup_schema'))
        "#,
    )
    .execute(&mut *tx)
    .await?;

    let statements = [
        r#"
        do $$
        begin
          create extension if not exists pgcrypto;
        exception
          when duplicate_object then null;
        end $$;
        "#,
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
        create unique index if not exists idx_test_settings_unique
          on settings(
            tenant_id,
            scope_type,
            coalesce(scope_id, '00000000-0000-0000-0000-000000000000'::uuid),
            key
          )
        "#,
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(&mut *tx).await?;
    }
    tx.commit().await?;
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
    .bind("planning-flow-test-tenant")
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
        .acquire_timeout(StdDuration::from_secs(1))
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
async fn planning_routes_validate_path_before_db_access() {
    let database_url = support::test_database_url();
    let db = Db::connect_lazy(&database_url).expect("lazy db should be constructible");
    let (app, jwt) = build_state(db);
    let token = build_token(
        &jwt,
        &uuid::Uuid::new_v4().to_string(),
        vec!["inventory.read", "inventory.write"],
    );

    let cases = [("GET", "/v1/tenants/not-a-uuid/planning/forecasts", "{}")];

    for (method, uri, body) in cases {
        let request = Request::builder()
            .uri(uri)
            .method(method)
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "route {method} {uri} should exist and validate tenant path param before DB access"
        );
    }
}

#[tokio::test]
async fn planning_replenishment_purchase_transfer_and_promo_flow() -> Result<()> {
    let Some(db) = connect_test_db("planning_replenishment_purchase_transfer_and_promo_flow").await
    else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;
    let tenant_id = seed_tenant(db.pool()).await?;

    let (app, jwt) = build_state(db);
    let token = build_token(
        &jwt,
        &tenant_id.to_string(),
        vec!["inventory.read", "inventory.write"],
    );

    let warehouse_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();

    let (status, upsert_forecast) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant_id}/planning/forecasts"),
        Some(json!({
          "sku_id": sku_id,
          "sku_code": "SKU-PLAN-001",
          "warehouse_id": warehouse_id,
          "forecast_qty": 520,
          "model": "SEASONAL",
          "confidence": "HIGH",
          "version": "2026-W15"
        })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "upsert planning forecast failed: {upsert_forecast}"
    );

    let (status, list_forecasts) = request_json(
        &app,
        &token,
        Method::GET,
        format!("/v1/tenants/{tenant_id}/planning/forecasts"),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "list planning forecasts failed: {list_forecasts}"
    );
    assert_eq!(
        list_forecasts
            .get("items")
            .and_then(Value::as_array)
            .map(|items| items.len()),
        Some(1)
    );

    let (status, upsert_rule) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant_id}/planning/replenishment-rules"),
        Some(json!({
          "warehouse_id": warehouse_id,
          "category": "BASIC",
          "safety_stock_days": 15,
          "replenishment_cycle_days": 7,
          "min_replenishment_qty": 120,
          "status": "ACTIVE"
        })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "upsert replenishment rule failed: {upsert_rule}"
    );
    let rule_id = upsert_rule
        .get("rule_id")
        .and_then(Value::as_str)
        .context("rule_id missing")?
        .to_string();

    let (status, generated_purchase_plans) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant_id}/planning/purchase-plans/generate"),
        Some(json!({
          "trigger": "REPLENISHMENT"
        })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "generate purchase plans failed: {generated_purchase_plans}"
    );
    let first_purchase_plan = generated_purchase_plans
        .get("items")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .context("generated purchase plan missing")?;
    assert_eq!(
        first_purchase_plan
            .get("source_rule_id")
            .and_then(Value::as_str),
        Some(rule_id.as_str())
    );
    assert_eq!(
        first_purchase_plan.get("status").and_then(Value::as_str),
        Some("PENDING_APPROVAL")
    );
    assert!(
        first_purchase_plan
            .get("total_qty")
            .and_then(Value::as_i64)
            .unwrap_or_default()
            >= 120,
        "generated purchase qty should honor replenishment lower bound"
    );

    let (status, purchase_plans) = request_json(
        &app,
        &token,
        Method::GET,
        format!("/v1/tenants/{tenant_id}/planning/purchase-plans"),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "list purchase plans failed: {purchase_plans}"
    );
    assert!(
        purchase_plans
            .get("total")
            .and_then(Value::as_i64)
            .unwrap_or_default()
            >= 1,
        "purchase plan should be created"
    );

    let (status, generated_transfers) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant_id}/planning/transfer-suggestions/generate"),
        Some(json!({
          "strategy": "BALANCE"
        })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "generate transfer suggestions failed: {generated_transfers}"
    );
    assert!(
        generated_transfers
            .get("total")
            .and_then(Value::as_i64)
            .unwrap_or_default()
            >= 1,
        "transfer suggestions should be generated"
    );
    let first_transfer = generated_transfers
        .get("items")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .context("generated transfer suggestion missing")?;
    assert_eq!(
        first_transfer.get("reason").and_then(Value::as_str),
        Some("AUTO_BALANCE")
    );

    let (status, generated_promo_stocking) = request_json(
        &app,
        &token,
        Method::POST,
        format!("/v1/tenants/{tenant_id}/planning/promo-stocking/generate"),
        Some(json!({
          "campaign_name": "618 Campaign",
          "period_start": "2026-06-01",
          "period_end": "2026-06-20"
        })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "generate promo stocking failed: {generated_promo_stocking}"
    );
    assert!(
        generated_promo_stocking
            .get("campaign_id")
            .and_then(Value::as_str)
            .is_some(),
        "campaign_id should be generated"
    );

    let (status, executions) = request_json(
        &app,
        &token,
        Method::GET,
        format!("/v1/tenants/{tenant_id}/planning/plan-executions"),
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "list plan executions failed: {executions}"
    );
    assert!(
        executions
            .get("total")
            .and_then(Value::as_i64)
            .unwrap_or_default()
            >= 1,
        "execution rows should be available"
    );
    let execution_types = executions
        .get("items")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|row| row.get("plan_type").and_then(Value::as_str))
                .collect::<std::collections::HashSet<_>>()
        })
        .unwrap_or_default();
    assert!(
        execution_types.contains("PURCHASE_PLAN"),
        "plan executions should include PURCHASE_PLAN"
    );
    assert!(
        execution_types.contains("TRANSFER_SUGGESTION"),
        "plan executions should include TRANSFER_SUGGESTION"
    );
    assert!(
        execution_types.contains("PROMO_STOCKING"),
        "plan executions should include PROMO_STOCKING"
    );

    Ok(())
}

#[tokio::test]
async fn planning_permissions_and_validation_failures() -> Result<()> {
    let Some(db) = connect_test_db("planning_permissions_and_validation_failures").await else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;
    let tenant_id = seed_tenant(db.pool()).await?;

    let (app, jwt) = build_state(db);
    let readonly_token = build_token(&jwt, &tenant_id.to_string(), vec!["inventory.read"]);

    let (status, forbidden_generate) = request_json(
        &app,
        &readonly_token,
        Method::POST,
        format!("/v1/tenants/{tenant_id}/planning/purchase-plans/generate"),
        Some(json!({
          "trigger": "REPLENISHMENT"
        })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "missing inventory.write should be forbidden: {forbidden_generate}"
    );

    let write_token = build_token(
        &jwt,
        &tenant_id.to_string(),
        vec!["inventory.read", "inventory.write"],
    );
    let (status, bad_rule) = request_json(
        &app,
        &write_token,
        Method::POST,
        format!("/v1/tenants/{tenant_id}/planning/replenishment-rules"),
        Some(json!({
          "warehouse_id": uuid::Uuid::new_v4(),
          "category": "",
          "safety_stock_days": 0,
          "replenishment_cycle_days": 7,
          "min_replenishment_qty": 120,
          "status": "ACTIVE"
        })),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "invalid replenishment rule should be rejected: {bad_rule}"
    );

    Ok(())
}
