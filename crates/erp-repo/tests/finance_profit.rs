use anyhow::Result;
use chrono::{NaiveDate, Utc};
use erp_repo::finance::ProfitSnapshotsRepo;
use erp_types::{OrderId, ShopId, TenantId};
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
          shop_name text not null default '',
          status text not null default 'ACTIVE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
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
            total_amount_cent bigint not null default 0,
          discount_amount_cent bigint,
          order_created_at timestamptz not null,
          updated_at timestamptz not null default now(),
          raw jsonb not null default '{}'::jsonb,
          primary key (tenant_id, order_month, order_id)
        )
        "#,
        r#"
        create table if not exists order_items (
          tenant_id uuid not null,
          order_month date not null,
          order_id uuid not null,
          line_id text not null,
          sku_id text,
          qty int not null default 1,
          amount_cent bigint,
          pay_amount_cent bigint,
          raw jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_month, order_id, line_id)
        )
        "#,
        r#"
        create table if not exists skus (
          sku_id uuid primary key,
          tenant_id uuid not null,
          product_id uuid,
          sku_code text not null,
          cost_cent bigint,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists platform_statement_lines (
          tenant_id uuid not null,
          statement_id uuid not null,
          platform_order_id text not null,
          gross_cent bigint not null default 0,
          fee_cent bigint not null default 0,
          refund_cent bigint not null default 0,
          net_cent bigint not null default 0,
          occurred_at timestamptz,
          raw_json jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          primary key (tenant_id, statement_id, platform_order_id)
        )
        "#,
        r#"
        create table if not exists shipments (
          shipment_id uuid primary key,
          tenant_id uuid not null,
          order_id uuid not null,
          tracking_no text,
          shipment_status text not null default 'REQUESTED',
          created_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists shipping_statement_lines (
          line_id uuid primary key,
          tenant_id uuid not null,
          tracking_no text not null,
          fee_cent bigint not null default 0,
          raw_json jsonb not null default '{}'::jsonb,
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
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(2))
        .connect(&url)
        .await
        .ok()
}

#[tokio::test]
async fn compute_profit_inputs_uses_fees_and_costs() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping compute_profit_inputs_uses_fees_and_costs: db unavailable");
        return Ok(());
    };

    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let shop_id = ShopId(uuid::Uuid::new_v4());
    let order_id = OrderId(uuid::Uuid::new_v4());
    let sku_id = uuid::Uuid::new_v4();
    let order_month = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();

    sqlx::query("insert into tenants (tenant_id, name) values ($1, 't')")
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;
    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, shop_name, status)
        values ($1, $2, 'taobao', 'S1', 'shop', 'ACTIVE')
        "#,
    )
    .bind(uuid::Uuid::from(shop_id))
    .bind(uuid::Uuid::from(tenant_id))
    .execute(&pool)
    .await?;
    sqlx::query(
        r#"
        insert into order_index (tenant_id, order_id, order_month, platform, platform_order_id, shop_id, order_created_at)
        values ($1,$2,$3,'taobao','P123',$4,$5)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .bind(order_month)
    .bind(uuid::Uuid::from(shop_id))
    .bind(Utc::now())
    .execute(&pool)
    .await?;
    sqlx::query(
        r#"
        insert into order_key_index (tenant_id, platform, platform_order_id, order_id, order_month, shop_id, order_created_at)
        values ($1,'taobao','P123',$2,$3,$4,$5)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .bind(order_month)
    .bind(uuid::Uuid::from(shop_id))
    .bind(Utc::now())
    .execute(&pool)
    .await?;
    sqlx::query(
        r#"
        insert into orders (tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status,
                            total_amount_cent, discount_amount_cent, order_created_at)
        values ($1,$2,$3,'taobao',$4,'P123','COMPLETED',1000,100,$5)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_month)
    .bind(uuid::Uuid::from(order_id))
    .bind(uuid::Uuid::from(shop_id))
    .bind(Utc::now())
    .execute(&pool)
    .await?;
    sqlx::query(
        r#"
        insert into skus (sku_id, tenant_id, product_id, sku_code, cost_cent)
        values ($1, $2, $3, 'SKU-1', 200)
        "#,
    )
    .bind(sku_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::new_v4())
    .execute(&pool)
    .await?;
    sqlx::query(
        r#"
        insert into order_items (
            tenant_id, order_month, order_id, line_id, sku_id, qty, amount_cent, pay_amount_cent
        ) values ($1,$2,$3,'L1',$4,2,1000,900)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_month)
    .bind(uuid::Uuid::from(order_id))
    .bind(sku_id.to_string())
    .execute(&pool)
    .await?;
    sqlx::query(
        r#"
        insert into order_cogs (
            tenant_id, order_id, shipment_id, sku_id, qty, unit_cost_cent, total_cost_cent,
            ref_type, ref_id
        ) values ($1,$2,$3,$4,2,200,400,'SHIPMENT_ITEM','S1')
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .bind(uuid::Uuid::new_v4())
    .bind(sku_id)
    .execute(&pool)
    .await?;
    sqlx::query(
        r#"
        insert into platform_statement_lines (tenant_id, statement_id, platform_order_id, fee_cent, net_cent)
        values ($1,$2,'P123',50,850)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::new_v4())
    .execute(&pool)
    .await?;
    sqlx::query(
        r#"
        insert into shipments (shipment_id, tenant_id, order_id, tracking_no)
        values ($1,$2,$3,'T123')
        "#,
    )
    .bind(uuid::Uuid::new_v4())
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .execute(&pool)
    .await?;
    sqlx::query(
        r#"
        insert into shipping_statement_lines (line_id, tenant_id, tracking_no, fee_cent)
        values ($1,$2,'T123',30)
        "#,
    )
    .bind(uuid::Uuid::new_v4())
    .bind(uuid::Uuid::from(tenant_id))
    .execute(&pool)
    .await?;

    let repo = ProfitSnapshotsRepo::new(&pool);
    let (revenue, cost, platform_fee, shipping_fee) =
        repo.compute_profit_inputs(tenant_id, order_id).await?;

    assert_eq!(revenue, 900);
    assert_eq!(cost, 400);
    assert_eq!(platform_fee, 50);
    assert_eq!(shipping_fee, 30);

    Ok(())
}
