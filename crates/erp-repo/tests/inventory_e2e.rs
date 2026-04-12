use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::collections::HashSet;
use std::time::Duration;

async fn try_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect(&url)
        .await
        .ok()
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
        create table if not exists products (
          product_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          name text not null,
          brand text,
          category text,
          attrs jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists skus (
          sku_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          product_id uuid not null references products(product_id),
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
        create table if not exists warehouses (
          warehouse_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
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
          tenant_id uuid not null references tenants(tenant_id),
          warehouse_id uuid not null references warehouses(warehouse_id),
          sku_id uuid not null references skus(sku_id),
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
          tenant_id uuid not null references tenants(tenant_id),
          warehouse_id uuid not null references warehouses(warehouse_id),
          sku_id uuid not null references skus(sku_id),
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
          tenant_id uuid not null references tenants(tenant_id),
          warehouse_id uuid not null references warehouses(warehouse_id),
          sku_id uuid not null references skus(sku_id),
          order_id uuid,
          order_item_id uuid,
          qty bigint not null,
          status text not null default 'ACTIVE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, order_item_id)
        )
        "#,
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}

async fn seed_tenant(pool: &PgPool, tenant_id: uuid::Uuid) -> Result<()> {
    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, 'test-tenant', 'ACTIVE')
        on conflict (tenant_id) do nothing
        "#,
    )
    .bind(tenant_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn seed_product(pool: &PgPool, tenant_id: uuid::Uuid) -> Result<uuid::Uuid> {
    let product_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into products (product_id, tenant_id, name)
        values ($1, $2, 'test-product')
        "#,
    )
    .bind(product_id)
    .bind(tenant_id)
    .execute(pool)
    .await?;
    Ok(product_id)
}

async fn seed_sku(
    pool: &PgPool,
    tenant_id: uuid::Uuid,
    product_id: uuid::Uuid,
) -> Result<uuid::Uuid> {
    let sku_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into skus (sku_id, tenant_id, product_id, sku_code, name)
        values ($1, $2, $3, 'SKU-001', 'test-sku')
        "#,
    )
    .bind(sku_id)
    .bind(tenant_id)
    .bind(product_id)
    .execute(pool)
    .await?;
    Ok(sku_id)
}

async fn seed_warehouse(pool: &PgPool, tenant_id: uuid::Uuid) -> Result<uuid::Uuid> {
    let warehouse_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into warehouses (warehouse_id, tenant_id, name, code, is_default)
        values ($1, $2, 'Main Warehouse', 'WH-001', true)
        "#,
    )
    .bind(warehouse_id)
    .bind(tenant_id)
    .execute(pool)
    .await?;
    Ok(warehouse_id)
}

async fn seed_balance(
    pool: &PgPool,
    tenant_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    on_hand: i64,
    reserved: i64,
) -> Result<()> {
    sqlx::query(
        r#"
        insert into inventory_balances (tenant_id, warehouse_id, sku_id, on_hand, reserved)
        values ($1, $2, $3, $4, $5)
        on conflict (tenant_id, warehouse_id, sku_id)
        do update set on_hand = excluded.on_hand, reserved = excluded.reserved, updated_at = now()
        "#,
    )
    .bind(tenant_id)
    .bind(warehouse_id)
    .bind(sku_id)
    .bind(on_hand)
    .bind(reserved)
    .execute(pool)
    .await?;
    Ok(())
}

async fn fetch_balance(
    pool: &PgPool,
    tenant_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
) -> Result<(i64, i64)> {
    let row = sqlx::query(
        r#"
        select on_hand, reserved
        from inventory_balances
        where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
        "#,
    )
    .bind(tenant_id)
    .bind(warehouse_id)
    .bind(sku_id)
    .fetch_one(pool)
    .await?;
    Ok((row.get("on_hand"), row.get("reserved")))
}

async fn reserve_inventory(
    pool: &PgPool,
    tenant_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    order_id: uuid::Uuid,
    order_item_id: uuid::Uuid,
    qty: i64,
) -> Result<()> {
    sqlx::query(
        r#"
        update inventory_balances
        set reserved = reserved + $1, updated_at = now()
        where tenant_id = $2 and warehouse_id = $3 and sku_id = $4
        "#,
    )
    .bind(qty)
    .bind(tenant_id)
    .bind(warehouse_id)
    .bind(sku_id)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into inventory_ledgers (
            tenant_id, warehouse_id, sku_id,
            delta_on_hand, delta_reserved, delta_in_transit, delta_frozen,
            reason_type, ref_type, ref_id
        ) values ($1, $2, $3, 0, $4, 0, 0, 'RESERVATION_CREATE', 'ORDER', $5)
        "#,
    )
    .bind(tenant_id)
    .bind(warehouse_id)
    .bind(sku_id)
    .bind(qty)
    .bind(order_item_id.to_string())
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into inventory_reservations (
            tenant_id,
            warehouse_id,
            sku_id,
            order_id,
            order_item_id,
            qty,
            status
        )
        values ($1, $2, $3, $4, $5, $6, 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .bind(warehouse_id)
    .bind(sku_id)
    .bind(order_id)
    .bind(order_item_id)
    .bind(qty)
    .execute(pool)
    .await?;
    Ok(())
}

