use anyhow::Result;
use erp_repo::inventory::{
    InventoryAdjustment, InventoryRepo, InventoryReservationAction, InventoryReservationInput,
};
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
        create table if not exists products (
          product_id uuid primary key,
          tenant_id uuid not null,
          name text not null,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists skus (
          sku_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          product_id uuid not null,
          sku_code text not null,
          sku_kind text not null default 'NORMAL',
          barcode text,
          name text,
          spec jsonb not null default '{}'::jsonb,
          weight_gram int,
          length_mm int,
          width_mm int,
          height_mm int,
          volume_cm3 int,
          status text not null default 'ACTIVE',
          batch_enabled boolean not null default false,
          serial_enabled boolean not null default false,
          is_active boolean not null default true,
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
        create table if not exists warehouses (
          warehouse_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          name text not null,
          code text not null,
          is_default boolean not null default false,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, code)
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
        create table if not exists inventory_reservations (
          reservation_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          warehouse_id uuid not null,
          sku_id uuid not null,
          order_id uuid,
          order_item_id uuid,
          qty bigint not null,
          status text not null default 'ACTIVE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, order_item_id)
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

async fn seed_product(pool: &PgPool, tenant_id: TenantId) -> Result<uuid::Uuid> {
    let product_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into products (product_id, tenant_id, name)
        values ($1, $2, 'test-product')
        "#,
    )
    .bind(product_id)
    .bind(uuid::Uuid::from(tenant_id))
    .execute(pool)
    .await?;
    Ok(product_id)
}

async fn seed_sku(
    pool: &PgPool,
    tenant_id: TenantId,
    product_id: uuid::Uuid,
) -> Result<uuid::Uuid> {
    let sku_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into skus (sku_id, tenant_id, product_id, sku_code, sku_kind, spec)
        values ($1, $2, $3, 'SKU-IDEMPOTENT', 'NORMAL', '{}'::jsonb)
        "#,
    )
    .bind(sku_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(product_id)
    .execute(pool)
    .await?;
    Ok(sku_id)
}

async fn seed_warehouse(pool: &PgPool, tenant_id: TenantId) -> Result<uuid::Uuid> {
    let warehouse_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into warehouses (warehouse_id, tenant_id, name, code, is_default)
        values ($1, $2, 'Main', 'MAIN', true)
        "#,
    )
    .bind(warehouse_id)
    .bind(uuid::Uuid::from(tenant_id))
    .execute(pool)
    .await?;
    Ok(warehouse_id)
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
async fn reserve_inventory_idempotent_single_ledger() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping reserve_inventory_idempotent_single_ledger: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    seed_tenant(&pool, tenant_id).await?;
    let product_id = seed_product(&pool, tenant_id).await?;
    let sku_id = seed_sku(&pool, tenant_id, product_id).await?;
    let warehouse_id = seed_warehouse(&pool, tenant_id).await?;

    let repo = InventoryRepo::new(&pool);
    repo.adjust(InventoryAdjustment {
        tenant_id,
        warehouse_id,
        sku_id,
        batch_id: None,
        qty_delta: 5,
        ref_type: None,
        ref_id: None,
    })
    .await?;

    let order_id = uuid::Uuid::new_v4();
    let order_item_id = uuid::Uuid::new_v4();
    let reservation = InventoryReservationInput {
        tenant_id,
        warehouse_id,
        sku_id,
        order_id: Some(order_id),
        order_item_id,
        qty: 2,
    };

    repo.reserve_for_order_item(reservation.clone()).await?;
    repo.reserve_for_order_item(reservation).await?;

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

    let ledger_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from inventory_ledgers
        where tenant_id = $1
          and reason_type = 'RESERVATION_CREATE'
          and ref_type = 'ORDER_ITEM'
          and ref_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_item_id.to_string())
    .fetch_one(&pool)
    .await?;
    assert_eq!(ledger_count, 1);
    Ok(())
}

#[tokio::test]
async fn consume_inventory_idempotent_single_ledger() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping consume_inventory_idempotent_single_ledger: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    seed_tenant(&pool, tenant_id).await?;
    let product_id = seed_product(&pool, tenant_id).await?;
    let sku_id = seed_sku(&pool, tenant_id, product_id).await?;
    let warehouse_id = seed_warehouse(&pool, tenant_id).await?;

    let repo = InventoryRepo::new(&pool);
    repo.adjust(InventoryAdjustment {
        tenant_id,
        warehouse_id,
        sku_id,
        batch_id: None,
        qty_delta: 5,
        ref_type: None,
        ref_id: None,
    })
    .await?;

    let order_item_id = uuid::Uuid::new_v4();
    repo.reserve_for_order_item(InventoryReservationInput {
        tenant_id,
        warehouse_id,
        sku_id,
        order_id: None,
        order_item_id,
        qty: 2,
    })
    .await?;

    let action = InventoryReservationAction {
        tenant_id,
        warehouse_id,
        sku_id,
        order_item_id,
        qty: 2,
    };
    repo.consume_reservation(action.clone()).await?;
    repo.consume_reservation(action).await?;

    let balance = repo
        .get_balance(tenant_id, warehouse_id, sku_id)
        .await?
        .expect("balance exists");
    assert_eq!(balance.on_hand, 3);
    assert_eq!(balance.reserved, 0);

    let ledger_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from inventory_ledgers
        where tenant_id = $1
          and reason_type = 'SALES_SHIP'
          and ref_type = 'ORDER_ITEM'
          and ref_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_item_id.to_string())
    .fetch_one(&pool)
    .await?;
    assert_eq!(ledger_count, 1);

    Ok(())
}
