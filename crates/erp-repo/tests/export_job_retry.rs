use anyhow::Result;
use erp_repo::export_jobs::ExportJobsRepo;
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
        create table if not exists export_jobs (
          job_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
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
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }
    sqlx::raw_sql(include_str!(
        "../../../migrations/0101_grid_product_selection_export_items.sql"
    ))
    .execute(pool)
    .await?;
    Ok(())
}

async fn try_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(1))
        .connect(&url)
        .await
        .ok()
}

#[tokio::test]
async fn export_job_retry_claims_failed_jobs() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping export_job_retry_claims_failed_jobs: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        on conflict (tenant_id) do nothing
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind("test-tenant")
    .execute(&pool)
    .await?;

    let job_id: uuid::Uuid = sqlx::query_scalar(
        r#"
        insert into export_jobs (
          tenant_id, job_type, params_json, status, attempts, last_error
        ) values ($1, 'RECONCILE_EXPORT', '{}'::jsonb, 'FAILED', 0, 'boom')
        returning job_id
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .fetch_one(&pool)
    .await?;

    let repo = ExportJobsRepo::new(&pool);
    let jobs = repo.claim_jobs(1, 3).await?;
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].job_id, job_id);
    assert_eq!(jobs[0].status, "RUNNING");

    let (status, attempts): (String, i32) = sqlx::query_as(
        r#"
        select status, attempts
        from export_jobs
        where job_id = $1
        "#,
    )
    .bind(job_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(status, "RUNNING");
    assert_eq!(attempts, 1);

    Ok(())
}
