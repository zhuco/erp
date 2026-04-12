use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use erp_api::{build_app, AppState, Claims, JwtService};
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
          updated_at timestamptz not null default now()
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
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}

fn build_token(jwt: &JwtService, tenant_id: &str, user_id: &str, perms: Vec<&str>) -> String {
    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        perms: perms.into_iter().map(|p| p.to_string()).collect(),
        is_admin: false,
        iat: now,
        exp: now + 3600,
    };
    jwt.encode(&claims).expect("token should encode")
}

#[tokio::test]
async fn print_job_audit_events_exposed() -> anyhow::Result<()> {
    if let Err(err) = print_job_audit_events_exposed_inner().await {
        let message = err.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping print_job_audit_events_exposed: {message}");
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

async fn print_job_audit_events_exposed_inner() -> anyhow::Result<()> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(());
    };

    let db = Db::connect(&database_url).await?;
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let user_id = uuid::Uuid::new_v4();
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
    let token = build_token(
        &jwt,
        &tenant_id.to_string(),
        &user_id.to_string(),
        vec!["print_jobs.write", "print_jobs.read"],
    );
    let state = AppState::new(db.clone(), Arc::new(jwt));
    let app = build_app(state);

    let ref_id = uuid::Uuid::new_v4();
    let ref_id_str = ref_id.to_string();
    let create_body = serde_json::json!({
        "job_type": "SHIPPING_LABEL",
        "ref_type": "ORDER",
        "ref_id": ref_id.to_string(),
        "template_id": template_id.to_string(),
        "payload": {"order_id": "ORDER-AUDIT"}
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/print-jobs"))
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
    let user_id_str = user_id.to_string();
    let template_id_str = template_id.to_string();
    assert_eq!(
        json.get("requested_by_user_id").and_then(|v| v.as_str()),
        Some(user_id_str.as_str())
    );
    assert_eq!(
        json.get("template_id").and_then(|v| v.as_str()),
        Some(template_id_str.as_str())
    );
    assert_eq!(
        json.get("template_version").and_then(|v| v.as_i64()),
        Some(1)
    );
    assert_eq!(
        json.get("ref_id").and_then(|v| v.as_str()),
        Some(ref_id_str.as_str())
    );
    let events = json
        .get("events")
        .and_then(|v| v.as_array())
        .expect("events");
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].get("status").and_then(|v| v.as_str()),
        Some("PENDING")
    );

    let update_body = serde_json::json!({
        "status": "FAILED",
        "last_error": "printer offline"
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/print-jobs/{job_id}"))
        .method("PUT")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&update_body)?))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    let events = json
        .get("events")
        .and_then(|v| v.as_array())
        .expect("events");
    assert_eq!(events.len(), 2);
    let last = events.last().expect("last event");
    assert_eq!(last.get("status").and_then(|v| v.as_str()), Some("FAILED"));
    assert_eq!(
        last.get("error").and_then(|v| v.as_str()),
        Some("printer offline")
    );

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/print-jobs?job_id={job_id}"
        ))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let list_json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    let jobs = list_json
        .get("data")
        .and_then(|v| v.as_array())
        .expect("data");
    assert_eq!(jobs.len(), 1);
    let list_events = jobs[0]
        .get("events")
        .and_then(|v| v.as_array())
        .expect("events");
    assert_eq!(list_events.len(), 2);
    let last = list_events.last().expect("last event");
    assert_eq!(last.get("status").and_then(|v| v.as_str()), Some("FAILED"));
    assert_eq!(
        last.get("error").and_then(|v| v.as_str()),
        Some("printer offline")
    );

    Ok(())
}
