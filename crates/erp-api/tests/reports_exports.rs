use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Datelike, Utc};
use erp_api::{AppState, Claims, JwtService, build_app};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use std::sync::Arc;
use tower::ServiceExt;

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
          total_amount_cent bigint not null default 0,
          currency text not null default 'CNY',
          order_created_at timestamptz not null,
          ship_at timestamptz,
          primary key (tenant_id, order_month, order_id)
        )
        "#,
        r#"
        create table if not exists order_items (
          tenant_id uuid not null,
          order_month date not null,
          order_id uuid not null,
          line_id text not null,
          sku_id text,
          qty int not null default 1,
          amount_cent bigint not null default 0,
          refund_amount_cent bigint,
          created_at timestamptz not null default now(),
          primary key (tenant_id, order_month, order_id, line_id)
        )
        "#,
        r#"
        create table if not exists export_jobs (
          job_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          job_type text not null,
          params_json jsonb not null default '{}'::jsonb,
          status text not null default 'PENDING',
          output_url text,
          progress int not null default 0,
          attempts int not null default 0,
          last_error text not null default '',
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
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}

fn build_token(jwt: &JwtService, tenant_id: &str, perms: Vec<&str>) -> String {
    let now = Utc::now().timestamp() as usize;
    let claims = Claims::bearer(
        uuid::Uuid::new_v4().to_string(),
        tenant_id.to_string(),
        perms.into_iter().map(|p| p.to_string()).collect(),
        false,
        now,
        now + 3600,
    );
    jwt.encode(&claims).expect("token should encode")
}

#[tokio::test]
async fn reports_and_exports_flow() -> anyhow::Result<()> {
    if let Err(err) = reports_and_exports_flow_inner().await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping reports_and_exports_flow: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn reports_and_exports_flow_inner() -> anyhow::Result<()> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(());
    };

    let db = Db::connect(&database_url).await?;
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let shop_id = uuid::Uuid::new_v4();
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

    let now = Utc::now();
    let order_month = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();
    let order_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into orders (
          tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status,
          total_amount_cent, currency, order_created_at, ship_at
        ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
        "#,
    )
    .bind(tenant_id)
    .bind(order_month)
    .bind(order_id)
    .bind("taobao")
    .bind(shop_id)
    .bind("ORDER-REPORT")
    .bind("PENDING_SHIPMENT")
    .bind(1200_i64)
    .bind("CNY")
    .bind(now)
    .bind(now)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into order_items (
          tenant_id, order_month, order_id, line_id, sku_id, qty, amount_cent, refund_amount_cent
        ) values ($1,$2,$3,$4,$5,$6,$7,$8)
        "#,
    )
    .bind(tenant_id)
    .bind(order_month)
    .bind(order_id)
    .bind("LINE-1")
    .bind("SKU-1")
    .bind(1_i32)
    .bind(1200_i64)
    .bind(200_i64)
    .execute(db.pool())
    .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let token = build_token(
        &jwt,
        &tenant_id.to_string(),
        vec!["reports.read", "exports.write", "exports.read"],
    );
    let state = AppState::new(db.clone(), Arc::new(jwt));
    let app = build_app(state);

    let report_uri = format!(
        "/reports/dashboard?start_at={}&end_at={}",
        (now - chrono::Duration::days(1)).to_rfc3339(),
        (now + chrono::Duration::days(1)).to_rfc3339()
    );
    let request = Request::builder()
        .uri(report_uri)
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    let by_platform = json
        .get("by_platform")
        .and_then(|v| v.as_array())
        .expect("by_platform");
    assert_eq!(by_platform.len(), 1);

    let create_body = serde_json::json!({
        "type": "ORDER_EXPORT",
        "params": { "start_at": now.to_rfc3339(), "end_at": now.to_rfc3339() }
    });
    let request = Request::builder()
        .uri("/exports/jobs")
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_body)?))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    let job_id = json
        .get("job_id")
        .and_then(|v| v.as_str())
        .expect("job_id")
        .to_string();

    let request = Request::builder()
        .uri(format!("/exports/jobs/{job_id}"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let audit_count: i64 = sqlx::query_scalar(
        r#"
        select count(*) from audit_logs
        where tenant_id = $1 and action = 'EXPORT.JOB.CREATE'
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(audit_count, 1);

    Ok(())
}
