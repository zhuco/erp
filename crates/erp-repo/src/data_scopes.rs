use erp_types::{ShopId, TenantId, UserId};
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct UserDataScopes {
    pub shop_ids: Vec<ShopId>,
    pub warehouse_ids: Vec<uuid::Uuid>,
    pub merchant_ids: Vec<uuid::Uuid>,
    pub organization_ids: Vec<uuid::Uuid>,
}

pub struct DataScopesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> DataScopesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_user_scopes(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> anyhow::Result<UserDataScopes> {
        let shop_ids: Vec<uuid::Uuid> = sqlx::query_scalar(
            r#"
            select shop_id
            from user_shop_access
            where tenant_id = $1 and user_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(user_id))
        .fetch_all(self.pool)
        .await?;

        let warehouse_ids: Vec<uuid::Uuid> = sqlx::query_scalar(
            r#"
            select warehouse_id
            from user_warehouse_access
            where tenant_id = $1 and user_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(user_id))
        .fetch_all(self.pool)
        .await?;

        let merchant_ids: Vec<uuid::Uuid> = sqlx::query_scalar(
            r#"
            select merchant_id
            from user_merchant_access
            where tenant_id = $1 and user_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(user_id))
        .fetch_all(self.pool)
        .await?;

        let organization_ids: Vec<uuid::Uuid> = sqlx::query_scalar(
            r#"
            select organization_id
            from user_organization_access
            where tenant_id = $1 and user_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(user_id))
        .fetch_all(self.pool)
        .await?;

        Ok(UserDataScopes {
            shop_ids: shop_ids.into_iter().map(ShopId).collect(),
            warehouse_ids,
            merchant_ids,
            organization_ids,
        })
    }

    pub async fn replace_user_shops(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
        shop_ids: &[ShopId],
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r#"
            delete from user_shop_access
            where tenant_id = $1 and user_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(user_id))
        .execute(&mut *tx)
        .await?;

        for shop_id in shop_ids {
            sqlx::query(
                r#"
                insert into user_shop_access(tenant_id, user_id, shop_id)
                values ($1, $2, $3)
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(uuid::Uuid::from(user_id))
            .bind(uuid::Uuid::from(*shop_id))
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn replace_user_warehouses(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
        warehouse_ids: &[uuid::Uuid],
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        // Serializes empty and nonempty scope replacement with OMS admission.
        let actor: Option<uuid::Uuid> = sqlx::query_scalar(
            "select user_id from users where tenant_id=$1 and user_id=$2 for update",
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(user_id))
        .fetch_optional(&mut *tx)
        .await?;
        if actor.is_none() {
            return Err(anyhow::anyhow!("warehouse scope actor not found"));
        }
        if !warehouse_ids.is_empty() {
            let owned: i64 = sqlx::query_scalar(
                r#"
                select count(*)::bigint
                from warehouses
                where tenant_id = $1 and warehouse_id = any($2)
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(warehouse_ids)
            .fetch_one(&mut *tx)
            .await?;
            if owned != warehouse_ids.len() as i64 {
                return Err(anyhow::anyhow!("warehouse is outside this tenant"));
            }
        }
        sqlx::query(
            r#"
            delete from user_warehouse_access
            where tenant_id = $1 and user_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(user_id))
        .execute(&mut *tx)
        .await?;

        for warehouse_id in warehouse_ids {
            sqlx::query(
                r#"
                insert into user_warehouse_access(tenant_id, user_id, warehouse_id)
                values ($1, $2, $3)
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(uuid::Uuid::from(user_id))
            .bind(*warehouse_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn replace_user_merchants(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
        merchant_ids: &[uuid::Uuid],
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r#"
            delete from user_merchant_access
            where tenant_id = $1 and user_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(user_id))
        .execute(&mut *tx)
        .await?;

        for merchant_id in merchant_ids {
            sqlx::query(
                r#"
                insert into user_merchant_access(tenant_id, user_id, merchant_id)
                values ($1, $2, $3)
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(uuid::Uuid::from(user_id))
            .bind(*merchant_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn replace_user_organizations(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
        organization_ids: &[uuid::Uuid],
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r#"
            delete from user_organization_access
            where tenant_id = $1 and user_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(user_id))
        .execute(&mut *tx)
        .await?;

        for organization_id in organization_ids {
            sqlx::query(
                r#"
                insert into user_organization_access(tenant_id, user_id, organization_id)
                values ($1, $2, $3)
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(uuid::Uuid::from(user_id))
            .bind(*organization_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}
