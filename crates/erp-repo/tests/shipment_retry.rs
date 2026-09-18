use anyhow::Result;
use erp_repo::shipments::{SHIPMENT_REQUESTED_EVENT, ShipmentsRepo};
use erp_types::{ShopId, TenantId};
use serde_json::Value;
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
        create table if not exists shops (
          shop_id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          platform text not null,
          platform_shop_id text not null,
          name text not null default '',
          shop_name text not null default '',
          status text not null default 'AUTHORIZED',
          default_carrier_code text,
          credentials jsonb not null default '{}'::jsonb,
          auth_meta_json jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists shipments (
          shipment_id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          shop_id uuid not null references shops(shop_id),
          order_id uuid not null,
          platform text not null,
          platform_order_id text not null,
          warehouse_id uuid,
          carrier_code text,
          tracking_no text,
          status text not null default 'REQUESTED',
          shipment_status text not null default 'REQUESTED',
          status_version int not null default 0,
          requested_at timestamptz,
          shipped_at timestamptz,
          updated_at timestamptz not null default now(),
          unique (tenant_id, order_id)
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

async fn seed_tenant_shop(pool: &PgPool, tenant_id: TenantId, shop_id: ShopId) -> Result<()> {
    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        on conflict (tenant_id) do nothing
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind("test-tenant")
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
        values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED')
        on conflict (shop_id) do nothing
        "#,
    )
    .bind(uuid::Uuid::from(shop_id))
    .bind(uuid::Uuid::from(tenant_id))
    .bind("taobao")
    .bind("SHOP-001")
    .bind("test-shop")
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
async fn retry_failed_shipment_enqueues_outbox() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping retry_failed_shipment_enqueues_outbox: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let shop_id = ShopId(uuid::Uuid::new_v4());
    seed_tenant_shop(&pool, tenant_id, shop_id).await?;

    let shipment_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();

    sqlx::query(
        r#"
        insert into shipments (
            shipment_id, tenant_id, shop_id, order_id,
            platform, platform_order_id, status, status_version
        ) values ($1, $2, $3, $4, $5, $6, 'FAILED', $7)
        "#,
    )
    .bind(shipment_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(shop_id))
    .bind(order_id)
    .bind("taobao")
    .bind("TB-ORDER-1")
    .bind(20_i32)
    .execute(&pool)
    .await?;

    let before: i64 = sqlx::query_scalar("select count(*) from outbox_events")
        .fetch_one(&pool)
        .await?;

    let repo = ShipmentsRepo::new(&pool);
    repo.retry_failed_shipment(tenant_id, shipment_id).await?;

    let after: i64 = sqlx::query_scalar("select count(*) from outbox_events")
        .fetch_one(&pool)
        .await?;
    assert_eq!(after, before + 1);

    let row: (String, Value) = sqlx::query_as(
        r#"
        select event_type, payload
        from outbox_events
        order by created_at desc
        limit 1
        "#,
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(row.0, SHIPMENT_REQUESTED_EVENT);
    let payload_shipment_id = row
        .1
        .get("shipment_id")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert_eq!(payload_shipment_id, shipment_id.to_string());

    Ok(())
}

#[tokio::test]
async fn retry_success_shipment_rejected() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping retry_success_shipment_rejected: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let shop_id = ShopId(uuid::Uuid::new_v4());
    seed_tenant_shop(&pool, tenant_id, shop_id).await?;

    let shipment_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();

    sqlx::query(
        r#"
        insert into shipments (
            shipment_id, tenant_id, shop_id, order_id,
            platform, platform_order_id, status, status_version
        ) values ($1, $2, $3, $4, $5, $6, 'SUCCESS', $7)
        "#,
    )
    .bind(shipment_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(shop_id))
    .bind(order_id)
    .bind("taobao")
    .bind("TB-ORDER-2")
    .bind(30_i32)
    .execute(&pool)
    .await?;

    let repo = ShipmentsRepo::new(&pool);
    let result = repo.retry_failed_shipment(tenant_id, shipment_id).await;
    assert!(result.is_err());

    let count: i64 = sqlx::query_scalar("select count(*) from outbox_events")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 0);

    Ok(())
}

#[tokio::test]
async fn shipment_status_does_not_regress() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping shipment_status_does_not_regress: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let shop_id = ShopId(uuid::Uuid::new_v4());
    seed_tenant_shop(&pool, tenant_id, shop_id).await?;

    let shipment_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();

    sqlx::query(
        r#"
        insert into shipments (
            shipment_id, tenant_id, shop_id, order_id,
            platform, platform_order_id, status, status_version
        ) values ($1, $2, $3, $4, $5, $6, 'SUCCESS', $7)
        "#,
    )
    .bind(shipment_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(shop_id))
    .bind(order_id)
    .bind("taobao")
    .bind("TB-ORDER-3")
    .bind(30_i32)
    .execute(&pool)
    .await?;

    let repo = ShipmentsRepo::new(&pool);
    repo.mark_failed(tenant_id, shipment_id).await?;

    let status: String = sqlx::query_scalar(
        r#"
        select status
        from shipments
        where tenant_id = $1 and shipment_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(shipment_id)
    .fetch_one(&pool)
    .await?;

    assert_eq!(status, "SUCCESS");
    Ok(())
}
