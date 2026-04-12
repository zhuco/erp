use anyhow::Result;
use chrono::{DateTime, TimeZone, Utc};
use erp_domain::{OrderItemProjection, OrderProjection};
use erp_repo::after_sale_cases::{AfterSaleCaseListFilters, AfterSaleCaseRepo};
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

async fn list_cases(
    repo: &AfterSaleCaseRepo<'_>,
    tenant_id: TenantId,
    platform: Platform,
    shop_id: ShopId,
) -> Result<Vec<erp_domain::AfterSaleCase>> {
    repo.list_cases(
        tenant_id,
        AfterSaleCaseListFilters {
            platform: Some(platform),
            shop_id: Some(shop_id),
            refund_state: None,
            after_sale_state: None,
            limit: 10,
            offset: 0,
        },
    )
    .await
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

#[allow(clippy::too_many_arguments)]
fn new_order_projection(
    tenant_id: TenantId,
    shop_id: ShopId,
    platform_order_id: &str,
    status: OrderStatus,
    platform_updated_at: Option<DateTime<Utc>>,
    tracking_no: Option<&str>,
    title: &str,
    after_sale_state: AfterSaleState,
) -> OrderProjection {
    let order_created_at = Utc.with_ymd_and_hms(2026, 1, 8, 10, 0, 0).unwrap();
    OrderProjection {
        tenant_id,
        shop_id,
        platform: Platform::Taobao,
        platform_order_id: platform_order_id.to_string(),
        status,
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
        tracking_no: tracking_no.map(|v| v.to_string()),
        order_created_at,
        platform_updated_at,
        raw: serde_json::json!({}),
        items: vec![OrderItemProjection {
            line_id: "LINE-1".to_string(),
            sku_id: Some("SKU-1".to_string()),
            title: Some(title.to_string()),
            qty: 2,
            price_cent: 100,
            amount_cent: 200,
            pay_amount_cent: None,
            discount_cent: None,
            refund_state: RefundState::None,
            after_sale_state,
            refund_amount_cent: None,
            after_sale_type: Some("refund".to_string()),
            after_sale_id: Some("CASE-ORDERING-1".to_string()),
            raw: serde_json::json!({}),
        }],
    }
}

#[tokio::test]
async fn order_out_of_order_keeps_newer_state() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping order_out_of_order_keeps_newer_state: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let shop_id = ShopId(uuid::Uuid::new_v4());
    seed_tenant_and_shop(&pool, tenant_id, shop_id).await?;

    let newer_updated = Utc.with_ymd_and_hms(2026, 1, 8, 10, 10, 0).unwrap();
    let older_updated = Utc.with_ymd_and_hms(2026, 1, 8, 10, 5, 0).unwrap();

    let newer = new_order_projection(
        tenant_id,
        shop_id,
        "ORDER-OUT-1",
        OrderStatus::Shipped,
        Some(newer_updated),
        Some("TRACK-NEW"),
        "New Item",
        AfterSaleState::None,
    );
    let older = new_order_projection(
        tenant_id,
        shop_id,
        "ORDER-OUT-1",
        OrderStatus::PendingShipment,
        Some(older_updated),
        Some("TRACK-OLD"),
        "Old Item",
        AfterSaleState::None,
    );

    let repo = OrdersRepo::new(&pool);
    let order_id = repo.upsert_order_projection(&newer).await?;
    let order_id_again = repo.upsert_order_projection(&older).await?;
    assert_eq!(order_id, order_id_again);

    #[derive(sqlx::FromRow)]
    struct OrderRow {
        status: String,
        tracking_no: Option<String>,
        total_amount_cent: i64,
        platform_updated_at: Option<DateTime<Utc>>,
    }

    let row: OrderRow = sqlx::query_as(
        r#"
        select o.status, o.tracking_no, o.total_amount_cent, o.platform_updated_at
        from orders o
        join order_index i
          on i.tenant_id = o.tenant_id
         and i.order_id = o.order_id
         and i.order_month = o.order_month
        where i.tenant_id = $1 and i.order_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .fetch_one(&pool)
    .await?;

    assert_eq!(row.status, "SHIPPED");
    assert_eq!(row.tracking_no.as_deref(), Some("TRACK-NEW"));
    assert_eq!(row.total_amount_cent, 200);
    assert_eq!(row.platform_updated_at, Some(newer_updated));

    #[derive(sqlx::FromRow)]
    struct ItemRow {
        title: Option<String>,
        qty: i32,
    }

    let item: ItemRow = sqlx::query_as(
        r#"
        select oi.title, oi.qty
        from order_items oi
        join order_index i
          on i.tenant_id = oi.tenant_id
         and i.order_id = oi.order_id
         and i.order_month = oi.order_month
        where i.tenant_id = $1 and i.order_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .fetch_one(&pool)
    .await?;

    assert_eq!(item.title.as_deref(), Some("New Item"));
    assert_eq!(item.qty, 2);
    Ok(())
}

#[tokio::test]
async fn aftersales_out_of_order_keeps_newer_state() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping aftersales_out_of_order_keeps_newer_state: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let shop_id = ShopId(uuid::Uuid::new_v4());
    seed_tenant_and_shop(&pool, tenant_id, shop_id).await?;

    let newer_updated = Utc.with_ymd_and_hms(2026, 1, 8, 11, 10, 0).unwrap();
    let older_updated = Utc.with_ymd_and_hms(2026, 1, 8, 11, 5, 0).unwrap();

    let newer = new_order_projection(
        tenant_id,
        shop_id,
        "ORDER-OUT-2",
        OrderStatus::Completed,
        Some(newer_updated),
        None,
        "Case Item New",
        AfterSaleState::Processing,
    );
    let older = new_order_projection(
        tenant_id,
        shop_id,
        "ORDER-OUT-2",
        OrderStatus::Completed,
        Some(older_updated),
        None,
        "Case Item Old",
        AfterSaleState::Closed,
    );

    let orders_repo = OrdersRepo::new(&pool);
    let cases_repo = AfterSaleCaseRepo::new(&pool);

    let order_id = orders_repo.upsert_order_projection(&newer).await?;
    cases_repo
        .sync_cases_for_order(tenant_id, Platform::Taobao, shop_id, order_id)
        .await?;

    let mut cases = list_cases(&cases_repo, tenant_id, Platform::Taobao, shop_id).await?;
    assert_eq!(cases.len(), 1);
    let first_case = cases.pop().unwrap();
    assert_eq!(first_case.after_sale_state, AfterSaleState::Processing);

    let order_id_again = orders_repo.upsert_order_projection(&older).await?;
    cases_repo
        .sync_cases_for_order(tenant_id, Platform::Taobao, shop_id, order_id_again)
        .await?;

    let mut cases = list_cases(&cases_repo, tenant_id, Platform::Taobao, shop_id).await?;
    assert_eq!(cases.len(), 1);
    let final_case = cases.pop().unwrap();

    assert_eq!(final_case.case_id, first_case.case_id);
    assert_eq!(final_case.after_sale_state, AfterSaleState::Processing);
    Ok(())
}
