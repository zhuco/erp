use chrono::{DateTime, Utc};
use erp_types::{Platform, ShopId, TenantId};
use sqlx::PgPool;

pub struct WebhookNoncesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> WebhookNoncesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn claim_nonce(
        &self,
        tenant_id: TenantId,
        platform: Platform,
        shop_id: ShopId,
        nonce: &str,
        expires_at: DateTime<Utc>,
    ) -> anyhow::Result<bool> {
        let result = sqlx::query(
            r#"
            insert into webhook_nonces (
              tenant_id, platform, shop_id, nonce, received_at, expires_at
            ) values ($1,$2,$3,$4,now(),$5)
            on conflict do nothing
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(platform.to_string())
        .bind(uuid::Uuid::from(shop_id))
        .bind(nonce)
        .bind(expires_at)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
