use anyhow::Result;
use chrono::Utc;
use erp_domain::{OrderItemProjection, OrderProjection};
use erp_repo::orders::OrdersRepo;
use erp_types::{AfterSaleState, OrderStatus, Platform, RefundState, ShopId, TenantId};
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
        values ($1, $2, 'taobao', 'SHOP-ORDER', 'shop', 'shop', 'AUTHORIZED')
        on conflict (shop_id) do nothing
        "#,
    )
    .bind(uuid::Uuid::from(shop_id))
    .bind(uuid::Uuid::from(tenant_id))
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
async fn order_upsert_idempotent_single_rows() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping order_upsert_idempotent_single_rows: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let shop_id = ShopId(uuid::Uuid::new_v4());
    seed_tenant_and_shop(&pool, tenant_id, shop_id).await?;

    let order = OrderProjection {
        tenant_id,
        shop_id,
        platform: Platform::Taobao,
        platform_order_id: "ORDER-ONE".to_string(),
        status: OrderStatus::PendingShipment,
        total_amount_cent: 200,
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
        items: vec![
            OrderItemProjection {
                line_id: "LINE-1".to_string(),
                sku_id: Some("SKU-1".to_string()),
                title: Some("Item-1".to_string()),
                qty: 1,
                price_cent: 100,
                amount_cent: 100,
                pay_amount_cent: None,
                discount_cent: None,
                refund_state: RefundState::None,
                after_sale_state: AfterSaleState::None,
                refund_amount_cent: None,
                after_sale_type: None,
                after_sale_id: None,
                raw: serde_json::json!({}),
            },
            OrderItemProjection {
                line_id: "LINE-2".to_string(),
                sku_id: Some("SKU-2".to_string()),
                title: Some("Item-2".to_string()),
                qty: 1,
                price_cent: 100,
                amount_cent: 100,
                pay_amount_cent: None,
                discount_cent: None,
                refund_state: RefundState::None,
                after_sale_state: AfterSaleState::None,
                refund_amount_cent: None,
                after_sale_type: None,
                after_sale_id: None,
                raw: serde_json::json!({}),
            },
        ],
    };

    let repo = OrdersRepo::new(&pool);
    let order_id = repo.upsert_order_projection(&order).await?;
    let order_id_again = repo.upsert_order_projection(&order).await?;
    assert_eq!(order_id, order_id_again);

    let order_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from orders
        where tenant_id = $1
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .fetch_one(&pool)
    .await?;
    assert_eq!(order_count, 1);

    let item_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from order_items
        where tenant_id = $1 and order_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .fetch_one(&pool)
    .await?;
    assert_eq!(item_count, 2);

    let mapping_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from order_key_index
        where tenant_id = $1
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .fetch_one(&pool)
    .await?;
    assert_eq!(mapping_count, 1);

    Ok(())
}
