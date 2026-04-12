use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PermissionDefinition {
    pub code: String,
    pub name: String,
    pub module: String,
    pub description: String,
}

pub struct PermissionsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> PermissionsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_permissions(
        &self,
        tenant_id: TenantId,
    ) -> anyhow::Result<Vec<PermissionDefinition>> {
        let rows: Vec<PermissionDefinition> = sqlx::query_as(
            r#"
            select code, name, module, description
            from permissions
            where tenant_id = $1
            order by module, code
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_all(self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn seed_permissions(
        &self,
        tenant_id: TenantId,
        items: &[PermissionDefinition],
    ) -> anyhow::Result<()> {
        if items.is_empty() {
            return Ok(());
        }

        let mut qb = QueryBuilder::<Postgres>::new(
            "insert into permissions (tenant_id, code, name, module, description) ",
        );
        qb.push_values(items, |mut builder, item| {
            builder
                .push_bind(uuid::Uuid::from(tenant_id))
                .push_bind(&item.code)
                .push_bind(&item.name)
                .push_bind(&item.module)
                .push_bind(&item.description);
        });
        qb.push(" on conflict (tenant_id, code) do nothing");
        qb.build().execute(self.pool).await?;
        Ok(())
    }
}
