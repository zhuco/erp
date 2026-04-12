use anyhow::Result;
use erp_repo::print_devices::{PrintDeviceCreate, PrintDeviceListFilters, PrintDevicesRepo};
use erp_repo::print_jobs::{PrintJobCreate, PrintJobListFilters, PrintJobUpdate, PrintJobsRepo};
use erp_repo::print_templates::{PrintTemplateCreate, PrintTemplatesRepo};
use erp_types::{TenantId, UserId};
use serde_json::json;
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
        create table if not exists print_devices (
          device_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          name text not null,
          fingerprint text not null,
          status text not null default 'ACTIVE',
          is_default boolean not null default false,
          device_token text,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, fingerprint)
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
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}

async fn seed_tenant(pool: &PgPool, tenant_id: TenantId) -> Result<()> {
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
async fn print_device_registration_idempotent() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping print_device_registration_idempotent: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;
    let tenant_id = TenantId(uuid::Uuid::new_v4());
    seed_tenant(&pool, tenant_id).await?;

    let repo = PrintDevicesRepo::new(&pool);
    let first = repo
        .create_device(PrintDeviceCreate {
            tenant_id,
            name: "Printer-A".to_string(),
            fingerprint: "FP-001".to_string(),
            status: "ACTIVE".to_string(),
            is_default: false,
            device_token: "token-001".to_string(),
        })
        .await?;
    let second = repo
        .create_device(PrintDeviceCreate {
            tenant_id,
            name: "Printer-A".to_string(),
            fingerprint: "FP-001".to_string(),
            status: "ACTIVE".to_string(),
            is_default: false,
            device_token: "token-001".to_string(),
        })
        .await?;

    assert_eq!(first.device_id, second.device_id);

    let list = repo
        .list_devices(
            tenant_id,
            PrintDeviceListFilters {
                fingerprint: None,
                status: None,
                cursor: None,
                limit: 10,
            },
        )
        .await?;

    assert_eq!(list.items.len(), 1);
    Ok(())
}

#[tokio::test]
async fn print_template_version_unique() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping print_template_version_unique: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;
    let tenant_id = TenantId(uuid::Uuid::new_v4());
    seed_tenant(&pool, tenant_id).await?;

    let repo = PrintTemplatesRepo::new(&pool);
    repo.create_template(PrintTemplateCreate {
        tenant_id,
        shop_id: uuid::Uuid::new_v4(),
        platform: "taobao".to_string(),
        template_type: "SHIPPING_LABEL".to_string(),
        platform_template_id: None,
        version: 1,
        content_format: "HTML".to_string(),
        content: json!({"layout": "A5"}),
        enabled: true,
    })
    .await?;

    let dup = repo
        .create_template(PrintTemplateCreate {
            tenant_id,
            shop_id: uuid::Uuid::new_v4(),
            platform: "taobao".to_string(),
            template_type: "SHIPPING_LABEL".to_string(),
            platform_template_id: None,
            version: 1,
            content_format: "HTML".to_string(),
            content: json!({"layout": "A5"}),
            enabled: true,
        })
        .await;

    assert!(dup.is_err());
    Ok(())
}

#[tokio::test]
async fn print_job_create_queryable() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping print_job_create_queryable: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;
    let tenant_id = TenantId(uuid::Uuid::new_v4());
    seed_tenant(&pool, tenant_id).await?;

    let template_repo = PrintTemplatesRepo::new(&pool);
    let template = template_repo
        .create_template(PrintTemplateCreate {
            tenant_id,
            shop_id: uuid::Uuid::new_v4(),
            platform: "taobao".to_string(),
            template_type: "SHIPPING_LABEL".to_string(),
            platform_template_id: None,
            version: 1,
            content_format: "HTML".to_string(),
            content: json!({"layout": "4x6"}),
            enabled: true,
        })
        .await?;

    let job_repo = PrintJobsRepo::new(&pool);
    let requested_by_user_id = UserId(uuid::Uuid::new_v4());
    let job = job_repo
        .create_job(PrintJobCreate {
            tenant_id,
            requested_by_user_id,
            job_type: "SHIPPING_LABEL".to_string(),
            ref_type: "ORDER".to_string(),
            ref_id: uuid::Uuid::new_v4(),
            template_id: template.template_id,
            device_fingerprint: None,
            payload: json!({"order_id": "ORDER-1"}),
            trace_id: None,
        })
        .await?;

    let list = job_repo
        .list_jobs(
            tenant_id,
            PrintJobListFilters {
                job_type: None,
                ref_type: None,
                ref_id: None,
                status: None,
                job_id: Some(job.job_id),
                cursor: None,
                limit: 10,
            },
        )
        .await?;

    assert_eq!(list.items.len(), 1);
    assert_eq!(list.items[0].status, "PENDING");
    assert_eq!(list.items[0].events.len(), 1);
    assert_eq!(list.items[0].events[0].status, "PENDING");

    let event_type: String = sqlx::query_scalar(
        r#"
        select event_type
        from outbox_events
        where payload->>'job_id' = $1
        "#,
    )
    .bind(job.job_id.to_string())
    .fetch_one(&pool)
    .await?;

    assert_eq!(event_type, "PRINT_JOB_QUEUED");
    Ok(())
}

#[tokio::test]
async fn print_job_event_history_records_failure() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping print_job_event_history_records_failure: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;
    let tenant_id = TenantId(uuid::Uuid::new_v4());
    seed_tenant(&pool, tenant_id).await?;

    let template_repo = PrintTemplatesRepo::new(&pool);
    let template = template_repo
        .create_template(PrintTemplateCreate {
            tenant_id,
            shop_id: uuid::Uuid::new_v4(),
            platform: "taobao".to_string(),
            template_type: "INVOICE".to_string(),
            platform_template_id: None,
            version: 2,
            content_format: "HTML".to_string(),
            content: json!({"layout": "A4"}),
            enabled: true,
        })
        .await?;

    let job_repo = PrintJobsRepo::new(&pool);
    let requested_by_user_id = UserId(uuid::Uuid::new_v4());
    let job = job_repo
        .create_job(PrintJobCreate {
            tenant_id,
            requested_by_user_id,
            job_type: "INVOICE".to_string(),
            ref_type: "ORDER".to_string(),
            ref_id: uuid::Uuid::new_v4(),
            template_id: template.template_id,
            device_fingerprint: None,
            payload: json!({"order_id": "ORDER-2"}),
            trace_id: None,
        })
        .await?;

    assert_eq!(job.events.len(), 1);
    assert_eq!(job.events[0].status, "PENDING");

    let updated = job_repo
        .update_job(
            tenant_id,
            job.job_id,
            PrintJobUpdate {
                status: Some("FAILED".to_string()),
                payload: None,
                output_url: None,
                output_content: None,
                attempts: None,
                last_error: Some("printer offline".to_string()),
                actor_user_id: Some(requested_by_user_id),
            },
        )
        .await?
        .expect("job exists");

    assert_eq!(updated.events.len(), 2);
    let last = updated.events.last().expect("event exists");
    assert_eq!(last.status, "FAILED");
    assert_eq!(last.error.as_deref(), Some("printer offline"));
    Ok(())
}
