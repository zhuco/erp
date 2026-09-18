use anyhow::Result;
use chrono::Utc;
use erp_repo::inventory::{InventoryDelta, InventoryService};
use erp_repo::procurement::{
    AP_SETTLEMENT_STATUS_SETTLED, AP_STATUS_CLOSED, AP_STATUS_PAID, ApDocsRepo,
    PurchaseOrderCreate, PurchaseOrderItemInput, PurchaseOrderReceiveInput,
    PurchaseOrderReceiveItem, PurchaseOrdersRepo, PurchaseReturnCreate, PurchaseReturnsRepo,
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
        create table if not exists suppliers (
          supplier_id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          name text not null,
          contact text,
          settlement_terms text,
          status text not null default 'ACTIVE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists warehouses (
          warehouse_id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          name text not null,
          code text,
          is_default bool not null default false,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists skus (
          sku_id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          sku_code text not null,
          barcode text,
          name text,
          sku_kind text,
          cost_cent bigint,
          is_active bool not null default true,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, sku_code)
        )
        "#,
        r#"
        create table if not exists sku_cost_ledgers (
          ledger_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          sku_id uuid not null references skus(sku_id),
          previous_cost_cent bigint,
          new_cost_cent bigint not null,
          ref_type text,
          ref_id text,
          created_at timestamptz not null default now()
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
        r#"
        create table if not exists purchase_orders (
          po_id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          supplier_id uuid not null references suppliers(supplier_id),
          warehouse_id uuid references warehouses(warehouse_id),
          expected_at timestamptz,
          remark text,
          status text not null default 'DRAFT',
          purchase_order_status text not null default 'DRAFT',
          total_cent bigint not null default 0,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists purchase_order_items (
          tenant_id uuid not null references tenants(tenant_id),
          po_id uuid not null references purchase_orders(po_id) on delete cascade,
          sku_id uuid not null references skus(sku_id),
          qty bigint not null,
          price_cent bigint not null,
          received_qty bigint not null default 0,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, po_id, sku_id)
        )
        "#,
        r#"
        create table if not exists purchase_receipts (
          receipt_id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          po_id uuid not null references purchase_orders(po_id),
          warehouse_id uuid not null references warehouses(warehouse_id),
          status text not null default 'DRAFT',
          receiving_status text not null default 'NOT_RECEIVED',
          confirmed_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists receipt_items (
          tenant_id uuid not null references tenants(tenant_id),
          receipt_id uuid not null references purchase_receipts(receipt_id) on delete cascade,
          sku_id uuid not null references skus(sku_id),
          qty bigint not null,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, receipt_id, sku_id)
        )
        "#,
        r#"
        create table if not exists purchase_returns (
          return_id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          po_id uuid not null references purchase_orders(po_id),
          warehouse_id uuid not null references warehouses(warehouse_id),
          status text not null default 'DRAFT',
          confirmed_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists return_items (
          tenant_id uuid not null references tenants(tenant_id),
          return_id uuid not null references purchase_returns(return_id) on delete cascade,
          sku_id uuid not null references skus(sku_id),
          qty bigint not null,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, return_id, sku_id)
        )
        "#,
        r#"
        create table if not exists inbound_docs (
          doc_id uuid primary key,
          tenant_id uuid not null,
          warehouse_id uuid not null references warehouses(warehouse_id),
          doc_type text not null,
          doc_no text not null,
          status text not null default 'DRAFT',
          ref_type text,
          ref_id text,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, doc_no)
        )
        "#,
        r#"
        create table if not exists inbound_doc_items (
          tenant_id uuid not null,
          doc_id uuid not null references inbound_docs(doc_id) on delete cascade,
          sku_id uuid not null references skus(sku_id),
          planned_qty bigint not null default 0,
          received_qty bigint not null default 0,
          putaway_qty bigint not null default 0,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, doc_id, sku_id)
        )
        "#,
        r#"
        create table if not exists inbound_putaway_tasks (
          task_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          doc_id uuid not null references inbound_docs(doc_id) on delete cascade,
          sku_id uuid not null references skus(sku_id),
          qty bigint not null,
          from_area text,
          to_location_id uuid,
          status text not null default 'DONE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists outbound_docs (
          outbound_doc_id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          warehouse_id uuid not null references warehouses(warehouse_id),
          doc_type text not null,
          shipment_id uuid,
          doc_no text not null,
          status text not null default 'DRAFT',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists outbound_doc_items (
          tenant_id uuid not null references tenants(tenant_id),
          outbound_doc_id uuid not null references outbound_docs(outbound_doc_id) on delete cascade,
          sku_id uuid not null references skus(sku_id),
          planned_qty bigint not null,
          picked_qty bigint not null default 0,
          packed_qty bigint not null default 0,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, outbound_doc_id, sku_id)
        )
        "#,
        r#"
        create table if not exists ap_docs (
          ap_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          supplier_id uuid not null references suppliers(supplier_id),
          ref_type text not null,
          ref_id text not null,
          amount_cent bigint not null,
          status text not null default 'OPEN',
          settlement_status text not null default 'UNSETTLED',
          paid_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, ref_type, ref_id)
        )
        "#,
        r#"
        create table if not exists settings (
          setting_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          scope_type text not null,
          scope_id uuid,
          key text not null,
          value_json jsonb not null default '{}'::jsonb,
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
        .acquire_timeout(Duration::from_secs(1))
        .connect(&url)
        .await
        .ok()
}

