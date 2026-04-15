use erp_types::TenantId;
use sqlx::PgPool;

pub struct TenantsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> TenantsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn is_pdd_only_mode(&self, tenant_id: TenantId) -> anyhow::Result<bool> {
        let value: Option<bool> = sqlx::query_scalar(
            r#"
            select pdd_only_mode
            from tenants
            where tenant_id = $1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_optional(self.pool)
        .await?;

        Ok(value.unwrap_or(false))
    }

    pub async fn multi_entity_flags(
        &self,
        tenant_id: TenantId,
    ) -> anyhow::Result<(bool, bool)> {
        let row: Option<(bool, bool)> = sqlx::query_as(
            r#"
            select multi_merchant_enabled, multi_organization_enabled
            from tenants
            where tenant_id = $1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_optional(self.pool)
        .await?;

        Ok(row.unwrap_or((false, false)))
    }
}
