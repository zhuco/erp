//! Explicit fixture master records obey the same product/SKU FK as a full install.
use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

pub(super) async fn sku(pool: &PgPool, tenant: Uuid, sku: Uuid, code: &str) -> Result<Uuid> {
    let product = Uuid::new_v4();
    let mut tx = pool.begin().await?;
    sqlx::query("insert into products(tenant_id,product_id,name) values($1,$2,$3)")
        .bind(tenant)
        .bind(product)
        .bind(format!("QA parent of {code}"))
        .execute(&mut *tx)
        .await?;
    sqlx::query("insert into skus(tenant_id,product_id,sku_id,sku_code) values($1,$2,$3,$4)")
        .bind(tenant)
        .bind(product)
        .bind(sku)
        .bind(code)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(product)
}
