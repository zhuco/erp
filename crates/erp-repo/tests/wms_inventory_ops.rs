use anyhow::Result;
use erp_repo::inventory_freezes::{InventoryFreezeCreateInput, InventoryFreezesRepo};
use erp_repo::stock_counts::{StockCountCreateInput, StockCountItemUpdate, StockCountsRepo};
use erp_repo::stock_moves::{StockMoveCreateInput, StockMoveItemInput, StockMovesRepo};
use erp_repo::stock_transfers::{
    StockTransferCreateInput, StockTransferItemInput, StockTransfersRepo,
};
use erp_types::TenantId;
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
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
          name text,
          status text not null default 'ACTIVE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, sku_code)
        )
        "#,
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
          frozen bigint not null default 0,
          in_transit bigint not null default 0,
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
          delta_on_hand bigint not null default 0,
          delta_reserved bigint not null default 0,
          delta_in_transit bigint not null default 0,
          delta_frozen bigint not null default 0,
          reason_type text,
          ref_type text,
          ref_id text,
          occurred_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists channel_sku_mappings (
          mapping_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          sku_id uuid not null references skus(sku_id),
          status text not null default 'INACTIVE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
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
        r#"
        create table if not exists stock_counts (
          count_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          warehouse_id uuid not null references warehouses(warehouse_id),
          status text not null default 'DRAFT',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists stock_count_items (
          count_item_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          count_id uuid not null references stock_counts(count_id),
          sku_id uuid not null references skus(sku_id),
          location_id uuid,
          system_qty bigint not null default 0,
          counted_qty bigint not null default 0,
          diff_qty bigint not null default 0,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, count_id, sku_id, location_id)
        )
        "#,
        r#"
        create table if not exists stock_transfers (
          transfer_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          from_warehouse_id uuid not null references warehouses(warehouse_id),
          to_warehouse_id uuid not null references warehouses(warehouse_id),
          status text not null default 'DRAFT',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists stock_transfer_items (
          transfer_item_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          transfer_id uuid not null references stock_transfers(transfer_id),
          sku_id uuid not null references skus(sku_id),
          qty bigint not null,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, transfer_id, sku_id)
        )
        "#,
        r#"
        create table if not exists inventory_freezes (
          freeze_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          warehouse_id uuid not null references warehouses(warehouse_id),
          sku_id uuid not null references skus(sku_id),
          qty bigint not null,
          reason text,
          status text not null default 'ACTIVE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists warehouse_locations (
          location_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          warehouse_id uuid not null references warehouses(warehouse_id),
          code text not null,
          name text,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, warehouse_id, code)
        )
        "#,
        r#"
        create table if not exists inventory_location_balances (
          tenant_id uuid not null references tenants(tenant_id),
          warehouse_id uuid not null references warehouses(warehouse_id),
          location_id uuid not null references warehouse_locations(location_id),
          sku_id uuid not null references skus(sku_id),
          on_hand bigint not null default 0,
          updated_at timestamptz not null default now(),
          primary key (tenant_id, warehouse_id, location_id, sku_id)
        )
        "#,
        r#"
        create table if not exists stock_move_docs (
          move_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          warehouse_id uuid not null references warehouses(warehouse_id),
          from_location_id uuid references warehouse_locations(location_id),
          to_location_id uuid references warehouse_locations(location_id),
          status text not null default 'DRAFT',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists stock_move_items (
          move_item_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          move_id uuid not null references stock_move_docs(move_id),
          sku_id uuid not null references skus(sku_id),
          qty bigint not null,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, move_id, sku_id)
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
    code: &str,
) -> Result<uuid::Uuid> {
    let sku_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into skus (sku_id, tenant_id, product_id, sku_code, name)
        values ($1, $2, $3, $4, $4)
        "#,
    )
    .bind(sku_id)
    .bind(tenant_id)
    .bind(product_id)
    .bind(code)
    .execute(pool)
    .await?;
    Ok(sku_id)
}

