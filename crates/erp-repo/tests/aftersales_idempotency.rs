use anyhow::Result;
use erp_domain::{AfterSaleCase, OrderProjection};
use erp_repo::after_sale_cases::{AfterSaleCaseListFilters, AfterSaleCaseRepo};
use erp_repo::orders::OrdersRepo;
use erp_types::{AfterSaleState, Platform, ShopId, TenantId};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::path::PathBuf;
use std::time::Duration;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn load_order_fixture(name: &str) -> Result<OrderProjection> {
    let payload = std::fs::read_to_string(fixture_path(name))?;
    Ok(serde_json::from_str(&payload)?)
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

async fn seed_tenant_and_shop(pool: &PgPool, order: &OrderProjection) -> Result<()> {
    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        on conflict (tenant_id) do nothing
        "#,
    )
    .bind(uuid::Uuid::from(order.tenant_id))
    .bind("test-tenant")
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
        values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED')
        on conflict (shop_id) do nothing
        "#,
    )
    .bind(uuid::Uuid::from(order.shop_id))
    .bind(uuid::Uuid::from(order.tenant_id))
    .bind(order.platform.to_string())
    .bind("TEST-SHOP")
    .bind("test-shop")
    .execute(pool)
    .await?;

    Ok(())
}

async fn list_cases(
    repo: &AfterSaleCaseRepo<'_>,
    tenant_id: TenantId,
    platform: Platform,
    shop_id: ShopId,
) -> Result<Vec<AfterSaleCase>> {
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

#[tokio::test]
async fn aftersales_case_idempotency_same_platform_case_id() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping aftersales_case_idempotency_same_platform_case_id: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;
    let order = load_order_fixture("aftersale_case_processing.json")?;
    seed_tenant_and_shop(&pool, &order).await?;

    let orders_repo = OrdersRepo::new(&pool);
    let cases_repo = AfterSaleCaseRepo::new(&pool);

    let order_id = orders_repo.upsert_order_projection(&order).await?;
    cases_repo
        .sync_cases_for_order(order.tenant_id, order.platform, order.shop_id, order_id)
        .await?;

    let mut cases = list_cases(&cases_repo, order.tenant_id, order.platform, order.shop_id).await?;
    assert_eq!(cases.len(), 1);
    let first_case = cases.pop().unwrap();

    let order_id_again = orders_repo.upsert_order_projection(&order).await?;
    cases_repo
        .sync_cases_for_order(
            order.tenant_id,
            order.platform,
            order.shop_id,
            order_id_again,
        )
        .await?;

    let mut cases = list_cases(&cases_repo, order.tenant_id, order.platform, order.shop_id).await?;
    assert_eq!(cases.len(), 1);
    let second_case = cases.pop().unwrap();

    assert_eq!(first_case.case_id, second_case.case_id);
    assert!(second_case.updated_at >= first_case.updated_at);
    Ok(())
}

#[tokio::test]
async fn aftersales_case_out_of_order_prefers_processing() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping aftersales_case_out_of_order_prefers_processing: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let closed_order = load_order_fixture("aftersale_case_closed.json")?;
    let processing_order = load_order_fixture("aftersale_case_processing_older.json")?;
    seed_tenant_and_shop(&pool, &closed_order).await?;

    let orders_repo = OrdersRepo::new(&pool);
    let cases_repo = AfterSaleCaseRepo::new(&pool);

    let closed_order_id = orders_repo.upsert_order_projection(&closed_order).await?;
    cases_repo
        .sync_cases_for_order(
            closed_order.tenant_id,
            closed_order.platform,
            closed_order.shop_id,
            closed_order_id,
        )
        .await?;

    let mut cases = list_cases(
        &cases_repo,
        closed_order.tenant_id,
        closed_order.platform,
        closed_order.shop_id,
    )
    .await?;
    assert_eq!(cases.len(), 1);
    let first_case = cases.pop().unwrap();

    let processing_order_id = orders_repo
        .upsert_order_projection(&processing_order)
        .await?;
    cases_repo
        .sync_cases_for_order(
            processing_order.tenant_id,
            processing_order.platform,
            processing_order.shop_id,
            processing_order_id,
        )
        .await?;

    let mut cases = list_cases(
        &cases_repo,
        processing_order.tenant_id,
        processing_order.platform,
        processing_order.shop_id,
    )
    .await?;
    assert_eq!(cases.len(), 1);
    let final_case = cases.pop().unwrap();

    assert_eq!(first_case.case_id, final_case.case_id);
    assert_eq!(final_case.after_sale_state, AfterSaleState::Processing);
    Ok(())
}
