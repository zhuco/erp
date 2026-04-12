use anyhow::Result;
use erp_repo::products::{ProductCreate, ProductListFilters, ProductUpdate, ProductsRepo};
use erp_repo::sku_batches::{SkuBatchCreate, SkuBatchListFilters, SkuBatchUpdate, SkuBatchesRepo};
use erp_repo::sku_serials::{SkuSerialGenerateInput, SkuSerialListFilters, SkuSerialsRepo};
use erp_types::TenantId;
use serde_json::json;
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
          product_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          name text not null,
          brand text,
          category text,
          attrs jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists skus (
          sku_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          product_id uuid not null references products(product_id),
          sku_code text not null,
          barcode text,
          name text,
          spec jsonb not null default '{}'::jsonb,
          weight_gram int,
          length_mm int,
          width_mm int,
          height_mm int,
          volume_cm3 int,
          sku_kind text not null default 'NORMAL',
          cost_cent bigint,
          is_active boolean not null default true,
          status text not null default 'ACTIVE',
          batch_enabled boolean not null default false,
          serial_enabled boolean not null default false,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, sku_code)
        )
        "#,
        r#"
        create table if not exists sku_batches (
          batch_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          sku_id uuid not null references skus(sku_id),
          batch_no text not null,
          expire_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists sku_serials (
          serial_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          sku_id uuid not null references skus(sku_id),
          serial_no text not null,
          status text not null default 'IN_STOCK',
          ref_doc_type text,
          ref_doc_id text,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists sku_serial_sequences (
          tenant_id uuid not null,
          sku_id uuid not null references skus(sku_id),
          prefix text not null,
          date_key text not null,
          last_seq bigint not null default 0,
          updated_at timestamptz not null default now(),
          primary key (tenant_id, sku_id, prefix, date_key)
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

async fn seed_product(pool: &PgPool, tenant_id: TenantId, name: &str) -> Result<uuid::Uuid> {
    let product_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        insert into products (product_id, tenant_id, name, brand)
        values ($1, $2, $3, 'Brand-A')
        "#,
    )
    .bind(product_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(name)
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
        values ($1, $2, $3, 'SKU-TEST', '{}'::jsonb)
        "#,
    )
    .bind(sku_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(product_id)
    .execute(pool)
    .await?;
    Ok(sku_id)
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
async fn product_repo_create_update_list() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping product_repo_create_update_list: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;
    let tenant_id = TenantId(uuid::Uuid::new_v4());
    seed_tenant(&pool, tenant_id).await?;

    let repo = ProductsRepo::new(&pool);
    let created = repo
        .create_product(ProductCreate {
            tenant_id,
            product_code: None,
            name: "Product-A".to_string(),
            alias: None,
            brand: Some("Brand-A".to_string()),
            category: Some("Cat-A".to_string()),
            base_unit: None,
            second_unit: None,
            default_supplier_id: None,
            default_warehouse_id: None,
            primary_barcode: None,
            product_tags: Vec::new(),
            remark: None,
            warning_stock: json!({}),
            batch_enabled: false,
            shelf_life_days: None,
            forbid_sale_days: None,
            serial_enabled: false,
            serial_push_platform: false,
            attrs: json!({"color": "red"}),
            allow_negative_stock: false,
        })
        .await?;

    let updated = repo
        .update_product(
            tenant_id,
            created.product_id,
            ProductUpdate {
                product_code: None,
                name: Some("Product-A+".to_string()),
                alias: None,
                brand: None,
                category: None,
                base_unit: None,
                second_unit: None,
                default_supplier_id: None,
                default_warehouse_id: None,
                primary_barcode: None,
                product_tags: None,
                remark: None,
                warning_stock: None,
                batch_enabled: None,
                shelf_life_days: None,
                forbid_sale_days: None,
                serial_enabled: None,
                serial_push_platform: None,
                attrs: None,
                status: None,
                allow_negative_stock: None,
            },
        )
        .await?
        .expect("updated");

    assert_eq!(updated.name, "Product-A+");

    let list = repo
        .list_products(
            tenant_id,
            ProductListFilters {
                keyword: Some("Product".to_string()),
                brand: Some("Brand-A".to_string()),
                category: None,
                status: None,
                cursor: None,
                limit: 20,
            },
        )
        .await?;
    assert!(!list.items.is_empty());
    Ok(())
}

#[tokio::test]
async fn sku_batch_repo_flow() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping sku_batch_repo_flow: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;
    let tenant_id = TenantId(uuid::Uuid::new_v4());
    seed_tenant(&pool, tenant_id).await?;
    let product_id = seed_product(&pool, tenant_id, "Product-B").await?;
    let sku_id = seed_sku(&pool, tenant_id, product_id).await?;

    let repo = SkuBatchesRepo::new(&pool);
    let batch = repo
        .create_batch(SkuBatchCreate {
            tenant_id,
            sku_id,
            batch_no: "BATCH-001".to_string(),
            expire_at: None,
        })
        .await?;

    let updated = repo
        .update_batch(
            tenant_id,
            batch.batch_id,
            SkuBatchUpdate {
                batch_no: Some("BATCH-002".to_string()),
                expire_at: None,
            },
        )
        .await?
        .expect("batch updated");
    assert_eq!(updated.batch_no, "BATCH-002");

    let list = repo
        .list_batches(
            tenant_id,
            SkuBatchListFilters {
                sku_id: Some(sku_id),
                batch_no: None,
                cursor: None,
                limit: 10,
            },
        )
        .await?;
    assert_eq!(list.items.len(), 1);
    Ok(())
}

#[tokio::test]
async fn sku_serial_generate_list() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping sku_serial_generate_list: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;
    let tenant_id = TenantId(uuid::Uuid::new_v4());
    seed_tenant(&pool, tenant_id).await?;
    let product_id = seed_product(&pool, tenant_id, "Product-C").await?;
    let sku_id = seed_sku(&pool, tenant_id, product_id).await?;

    let repo = SkuSerialsRepo::new(&pool);
    let serials = repo
        .generate_serials(SkuSerialGenerateInput {
            tenant_id,
            sku_id,
            prefix: "SN".to_string(),
            count: 2,
            seq_length: 6,
            increment_step: 1,
        })
        .await?;
    assert_eq!(serials.len(), 2);

    let list = repo
        .list_serials(
            tenant_id,
            SkuSerialListFilters {
                sku_id: Some(sku_id),
                status: None,
                serial_no: None,
                cursor: None,
                limit: 10,
            },
        )
        .await?;
    assert_eq!(list.items.len(), 2);
    Ok(())
}