async fn seed_warehouse(pool: &PgPool, tenant_id: uuid::Uuid, code: &str) -> Result<uuid::Uuid> {
    let warehouse_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into warehouses (warehouse_id, tenant_id, name, code)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(warehouse_id)
    .bind(tenant_id)
    .bind(code)
    .bind(code)
    .execute(pool)
    .await?;
    Ok(warehouse_id)
}

async fn seed_location(
    pool: &PgPool,
    tenant_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    code: &str,
) -> Result<uuid::Uuid> {
    let location_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into warehouse_locations (location_id, tenant_id, warehouse_id, code, name)
        values ($1, $2, $3, $4, $4)
        "#,
    )
    .bind(location_id)
    .bind(tenant_id)
    .bind(warehouse_id)
    .bind(code)
    .execute(pool)
    .await?;
    Ok(location_id)
}

async fn seed_location_balance(
    pool: &PgPool,
    tenant_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    location_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    on_hand: i64,
) -> Result<()> {
    sqlx::query(
        r#"
        insert into inventory_location_balances (
            tenant_id, warehouse_id, location_id, sku_id, on_hand
        ) values ($1, $2, $3, $4, $5)
        on conflict (tenant_id, warehouse_id, location_id, sku_id)
        do update set on_hand = excluded.on_hand, updated_at = now()
        "#,
    )
    .bind(tenant_id)
    .bind(warehouse_id)
    .bind(location_id)
    .bind(sku_id)
    .bind(on_hand)
    .execute(pool)
    .await?;
    Ok(())
}

async fn seed_balance(
    pool: &PgPool,
    tenant_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    on_hand: i64,
) -> Result<()> {
    sqlx::query(
        r#"
        insert into inventory_balances (tenant_id, warehouse_id, sku_id, on_hand)
        values ($1, $2, $3, $4)
        on conflict (tenant_id, warehouse_id, sku_id)
        do update set on_hand = excluded.on_hand, updated_at = now()
        "#,
    )
    .bind(tenant_id)
    .bind(warehouse_id)
    .bind(sku_id)
    .bind(on_hand)
    .execute(pool)
    .await?;
    Ok(())
}

async fn fetch_location_balance(
    pool: &PgPool,
    tenant_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    location_id: uuid::Uuid,
    sku_id: uuid::Uuid,
) -> Result<i64> {
    let on_hand: i64 = sqlx::query_scalar(
        r#"
        select on_hand
        from inventory_location_balances
        where tenant_id = $1 and warehouse_id = $2
          and location_id = $3 and sku_id = $4
        "#,
    )
    .bind(tenant_id)
    .bind(warehouse_id)
    .bind(location_id)
    .bind(sku_id)
    .fetch_one(pool)
    .await?;
    Ok(on_hand)
}

async fn fetch_balance(
    pool: &PgPool,
    tenant_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
) -> Result<(i64, i64, i64, i64)> {
    let row = sqlx::query(
        r#"
        select on_hand, reserved, frozen, in_transit
        from inventory_balances
        where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
        "#,
    )
    .bind(tenant_id)
    .bind(warehouse_id)
    .bind(sku_id)
    .fetch_one(pool)
    .await?;
    Ok((
        row.get("on_hand"),
        row.get("reserved"),
        row.get("frozen"),
        row.get("in_transit"),
    ))
}

