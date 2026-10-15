use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
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
        create table if not exists print_templates (
          template_id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          shop_id uuid not null,
          platform text not null,
          template_type text not null,
          platform_template_id text,
          version int not null default 1,
          content_format text not null default 'HTML',
          content jsonb not null default '{}'::jsonb,
          is_active boolean not null default true,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, shop_id, template_type, version)
        )
        "#,
        r#"
        create table if not exists print_jobs (
          job_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          type text,
          ref_type text,
          ref_id uuid,
          template_id uuid not null references print_templates(template_id),
          template_version int not null,
          requested_by_user_id uuid not null,
          status text not null default 'PENDING',
          payload jsonb not null default '{}'::jsonb,
          output_url text,
          output_content text,
          attempts int not null default 0,
          last_error text not null default '',
          device_fingerprint text,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists print_job_events (
          event_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          job_id uuid not null references print_jobs(job_id),
          status text not null,
          actor_type text not null,
          actor_user_id uuid,
          error text,
          created_at timestamptz not null default now()
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
        create table if not exists idempotency_keys (
          tenant_id uuid not null,
          "key" text not null,
          route text not null,
          request_hash text not null,
          response_status int not null,
          response_body jsonb not null,
          created_at timestamptz not null default now(),
          unique (tenant_id, route, "key")
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
    let claims = Claims::bearer(
        uuid::Uuid::new_v4().to_string(),
        tenant_id.to_string(),
        vec!["print_jobs.write".to_string()],
        false,
        now,
        now + 3600,
    );
    jwt.encode(&claims).expect("token should encode")
}

struct TestContext {
    app: Router,
    db: Db,
    token: String,
    tenant_id: uuid::Uuid,
    template_id: uuid::Uuid,
}

async fn setup_context() -> anyhow::Result<Option<TestContext>> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(None);
    };

    let db = Db::connect(&database_url).await?;
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
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

    let template_id = uuid::Uuid::new_v4();
    let shop_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into print_templates (
          template_id, tenant_id, shop_id, platform, template_type, version, content_format, content, is_active
        )
        values ($1, $2, $3, $4, $5, $6, 'HTML', $7, true)
        "#,
    )
    .bind(template_id)
    .bind(tenant_id)
    .bind(shop_id)
    .bind("taobao")
    .bind("SHIPPING_LABEL")
    .bind(1_i32)
    .bind(serde_json::json!({"layout": "A5"}))
    .execute(db.pool())
    .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let token = build_token(&jwt, &tenant_id.to_string());
    let state = AppState::new(db.clone(), Arc::new(jwt));
    let app = build_app(state);

    Ok(Some(TestContext {
        app,
        db,
        token,
        tenant_id,
        template_id,
    }))
}

#[tokio::test]
async fn idempotency_same_key_hits_cache() -> anyhow::Result<()> {
    if let Err(err) = idempotency_same_key_hits_cache_inner().await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping idempotency_same_key_hits_cache: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn idempotency_same_key_hits_cache_inner() -> anyhow::Result<()> {
    let Some(ctx) = setup_context().await? else {
        return Ok(());
    };

    let ref_id = uuid::Uuid::new_v4();
    let body = serde_json::json!({
        "job_type": "SHIPPING_LABEL",
        "ref_type": "ORDER",
        "ref_id": ref_id.to_string(),
        "template_id": ctx.template_id.to_string(),
        "payload": {"order_id": "ORDER-1"}
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{}/print-jobs", ctx.tenant_id))
        .method("POST")
        .header("authorization", format!("Bearer {}", ctx.token))
        .header("Idempotency-Key", "print-key-1")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body)?))
        .unwrap();

    let response = ctx.app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    let job_id = json
        .get("job_id")
        .and_then(|v| v.as_str())
        .expect("job_id")
        .to_string();

    let request = Request::builder()
        .uri(format!("/v1/tenants/{}/print-jobs", ctx.tenant_id))
        .method("POST")
        .header("authorization", format!("Bearer {}", ctx.token))
        .header("Idempotency-Key", "print-key-1")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body)?))
        .unwrap();

    let response = ctx.app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    let cached_job_id = json.get("job_id").and_then(|v| v.as_str()).expect("job_id");
    assert_eq!(cached_job_id, job_id);

    let job_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from print_jobs
        where tenant_id = $1
        "#,
    )
    .bind(ctx.tenant_id)
    .fetch_one(ctx.db.pool())
    .await?;
    assert_eq!(job_count, 1);

    let outbox_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from outbox_events
        where tenant_id = $1 and event_type = 'PRINT_JOB_QUEUED'
        "#,
    )
    .bind(ctx.tenant_id)
    .fetch_one(ctx.db.pool())
    .await?;
    assert_eq!(outbox_count, 1);

    Ok(())
}

#[tokio::test]
async fn idempotency_same_key_conflict() -> anyhow::Result<()> {
    if let Err(err) = idempotency_same_key_conflict_inner().await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping idempotency_same_key_conflict: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn idempotency_same_key_conflict_inner() -> anyhow::Result<()> {
    let Some(ctx) = setup_context().await? else {
        return Ok(());
    };

    let ref_id = uuid::Uuid::new_v4();
    let body = serde_json::json!({
        "job_type": "SHIPPING_LABEL",
        "ref_type": "ORDER",
        "ref_id": ref_id.to_string(),
        "template_id": ctx.template_id.to_string(),
        "payload": {"order_id": "ORDER-2"}
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{}/print-jobs", ctx.tenant_id))
        .method("POST")
        .header("authorization", format!("Bearer {}", ctx.token))
        .header("Idempotency-Key", "print-key-2")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body)?))
        .unwrap();

    let response = ctx.app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let conflict_body = serde_json::json!({
        "job_type": "SHIPPING_LABEL",
        "ref_type": "ORDER",
        "ref_id": ref_id.to_string(),
        "template_id": ctx.template_id.to_string(),
        "payload": {"order_id": "ORDER-2-CHANGED"}
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{}/print-jobs", ctx.tenant_id))
        .method("POST")
        .header("authorization", format!("Bearer {}", ctx.token))
        .header("Idempotency-Key", "print-key-2")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&conflict_body)?))
        .unwrap();

    let response = ctx.app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);

    Ok(())
}
