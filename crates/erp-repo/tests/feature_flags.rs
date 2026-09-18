use anyhow::Result;
use erp_repo::feature_flags::{FeatureFlag, FeatureFlagsRepo};
use erp_types::{ShopId, TenantId};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::str::FromStr;
use std::time::Duration;

async fn setup_schema(pool: &PgPool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS feature_flags (
          tenant_id UUID NOT NULL,
          scope TEXT NOT NULL DEFAULT 'GLOBAL',
          shop_id UUID,
          key TEXT NOT NULL,
          enabled BOOLEAN NOT NULL DEFAULT false,
          created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
          updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
          UNIQUE (tenant_id, scope, COALESCE(shop_id, '00000000-0000-0000-0000-000000000000'::UUID), key)
        )
        "#,
    )
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
async fn read_only_flag_blocks_operations() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping read_only_flag_blocks_operations: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let repo = FeatureFlagsRepo::new(&pool);

    // Initially disabled
    let is_read_only = repo
        .is_enabled(tenant_id, None, FeatureFlag::ReadOnly)
        .await?;
    assert!(!is_read_only, "READ_ONLY should be disabled by default");

    // Enable READ_ONLY globally
    sqlx::query(
        r#"
        INSERT INTO feature_flags (tenant_id, scope, key, enabled)
        VALUES ($1, 'GLOBAL', 'READ_ONLY', true)
        "#,
    )
    .bind(uuid::Uuid::from_str("00000000-0000-0000-0000-000000000000").unwrap())
    .execute(&pool)
    .await?;

    // Now should be enabled
    let is_read_only = repo
        .is_enabled(tenant_id, None, FeatureFlag::ReadOnly)
        .await?;
    assert!(is_read_only, "READ_ONLY should be enabled globally");

    // Disable for specific tenant
    sqlx::query(
        r#"
        INSERT INTO feature_flags (tenant_id, scope, key, enabled)
        VALUES ($1, 'TENANT', 'READ_ONLY', false)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .execute(&pool)
    .await?;

    // Should be disabled for this tenant (TENANT scope overrides GLOBAL)
    let is_read_only = repo
        .is_enabled(tenant_id, None, FeatureFlag::ReadOnly)
        .await?;
    assert!(
        !is_read_only,
        "READ_ONLY should be disabled for specific tenant"
    );

    Ok(())
}

#[tokio::test]
async fn pdd_only_mode_filters_platforms() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping pdd_only_mode_filters_platforms: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let repo = FeatureFlagsRepo::new(&pool);

    // Initially disabled
    let pdd_only = repo
        .is_enabled(tenant_id, None, FeatureFlag::PddOnly)
        .await?;
    assert!(!pdd_only, "PDD_ONLY should be disabled by default");

    // Enable PDD_ONLY globally
    sqlx::query(
        r#"
        INSERT INTO feature_flags (tenant_id, scope, key, enabled)
        VALUES ($1, 'GLOBAL', 'PDD_ONLY', true)
        "#,
    )
    .bind(uuid::Uuid::from_str("00000000-0000-0000-0000-000000000000").unwrap())
    .execute(&pool)
    .await?;

    // Now should be enabled
    let pdd_only = repo
        .is_enabled(tenant_id, None, FeatureFlag::PddOnly)
        .await?;
    assert!(pdd_only, "PDD_ONLY should be enabled globally");

    // Verify other flags still work
    let shipping_enabled = repo
        .is_enabled(tenant_id, None, FeatureFlag::ShippingWrite)
        .await?;
    assert!(
        !shipping_enabled,
        "SHIPPING_WRITE should be disabled by default"
    );

    Ok(())
}

#[tokio::test]
async fn shop_scope_overrides_tenant_scope() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping shop_scope_overrides_tenant_scope: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let shop_id = ShopId(uuid::Uuid::new_v4());
    let repo = FeatureFlagsRepo::new(&pool);

    // Enable at TENANT scope
    sqlx::query(
        r#"
        INSERT INTO feature_flags (tenant_id, scope, key, enabled)
        VALUES ($1, 'TENANT', 'SHIPPING_WRITE', true)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .execute(&pool)
    .await?;

    let enabled = repo
        .is_enabled(tenant_id, Some(shop_id), FeatureFlag::ShippingWrite)
        .await?;
    assert!(enabled, "SHIPPING_WRITE should be enabled at TENANT scope");

    // Disable at SHOP scope
    sqlx::query(
        r#"
        INSERT INTO feature_flags (tenant_id, scope, shop_id, key, enabled)
        VALUES ($1, 'SHOP', $2, 'SHIPPING_WRITE', false)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(shop_id))
    .execute(&pool)
    .await?;

    let enabled = repo
        .is_enabled(tenant_id, Some(shop_id), FeatureFlag::ShippingWrite)
        .await?;
    assert!(
        !enabled,
        "SHIPPING_WRITE should be disabled at SHOP scope (overrides TENANT)"
    );

    Ok(())
}
