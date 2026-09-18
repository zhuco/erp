use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Datelike, NaiveDate, TimeZone, Utc};
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
        create table if not exists shops (
          shop_id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          platform text not null,
          name text not null,
          status text not null default 'ACTIVE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists orders (
          tenant_id uuid not null references tenants(tenant_id),
          order_month date not null,
          order_id uuid not null,
          platform text not null,
          shop_id uuid not null references shops(shop_id),
          platform_order_id text not null,
          status text not null,
          oms_status text not null default 'NEW',
          order_status text not null default 'PENDING_REVIEW',
          fulfillment_status text not null default 'NOT_ALLOCATED',
          exception_status text not null default 'NONE',
          review_status text not null default 'NOT_REVIEWED',
          exception_flags jsonb not null default '[]'::jsonb,
          total_amount_cent bigint not null default 0,
          discount_amount_cent bigint not null default 0,
          paid_amount_cent bigint,
          currency text not null default 'CNY',
          order_created_at timestamptz not null,
          primary key (tenant_id, order_month, order_id)
        )
        "#,
        r#"
        create table if not exists order_key_index (
          tenant_id uuid not null references tenants(tenant_id),
          platform text not null,
          platform_order_id text not null,
          order_id uuid not null,
          order_month date not null,
          shop_id uuid not null references shops(shop_id),
          order_created_at timestamptz not null,
          updated_at timestamptz not null default now(),
          primary key (tenant_id, platform, platform_order_id)
        )
        "#,
        r#"
        create table if not exists order_index (
          tenant_id uuid not null references tenants(tenant_id),
          order_id uuid not null,
          order_month date not null,
          platform text not null,
          platform_order_id text not null,
          shop_id uuid not null references shops(shop_id),
          order_created_at timestamptz not null,
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_id)
        )
        "#,
        r#"
        create table if not exists platform_statements (
          statement_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          platform text not null,
          shop_id uuid not null references shops(shop_id),
          period_start date not null,
          period_end date not null,
          status text not null default 'IMPORTED',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, platform, shop_id, period_start, period_end)
        )
        "#,
        r#"
        create table if not exists platform_statement_lines (
          tenant_id uuid not null references tenants(tenant_id),
          statement_id uuid not null references platform_statements(statement_id) on delete cascade,
          platform_order_id text not null,
          gross_cent bigint not null default 0,
          fee_cent bigint not null default 0,
          refund_cent bigint not null default 0,
          net_cent bigint not null default 0,
          occurred_at timestamptz,
          raw_json jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          primary key (tenant_id, statement_id, platform_order_id)
        )
        "#,
        r#"
        create table if not exists payment_statements (
          statement_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          channel text not null,
          period_start date,
          period_end date,
          status text not null default 'IMPORTED',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, channel, period_start, period_end)
        )
        "#,
        r#"
        create table if not exists payment_statement_lines (
          tenant_id uuid not null references tenants(tenant_id),
          statement_id uuid not null references payment_statements(statement_id) on delete cascade,
          platform_order_id text not null,
          paid_cent bigint not null default 0,
          fee_cent bigint not null default 0,
          net_cent bigint not null default 0,
          occurred_at timestamptz,
          raw_json jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          primary key (tenant_id, statement_id, platform_order_id)
        )
        "#,
        r#"
        create table if not exists reconcile_jobs (
          job_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          job_type text not null,
          status text not null,
          started_at timestamptz,
          finished_at timestamptz,
          last_error text,
          created_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists reconcile_job_items (
          tenant_id uuid not null references tenants(tenant_id),
          job_id uuid not null references reconcile_jobs(job_id) on delete cascade,
          ref_type text not null,
          ref_id text not null,
          expected_cent bigint not null default 0,
          actual_cent bigint not null default 0,
          diff_cent bigint not null default 0,
          diff_reason text,
          raw_json jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          primary key (tenant_id, job_id, ref_type, ref_id)
        )
        "#,
        r#"
        create table if not exists payment_reconcile_jobs (
          job_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          statement_id uuid references payment_statements(statement_id) on delete set null,
          status text not null,
          diff_count bigint not null default 0,
          started_at timestamptz,
          finished_at timestamptz,
          last_error text,
          created_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists payment_reconcile_items (
          tenant_id uuid not null references tenants(tenant_id),
          job_id uuid not null references payment_reconcile_jobs(job_id) on delete cascade,
          platform_order_id text not null,
          order_id uuid,
          expected_cent bigint not null default 0,
          actual_cent bigint not null default 0,
          diff_cent bigint not null default 0,
          diff_reason text,
          raw_json jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          primary key (tenant_id, job_id, platform_order_id)
        )
        "#,
        r#"
        create table if not exists after_sales (
          id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          order_id uuid not null,
          type text not null default 'REFUND',
          status text not null default 'OPEN',
          after_sale_status text not null default 'SUBMITTED',
          amount_cent bigint not null default 0,
          raw_json jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists refund_docs (
          id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          after_sales_id uuid not null,
          amount_cent bigint not null default 0,
          status text not null,
          refund_status text not null,
          channel text not null,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, after_sales_id)
        )
        "#,
        r#"
        create table if not exists outbox_events (
          tenant_id uuid not null references tenants(tenant_id),
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
        create table if not exists audit_logs (
          audit_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
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
    let claims = Claims {
        sub: uuid::Uuid::new_v4().to_string(),
        tenant_id: tenant_id.to_string(),
        perms: perms.into_iter().map(|p| p.to_string()).collect(),
        is_admin: false,
        iat: now,
        exp: now + 3600,
    };
    jwt.encode(&claims).expect("token should encode")
}

#[tokio::test]
async fn finance_reconcile_flow() -> anyhow::Result<()> {
    if let Err(err) = finance_reconcile_flow_inner().await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping finance_reconcile_flow: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn finance_reconcile_flow_inner() -> anyhow::Result<()> {
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

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, name, status)
        values ($1, $2, $3, $4, 'ACTIVE')
        "#,
    )
    .bind(shop_id)
    .bind(tenant_id)
    .bind("taobao")
    .bind("test-shop")
    .execute(db.pool())
    .await?;

    let now = Utc::now();
    let order_month = NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();
    let order_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into orders (
          tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status,
          total_amount_cent, discount_amount_cent, paid_amount_cent, currency, order_created_at
        ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
        "#,
    )
    .bind(tenant_id)
    .bind(order_month)
    .bind(order_id)
    .bind("taobao")
    .bind(shop_id)
    .bind("PLAT-ORDER-1")
    .bind("PENDING")
    .bind(1000_i64)
    .bind(0_i64)
    .bind(1000_i64)
    .bind("CNY")
    .bind(now)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into order_key_index (
          tenant_id, platform, platform_order_id, order_id, order_month, shop_id, order_created_at
        ) values ($1,$2,$3,$4,$5,$6,$7)
        "#,
    )
    .bind(tenant_id)
    .bind("taobao")
    .bind("PLAT-ORDER-1")
    .bind(order_id)
    .bind(order_month)
    .bind(shop_id)
    .bind(now)
    .execute(db.pool())
    .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let token = build_token(
        &jwt,
        &tenant_id.to_string(),
        vec!["finance.write", "finance.read"],
    );
    let state = AppState::new(db.clone(), Arc::new(jwt));
    let app = build_app(state);

    let import_body = serde_json::json!({
        "statement_type": "PLATFORM",
        "platform": "taobao",
        "shop_id": shop_id.to_string(),
        "period_start": now.date_naive().to_string(),
        "period_end": now.date_naive().to_string(),
        "lines": [
            {
                "platform_order_id": "PLAT-ORDER-1",
                "gross_cent": 1000,
                "fee_cent": 0,
                "refund_cent": 0,
                "net_cent": 900
            }
        ]
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/fin/statements/import"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&import_body)?))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    let statement_id = json
        .get("statement_id")
        .and_then(|v| v.as_str())
        .expect("statement_id");

    let run_body = serde_json::json!({
        "job_type": "PLATFORM",
        "statement_id": statement_id
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/fin/reconcile/run"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&run_body)?))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    assert_eq!(json.get("diff_count").and_then(|v| v.as_i64()), Some(1));
    let job_id = json.get("job_id").and_then(|v| v.as_str()).expect("job_id");

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/fin/reconcile/items?job_id={job_id}&diff_only=true"
        ))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    let items = json.get("items").and_then(|v| v.as_array()).expect("items");
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0].get("diff_reason").and_then(|v| v.as_str()),
        Some("AMOUNT_MISMATCH")
    );

    let outbox_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from outbox_events
        where tenant_id = $1 and event_type = 'PROFIT_COMPUTE_REQUESTED'
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(outbox_count, 1);

    Ok(())
}

#[tokio::test]
async fn payment_reconcile_flow() -> anyhow::Result<()> {
    if let Err(err) = payment_reconcile_flow_inner().await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping payment_reconcile_flow: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn payment_reconcile_flow_inner() -> anyhow::Result<()> {
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

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, name, status)
        values ($1, $2, $3, $4, 'ACTIVE')
        "#,
    )
    .bind(shop_id)
    .bind(tenant_id)
    .bind("alipay")
    .bind("test-shop")
    .execute(db.pool())
    .await?;

    let now = Utc::now();
    let order_month = NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();
    let order_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into orders (
          tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status,
          total_amount_cent, discount_amount_cent, paid_amount_cent, currency, order_created_at
        ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
        "#,
    )
    .bind(tenant_id)
    .bind(order_month)
    .bind(order_id)
    .bind("alipay")
    .bind(shop_id)
    .bind("PAY-ORDER-1")
    .bind("PENDING")
    .bind(1000_i64)
    .bind(0_i64)
    .bind(1000_i64)
    .bind("CNY")
    .bind(now)
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
    .bind("alipay")
    .bind("PAY-ORDER-1")
    .bind(shop_id)
    .bind(now)
    .execute(db.pool())
    .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let token = build_token(
        &jwt,
        &tenant_id.to_string(),
        vec!["finance.write", "finance.read"],
    );
    let state = AppState::new(db.clone(), Arc::new(jwt));
    let app = build_app(state);

    let import_body = serde_json::json!({
        "statement_type": "PAYMENT",
        "channel": "alipay",
        "period_start": now.date_naive().to_string(),
        "period_end": now.date_naive().to_string(),
        "payment_lines": [
            {
                "platform_order_id": "PAY-ORDER-1",
                "paid_cent": 900,
                "fee_cent": 0,
                "net_cent": 900
            }
        ]
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/fin/statements/import"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&import_body)?))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    let statement_id = json
        .get("statement_id")
        .and_then(|v| v.as_str())
        .expect("statement_id");

    let run_body = serde_json::json!({
        "statement_id": statement_id
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/fin/payment-reconcile"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&run_body)?))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    assert_eq!(json.get("diff_count").and_then(|v| v.as_i64()), Some(1));
    let job_id = json.get("job_id").and_then(|v| v.as_str()).expect("job_id");

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/fin/payment-reconcile/items?job_id={job_id}&diff_only=true"
        ))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    let items = json.get("items").and_then(|v| v.as_array()).expect("items");
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0].get("diff_reason").and_then(|v| v.as_str()),
        Some("AMOUNT_MISMATCH")
    );

    Ok(())
}

