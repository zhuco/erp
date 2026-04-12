use anyhow::Result;
use chrono::{Datelike, Utc};
use erp_repo::fulfillment::{FulfillmentRepo, SHIPMENT_STATUS_SHIPPED, SHIPMENT_TYPE_OMS};
use erp_types::{OrderId, TenantId};
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
        create table if not exists warehouses (
          warehouse_id uuid primary key,
          tenant_id uuid not null,
          name text not null,
          code text not null,
          is_default boolean not null default false,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists orders (
          tenant_id uuid not null,
          order_month date not null,
          order_id uuid not null,
            platform text not null,
            shop_id uuid not null,
            platform_order_id text not null,
            receiver_address text,
            status text not null default 'PENDING_SHIPMENT',
            oms_status text not null default 'NEW',
            order_status text not null default 'PENDING_REVIEW',
            fulfillment_status text not null default 'NOT_ALLOCATED',
            exception_status text not null default 'NONE',
            review_status text not null default 'NOT_REVIEWED',
            inventory_shortage boolean not null default false,
            exception_flags jsonb not null default '[]'::jsonb,
          allocated_at timestamptz,
          ship_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_month, order_id)
        )
        "#,
        r#"
        create table if not exists order_index (
          tenant_id uuid not null,
          order_id uuid not null,
          order_month date not null,
          platform text not null,
          platform_order_id text not null,
          shop_id uuid not null,
          order_created_at timestamptz not null,
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_id)
        )
        "#,
        r#"
        create table if not exists order_items (
          tenant_id uuid not null,
          order_month date not null,
          order_id uuid not null,
          line_id text not null,
          sku_id text,
          qty int not null,
          primary key (tenant_id, order_month, order_id, line_id)
        )
        "#,
        r#"
        create table if not exists skus (
          sku_id uuid primary key,
          tenant_id uuid not null,
          sku_code text not null,
          sku_kind text not null default 'NORMAL',
          weight_gram int,
          cost_cent bigint,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, sku_code)
        )
        "#,
        r#"
        create table if not exists channel_sku_mappings (
          mapping_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          shop_id uuid,
          platform text,
          platform_sku_id text,
          sku_id uuid not null,
          status text not null default 'ACTIVE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists sku_boms (
          tenant_id uuid not null,
          bundle_sku_id uuid not null,
          component_sku_id uuid not null,
          qty bigint not null,
          bom_version int not null default 1,
          updated_at timestamptz not null default now(),
          primary key (tenant_id, bundle_sku_id, component_sku_id)
        )
        "#,
        r#"
        create table if not exists inventory_balances (
          tenant_id uuid not null,
          warehouse_id uuid not null,
          sku_id uuid not null,
          on_hand bigint not null default 0,
          reserved bigint not null default 0,
          in_transit bigint not null default 0,
          frozen bigint not null default 0,
          updated_at timestamptz not null default now(),
          primary key (tenant_id, warehouse_id, sku_id)
        )
        "#,
        r#"
        create table if not exists inventory_reservations (
          reservation_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          warehouse_id uuid not null,
          sku_id uuid not null,
          order_id uuid,
          order_item_id uuid,
          qty bigint not null,
          status text not null default 'RESERVED',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, order_item_id)
        )
        "#,
        r#"
        create table if not exists inventory_ledgers (
          ledger_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          warehouse_id uuid not null,
          sku_id uuid not null,
          delta_on_hand bigint not null,
          delta_reserved bigint not null,
          delta_in_transit bigint not null,
          delta_frozen bigint not null,
          reason_type text not null,
          ref_type text,
          ref_id text,
          occurred_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists shipments (
          shipment_id uuid primary key,
          tenant_id uuid not null,
          shop_id uuid not null,
          order_id uuid not null,
          platform text not null,
          platform_order_id text not null,
            warehouse_id uuid,
            carrier_code text,
            tracking_no text,
            status text not null default 'CREATED',
            shipment_status text not null default 'REQUESTED',
            status_version int not null default 0,
          shipment_type text not null default 'OMS',
          shipment_no text,
          weight_gram int,
          shipped_at timestamptz,
          requested_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          last_error text not null default ''
        )
        "#,
        r#"
        create table if not exists shipment_items (
          shipment_item_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          shipment_id uuid not null,
          order_id uuid not null,
          order_item_id uuid not null,
          sku_id uuid not null,
          qty bigint not null,
          created_at timestamptz not null default now(),
          unique (tenant_id, shipment_id, order_item_id)
        )
        "#,
        r#"
        create table if not exists order_cogs (
          ledger_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          order_id uuid not null,
          shipment_id uuid,
          outbound_doc_id uuid,
          order_item_id uuid,
          sku_id uuid not null,
          qty bigint not null,
          unit_cost_cent bigint not null,
          total_cost_cent bigint not null,
          ref_type text not null,
          ref_id text not null,
          created_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists warehouse_rules (
          rule_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          warehouse_id uuid not null,
          province text,
          city text,
          carrier_code text,
          priority int not null default 0,
          is_active boolean not null default true,
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

async fn seed_base(
    pool: &PgPool,
    tenant_id: TenantId,
    order_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
) -> Result<()> {
    let now = Utc::now();
    let order_month =
        chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).expect("valid date");
    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, 'tenant', 'ACTIVE')
        on conflict (tenant_id) do nothing
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into warehouses (warehouse_id, tenant_id, name, code, is_default)
        values ($1, $2, 'Main', 'MAIN', true)
        on conflict (warehouse_id) do nothing
        "#,
    )
    .bind(warehouse_id)
    .bind(uuid::Uuid::from(tenant_id))
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into skus (sku_id, tenant_id, sku_code, sku_kind, weight_gram)
        values ($1, $2, 'SKU-1', 'NORMAL', 100)
        on conflict (sku_id) do nothing
        "#,
    )
    .bind(sku_id)
    .bind(uuid::Uuid::from(tenant_id))
    .execute(pool)
    .await?;

    let shop_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into orders (
          tenant_id, order_month, order_id, platform, shop_id, platform_order_id,
          receiver_address, status
        ) values ($1,$2,$3,'taobao',$4,'PLAT-ORDER-1','Shanghai','PENDING_SHIPMENT')
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_month)
    .bind(order_id)
    .bind(shop_id)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into order_index (
          tenant_id, order_id, order_month, platform, platform_order_id, shop_id, order_created_at
        ) values ($1,$2,$3,'taobao','PLAT-ORDER-1',$4,$5)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_id)
    .bind(order_month)
    .bind(shop_id)
    .bind(now)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into order_items (
          tenant_id, order_month, order_id, line_id, sku_id, qty
        ) values ($1,$2,$3,'LINE-1',$4,2)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_month)
    .bind(order_id)
    .bind(sku_id.to_string())
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into inventory_balances (
          tenant_id, warehouse_id, sku_id, on_hand, reserved
        ) values ($1,$2,$3,5,0)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(warehouse_id)
    .bind(sku_id)
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
async fn allocate_creates_shipments_and_reservations() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping allocate_creates_shipments_and_reservations: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let order_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();
    seed_base(&pool, tenant_id, order_id, sku_id, warehouse_id).await?;

    let repo = FulfillmentRepo::new(&pool);
    let result = repo
        .allocate_order(tenant_id, OrderId(order_id), None)
        .await?;

    assert_eq!(result.shipments.len(), 1);
    let shipment = &result.shipments[0];
    assert_eq!(shipment.status, "CREATED");
    assert_eq!(shipment.warehouse_id, warehouse_id);

    let reserved: i64 = sqlx::query_scalar(
        r#"
        select reserved
        from inventory_balances
        where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(warehouse_id)
    .bind(sku_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(reserved, 2);

    let order_item_id = uuid::Uuid::new_v5(&order_id, "LINE-1".as_bytes());
    let ledger_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from inventory_ledgers
        where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
          and reason_type = 'RESERVATION_CREATE' and ref_id = $4
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(warehouse_id)
    .bind(sku_id)
    .bind(order_item_id.to_string())
    .fetch_one(&pool)
    .await?;
    assert_eq!(ledger_count, 1);

    let status: String = sqlx::query_scalar(
        r#"
        select status
        from orders
        where tenant_id = $1 and order_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(status, "ALLOCATED");

    let shipment_type: String = sqlx::query_scalar(
        r#"
        select shipment_type
        from shipments
        where tenant_id = $1 and order_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(shipment_type, SHIPMENT_TYPE_OMS);

    let item_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from shipment_items
        where tenant_id = $1 and order_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(item_count, 1);

    Ok(())
}

#[tokio::test]
async fn ship_consumes_inventory_and_updates_order() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping ship_consumes_inventory_and_updates_order: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let order_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();
    seed_base(&pool, tenant_id, order_id, sku_id, warehouse_id).await?;

    let repo = FulfillmentRepo::new(&pool);
    let result = repo
        .allocate_order(tenant_id, OrderId(order_id), None)
        .await?;
    let shipment_id = result.shipments[0].shipment_id;

    let (shipment, shipped_now) = repo
        .ship_oms_shipment(
            tenant_id,
            shipment_id,
            Some("SF".to_string()),
            Some("TRACK-1".to_string()),
            Utc::now(),
            Some(120),
        )
        .await?;
    assert!(shipped_now);
    assert_eq!(shipment.status, SHIPMENT_STATUS_SHIPPED);

    let (on_hand, reserved): (i64, i64) = sqlx::query_as(
        r#"
        select on_hand, reserved
        from inventory_balances
        where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(warehouse_id)
    .bind(sku_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(on_hand, 3);
    assert_eq!(reserved, 0);

    let order_item_id = uuid::Uuid::new_v5(&order_id, "LINE-1".as_bytes());
    let ship_ledger_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from inventory_ledgers
        where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
          and reason_type = 'SALES_SHIP' and ref_id = $4
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(warehouse_id)
    .bind(sku_id)
    .bind(order_item_id.to_string())
    .fetch_one(&pool)
    .await?;
    assert_eq!(ship_ledger_count, 1);

    let reservation_status: String = sqlx::query_scalar(
        r#"
        select status
        from inventory_reservations
        where tenant_id = $1
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .fetch_one(&pool)
    .await?;
    assert_eq!(reservation_status, "CONSUMED");

    let order_status: String = sqlx::query_scalar(
        r#"
        select status
        from orders
        where tenant_id = $1 and order_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(order_status, "SHIPPED");

    Ok(())
}
