use anyhow::Result;
use erp_repo::couriers::{CourierCreate, CouriersRepo};
use erp_types::TenantId;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;

async fn setup_schema(pool: &PgPool) -> Result<()> {
    sqlx::query(
        r#"
        create table if not exists couriers (
          courier_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          name text not null,
          code text not null,
          status text not null default 'ACTIVE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, code)
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
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
async fn create_courier_upsert_updates_status_and_name() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping create_courier_upsert_updates_status_and_name: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let repo = CouriersRepo::new(&pool);

    let first = repo
        .create_courier(CourierCreate {
            tenant_id,
            name: "SF old".to_string(),
            code: "SF".to_string(),
            status: "DISABLED".to_string(),
        })
        .await?;
    assert_eq!(first.status, "DISABLED");

    let second = repo
        .create_courier(CourierCreate {
            tenant_id,
            name: "SF new".to_string(),
            code: "SF".to_string(),
            status: "ACTIVE".to_string(),
        })
        .await?;

    assert_eq!(second.courier_id, first.courier_id);
    assert_eq!(second.name, "SF new");
    assert_eq!(second.status, "ACTIVE");

    let persisted_status: String =
        sqlx::query_scalar("select status from couriers where tenant_id = $1 and code = $2")
            .bind(uuid::Uuid::from(tenant_id))
            .bind("SF")
            .fetch_one(&pool)
            .await?;
    assert_eq!(persisted_status, "ACTIVE");

    Ok(())
}
