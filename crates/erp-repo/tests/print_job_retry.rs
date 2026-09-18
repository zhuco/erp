use anyhow::Result;
use erp_repo::print_jobs::PrintJobsRepo;
use erp_types::TenantId;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;

async fn setup_schema(pool: &PgPool) -> Result<()> {
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
          template_id uuid primary key default gen_random_uuid(),
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
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}

async fn seed_tenant_template(pool: &PgPool, tenant_id: TenantId) -> Result<uuid::Uuid> {
    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, 'test-tenant', 'ACTIVE')
        on conflict (tenant_id) do nothing
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .execute(pool)
    .await?;

    let shop_id = uuid::Uuid::new_v4();
    let template_id: uuid::Uuid = sqlx::query_scalar(
        r#"
        insert into print_templates (
          tenant_id, shop_id, platform, template_type, version, content_format, content, is_active
        )
        values ($1, $2, 'taobao', 'SHIPPING_LABEL', 1, 'HTML', '{}'::jsonb, true)
        returning template_id
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(shop_id)
    .fetch_one(pool)
    .await?;

    Ok(template_id)
}

async fn try_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect(&url)
        .await
        .ok()
}

#[tokio::test]
async fn retry_failed_print_job_enqueues_outbox() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping retry_failed_print_job_enqueues_outbox: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let template_id = seed_tenant_template(&pool, tenant_id).await?;
    let job_id = uuid::Uuid::new_v4();
    let requested_by_user_id = uuid::Uuid::new_v4();

    sqlx::query(
        r#"
        insert into print_jobs (
          job_id, tenant_id, type, ref_type, ref_id, template_id, template_version,
          requested_by_user_id, status, payload
        )
        values ($1, $2, 'SHIPPING_LABEL', 'ORDER', $3, $4, 1, $5, 'FAILED', '{"order_id": "ORDER-1"}'::jsonb)
        "#,
    )
    .bind(job_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::new_v4())
    .bind(template_id)
    .bind(requested_by_user_id)
    .execute(&pool)
    .await?;

    let repo = PrintJobsRepo::new(&pool);
    repo.retry_failed_job(tenant_id, job_id).await?;
    repo.retry_failed_job(tenant_id, job_id).await?;

    let count: i64 = sqlx::query_scalar("select count(*) from outbox_events")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 1);

    let status: String = sqlx::query_scalar(
        r#"
        select status
        from print_jobs
        where tenant_id = $1 and job_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(job_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(status, "PENDING");

    Ok(())
}

#[tokio::test]
async fn retry_success_print_job_rejected() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping retry_success_print_job_rejected: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let template_id = seed_tenant_template(&pool, tenant_id).await?;
    let job_id = uuid::Uuid::new_v4();
    let requested_by_user_id = uuid::Uuid::new_v4();

    sqlx::query(
        r#"
        insert into print_jobs (
          job_id, tenant_id, type, ref_type, ref_id, template_id, template_version,
          requested_by_user_id, status, payload
        )
        values ($1, $2, 'SHIPPING_LABEL', 'ORDER', $3, $4, 1, $5, 'DONE', '{}'::jsonb)
        "#,
    )
    .bind(job_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::new_v4())
    .bind(template_id)
    .bind(requested_by_user_id)
    .execute(&pool)
    .await?;

    let repo = PrintJobsRepo::new(&pool);
    let result = repo.retry_failed_job(tenant_id, job_id).await;
    assert!(result.is_err());

    let count: i64 = sqlx::query_scalar("select count(*) from outbox_events")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 0);

    Ok(())
}