async fn release_inventory(
    pool: &PgPool,
    tenant_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    order_item_id: uuid::Uuid,
    qty: i64,
) -> Result<()> {
    sqlx::query(
        r#"
        update inventory_balances
        set reserved = reserved - $1, updated_at = now()
        where tenant_id = $2 and warehouse_id = $3 and sku_id = $4
        "#,
    )
    .bind(qty)
    .bind(tenant_id)
    .bind(warehouse_id)
    .bind(sku_id)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into inventory_ledgers (
            tenant_id, warehouse_id, sku_id,
            delta_on_hand, delta_reserved, delta_in_transit, delta_frozen,
            reason_type, ref_type, ref_id
        ) values ($1, $2, $3, 0, $4, 0, 0, 'RESERVATION_RELEASE', 'ORDER', $5)
        "#,
    )
    .bind(tenant_id)
    .bind(warehouse_id)
    .bind(sku_id)
    .bind(-qty)
    .bind(order_item_id.to_string())
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        update inventory_reservations
        set status = 'RELEASED', updated_at = now()
        where tenant_id = $1 and order_item_id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(order_item_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn ship_inventory(
    pool: &PgPool,
    tenant_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    order_item_id: uuid::Uuid,
    qty: i64,
) -> Result<()> {
    sqlx::query(
        r#"
        update inventory_balances
        set on_hand = on_hand - $1, reserved = reserved - $1, updated_at = now()
        where tenant_id = $2 and warehouse_id = $3 and sku_id = $4
        "#,
    )
    .bind(qty)
    .bind(tenant_id)
    .bind(warehouse_id)
    .bind(sku_id)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into inventory_ledgers (
            tenant_id, warehouse_id, sku_id,
            delta_on_hand, delta_reserved, delta_in_transit, delta_frozen,
            reason_type, ref_type, ref_id
        ) values ($1, $2, $3, -$4, -$4, 0, 0, 'SALES_SHIP', 'ORDER', $5)
        "#,
    )
    .bind(tenant_id)
    .bind(warehouse_id)
    .bind(sku_id)
    .bind(qty)
    .bind(order_item_id.to_string())
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        update inventory_reservations
        set status = 'SHIPPED', updated_at = now()
        where tenant_id = $1 and order_item_id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(order_item_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn list_ledger_kinds(
    pool: &PgPool,
    tenant_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
) -> Result<HashSet<String>> {
    let rows = sqlx::query(
        r#"
        select reason_type
        from inventory_ledgers
        where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
        "#,
    )
    .bind(tenant_id)
    .bind(warehouse_id)
    .bind(sku_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| row.get::<String, _>("reason_type"))
        .collect())
}

#[tokio::test]
async fn inventory_flow_end_to_end() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping inventory_flow_end_to_end: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = uuid::Uuid::new_v4();
    seed_tenant(&pool, tenant_id).await?;
    let product_id = seed_product(&pool, tenant_id).await?;
    let sku_id = seed_sku(&pool, tenant_id, product_id).await?;
    let warehouse_id = seed_warehouse(&pool, tenant_id).await?;
    seed_balance(&pool, tenant_id, warehouse_id, sku_id, 10, 0).await?;

    let order_id = uuid::Uuid::new_v4();
    let order_item_id = uuid::Uuid::new_v4();
    reserve_inventory(
        &pool,
        tenant_id,
        warehouse_id,
        sku_id,
        order_id,
        order_item_id,
        3,
    )
    .await?;
    let (on_hand, reserved) = fetch_balance(&pool, tenant_id, warehouse_id, sku_id).await?;
    assert_eq!(on_hand - reserved, 7);

    release_inventory(&pool, tenant_id, warehouse_id, sku_id, order_item_id, 3).await?;
    let (on_hand, reserved) = fetch_balance(&pool, tenant_id, warehouse_id, sku_id).await?;
    assert_eq!(on_hand - reserved, 10);

    let order_item_id_second = uuid::Uuid::new_v4();
    reserve_inventory(
        &pool,
        tenant_id,
        warehouse_id,
        sku_id,
        order_id,
        order_item_id_second,
        4,
    )
    .await?;
    ship_inventory(
        &pool,
        tenant_id,
        warehouse_id,
        sku_id,
        order_item_id_second,
        4,
    )
    .await?;

    let (on_hand, reserved) = fetch_balance(&pool, tenant_id, warehouse_id, sku_id).await?;
    assert_eq!(on_hand, 6);
    assert_eq!(reserved, 0);

    let kinds = list_ledger_kinds(&pool, tenant_id, warehouse_id, sku_id).await?;
    assert!(kinds.contains("RESERVATION_CREATE"));
    assert!(kinds.contains("RESERVATION_RELEASE"));
    assert!(kinds.contains("SALES_SHIP"));
    Ok(())
}
