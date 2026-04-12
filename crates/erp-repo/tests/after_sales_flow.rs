use anyhow::Result;
use erp_repo::after_sales::{
    AfterSalesCreateInput, AfterSalesItemCreate, AfterSalesRepo, AfterSalesShipmentCreate,
    RefundDocCreateInput, ReturnReceiptCreateInput,
};
use erp_types::{AfterSalesStatus, AfterSalesType, RefundChannel, TenantId};
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
        create table if not exists skus (
          sku_id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          product_id uuid,
          sku_code text not null
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
          warehouse_id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          name text not null,
          code text not null
        )
        "#,
        r#"
        create table if not exists after_sales (
          id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          order_id uuid not null,
          type text not null,
          status text not null default 'OPEN',
          after_sale_status text not null default 'SUBMITTED',
          compensation_status text,
          evidence_status text,
          reason_code text,
          amount_cent bigint not null default 0,
          raw_json jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists after_sales_items (
          tenant_id uuid not null references tenants(tenant_id),
          after_sales_id uuid not null references after_sales(id) on delete cascade,
          sku_id uuid not null references skus(sku_id),
          qty bigint not null default 0,
          refund_cent bigint not null default 0,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, after_sales_id, sku_id)
        )
        "#,
        r#"
        create table if not exists return_receipts (
          id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          after_sales_id uuid not null references after_sales(id),
          warehouse_id uuid not null references warehouses(warehouse_id),
          status text not null default 'RECEIVING',
          return_status text not null default 'WAITING_RETURN',
          received_at timestamptz,
          putaway_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, after_sales_id)
        )
        "#,
        r#"
        create table if not exists inbound_docs (
          doc_id uuid primary key,
          tenant_id uuid not null,
          warehouse_id uuid not null,
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
        create table if not exists refund_docs (
          id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          after_sales_id uuid not null references after_sales(id),
          amount_cent bigint not null,
          status text not null default 'PENDING',
          refund_status text not null default 'PENDING_APPROVAL',
          channel text not null,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, after_sales_id)
        )
        "#,
        r#"
        create table if not exists after_sales_shipments (
          tenant_id uuid not null references tenants(tenant_id),
          after_sales_id uuid not null references after_sales(id) on delete cascade,
          shipment_id uuid not null,
          shipment_type text not null,
          origin_shipment_id uuid,
          created_at timestamptz not null default now(),
          primary key (tenant_id, after_sales_id, shipment_id)
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
        .acquire_timeout(Duration::from_secs(2))
        .connect(&url)
        .await
        .ok()
}

#[tokio::test]
async fn return_receipt_updates_inventory() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping return_receipt_updates_inventory: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let sku_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();

    sqlx::query("insert into tenants (tenant_id, name, status) values ($1,'t','ACTIVE')")
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus (sku_id, tenant_id, product_id, sku_code) values ($1,$2,$3,'SKU-1')",
    )
    .bind(sku_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::new_v4())
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into warehouses (warehouse_id, tenant_id, name, code) values ($1,$2,'W','W1')",
    )
    .bind(warehouse_id)
    .bind(uuid::Uuid::from(tenant_id))
    .execute(&pool)
    .await?;

    let repo = AfterSalesRepo::new(&pool);
    let detail = repo
        .create_after_sales(AfterSalesCreateInput {
            after_sales_id: None,
            tenant_id,
            order_id: erp_types::OrderId(order_id),
            after_sales_type: AfterSalesType::ReturnRefund,
            reason_code: Some("DEFECT".to_string()),
            amount_cent: 1200,
            raw_json: None,
            compensation_status: None,
            evidence_status: None,
            items: vec![AfterSalesItemCreate {
                sku_id,
                qty: 2,
                refund_cent: 1200,
            }],
        })
        .await?;

    let updated = repo
        .receive_return(ReturnReceiptCreateInput {
            tenant_id,
            after_sales_id: detail.after_sales.id,
            warehouse_id,
            putaway_done: true,
            received_at: None,
            link_inbound: true,
        })
        .await?;

    assert_eq!(updated.after_sales.status, AfterSalesStatus::Received);
    let on_hand: i64 = sqlx::query_scalar(
        "select on_hand from inventory_balances where tenant_id = $1 and warehouse_id = $2 and sku_id = $3",
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(warehouse_id)
    .bind(sku_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(on_hand, 2);

    let inbound_count: i64 = sqlx::query_scalar(
        "select count(*) from inbound_docs where tenant_id = $1 and ref_type = 'AFTER_SALES' and ref_id = $2",
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(detail.after_sales.id.to_string())
    .fetch_one(&pool)
    .await?;
    assert_eq!(inbound_count, 1);
    Ok(())
}

#[tokio::test]
async fn refund_failed_retry_sets_pending() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping refund_failed_retry_sets_pending: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let after_sales_id = uuid::Uuid::new_v4();

    sqlx::query("insert into tenants (tenant_id, name, status) values ($1,'t','ACTIVE')")
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into after_sales (id, tenant_id, order_id, type, status, amount_cent) values ($1,$2,$3,'REFUND_ONLY','OPEN',100)",
    )
    .bind(after_sales_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::new_v4())
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into refund_docs (id, tenant_id, after_sales_id, amount_cent, status, channel) values ($1,$2,$3,100,'FAILED','MANUAL')",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(uuid::Uuid::from(tenant_id))
    .bind(after_sales_id)
    .execute(&pool)
    .await?;

    let repo = AfterSalesRepo::new(&pool);
    let (doc, should_enqueue) = repo
        .upsert_refund_doc(RefundDocCreateInput {
            tenant_id,
            after_sales_id,
            amount_cent: 100,
            channel: RefundChannel::Manual,
        })
        .await?;

    assert_eq!(doc.status.to_string(), "PENDING");
    assert!(should_enqueue);
    Ok(())
}

