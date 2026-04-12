use anyhow::Result;
use erp_repo::worker_effects::WorkerEffectsRepo;
use erp_types::TenantId;
use sqlx::{postgres::PgPoolOptions, PgPool};
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
        create table if not exists worker_effects (
          tenant_id uuid not null references tenants(tenant_id),
          outbox_id uuid not null,
          event_type text not null,
          status text not null default 'PROCESSING',
          last_error text not null default '',
          processed_at timestamptz not null default now(),
          primary key (tenant_id, outbox_id)
        )
        "#,
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }
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
async fn worker_effects_claim_and_retry() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping worker_effects_claim_and_retry: db unavailable");
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

    let repo = WorkerEffectsRepo::new(&pool);
    let outbox_id = uuid::Uuid::new_v4();

    let first = repo
        .claim_processing(tenant_id, outbox_id, "ORDER_UPSERTED")
        .await?;
    assert!(first);

    let second = repo
        .claim_processing(tenant_id, outbox_id, "ORDER_UPSERTED")
        .await?;
    assert!(!second);

    repo.mark_failed(tenant_id, outbox_id, "boom").await?;
    let third = repo
        .claim_processing(tenant_id, outbox_id, "ORDER_UPSERTED")
        .await?;
    assert!(third);

    repo.mark_succeeded(tenant_id, outbox_id).await?;
    let fourth = repo
        .claim_processing(tenant_id, outbox_id, "ORDER_UPSERTED")
        .await?;
    assert!(!fourth);

    let effect = repo.ensure_effect(tenant_id, outbox_id).await?;
    assert_eq!(effect.status, "SUCCEEDED");

    Ok(())
}
