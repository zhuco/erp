use anyhow::Result;
use chrono::Utc;
use erp_domain::{OrderItemProjection, OrderProjection};
use erp_repo::after_sale_cases::AfterSaleCaseRepo;
use erp_repo::inventory::InventoryRepo;
use erp_repo::orders::OrdersRepo;
use erp_types::{AfterSaleState, OrderStatus, Platform, RefundState, ShopId, TenantId};
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
          tenant_id uuid not null,
          platform text not null,
          platform_shop_id text not null,
          name text not null default '',
          shop_name text not null default '',
          status text not null default 'AUTHORIZED',
          default_carrier_code text,
          credentials jsonb not null default '{}'::jsonb,
          auth_meta_json jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, platform, platform_shop_id)
        )
        "#,
        r#"
        create table if not exists order_key_index (
          tenant_id uuid not null,
          platform text not null,
          platform_order_id text not null,
          order_id uuid not null,
          order_month date not null,
          shop_id uuid not null,
          order_created_at timestamptz not null,
          updated_at timestamptz not null default now(),
          primary key (tenant_id, platform, platform_order_id)
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
        create table if not exists orders (
          tenant_id uuid not null,
          order_month date not null,
          order_id uuid not null,
            platform text not null,
            shop_id uuid not null,
            platform_order_id text not null,
            status text not null,
            oms_status text not null default 'NEW',
            order_status text not null default 'PENDING_REVIEW',
            fulfillment_status text not null default 'NOT_ALLOCATED',
            exception_status text not null default 'NONE',
            review_status text not null default 'NOT_REVIEWED',
            exception_flags jsonb not null default '[]'::jsonb,
            total_amount_cent bigint not null,
          goods_amount_cent bigint,
          postage_amount_cent bigint,
          discount_amount_cent bigint,
          platform_discount_cent bigint,
          seller_discount_cent bigint,
          coupon_discount_cent bigint,
          currency text not null,
          buyer_name text,
          receiver_name text,
          receiver_phone text,
          receiver_address text,
          pay_at timestamptz,
          ship_at timestamptz,
          finish_at timestamptz,
          cancel_at timestamptz,
          buyer_message text,
          seller_remark text,
          logistics_company text,
          tracking_no text,
          order_created_at timestamptz not null,
          platform_updated_at timestamptz,
          raw jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_month, order_id),
          unique (tenant_id, order_month, platform, platform_order_id)
        )
        "#,
        r#"
        create table if not exists order_items (
          tenant_id uuid not null,
          order_month date not null,
          order_id uuid not null,
          line_id text not null,
          sku_id text,
          title text,
          qty int not null,
          price_cent bigint not null,
          amount_cent bigint not null,
          pay_amount_cent bigint,
          discount_cent bigint,
          refund_state text not null,
          after_sale_state text not null,
          refund_amount_cent bigint,
          after_sale_type text,
          after_sale_id text,
          raw jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_month, order_id, line_id)
        )
        "#,
        r#"
        create table if not exists after_sale_cases (
          tenant_id uuid not null,
          case_id uuid not null default gen_random_uuid(),
          platform text not null,
          shop_id uuid not null,
          after_sale_id text not null,
          after_sale_type text,
          refund_state text not null default 'NONE',
          after_sale_state text not null default 'NONE',
          refund_amount_cent bigint not null default 0,
          item_count int not null default 0,
          total_qty int not null default 0,
          order_count int not null default 0,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, case_id),
          unique (tenant_id, platform, shop_id, after_sale_id)
        )
        "#,
        r#"
        create table if not exists after_sale_case_items (
          tenant_id uuid not null,
          case_id uuid not null,
          order_id uuid not null,
          order_month date not null,
          line_id text not null,
          sku_id text,
          title text,
          qty int not null,
          refund_state text not null default 'NONE',
          after_sale_state text not null default 'NONE',
          refund_amount_cent bigint,
          after_sale_type text,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, case_id, order_id, line_id)
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
          sku_id uuid primary key,
          tenant_id uuid not null,
          product_id uuid not null,
          sku_code text not null,
          spec jsonb not null default '{}'::jsonb,
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
          warehouse_id uuid primary key,
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
        create unique index if not exists idx_inventory_ledgers_return_unique
          on inventory_ledgers(tenant_id, ref_type, ref_id, sku_id)
          where ref_type = 'AFTER_SALE_CASE'
        "#,
        r#"
        create table if not exists shipments (
          shipment_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
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
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}