async fn seed_base(pool: &PgPool) -> Result<(TenantId, uuid::Uuid, uuid::Uuid, uuid::Uuid)> {
    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let supplier_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind("test-tenant")
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into suppliers (supplier_id, tenant_id, name, status)
        values ($1, $2, $3, 'ACTIVE')
        "#,
    )
    .bind(supplier_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind("Test Supplier")
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into warehouses (warehouse_id, tenant_id, name)
        values ($1, $2, $3)
        "#,
    )
    .bind(warehouse_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind("Main Warehouse")
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into skus (sku_id, tenant_id, sku_code, name)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(sku_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind("SKU-001")
    .bind("Test SKU")
    .execute(pool)
    .await?;

    Ok((tenant_id, supplier_id, warehouse_id, sku_id))
}

#[tokio::test]
async fn purchase_receipt_creates_inventory_and_ap() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping purchase_receipt_creates_inventory_and_ap: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let (tenant_id, supplier_id, warehouse_id, sku_id) = seed_base(&pool).await?;

    let repo = PurchaseOrdersRepo::new(&pool);
    let detail = repo
        .create_order(PurchaseOrderCreate {
            tenant_id,
            supplier_id,
            warehouse_id,
            expected_at: None,
            remark: None,
            items: vec![PurchaseOrderItemInput {
                sku_id,
                qty: 5,
                price_cent: 120,
            }],
        })
        .await?;

    let result = repo
        .receive_order(PurchaseOrderReceiveInput {
            tenant_id,
            po_id: detail.order.po_id,
            warehouse_id,
            items: vec![PurchaseOrderReceiveItem { sku_id, qty: 5 }],
            update_cost: true,
        })
        .await?;

    let on_hand: i64 = sqlx::query_scalar(
        r#"
        select on_hand
        from inventory_balances
        where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(warehouse_id)
    .bind(sku_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(on_hand, 5);

    #[derive(sqlx::FromRow)]
    struct ApRow {
        status: String,
        amount_cent: i64,
    }

    let ap_row: ApRow = sqlx::query_as(
        r#"
        select status, amount_cent
        from ap_docs
        where tenant_id = $1 and ref_type = 'RECEIPT' and ref_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(result.receipt.receipt_id.to_string())
    .fetch_one(&pool)
    .await?;
    assert_eq!(ap_row.status, "OPEN");
    assert_eq!(ap_row.amount_cent, 600);

    let inbound_status: String = sqlx::query_scalar(
        r#"
        select status
        from inbound_docs
        where tenant_id = $1 and doc_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(result.inbound_doc_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(inbound_status, "PUTAWAY_DONE");
    assert_eq!(result.order.status, "DONE");

    let cost_cent: Option<i64> = sqlx::query_scalar(
        r#"
        select cost_cent
        from skus
        where tenant_id = $1 and sku_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(sku_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(cost_cent, Some(120));

    let ledger_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from sku_cost_ledgers
        where tenant_id = $1 and sku_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(sku_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(ledger_count, 1);

    Ok(())
}

#[tokio::test]
async fn ap_doc_mark_paid_and_close() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping ap_doc_mark_paid_and_close: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let (tenant_id, supplier_id, warehouse_id, sku_id) = seed_base(&pool).await?;

    let repo = PurchaseOrdersRepo::new(&pool);
    let detail = repo
        .create_order(PurchaseOrderCreate {
            tenant_id,
            supplier_id,
            warehouse_id,
            expected_at: None,
            remark: None,
            items: vec![PurchaseOrderItemInput {
                sku_id,
                qty: 2,
                price_cent: 200,
            }],
        })
        .await?;

    let result = repo
        .receive_order(PurchaseOrderReceiveInput {
            tenant_id,
            po_id: detail.order.po_id,
            warehouse_id,
            items: vec![PurchaseOrderReceiveItem { sku_id, qty: 2 }],
            update_cost: true,
        })
        .await?;

    let ap_id = result.ap_doc_id.expect("ap doc id");
    let ap_repo = ApDocsRepo::new(&pool);
    let paid = ap_repo
        .mark_paid(tenant_id, ap_id)
        .await?
        .expect("ap doc paid");
    assert_eq!(paid.status, AP_STATUS_PAID);
    assert_eq!(paid.settlement_status, AP_SETTLEMENT_STATUS_SETTLED);
    assert!(paid.paid_at.is_some());

    let closed = ap_repo
        .close_doc(tenant_id, ap_id)
        .await?
        .expect("ap doc closed");
    assert_eq!(closed.status, AP_STATUS_CLOSED);
    assert_eq!(closed.settlement_status, AP_SETTLEMENT_STATUS_SETTLED);

    Ok(())
}

#[tokio::test]
async fn purchase_return_confirms_and_deducts_inventory() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping purchase_return_confirms_and_deducts_inventory: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let (tenant_id, supplier_id, warehouse_id, sku_id) = seed_base(&pool).await?;

    let inventory = InventoryService::new(&pool);
    inventory
        .apply(InventoryDelta {
            tenant_id,
            warehouse_id,
            sku_id,
            delta_on_hand: 10,
            delta_reserved: 0,
            delta_in_transit: 0,
            delta_frozen: 0,
            reason_type: "SEED".to_string(),
            ref_type: None,
            ref_id: None,
            occurred_at: Utc::now(),
        })
        .await?;

    let orders_repo = PurchaseOrdersRepo::new(&pool);
    let order = orders_repo
        .create_order(PurchaseOrderCreate {
            tenant_id,
            supplier_id,
            warehouse_id,
            expected_at: None,
            remark: None,
            items: vec![PurchaseOrderItemInput {
                sku_id,
                qty: 4,
                price_cent: 150,
            }],
        })
        .await?;

    let returns_repo = PurchaseReturnsRepo::new(&pool);
    let created = returns_repo
        .create_return(PurchaseReturnCreate {
            tenant_id,
            po_id: order.order.po_id,
            warehouse_id,
            items: vec![erp_repo::procurement::PurchaseReturnItem { sku_id, qty: 3 }],
        })
        .await?;

    let confirmed = returns_repo
        .confirm_return(tenant_id, created.purchase_return.return_id)
        .await?;

    let on_hand: i64 = sqlx::query_scalar(
        r#"
        select on_hand
        from inventory_balances
        where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(warehouse_id)
    .bind(sku_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(on_hand, 7);

    let outbound_status: String = sqlx::query_scalar(
        r#"
        select status
        from outbound_docs
        where tenant_id = $1 and outbound_doc_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(confirmed.outbound_doc_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(outbound_status, "SHIPPED");

    let return_status: String = sqlx::query_scalar(
        r#"
        select status
        from purchase_returns
        where tenant_id = $1 and return_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(confirmed.purchase_return.return_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(return_status, "DONE");

    Ok(())
}