#[tokio::test]
async fn cancel_return_receipt_rolls_back_inventory() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping cancel_return_receipt_rolls_back_inventory: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let sku_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();

    sqlx::query("insert into tenants (tenant_id, name, status) values ($1,'t','ACTIVE')")
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus (sku_id, tenant_id, product_id, sku_code) values ($1,$2,$3,'SKU-1')",
    )
    .bind(sku_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::new_v4())
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into warehouses (warehouse_id, tenant_id, name, code) values ($1,$2,'W','W1')",
    )
    .bind(warehouse_id)
    .bind(uuid::Uuid::from(tenant_id))
    .execute(&pool)
    .await?;

    let repo = AfterSalesRepo::new(&pool);
    let detail = repo
        .create_after_sales(AfterSalesCreateInput {
            after_sales_id: None,
            tenant_id,
            order_id: erp_types::OrderId(order_id),
            after_sales_type: AfterSalesType::ReturnRefund,
            reason_code: Some("DEFECT".to_string()),
            amount_cent: 1200,
            raw_json: None,
            compensation_status: None,
            evidence_status: None,
            items: vec![AfterSalesItemCreate {
                sku_id,
                qty: 2,
                refund_cent: 1200,
            }],
        })
        .await?;

    repo.receive_return(ReturnReceiptCreateInput {
        tenant_id,
        after_sales_id: detail.after_sales.id,
        warehouse_id,
        putaway_done: true,
        received_at: None,
        link_inbound: true,
    })
    .await?;

    let cancelled = repo
        .cancel_return_receipt(tenant_id, detail.after_sales.id)
        .await?;
    assert_eq!(cancelled.after_sales.status, AfterSalesStatus::Approved);
    assert_eq!(
        cancelled
            .return_receipt
            .as_ref()
            .map(|receipt| receipt.status.to_string())
            .unwrap_or_default(),
        "CANCELLED"
    );

    let on_hand: i64 = sqlx::query_scalar(
        "select on_hand from inventory_balances where tenant_id = $1 and warehouse_id = $2 and sku_id = $3",
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(warehouse_id)
    .bind(sku_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(on_hand, 0);

    let inbound_status: Option<String> =
        sqlx::query_scalar("select status from inbound_docs where tenant_id = $1 and doc_no = $2")
            .bind(uuid::Uuid::from(tenant_id))
            .bind(format!("RETURN-{}", detail.after_sales.id))
            .fetch_optional(&pool)
            .await?;
    assert_eq!(inbound_status.as_deref(), Some("CANCELLED"));
    Ok(())
}

#[tokio::test]
async fn exchange_shipment_closes_after_sales() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping exchange_shipment_closes_after_sales: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let sku_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();

    sqlx::query("insert into tenants (tenant_id, name, status) values ($1,'t','ACTIVE')")
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus (sku_id, tenant_id, product_id, sku_code) values ($1,$2,$3,'SKU-1')",
    )
    .bind(sku_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::new_v4())
    .execute(&pool)
    .await?;

    let repo = AfterSalesRepo::new(&pool);
    let detail = repo
        .create_after_sales(AfterSalesCreateInput {
            after_sales_id: None,
            tenant_id,
            order_id: erp_types::OrderId(order_id),
            after_sales_type: AfterSalesType::Exchange,
            reason_code: Some("DEFECT".to_string()),
            amount_cent: 1200,
            raw_json: None,
            compensation_status: None,
            evidence_status: None,
            items: vec![AfterSalesItemCreate {
                sku_id,
                qty: 1,
                refund_cent: 1200,
            }],
        })
        .await?;

    repo.set_status(
        tenant_id,
        detail.after_sales.id,
        AfterSalesStatus::Approved,
        &[AfterSalesStatus::Open],
    )
    .await?;

    let shipment_id = uuid::Uuid::new_v4();
    repo.record_shipments(
        tenant_id,
        detail.after_sales.id,
        &[AfterSalesShipmentCreate {
            shipment_id,
            shipment_type: "EXCHANGE".to_string(),
            origin_shipment_id: None,
        }],
    )
    .await?;

    let closed = repo
        .set_status(
            tenant_id,
            detail.after_sales.id,
            AfterSalesStatus::Closed,
            &[AfterSalesStatus::Approved, AfterSalesStatus::Received],
        )
        .await?;
    assert_eq!(closed.status, AfterSalesStatus::Closed);

    let shipment_type: String = sqlx::query_scalar(
        "select shipment_type from after_sales_shipments where tenant_id = $1 and after_sales_id = $2 and shipment_id = $3",
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(detail.after_sales.id)
    .bind(shipment_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(shipment_type, "EXCHANGE");

    Ok(())
}

#[tokio::test]
async fn resend_shipment_closes_after_sales() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping resend_shipment_closes_after_sales: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let sku_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();

    sqlx::query("insert into tenants (tenant_id, name, status) values ($1,'t','ACTIVE')")
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus (sku_id, tenant_id, product_id, sku_code) values ($1,$2,$3,'SKU-1')",
    )
    .bind(sku_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::new_v4())
    .execute(&pool)
    .await?;

    let repo = AfterSalesRepo::new(&pool);
    let detail = repo
        .create_after_sales(AfterSalesCreateInput {
            after_sales_id: None,
            tenant_id,
            order_id: erp_types::OrderId(order_id),
            after_sales_type: AfterSalesType::Resend,
            reason_code: Some("DEFECT".to_string()),
            amount_cent: 1200,
            raw_json: None,
            compensation_status: None,
            evidence_status: None,
            items: vec![AfterSalesItemCreate {
                sku_id,
                qty: 1,
                refund_cent: 1200,
            }],
        })
        .await?;

    repo.set_status(
        tenant_id,
        detail.after_sales.id,
        AfterSalesStatus::Approved,
        &[AfterSalesStatus::Open],
    )
    .await?;

    let shipment_id = uuid::Uuid::new_v4();
    repo.record_shipments(
        tenant_id,
        detail.after_sales.id,
        &[AfterSalesShipmentCreate {
            shipment_id,
            shipment_type: "RESEND".to_string(),
            origin_shipment_id: None,
        }],
    )
    .await?;

    let closed = repo
        .set_status(
            tenant_id,
            detail.after_sales.id,
            AfterSalesStatus::Closed,
            &[AfterSalesStatus::Approved, AfterSalesStatus::Received],
        )
        .await?;
    assert_eq!(closed.status, AfterSalesStatus::Closed);

    let shipment_type: String = sqlx::query_scalar(
        "select shipment_type from after_sales_shipments where tenant_id = $1 and after_sales_id = $2 and shipment_id = $3",
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(detail.after_sales.id)
    .bind(shipment_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(shipment_type, "RESEND");

    Ok(())
}