async fn seed_tenant_and_shop(pool: &PgPool, tenant_id: TenantId, shop_id: ShopId) -> Result<()> {
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

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
        values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED')
        on conflict (shop_id) do nothing
        "#,
    )
    .bind(uuid::Uuid::from(shop_id))
    .bind(uuid::Uuid::from(tenant_id))
    .bind(Platform::Taobao.to_string())
    .bind("TEST-SHOP")
    .bind("test-shop")
    .execute(pool)
    .await?;

    Ok(())
}

async fn seed_sku_and_warehouse(
    pool: &PgPool,
    tenant_id: TenantId,
) -> Result<(uuid::Uuid, uuid::Uuid)> {
    let product_id = uuid::Uuid::new_v4();
    let sku_id = uuid::Uuid::new_v4();
    let warehouse_id = uuid::Uuid::new_v4();

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

    sqlx::query(
        r#"
        insert into skus (sku_id, tenant_id, product_id, sku_code, spec)
        values ($1, $2, $3, $4, '{}'::jsonb)
        "#,
    )
    .bind(sku_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(product_id)
    .bind("SKU-RET-1")
    .execute(pool)
    .await?;

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

    Ok((sku_id, warehouse_id))
}

#[allow(clippy::too_many_arguments)]
fn build_order(
    tenant_id: TenantId,
    shop_id: ShopId,
    platform_order_id: &str,
    sku_id: uuid::Uuid,
    qty: i32,
    refund_state: RefundState,
    after_sale_type: Option<&str>,
    after_sale_id: &str,
) -> OrderProjection {
    OrderProjection {
        tenant_id,
        shop_id,
        platform: Platform::Taobao,
        platform_order_id: platform_order_id.to_string(),
        status: OrderStatus::Completed,
        total_amount_cent: 1000,
        goods_amount_cent: None,
        postage_amount_cent: None,
        discount_amount_cent: None,
        platform_discount_cent: None,
        seller_discount_cent: None,
        coupon_discount_cent: None,
        currency: "CNY".to_string(),
        buyer_name: None,
        receiver_name: None,
        receiver_phone: None,
        receiver_address: None,
        pay_at: None,
        ship_at: None,
        finish_at: None,
        cancel_at: None,
        buyer_message: None,
        seller_remark: None,
        logistics_company: None,
        tracking_no: None,
        order_created_at: Utc::now(),
        platform_updated_at: None,
        raw: serde_json::json!({}),
        items: vec![OrderItemProjection {
            line_id: "LINE-1".to_string(),
            sku_id: Some(sku_id.to_string()),
            title: Some("Return Item".to_string()),
            qty,
            price_cent: 1000,
            amount_cent: 1000,
            pay_amount_cent: None,
            discount_cent: None,
            refund_state,
            after_sale_state: AfterSaleState::Success,
            refund_amount_cent: Some(1000),
            after_sale_type: after_sale_type.map(|v| v.to_string()),
            after_sale_id: Some(after_sale_id.to_string()),
            raw: serde_json::json!({}),
        }],
    }
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
async fn refund_only_does_not_stock() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping refund_only_does_not_stock: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;
    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let shop_id = ShopId(uuid::Uuid::new_v4());
    seed_tenant_and_shop(&pool, tenant_id, shop_id).await?;
    let (sku_id, warehouse_id) = seed_sku_and_warehouse(&pool, tenant_id).await?;

    let order = build_order(
        tenant_id,
        shop_id,
        "ORDER-REFUND-ONLY",
        sku_id,
        2,
        RefundState::Success,
        Some("refund"),
        "CASE-REFUND-1",
    );

    let orders_repo = OrdersRepo::new(&pool);
    let cases_repo = AfterSaleCaseRepo::new(&pool);
    let inventory_repo = InventoryRepo::new(&pool);

    let order_id = orders_repo.upsert_order_projection(&order).await?;
    cases_repo
        .sync_cases_for_order(tenant_id, order.platform, shop_id, order_id)
        .await?;

    let balance = inventory_repo
        .get_balance(tenant_id, warehouse_id, sku_id)
        .await?;
    assert!(balance.is_none());

    let case_id: uuid::Uuid = sqlx::query_scalar(
        r#"
        select case_id
        from after_sale_cases
        where tenant_id = $1
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .fetch_one(&pool)
    .await?;

    #[derive(sqlx::FromRow)]
    struct LedgerRow {
        qty: i64,
    }

    let ledger: LedgerRow = sqlx::query_as(
        r#"
        select delta_on_hand as qty
        from inventory_ledgers
        where tenant_id = $1 and ref_type = 'AFTER_SALE_CASE' and ref_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(case_id.to_string())
    .fetch_one(&pool)
    .await?;

    assert_eq!(ledger.qty, 0);
    Ok(())
}

#[tokio::test]
async fn return_refund_stocks_inventory() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping return_refund_stocks_inventory: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;
    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let shop_id = ShopId(uuid::Uuid::new_v4());
    seed_tenant_and_shop(&pool, tenant_id, shop_id).await?;
    let (sku_id, warehouse_id) = seed_sku_and_warehouse(&pool, tenant_id).await?;

    let order = build_order(
        tenant_id,
        shop_id,
        "ORDER-RETURN-1",
        sku_id,
        3,
        RefundState::Success,
        Some("return_refund"),
        "CASE-RETURN-1",
    );

    let orders_repo = OrdersRepo::new(&pool);
    let cases_repo = AfterSaleCaseRepo::new(&pool);
    let inventory_repo = InventoryRepo::new(&pool);

    let order_id = orders_repo.upsert_order_projection(&order).await?;
    cases_repo
        .sync_cases_for_order(tenant_id, order.platform, shop_id, order_id)
        .await?;

    let balance = inventory_repo
        .get_balance(tenant_id, warehouse_id, sku_id)
        .await?
        .expect("balance exists");
    assert_eq!(balance.on_hand, 3);

    let case_id: uuid::Uuid = sqlx::query_scalar(
        r#"
        select case_id
        from after_sale_cases
        where tenant_id = $1
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .fetch_one(&pool)
    .await?;

    #[derive(sqlx::FromRow)]
    struct LedgerRow {
        qty: i64,
        warehouse_id: uuid::Uuid,
    }

    let ledger: LedgerRow = sqlx::query_as(
        r#"
        select delta_on_hand as qty, warehouse_id
        from inventory_ledgers
        where tenant_id = $1 and ref_type = 'AFTER_SALE_CASE' and ref_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(case_id.to_string())
    .fetch_one(&pool)
    .await?;

    assert_eq!(ledger.qty, 3);
    assert_eq!(ledger.warehouse_id, warehouse_id);

    let detail = cases_repo
        .get_case_detail(tenant_id, erp_types::AfterSaleCaseId(case_id))
        .await?
        .expect("case detail");
    let item = detail.items.first().expect("case item");
    assert!(item.return_stocked);
    assert_eq!(item.return_warehouse_id, Some(warehouse_id));
    assert!(item.return_at.is_some());
    Ok(())
}

#[tokio::test]
async fn refund_success_is_idempotent() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping refund_success_is_idempotent: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;
    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let shop_id = ShopId(uuid::Uuid::new_v4());
    seed_tenant_and_shop(&pool, tenant_id, shop_id).await?;
    let (sku_id, warehouse_id) = seed_sku_and_warehouse(&pool, tenant_id).await?;

    let order = build_order(
        tenant_id,
        shop_id,
        "ORDER-RETURN-2",
        sku_id,
        1,
        RefundState::Success,
        Some("return_refund"),
        "CASE-RETURN-2",
    );

    let orders_repo = OrdersRepo::new(&pool);
    let cases_repo = AfterSaleCaseRepo::new(&pool);
    let inventory_repo = InventoryRepo::new(&pool);

    let order_id = orders_repo.upsert_order_projection(&order).await?;
    cases_repo
        .sync_cases_for_order(tenant_id, order.platform, shop_id, order_id)
        .await?;

    let order_id_again = orders_repo.upsert_order_projection(&order).await?;
    cases_repo
        .sync_cases_for_order(tenant_id, order.platform, shop_id, order_id_again)
        .await?;

    let balance = inventory_repo
        .get_balance(tenant_id, warehouse_id, sku_id)
        .await?
        .expect("balance exists");
    assert_eq!(balance.on_hand, 1);

    let ledger_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from inventory_ledgers
        where tenant_id = $1 and ref_type = 'AFTER_SALE_CASE'
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .fetch_one(&pool)
    .await?;

    assert_eq!(ledger_count, 1);
    Ok(())
}