#[tokio::test]
async fn stock_count_apply_updates_balance() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping stock_count_apply_updates_balance: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = uuid::Uuid::new_v4();
    seed_tenant(&pool, tenant_id).await?;
    let product_id = seed_product(&pool, tenant_id).await?;
    let sku_id = seed_sku(&pool, tenant_id, product_id, "SKU-COUNT").await?;
    let warehouse_id = seed_warehouse(&pool, tenant_id, "WH-COUNT").await?;
    seed_balance(&pool, tenant_id, warehouse_id, sku_id, 10).await?;

    let repo = StockCountsRepo::new(&pool);
    let count = repo
        .create_count(StockCountCreateInput {
            tenant_id: TenantId(tenant_id),
            warehouse_id,
            strategy: "FULL".to_string(),
            dimension: "SKU".to_string(),
            recent_days: None,
            brand: None,
            category: None,
            scope_type: None,
            scope_ids: None,
            count_mode: None,
            count_type: None,
            freeze_stock: None,
            created_by: None,
        })
        .await?;

    let detail = repo
        .get_detail(TenantId(tenant_id), count.count_id)
        .await?
        .expect("count detail");
    let item = detail.items.first().expect("count item");

    repo.update_items(
        TenantId(tenant_id),
        count.count_id,
        vec![StockCountItemUpdate {
            count_item_id: item.count_item_id,
            counted_qty: 8,
            diff_reason: None,
        }],
    )
    .await?;
    repo.confirm_diff(TenantId(tenant_id), count.count_id)
        .await?;
    repo.apply_count(TenantId(tenant_id), count.count_id)
        .await?;

    let (on_hand, _, _, _) = fetch_balance(&pool, tenant_id, warehouse_id, sku_id).await?;
    assert_eq!(on_hand, 8);

    let delta_on_hand: i64 = sqlx::query_scalar(
        r#"
        select coalesce(sum(delta_on_hand), 0)
        from inventory_ledgers
        where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
          and reason_type = 'STOCK_COUNT_APPLY'
        "#,
    )
    .bind(tenant_id)
    .bind(warehouse_id)
    .bind(sku_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(delta_on_hand, -2);
    Ok(())
}

#[tokio::test]
async fn stock_transfer_out_in_updates_in_transit() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping stock_transfer_out_in_updates_in_transit: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = uuid::Uuid::new_v4();
    seed_tenant(&pool, tenant_id).await?;
    let product_id = seed_product(&pool, tenant_id).await?;
    let sku_id = seed_sku(&pool, tenant_id, product_id, "SKU-TRANSFER").await?;
    let from_warehouse_id = seed_warehouse(&pool, tenant_id, "WH-FROM").await?;
    let to_warehouse_id = seed_warehouse(&pool, tenant_id, "WH-TO").await?;
    seed_balance(&pool, tenant_id, from_warehouse_id, sku_id, 10).await?;

    let repo = StockTransfersRepo::new(&pool);
    let transfer = repo
        .create_transfer(StockTransferCreateInput {
            tenant_id: TenantId(tenant_id),
            from_warehouse_id,
            to_warehouse_id,
            items: vec![StockTransferItemInput { sku_id, qty: 4 }],
        })
        .await?;

    repo.confirm_outbound(TenantId(tenant_id), transfer.transfer_id)
        .await?;
    let (from_on_hand, _, _, _) =
        fetch_balance(&pool, tenant_id, from_warehouse_id, sku_id).await?;
    let (to_on_hand, _, _, to_in_transit) =
        fetch_balance(&pool, tenant_id, to_warehouse_id, sku_id).await?;
    assert_eq!(from_on_hand, 6);
    assert_eq!(to_on_hand, 0);
    assert_eq!(to_in_transit, 4);

    repo.confirm_inbound(TenantId(tenant_id), transfer.transfer_id)
        .await?;
    let (to_on_hand, _, _, to_in_transit) =
        fetch_balance(&pool, tenant_id, to_warehouse_id, sku_id).await?;
    assert_eq!(to_on_hand, 4);
    assert_eq!(to_in_transit, 0);

    let transfer_out_ledgers: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from inventory_ledgers
        where tenant_id = $1 and sku_id = $2 and reason_type = 'TRANSFER_OUT' and ref_id = $3
        "#,
    )
    .bind(tenant_id)
    .bind(sku_id)
    .bind(transfer.transfer_id.to_string())
    .fetch_one(&pool)
    .await?;
    assert_eq!(transfer_out_ledgers, 1);

    let transfer_in_transit_ledgers: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from inventory_ledgers
        where tenant_id = $1
          and sku_id = $2
          and reason_type = 'TRANSFER_IN_TRANSIT'
          and ref_id = $3
        "#,
    )
    .bind(tenant_id)
    .bind(sku_id)
    .bind(transfer.transfer_id.to_string())
    .fetch_one(&pool)
    .await?;
    assert_eq!(transfer_in_transit_ledgers, 1);

    let transfer_in_ledgers: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from inventory_ledgers
        where tenant_id = $1 and sku_id = $2 and reason_type = 'TRANSFER_IN' and ref_id = $3
        "#,
    )
    .bind(tenant_id)
    .bind(sku_id)
    .bind(transfer.transfer_id.to_string())
    .fetch_one(&pool)
    .await?;
    assert_eq!(transfer_in_ledgers, 1);
    Ok(())
}

