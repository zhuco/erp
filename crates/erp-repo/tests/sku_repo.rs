use anyhow::Result;
use erp_repo::channel_sku_mappings::{
    ChannelSkuMappingCreate, ChannelSkuMappingListFilters, ChannelSkuMappingsRepo,
};
use erp_repo::skus::{SkuCreate, SkusRepo};
use erp_types::{Platform, ShopId, TenantId};
use serde_json::json;
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
        "alter table skus add column if not exists sku_kind text not null default 'NORMAL'",
        r#"
        create table if not exists channel_sku_mappings (
          mapping_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          shop_id uuid not null,
          platform text not null,
          platform_sku_id text not null,
          platform_outer_sku_id text,
          sku_id uuid not null,
          status text not null default 'ACTIVE',
          confidence int not null default 0,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, shop_id, platform, platform_sku_id)
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

async fn seed_shop(
    pool: &PgPool,
    tenant_id: TenantId,
    shop_id: ShopId,
    platform: Platform,
) -> Result<()> {
    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
        values ($1, $2, $3, 'SHOP', 'test-shop', 'test-shop', 'AUTHORIZED')
        on conflict (shop_id) do nothing
        "#,
    )
    .bind(uuid::Uuid::from(shop_id))
    .bind(uuid::Uuid::from(tenant_id))
    .bind(platform.to_string())
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
async fn sku_unique_constraint_per_tenant() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping sku_unique_constraint_per_tenant: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;
    let tenant_id = TenantId(uuid::Uuid::new_v4());
    seed_tenant(&pool, tenant_id).await?;
    let product_id = seed_product(&pool, tenant_id).await?;

    let repo = SkusRepo::new(&pool);
    let input = SkuCreate {
        tenant_id,
        product_id,
        sku_code: "SKU-001".to_string(),
        barcode: None,
        name: Some("sku".to_string()),
        note: None,
        spec: json!({}),
        tags: vec![],
        image_urls: vec![],
        weight_gram: None,
        length_mm: None,
        width_mm: None,
        height_mm: None,
        volume_cm3: None,
        sku_kind: "NORMAL".to_string(),
        cost_cent: None,
        is_active: true,
        status: None,
        batch_enabled: false,
        serial_enabled: false,
    };
    repo.create_sku(input).await?;

    let dup = repo
        .create_sku(SkuCreate {
            tenant_id,
            product_id,
            sku_code: "SKU-001".to_string(),
            barcode: None,
            name: Some("sku-dup".to_string()),
            note: None,
            spec: json!({}),
            tags: vec![],
            image_urls: vec![],
            weight_gram: None,
            length_mm: None,
            width_mm: None,
            height_mm: None,
            volume_cm3: None,
            sku_kind: "NORMAL".to_string(),
            cost_cent: None,
            is_active: true,
            status: None,
            batch_enabled: false,
            serial_enabled: false,
        })
        .await;

    assert!(dup.is_err());
    Ok(())
}

#[tokio::test]
async fn channel_mapping_filter_by_shop() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping channel_mapping_filter_by_shop: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;
    let tenant_id = TenantId(uuid::Uuid::new_v4());
    seed_tenant(&pool, tenant_id).await?;
    let product_id = seed_product(&pool, tenant_id).await?;

    let shop_a = ShopId(uuid::Uuid::new_v4());
    let shop_b = ShopId(uuid::Uuid::new_v4());
    seed_shop(&pool, tenant_id, shop_a, Platform::Taobao).await?;
    seed_shop(&pool, tenant_id, shop_b, Platform::Taobao).await?;

    let sku_repo = SkusRepo::new(&pool);
    let sku_a = sku_repo
        .create_sku(SkuCreate {
            tenant_id,
            product_id,
            sku_code: "SKU-A".to_string(),
            barcode: None,
            name: None,
            note: None,
            spec: json!({}),
            tags: vec![],
            image_urls: vec![],
            weight_gram: None,
            length_mm: None,
            width_mm: None,
            height_mm: None,
            volume_cm3: None,
            sku_kind: "NORMAL".to_string(),
            cost_cent: None,
            is_active: true,
            status: None,
            batch_enabled: false,
            serial_enabled: false,
        })
        .await?;
    let sku_b = sku_repo
        .create_sku(SkuCreate {
            tenant_id,
            product_id,
            sku_code: "SKU-B".to_string(),
            barcode: None,
            name: None,
            note: None,
            spec: json!({}),
            tags: vec![],
            image_urls: vec![],
            weight_gram: None,
            length_mm: None,
            width_mm: None,
            height_mm: None,
            volume_cm3: None,
            sku_kind: "NORMAL".to_string(),
            cost_cent: None,
            is_active: true,
            status: None,
            batch_enabled: false,
            serial_enabled: false,
        })
        .await?;

    let mapping_repo = ChannelSkuMappingsRepo::new(&pool);
    mapping_repo
        .create_mapping(ChannelSkuMappingCreate {
            tenant_id,
            shop_id: shop_a,
            platform: Platform::Taobao,
            platform_sku_id: "P-SKU-A".to_string(),
            platform_outer_sku_id: None,
            sku_id: sku_a.sku_id,
            status: "ACTIVE".to_string(),
            conflict_reason: None,
            confidence: 80,
            match_source: "MANUAL".to_string(),
        })
        .await?;
    mapping_repo
        .create_mapping(ChannelSkuMappingCreate {
            tenant_id,
            shop_id: shop_b,
            platform: Platform::Taobao,
            platform_sku_id: "P-SKU-B".to_string(),
            platform_outer_sku_id: None,
            sku_id: sku_b.sku_id,
            status: "ACTIVE".to_string(),
            conflict_reason: None,
            confidence: 80,
            match_source: "MANUAL".to_string(),
        })
        .await?;

    let list = mapping_repo
        .list_mappings(
            tenant_id,
            ChannelSkuMappingListFilters {
                shop_id: Some(shop_a),
                platform: Some(Platform::Taobao),
                sku_id: None,
                platform_sku_id: None,
                platform_outer_sku_id: None,
                status: None,
                cursor: None,
                limit: 10,
            },
        )
        .await?;

    assert_eq!(list.items.len(), 1);
    assert_eq!(list.items[0].shop_id, shop_a);
    Ok(())
}
