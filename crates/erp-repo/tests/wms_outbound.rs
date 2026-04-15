use anyhow::Result;
use erp_repo::wms::{OutboundDocCreateInput, OutboundDocItemInput, OutboundPackInput, WmsRepo};
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
          batch_enabled boolean not null default false,
          serial_enabled boolean not null default false,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, sku_code)
        )
        "#,
        r#"
        create table if not exists sku_batches (
          batch_id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          sku_id uuid not null references skus(sku_id),
          batch_no text not null,
          production_date timestamptz,
          expire_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, sku_id, batch_no)
        )
        "#,
        r#"
        create table if not exists sku_serials (
          serial_id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          sku_id uuid not null references skus(sku_id),
          serial_no text not null,
          status text not null default 'IN_STOCK',
          ref_doc_type text,
          ref_doc_id text,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, serial_no)
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
        create table if not exists order_index (
          tenant_id uuid not null references tenants(tenant_id),
          order_id uuid not null,
          order_month date not null,
          platform text not null default 'taobao',
          shop_id uuid,
          platform_order_id text not null default '',
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_id)
        )
        "#,
        r#"
        create table if not exists orders (
          tenant_id uuid not null references tenants(tenant_id),
          order_month date not null,
          order_id uuid not null,
          status text not null,
          oms_status text not null default 'NEW',
          order_status text not null default 'PENDING_REVIEW',
          fulfillment_status text not null default 'NOT_ALLOCATED',
          exception_status text not null default 'NONE',
          review_status text not null default 'NOT_REVIEWED',
          exception_flags jsonb not null default '[]'::jsonb,
          ship_at timestamptz,
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_month, order_id)
        )
        "#,
        r#"
        create table if not exists shipments (
          shipment_id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          shop_id uuid not null,
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
        create table if not exists shipment_item_serials (
          tenant_id uuid not null references tenants(tenant_id),
          shipment_id uuid not null references shipments(shipment_id),
          order_id uuid not null,
          order_item_id uuid not null,
          sku_id uuid not null references skus(sku_id),
          serial_id uuid not null references sku_serials(serial_id),
          serial_no text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, shipment_id, serial_id)
        )
        "#,
        r#"
        create table if not exists shipment_sku_batches (
          tenant_id uuid not null references tenants(tenant_id),
          shipment_id uuid not null references shipments(shipment_id),
          sku_id uuid not null references skus(sku_id),
          batch_id uuid not null references sku_batches(batch_id),
          batch_no text not null,
          qty bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, shipment_id, sku_id, batch_id)
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
        create table if not exists outbound_docs (
          outbound_doc_id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          warehouse_id uuid not null references warehouses(warehouse_id),
          doc_type text not null,
          shipment_id uuid references shipments(shipment_id),
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
        create table if not exists pack_records (
          pack_record_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          outbound_doc_id uuid not null references outbound_docs(outbound_doc_id) on delete cascade,
          shipment_id uuid references shipments(shipment_id),
          package_no text not null,
          weight_gram int,
          operator_id uuid,
          packed_at timestamptz not null default now(),
          created_at timestamptz not null default now()
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
async fn ship_outbound_is_idempotent() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping ship_outbound_is_idempotent: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let warehouse_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();
    let shipment_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();

    sqlx::query("insert into tenants (tenant_id, name) values ($1, 'tenant')")
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;
    sqlx::query("insert into warehouses (warehouse_id, tenant_id, name) values ($1, $2, 'wh')")
        .bind(warehouse_id)
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;
    sqlx::query("insert into skus (sku_id, tenant_id, sku_code) values ($1, $2, 'SKU-1')")
        .bind(sku_id)
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into inventory_balances (tenant_id, warehouse_id, sku_id, on_hand) values ($1,$2,$3,10)",
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(warehouse_id)
    .bind(sku_id)
    .execute(&pool)
    .await?;

    sqlx::query(
        "insert into order_index (tenant_id, order_id, order_month, platform_order_id) values ($1,$2,current_date,'TB-1')",
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_id)
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into orders (tenant_id, order_month, order_id, status) values ($1,current_date,$2,'PENDING_SHIPMENT')",
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_id)
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into shipments (shipment_id, tenant_id, shop_id, order_id, platform, platform_order_id, warehouse_id) values ($1,$2,$3,$4,'taobao','TB-1',$5)",
    )
    .bind(shipment_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::new_v4())
    .bind(order_id)
    .bind(warehouse_id)
    .execute(&pool)
    .await?;

    let repo = WmsRepo::new(&pool);
    let detail = repo
        .create_outbound_doc(OutboundDocCreateInput {
            tenant_id,
            warehouse_id,
            doc_type: "SALES".to_string(),
            shipment_id: Some(shipment_id),
            doc_no: "OUT-001".to_string(),
            items: vec![OutboundDocItemInput {
                sku_id,
                planned_qty: 2,
            }],
        })
        .await?;

    sqlx::query(
        "update outbound_doc_items set picked_qty = planned_qty where tenant_id = $1 and outbound_doc_id = $2",
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(detail.doc.outbound_doc_id)
    .execute(&pool)
    .await?;

    repo.pack_outbound(OutboundPackInput {
        tenant_id,
        outbound_doc_id: detail.doc.outbound_doc_id,
        package_no: "PKG-1".to_string(),
        weight_gram: Some(120),
        operator_id: None,
        items: vec![erp_repo::wms::OutboundPackItemInput {
            sku_id,
            packed_qty: 2,
        }],
        batches: vec![],
    })
    .await?;

    repo.ship_outbound(erp_repo::wms::OutboundShipInput {
        tenant_id,
        outbound_doc_id: detail.doc.outbound_doc_id,
        shipped_at: None,
        allow_unpacked: false,
        inspect_required: false,
    })
    .await?;
    repo.ship_outbound(erp_repo::wms::OutboundShipInput {
        tenant_id,
        outbound_doc_id: detail.doc.outbound_doc_id,
        shipped_at: None,
        allow_unpacked: false,
        inspect_required: false,
    })
    .await?;

    let balance: i64 = sqlx::query_scalar(
        "select on_hand from inventory_balances where tenant_id = $1 and warehouse_id = $2 and sku_id = $3",
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(warehouse_id)
    .bind(sku_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(balance, 8);

    let ledger_count: i64 =
        sqlx::query_scalar("select count(*) from inventory_ledgers where tenant_id = $1")
            .bind(uuid::Uuid::from(tenant_id))
            .fetch_one(&pool)
            .await?;
    assert_eq!(ledger_count, 1);

    let outbox_count: i64 =
        sqlx::query_scalar("select count(*) from outbox_events where tenant_id = $1")
            .bind(uuid::Uuid::from(tenant_id))
            .fetch_one(&pool)
            .await?;
    assert_eq!(outbox_count, 1);

    Ok(())
}

#[tokio::test]
async fn pack_outbound_requires_serials() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping pack_outbound_requires_serials: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let warehouse_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();
    let shipment_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    let order_item_id = uuid::Uuid::new_v4();

    sqlx::query("insert into tenants (tenant_id, name) values ($1, 'tenant')")
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;
    sqlx::query("insert into warehouses (warehouse_id, tenant_id, name) values ($1, $2, 'wh')")
        .bind(warehouse_id)
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus (sku_id, tenant_id, sku_code, serial_enabled) values ($1, $2, 'SKU-2', true)",
    )
    .bind(sku_id)
    .bind(uuid::Uuid::from(tenant_id))
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into shipments (shipment_id, tenant_id, shop_id, order_id, platform, platform_order_id, warehouse_id) values ($1,$2,$3,$4,'taobao','TB-2',$5)",
    )
    .bind(shipment_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::new_v4())
    .bind(order_id)
    .bind(warehouse_id)
    .execute(&pool)
    .await?;

    let repo = WmsRepo::new(&pool);
    let detail = repo
        .create_outbound_doc(OutboundDocCreateInput {
            tenant_id,
            warehouse_id,
            doc_type: "SALES".to_string(),
            shipment_id: Some(shipment_id),
            doc_no: "OUT-002".to_string(),
            items: vec![OutboundDocItemInput {
                sku_id,
                planned_qty: 1,
            }],
        })
        .await?;

    sqlx::query(
        "update outbound_doc_items set picked_qty = planned_qty where tenant_id = $1 and outbound_doc_id = $2",
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(detail.doc.outbound_doc_id)
    .execute(&pool)
    .await?;

    let result = repo
        .pack_outbound(OutboundPackInput {
            tenant_id,
            outbound_doc_id: detail.doc.outbound_doc_id,
            package_no: "PKG-2".to_string(),
            weight_gram: Some(50),
            operator_id: None,
            items: vec![erp_repo::wms::OutboundPackItemInput {
                sku_id,
                packed_qty: 1,
            }],
            batches: vec![],
        })
        .await;
    let err = result.expect_err("pack should fail without serials");
    assert!(err.to_string().contains("serials required"));

    let serial_id = uuid::Uuid::new_v4();
    sqlx::query(
        "insert into sku_serials (serial_id, tenant_id, sku_id, serial_no) values ($1,$2,$3,'SN-1')",
    )
    .bind(serial_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(sku_id)
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into shipment_item_serials (tenant_id, shipment_id, order_id, order_item_id, sku_id, serial_id, serial_no) values ($1,$2,$3,$4,$5,$6,'SN-1')",
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(shipment_id)
    .bind(order_id)
    .bind(order_item_id)
    .bind(sku_id)
    .bind(serial_id)
    .execute(&pool)
    .await?;

    repo.pack_outbound(OutboundPackInput {
        tenant_id,
        outbound_doc_id: detail.doc.outbound_doc_id,
        package_no: "PKG-2".to_string(),
        weight_gram: Some(50),
        operator_id: None,
        items: vec![erp_repo::wms::OutboundPackItemInput {
            sku_id,
            packed_qty: 1,
        }],
        batches: vec![],
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn pack_outbound_records_operator() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping pack_outbound_records_operator: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let warehouse_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();
    let operator_id = uuid::Uuid::new_v4();

    sqlx::query("insert into tenants (tenant_id, name) values ($1, 'tenant')")
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;
    sqlx::query("insert into warehouses (warehouse_id, tenant_id, name) values ($1, $2, 'wh')")
        .bind(warehouse_id)
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;
    sqlx::query("insert into skus (sku_id, tenant_id, sku_code) values ($1, $2, 'SKU-OP')")
        .bind(sku_id)
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;

    let repo = WmsRepo::new(&pool);
    let detail = repo
        .create_outbound_doc(OutboundDocCreateInput {
            tenant_id,
            warehouse_id,
            doc_type: "SALES".to_string(),
            shipment_id: None,
            doc_no: "OUT-OP-1".to_string(),
            items: vec![OutboundDocItemInput {
                sku_id,
                planned_qty: 1,
            }],
        })
        .await?;

    repo.pack_outbound(OutboundPackInput {
        tenant_id,
        outbound_doc_id: detail.doc.outbound_doc_id,
        package_no: "PKG-OP-1".to_string(),
        weight_gram: None,
        operator_id: Some(operator_id),
        items: vec![erp_repo::wms::OutboundPackItemInput {
            sku_id,
            packed_qty: 1,
        }],
        batches: vec![],
    })
    .await?;

    let stored: Option<uuid::Uuid> = sqlx::query_scalar(
        "select operator_id from pack_records where tenant_id = $1 and outbound_doc_id = $2",
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(detail.doc.outbound_doc_id)
    .fetch_optional(&pool)
    .await?;
    assert_eq!(stored, Some(operator_id));

    Ok(())
}