#[tokio::test]
async fn inventory_freeze_updates_available() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping inventory_freeze_updates_available: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = uuid::Uuid::new_v4();
    seed_tenant(&pool, tenant_id).await?;
    let product_id = seed_product(&pool, tenant_id).await?;
    let sku_id = seed_sku(&pool, tenant_id, product_id, "SKU-FREEZE").await?;
    let warehouse_id = seed_warehouse(&pool, tenant_id, "WH-FREEZE").await?;
    seed_balance(&pool, tenant_id, warehouse_id, sku_id, 12).await?;

    let repo = InventoryFreezesRepo::new(&pool);
    repo.create_freeze(InventoryFreezeCreateInput {
        tenant_id: TenantId(tenant_id),
        warehouse_id,
        sku_id,
        qty: 3,
        reason: Some("quality_hold".to_string()),
    })
    .await?;

    let (on_hand, reserved, frozen, _) =
        fetch_balance(&pool, tenant_id, warehouse_id, sku_id).await?;
    assert_eq!(on_hand - reserved - frozen, 9);
    assert_eq!(frozen, 3);
    Ok(())
}

#[tokio::test]
async fn stock_move_apply_writes_ledger() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping stock_move_apply_writes_ledger: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = uuid::Uuid::new_v4();
    seed_tenant(&pool, tenant_id).await?;
    let product_id = seed_product(&pool, tenant_id).await?;
    let sku_id = seed_sku(&pool, tenant_id, product_id, "SKU-MOVE").await?;
    let warehouse_id = seed_warehouse(&pool, tenant_id, "WH-MOVE").await?;
    let from_location_id = seed_location(&pool, tenant_id, warehouse_id, "LOC-A").await?;
    let to_location_id = seed_location(&pool, tenant_id, warehouse_id, "LOC-B").await?;
    seed_location_balance(&pool, tenant_id, warehouse_id, from_location_id, sku_id, 5).await?;

    let repo = StockMovesRepo::new(&pool);
    let doc = repo
        .create_move(StockMoveCreateInput {
            tenant_id: TenantId(tenant_id),
            warehouse_id,
            from_location_id: Some(from_location_id),
            to_location_id: Some(to_location_id),
            items: vec![StockMoveItemInput { sku_id, qty: 2 }],
        })
        .await?;

    repo.apply_move(TenantId(tenant_id), doc.move_id).await?;

    let from_on_hand =
        fetch_location_balance(&pool, tenant_id, warehouse_id, from_location_id, sku_id).await?;
    let to_on_hand =
        fetch_location_balance(&pool, tenant_id, warehouse_id, to_location_id, sku_id).await?;
    assert_eq!(from_on_hand, 3);
    assert_eq!(to_on_hand, 2);

    let ledger_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from inventory_ledgers
        where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
          and reason_type = 'STOCK_MOVE_APPLY' and ref_id = $4
        "#,
    )
    .bind(tenant_id)
    .bind(warehouse_id)
    .bind(sku_id)
    .bind(doc.move_id.to_string())
    .fetch_one(&pool)
    .await?;
    assert_eq!(ledger_count, 1);
    Ok(())
}
