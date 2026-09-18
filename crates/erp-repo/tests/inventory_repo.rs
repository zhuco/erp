use anyhow::Result;
use erp_repo::inventory::{
    InventoryAdjustment, InventoryError, InventoryRepo, InventoryReservationInput,
};
use erp_types::TenantId;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{sync::Arc, time::Duration};
use tokio::sync::Barrier;

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
        "alter table skus add column if not exists sku_kind text not null default 'NORMAL'",
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
    seed_sku_with_kind(pool, tenant_id, product_id, "SKU-INV", "NORMAL").await
}

async fn seed_sku_with_kind(
    pool: &PgPool,
    tenant_id: TenantId,
    product_id: uuid::Uuid,
    sku_code: &str,
    sku_kind: &str,
) -> Result<uuid::Uuid> {
    let sku_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into skus (sku_id, tenant_id, product_id, sku_code, sku_kind, spec)
        values ($1, $2, $3, $4, $5, '{}'::jsonb)
        "#,
    )
    .bind(sku_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(product_id)
    .bind(sku_code)
    .bind(sku_kind)
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

async fn seed_kit_bom(
    pool: &PgPool,
    tenant_id: TenantId,
    kit_sku_id: uuid::Uuid,
    component_sku_id: uuid::Uuid,
    qty: i64,
) -> Result<()> {
    sqlx::query(
        r#"
        insert into sku_boms (tenant_id, bundle_sku_id, component_sku_id, qty, bom_version)
        values ($1, $2, $3, $4, 1)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(kit_sku_id)
    .bind(component_sku_id)
    .bind(qty)
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
async fn concurrent_reserve_one_wins() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping concurrent_reserve_one_wins: db unavailable");
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
        qty_delta: 10,
        ref_type: None,
        ref_id: None,
    })
    .await?;

    let barrier = Arc::new(Barrier::new(2));
    let pool_a = pool.clone();
    let pool_b = pool.clone();
    let barrier_a = barrier.clone();
    let barrier_b = barrier.clone();
    let order_item_a = uuid::Uuid::new_v4();
    let order_item_b = uuid::Uuid::new_v4();

    let task_a = tokio::spawn(async move {
        barrier_a.wait().await;
        let repo = InventoryRepo::new(&pool_a);
        repo.reserve_for_order_item(InventoryReservationInput {
            tenant_id,
            warehouse_id,
            sku_id,
            order_id: None,
            order_item_id: order_item_a,
            qty: 10,
        })
        .await
    });

    let task_b = tokio::spawn(async move {
        barrier_b.wait().await;
        let repo = InventoryRepo::new(&pool_b);
        repo.reserve_for_order_item(InventoryReservationInput {
            tenant_id,
            warehouse_id,
            sku_id,
            order_id: None,
            order_item_id: order_item_b,
            qty: 10,
        })
        .await
    });

    let res_a = task_a.await.expect("task a join");
    let res_b = task_b.await.expect("task b join");

    let ok_count = [res_a.is_ok(), res_b.is_ok()]
        .into_iter()
        .filter(|ok| *ok)
        .count();
    assert_eq!(ok_count, 1);

    let err = match (res_a, res_b) {
        (Err(err), _) => err,
        (_, Err(err)) => err,
        _ => unreachable!("expected one reservation to fail"),
    };

    assert!(matches!(
        err.downcast_ref::<InventoryError>(),
        Some(InventoryError::InsufficientAvailable { .. })
    ));
    Ok(())
}

#[tokio::test]
async fn reserve_idempotent_on_order_item() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping reserve_idempotent_on_order_item: db unavailable");
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
        qty_delta: 8,
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
        qty: 4,
    })
    .await?;

    repo.reserve_for_order_item(InventoryReservationInput {
        tenant_id,
        warehouse_id,
        sku_id,
        order_id: None,
        order_item_id,
        qty: 4,
    })
    .await?;

    let balance = repo
        .get_balance(tenant_id, warehouse_id, sku_id)
        .await?
        .expect("balance exists");
    assert_eq!(balance.reserved, 4);
    Ok(())
}

#[tokio::test]
async fn reserve_kit_rolls_back_on_component_shortage() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping reserve_kit_rolls_back_on_component_shortage: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;
    let tenant_id = TenantId(uuid::Uuid::new_v4());
    seed_tenant(&pool, tenant_id).await?;
    let product_id = seed_product(&pool, tenant_id).await?;
    let warehouse_id = seed_warehouse(&pool, tenant_id).await?;

    let kit_sku_id = seed_sku_with_kind(&pool, tenant_id, product_id, "KIT-001", "KIT").await?;
    let component_a = seed_sku_with_kind(&pool, tenant_id, product_id, "COMP-A", "NORMAL").await?;
    let component_b = seed_sku_with_kind(&pool, tenant_id, product_id, "COMP-B", "NORMAL").await?;

    seed_kit_bom(&pool, tenant_id, kit_sku_id, component_a, 2).await?;
    seed_kit_bom(&pool, tenant_id, kit_sku_id, component_b, 3).await?;

    let repo = InventoryRepo::new(&pool);
    repo.adjust(InventoryAdjustment {
        tenant_id,
        warehouse_id,
        sku_id: component_a,
        batch_id: None,
        qty_delta: 10,
        ref_type: None,
        ref_id: None,
    })
    .await?;
    repo.adjust(InventoryAdjustment {
        tenant_id,
        warehouse_id,
        sku_id: component_b,
        batch_id: None,
        qty_delta: 2,
        ref_type: None,
        ref_id: None,
    })
    .await?;

    let order_item_id = uuid::Uuid::new_v4();
    let result = repo
        .reserve_for_order_item(InventoryReservationInput {
            tenant_id,
            warehouse_id,
            sku_id: kit_sku_id,
            order_id: None,
            order_item_id,
            qty: 1,
        })
        .await;

    assert!(matches!(
        result
            .err()
            .and_then(|err| err.downcast::<InventoryError>().ok()),
        Some(InventoryError::InsufficientAvailable { .. })
    ));

    let balance_a = repo
        .get_balance(tenant_id, warehouse_id, component_a)
        .await?
        .expect("component a balance");
    let balance_b = repo
        .get_balance(tenant_id, warehouse_id, component_b)
        .await?
        .expect("component b balance");

    assert_eq!(balance_a.reserved, 0);
    assert_eq!(balance_b.reserved, 0);

    let reservation_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from inventory_reservations
        where tenant_id = $1
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .fetch_one(&pool)
    .await?;

    assert_eq!(reservation_count, 0);
    Ok(())
}