#[tokio::test]
async fn payment_reconcile_cross_period_partial_refund_delta_flow() -> anyhow::Result<()> {
    if let Err(err) = payment_reconcile_cross_period_partial_refund_delta_flow_inner().await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!(
                "skipping payment_reconcile_cross_period_partial_refund_delta_flow: {message}"
            );
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn payment_reconcile_cross_period_partial_refund_delta_flow_inner() -> anyhow::Result<()> {
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

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, name, status)
        values ($1, $2, $3, $4, 'ACTIVE')
        "#,
    )
    .bind(shop_id)
    .bind(tenant_id)
    .bind("alipay")
    .bind("test-shop")
    .execute(db.pool())
    .await?;

    let order_created_at = Utc.with_ymd_and_hms(2026, 1, 10, 8, 0, 0).unwrap();
    let order_month = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    let order_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into orders (
          tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status,
          total_amount_cent, discount_amount_cent, paid_amount_cent, currency, order_created_at
        ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
        "#,
    )
    .bind(tenant_id)
    .bind(order_month)
    .bind(order_id)
    .bind("alipay")
    .bind(shop_id)
    .bind("PAY-REFUND-1")
    .bind("PENDING")
    .bind(1000_i64)
    .bind(0_i64)
    .bind(1000_i64)
    .bind("CNY")
    .bind(order_created_at)
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
    .bind("alipay")
    .bind("PAY-REFUND-1")
    .bind(shop_id)
    .bind(order_created_at)
    .execute(db.pool())
    .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let token = build_token(
        &jwt,
        &tenant_id.to_string(),
        vec!["finance.write", "finance.read"],
    );
    let state = AppState::new(db.clone(), Arc::new(jwt));
    let app = build_app(state);

    let import_first = serde_json::json!({
        "statement_type": "PAYMENT",
        "channel": "alipay",
        "period_start": "2026-01-01",
        "period_end": "2026-01-31",
        "payment_lines": [
            {
                "platform_order_id": "PAY-REFUND-1",
                "paid_cent": 1000,
                "fee_cent": 0,
                "net_cent": 1000,
                "occurred_at": "2026-01-20T10:00:00Z"
            }
        ]
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/fin/statements/import"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&import_first)?))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    let first_statement_id = json
        .get("statement_id")
        .and_then(|v| v.as_str())
        .expect("statement_id");

    let run_first = serde_json::json!({
        "statement_id": first_statement_id
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/fin/payment-reconcile"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&run_first)?))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    assert_eq!(json.get("diff_count").and_then(|v| v.as_i64()), Some(0));

    let after_sales_id = uuid::Uuid::new_v4();
    let refund_done_at = Utc.with_ymd_and_hms(2026, 2, 15, 12, 0, 0).unwrap();
    sqlx::query(
        r#"
        insert into after_sales (
          id, tenant_id, order_id, type, status, after_sale_status, amount_cent, raw_json, created_at
        ) values ($1,$2,$3,'RETURN_REFUND','REFUNDED','CLOSED',200,'{}'::jsonb,$4)
        "#,
    )
    .bind(after_sales_id)
    .bind(tenant_id)
    .bind(order_id)
    .bind(refund_done_at)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into refund_docs (
          id, tenant_id, after_sales_id, amount_cent, status, refund_status, channel, created_at
        ) values ($1,$2,$3,$4,'DONE','REFUNDED','MANUAL',$5)
        "#,
    )
    .bind(uuid::Uuid::new_v4())
    .bind(tenant_id)
    .bind(after_sales_id)
    .bind(200_i64)
    .bind(refund_done_at)
    .execute(db.pool())
    .await?;

    let import_second = serde_json::json!({
        "statement_type": "PAYMENT",
        "channel": "alipay",
        "period_start": "2026-02-01",
        "period_end": "2026-02-28",
        "payment_lines": [
            {
                "platform_order_id": "PAY-REFUND-1",
                "paid_cent": -200,
                "fee_cent": 0,
                "net_cent": -200,
                "occurred_at": "2026-02-15T12:00:00Z"
            }
        ]
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/fin/statements/import"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&import_second)?))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    let second_statement_id = json
        .get("statement_id")
        .and_then(|v| v.as_str())
        .expect("statement_id");

    let run_second = serde_json::json!({
        "statement_id": second_statement_id
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/fin/payment-reconcile"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&run_second)?))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    assert_eq!(json.get("diff_count").and_then(|v| v.as_i64()), Some(0));
    let second_job_id = json.get("job_id").and_then(|v| v.as_str()).expect("job_id");

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/fin/payment-reconcile/items?job_id={second_job_id}&diff_only=true"
        ))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    let items = json.get("items").and_then(|v| v.as_array()).expect("items");
    assert!(items.is_empty());

    Ok(())
}
