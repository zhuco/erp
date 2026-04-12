use anyhow::Result;
use chrono::{Datelike, NaiveDate, Utc};
use erp_repo::outbox::{OutboxListFilters, OutboxRepo};
use erp_types::TenantId;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

fn month_start(dt: chrono::DateTime<Utc>) -> NaiveDate {
    let d = dt.date_naive();
    NaiveDate::from_ymd_opt(d.year(), d.month(), 1).expect("valid date")
}

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
async fn outbox_list_and_requeue() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping outbox_list_and_requeue: db unavailable");
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

    let event_month = month_start(Utc::now());
    let dead_id: uuid::Uuid = sqlx::query_scalar(
        r#"
        insert into outbox_events(
          tenant_id, event_month, event_type, payload, status, last_error
        ) values ($1, $2, 'ORDER_UPSERTED', '{}'::jsonb, 'DEAD', 'boom')
        returning outbox_id
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(event_month)
    .fetch_one(&pool)
    .await?;

    let _sent_id: uuid::Uuid = sqlx::query_scalar(
        r#"
        insert into outbox_events(
          tenant_id, event_month, event_type, payload, status, published_at
        ) values ($1, $2, 'ORDER_UPSERTED', '{}'::jsonb, 'SENT', now())
        returning outbox_id
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(event_month)
    .fetch_one(&pool)
    .await?;

    let repo = OutboxRepo::new(&pool);
    let list = repo
        .list_outbox_events(
            tenant_id,
            OutboxListFilters {
                status: Some("DEAD".to_string()),
                event_type: None,
                cursor: None,
                limit: 10,
            },
        )
        .await?;
    assert_eq!(list.items.len(), 1);
    assert_eq!(list.items[0].outbox_id, dead_id);

    repo.requeue_outbox(tenant_id, dead_id).await?;
    let status: String = sqlx::query_scalar(
        r#"
        select status
        from outbox_events
        where tenant_id = $1 and outbox_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(dead_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(status, "PENDING");

    Ok(())
}
