use erp_types::{TenantId, UserId};
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct UserRow {
    pub user_id: UserId,
    pub tenant_id: TenantId,
    pub email: String,
    pub password_hash: String,
    pub status: String,
}

pub struct AuthRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> AuthRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_user_by_email(
        &self,
        tenant_id: TenantId,
        email: &str,
    ) -> anyhow::Result<Option<UserRow>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            user_id: uuid::Uuid,
            tenant_id: uuid::Uuid,
            email: String,
            password_hash: String,
            status: String,
        }

        let row = sqlx::query_as::<_, Row>(
            r#"
            select user_id, tenant_id, email, password_hash, status
            from users
            where tenant_id = $1 and email = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(email)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|r| UserRow {
            user_id: UserId(r.user_id),
            tenant_id: TenantId(r.tenant_id),
            email: r.email,
            password_hash: r.password_hash,
            status: r.status,
        }))
    }

    pub async fn get_user_by_id(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> anyhow::Result<Option<UserRow>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            user_id: uuid::Uuid,
            tenant_id: uuid::Uuid,
            email: String,
            password_hash: String,
            status: String,
        }

        let row = sqlx::query_as::<_, Row>(
            r#"
            select user_id, tenant_id, email, password_hash, status
            from users
            where tenant_id = $1 and user_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(user_id))
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|r| UserRow {
            user_id: UserId(r.user_id),
            tenant_id: TenantId(r.tenant_id),
            email: r.email,
            password_hash: r.password_hash,
            status: r.status,
        }))
    }

    pub async fn list_permissions(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> anyhow::Result<Vec<String>> {
        let perms: Vec<String> = sqlx::query_scalar(
            r#"
            select distinct rp.perm_code
            from user_roles ur
            join role_permissions rp
              on rp.tenant_id = ur.tenant_id and rp.role_id = ur.role_id
            where ur.tenant_id = $1 and ur.user_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(user_id))
        .fetch_all(self.pool)
        .await?;

        Ok(perms)
    }
}
