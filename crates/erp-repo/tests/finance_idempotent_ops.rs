use anyhow::Result;
use erp_repo::finance_ap_plans::{ApPaymentPlanCreate, ApPaymentPlansRepo};
use erp_repo::finance_ar::{ArDocCreate, ArDocsRepo};
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
        create table if not exists customers (
          customer_id uuid primary key,
          tenant_id uuid not null,
          name text not null,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists suppliers (
          supplier_id uuid primary key,
          tenant_id uuid not null,
          name text not null,
          status text not null default 'ACTIVE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists ar_docs (
          ar_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          customer_id uuid,
          ref_type text not null,
          ref_id text not null,
          amount_cent bigint not null,
          currency text not null,
          status text not null,
          due_date date,
          written_off_at timestamptz,
          created_by text,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists ap_payment_plans (
          plan_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          ap_id uuid not null,
          supplier_id uuid,
          planned_date date,
          amount_cent bigint not null,
          status text not null,
          paid_at timestamptz,
          notes text,
          created_by text,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
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
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(2))
        .connect(&url)
        .await
        .ok()
}

#[tokio::test]
async fn ar_write_off_should_be_idempotent() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping ar_write_off_should_be_idempotent: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let customer_id = uuid::Uuid::new_v4();
    sqlx::query("insert into tenants (tenant_id, name) values ($1, 'tenant')")
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;
    sqlx::query("insert into customers (customer_id, tenant_id, name) values ($1, $2, 'customer')")
        .bind(customer_id)
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;

    let repo = ArDocsRepo::new(&pool);
    let created = repo
        .create_doc(ArDocCreate {
            tenant_id,
            customer_id: Some(customer_id),
            ref_type: "ORDER".to_string(),
            ref_id: "SO-1001".to_string(),
            amount_cent: 5000,
            currency: "CNY".to_string(),
            status: "OPEN".to_string(),
            due_date: None,
            created_by: Some("tester".to_string()),
        })
        .await?;

    let first = repo
        .mark_written_off(tenant_id, created.ar_id)
        .await?
        .expect("first write-off");
    tokio::time::sleep(Duration::from_millis(10)).await;
    let second = repo
        .mark_written_off(tenant_id, created.ar_id)
        .await?
        .expect("second write-off");

    assert_eq!(first.status, "WRITTEN_OFF");
    assert_eq!(second.status, "WRITTEN_OFF");
    assert_eq!(first.written_off_at, second.written_off_at);

    Ok(())
}

#[tokio::test]
async fn ap_plan_mark_paid_should_be_idempotent() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping ap_plan_mark_paid_should_be_idempotent: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let supplier_id = uuid::Uuid::new_v4();
    sqlx::query("insert into tenants (tenant_id, name) values ($1, 'tenant')")
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;
    sqlx::query("insert into suppliers (supplier_id, tenant_id, name) values ($1, $2, 'supplier')")
        .bind(supplier_id)
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;

    let repo = ApPaymentPlansRepo::new(&pool);
    let created = repo
        .create_plan(ApPaymentPlanCreate {
            tenant_id,
            ap_id: uuid::Uuid::new_v4(),
            supplier_id: Some(supplier_id),
            planned_date: None,
            amount_cent: 7600,
            status: "PLANNED".to_string(),
            notes: Some("initial plan".to_string()),
            created_by: Some("tester".to_string()),
        })
        .await?;

    let first = repo
        .mark_paid(tenant_id, created.plan_id)
        .await?
        .expect("first paid");
    tokio::time::sleep(Duration::from_millis(10)).await;
    let second = repo
        .mark_paid(tenant_id, created.plan_id)
        .await?
        .expect("second paid");

    assert_eq!(first.status, "PAID");
    assert_eq!(second.status, "PAID");
    assert_eq!(first.paid_at, second.paid_at);

    Ok(())
}
