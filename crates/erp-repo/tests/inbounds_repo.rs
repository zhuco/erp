use anyhow::Result;
use erp_repo::inbounds::{
    InboundDocCreate, InboundDocItemCreate, InboundPutawayInput, InboundPutawayItem,
    InboundReceiveInput, InboundReceiveItem, InboundsRepo, INBOUND_LEDGER_KIND,
    INBOUND_STATUS_RECEIVING,
};
use erp_repo::inventory::InventoryRepo;
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
        create table if not exists inbound_docs (
          doc_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          warehouse_id uuid not null,
          doc_type text not null,
          doc_no text not null,
          status text not null default 'DRAFT',
          ref_type text,
          ref_id text,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists inbound_doc_items (
          tenant_id uuid not null,
          doc_id uuid not null,
          sku_id uuid not null,
          planned_qty bigint not null default 0,
          received_qty bigint not null default 0,
          putaway_qty bigint not null default 0,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, doc_id, sku_id)
        )
        "#,
        r#"
        create table if not exists inbound_item_ops (
          tenant_id uuid not null,
          operation_id uuid not null,
          doc_id uuid not null,
          sku_id uuid not null,
          action text not null,
          qty bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, operation_id)
        )
        "#,
        r#"
        create table if not exists inbound_putaway_tasks (
          task_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          doc_id uuid not null,
          sku_id uuid not null,
          qty bigint not null,
          from_area text,
          to_location_id uuid,
          status text not null default 'DONE',
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
        insert into skus (sku_id, tenant_id, product_id, sku_code, spec)
        values ($1, $2, $3, 'SKU-001', '{}'::jsonb)
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
async fn receive_in_batches() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping receive_in_batches: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;
    let tenant_id = TenantId(uuid::Uuid::new_v4());
    seed_tenant(&pool, tenant_id).await?;
    let product_id = seed_product(&pool, tenant_id).await?;
    let sku_id = seed_sku(&pool, tenant_id, product_id).await?;
    let warehouse_id = seed_warehouse(&pool, tenant_id).await?;

    let repo = InboundsRepo::new(&pool);
    let doc_id = uuid::Uuid::new_v4();
    repo.create_doc(InboundDocCreate {
        doc_id,
        tenant_id,
        warehouse_id,
        doc_type: "PURCHASE".to_string(),
        doc_no: "INB-RECV-001".to_string(),
        ref_type: None,
        ref_id: None,
        items: vec![InboundDocItemCreate {
            sku_id,
            planned_qty: 10,
        }],
    })
    .await?;

    let first = repo
        .receive(InboundReceiveInput {
            tenant_id,
            doc_id,
            items: vec![InboundReceiveItem {
                sku_id,
                qty: 4,
                operation_id: uuid::Uuid::new_v4(),
            }],
        })
        .await?;
    assert_eq!(first.doc.status, INBOUND_STATUS_RECEIVING);
    assert_eq!(first.items[0].received_qty, 4);
    assert_eq!(first.items[0].putaway_qty, 0);

    let second = repo
        .receive(InboundReceiveInput {
            tenant_id,
            doc_id,
            items: vec![InboundReceiveItem {
                sku_id,
                qty: 6,
                operation_id: uuid::Uuid::new_v4(),
            }],
        })
        .await?;
    assert_eq!(second.doc.status, INBOUND_STATUS_RECEIVING);
    assert_eq!(second.items[0].received_qty, 10);
    assert_eq!(second.items[0].putaway_qty, 0);

    Ok(())
}

#[tokio::test]
async fn putaway_idempotent() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping putaway_idempotent: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;
    let tenant_id = TenantId(uuid::Uuid::new_v4());
    seed_tenant(&pool, tenant_id).await?;
    let product_id = seed_product(&pool, tenant_id).await?;
    let sku_id = seed_sku(&pool, tenant_id, product_id).await?;
    let warehouse_id = seed_warehouse(&pool, tenant_id).await?;

    let repo = InboundsRepo::new(&pool);
    let doc_id = uuid::Uuid::new_v4();
    repo.create_doc(InboundDocCreate {
        doc_id,
        tenant_id,
        warehouse_id,
        doc_type: "PURCHASE".to_string(),
        doc_no: "INB-001".to_string(),
        ref_type: None,
        ref_id: None,
        items: vec![InboundDocItemCreate {
            sku_id,
            planned_qty: 10,
        }],
    })
    .await?;

    repo.receive(InboundReceiveInput {
        tenant_id,
        doc_id,
        items: vec![InboundReceiveItem {
            sku_id,
            qty: 10,
            operation_id: uuid::Uuid::new_v4(),
        }],
    })
    .await?;

    let putaway_op = uuid::Uuid::new_v4();
    repo.putaway(InboundPutawayInput {
        tenant_id,
        doc_id,
        items: vec![InboundPutawayItem {
            sku_id,
            qty: 10,
            operation_id: putaway_op,
            from_area: None,
            to_location_id: None,
        }],
    })
    .await?;

    repo.putaway(InboundPutawayInput {
        tenant_id,
        doc_id,
        items: vec![InboundPutawayItem {
            sku_id,
            qty: 10,
            operation_id: putaway_op,
            from_area: None,
            to_location_id: None,
        }],
    })
    .await?;

    let inventory_repo = InventoryRepo::new(&pool);
    let balance = inventory_repo
        .get_balance(tenant_id, warehouse_id, sku_id)
        .await?
        .expect("inventory balance");
    assert_eq!(balance.on_hand, 10);

    let detail = repo
        .get_detail(tenant_id, doc_id)
        .await?
        .expect("doc detail");
    assert_eq!(detail.items[0].putaway_qty, 10);

    let ledger_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from inventory_ledgers
        where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
          and reason_type = $4 and ref_id = $5
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(warehouse_id)
    .bind(sku_id)
    .bind(INBOUND_LEDGER_KIND)
    .bind(doc_id.to_string())
    .fetch_one(&pool)
    .await?;
    assert_eq!(ledger_count, 1);

    Ok(())
}
