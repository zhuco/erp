use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::{RoleId, TenantId, UserId};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::collections::HashSet;

fn normalize_non_blank(field: &str, value: &str) -> anyhow::Result<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(anyhow!("{field} must not be blank"));
    }
    Ok(normalized.to_string())
}

fn normalize_permission_codes(perms: &[String]) -> anyhow::Result<Vec<String>> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::with_capacity(perms.len());
    for perm in perms {
        let code = normalize_non_blank("permission code", perm)?;
        if seen.insert(code.clone()) {
            normalized.push(code);
        }
    }
    Ok(normalized)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub role_id: RoleId,
    pub tenant_id: TenantId,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct RoleCreate {
    pub tenant_id: TenantId,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct RoleUpdate {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub user_id: UserId,
    pub tenant_id: TenantId,
    pub email: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UserCreate {
    pub tenant_id: TenantId,
    pub email: String,
    pub password_hash: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct UserUpdate {
    pub email: Option<String>,
    pub password_hash: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UserListFilters {
    pub email: Option<String>,
    pub status: Option<String>,
}

pub struct RbacRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> RbacRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_roles(&self, tenant_id: TenantId) -> anyhow::Result<Vec<Role>> {
        let rows: Vec<RoleRow> = sqlx::query_as(
            r#"
            select role_id, tenant_id, name, created_at
            from roles
            where tenant_id = $1
            order by created_at desc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_all(self.pool)
        .await?;

        Ok(rows.into_iter().map(RoleRow::into_role).collect())
    }

    pub async fn create_role(&self, input: RoleCreate) -> anyhow::Result<Role> {
        let name = normalize_non_blank("role name", &input.name)?;
        let row: RoleRow = sqlx::query_as(
            r#"
            insert into roles(tenant_id, name)
            values ($1, $2)
            returning role_id, tenant_id, name, created_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&name)
        .fetch_one(self.pool)
        .await?;
        Ok(row.into_role())
    }

    pub async fn update_role(
        &self,
        tenant_id: TenantId,
        role_id: RoleId,
        input: RoleUpdate,
    ) -> anyhow::Result<Option<Role>> {
        let name = normalize_non_blank("role name", &input.name)?;
        let row: Option<RoleRow> = sqlx::query_as(
            r#"
            update roles
            set name = $3
            where tenant_id = $1 and role_id = $2
            returning role_id, tenant_id, name, created_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(role_id))
        .bind(&name)
        .fetch_optional(self.pool)
        .await?;
        Ok(row.map(RoleRow::into_role))
    }

    pub async fn list_role_permissions(
        &self,
        tenant_id: TenantId,
        role_id: RoleId,
    ) -> anyhow::Result<Vec<String>> {
        let perms: Vec<String> = sqlx::query_scalar(
            r#"
            select perm_code
            from role_permissions
            where tenant_id = $1 and role_id = $2
            order by perm_code
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(role_id))
        .fetch_all(self.pool)
        .await?;
        Ok(perms)
    }

    pub async fn replace_role_permissions(
        &self,
        tenant_id: TenantId,
        role_id: RoleId,
        perms: &[String],
    ) -> anyhow::Result<()> {
        let normalized_perms = normalize_permission_codes(perms)?;
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r#"
            delete from role_permissions
            where tenant_id = $1 and role_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(role_id))
        .execute(&mut *tx)
        .await?;

        for perm in normalized_perms {
            sqlx::query(
                r#"
                insert into role_permissions(tenant_id, role_id, perm_code)
                values ($1, $2, $3)
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(uuid::Uuid::from(role_id))
            .bind(perm)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn list_users(
        &self,
        tenant_id: TenantId,
        filters: UserListFilters,
    ) -> anyhow::Result<Vec<User>> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select user_id, tenant_id, email, status, created_at
            from users
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(email) = filters.email {
            qb.push(" and email ilike ");
            qb.push_bind(format!("%{email}%"));
        }
        if let Some(status) = filters.status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }

        qb.push(" order by created_at desc");

        let rows: Vec<UserRow> = qb.build_query_as().fetch_all(self.pool).await?;
        Ok(rows.into_iter().map(UserRow::into_user).collect())
    }

    pub async fn get_user(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> anyhow::Result<Option<User>> {
        let row: Option<UserRow> = sqlx::query_as(
            r#"
            select user_id, tenant_id, email, status, created_at
            from users
            where tenant_id = $1 and user_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(user_id))
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(UserRow::into_user))
    }

    pub async fn create_user(&self, input: UserCreate) -> anyhow::Result<User> {
        let email = normalize_non_blank("email", &input.email)?;
        let status = normalize_non_blank("status", &input.status)?;
        let row: UserRow = sqlx::query_as(
            r#"
            insert into users(tenant_id, email, password_hash, status)
            values ($1, $2, $3, $4)
            returning user_id, tenant_id, email, status, created_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&email)
        .bind(&input.password_hash)
        .bind(&status)
        .fetch_one(self.pool)
        .await?;
        Ok(row.into_user())
    }

    pub async fn create_user_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        input: UserCreate,
    ) -> anyhow::Result<User> {
        let email = normalize_non_blank("email", &input.email)?;
        let status = normalize_non_blank("status", &input.status)?;
        let row: UserRow = sqlx::query_as(
            r#"
            insert into users(tenant_id, email, password_hash, status)
            values ($1, $2, $3, $4)
            returning user_id, tenant_id, email, status, created_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&email)
        .bind(&input.password_hash)
        .bind(&status)
        .fetch_one(&mut **tx)
        .await?;
        Ok(row.into_user())
    }

    pub async fn update_user_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        tenant_id: TenantId,
        user_id: UserId,
        input: UserUpdate,
    ) -> anyhow::Result<Option<User>> {
        let existing: Option<UserRow> = sqlx::query_as(
            r#"
            select user_id, tenant_id, email, status, created_at
            from users
            where tenant_id = $1 and user_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(user_id))
        .fetch_optional(&mut **tx)
        .await?;
        let Some(existing) = existing else {
            return Ok(None);
        };
        let email = match input.email.as_deref() {
            Some(email) => normalize_non_blank("email", email)?,
            None => existing.email,
        };
        let status = match input.status.as_deref() {
            Some(status) => normalize_non_blank("status", status)?,
            None => existing.status,
        };
        let row: UserRow = sqlx::query_as(
            r#"
            update users
            set email = $3,
                password_hash = coalesce($4, password_hash),
                status = $5,
                updated_at = now()
            where tenant_id = $1 and user_id = $2
            returning user_id, tenant_id, email, status, created_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(user_id))
        .bind(email)
        .bind(input.password_hash)
        .bind(status)
        .fetch_one(&mut **tx)
        .await?;
        Ok(Some(row.into_user()))
    }

    pub async fn update_user(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
        input: UserUpdate,
    ) -> anyhow::Result<Option<User>> {
        let existing: Option<UserRow> = sqlx::query_as(
            r#"
            select user_id, tenant_id, email, status, created_at
            from users
            where tenant_id = $1 and user_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(user_id))
        .fetch_optional(self.pool)
        .await?;

        let Some(current) = existing else {
            return Ok(None);
        };

        let next_email = match input.email {
            Some(email) => normalize_non_blank("email", &email)?,
            None => current.email.clone(),
        };
        let next_status = match input.status {
            Some(status) => normalize_non_blank("status", &status)?,
            None => current.status.clone(),
        };

        sqlx::query(
            r#"
            update users
            set email = $3,
                status = $4,
                password_hash = coalesce($5, password_hash),
                updated_at = now()
            where tenant_id = $1 and user_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(user_id))
        .bind(&next_email)
        .bind(&next_status)
        .bind(input.password_hash)
        .execute(self.pool)
        .await?;

        Ok(Some(User {
            user_id,
            tenant_id,
            email: next_email,
            status: next_status,
            created_at: current.created_at,
        }))
    }

    pub async fn list_user_roles(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> anyhow::Result<Vec<Role>> {
        let rows: Vec<RoleRow> = sqlx::query_as(
            r#"
            select r.role_id, r.tenant_id, r.name, r.created_at
            from roles r
            join user_roles ur
              on ur.tenant_id = r.tenant_id and ur.role_id = r.role_id
            where ur.tenant_id = $1 and ur.user_id = $2
            order by r.created_at desc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(user_id))
        .fetch_all(self.pool)
        .await?;

        Ok(rows.into_iter().map(RoleRow::into_role).collect())
    }

    pub async fn replace_user_roles(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
        role_ids: &[RoleId],
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r#"
            delete from user_roles
            where tenant_id = $1 and user_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(user_id))
        .execute(&mut *tx)
        .await?;

        for role_id in role_ids {
            sqlx::query(
                r#"
                insert into user_roles(tenant_id, user_id, role_id)
                values ($1, $2, $3)
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(uuid::Uuid::from(user_id))
            .bind(uuid::Uuid::from(*role_id))
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn ensure_user_exists(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> anyhow::Result<()> {
        let exists: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            select user_id
            from users
            where tenant_id = $1 and user_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(user_id))
        .fetch_optional(self.pool)
        .await?;

        if exists.is_some() {
            Ok(())
        } else {
            Err(anyhow!("user not found"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_non_blank, normalize_permission_codes};

    #[test]
    fn normalize_non_blank_rejects_whitespace_only() {
        assert!(normalize_non_blank("email", "   ").is_err());
        assert!(normalize_non_blank("email", "\n\t").is_err());
        assert_eq!(
            normalize_non_blank("email", "  a@example.com  ").expect("should normalize"),
            "a@example.com"
        );
    }

    #[test]
    fn normalize_permission_codes_deduplicates_preserving_order() {
        let input = vec![
            " orders.read ".to_string(),
            "orders.write".to_string(),
            "orders.read".to_string(),
        ];
        let normalized = normalize_permission_codes(&input).expect("should normalize");
        assert_eq!(
            normalized,
            vec!["orders.read".to_string(), "orders.write".to_string()]
        );
    }
}

#[derive(sqlx::FromRow)]
struct RoleRow {
    role_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    name: String,
    created_at: DateTime<Utc>,
}

impl RoleRow {
    fn into_role(self) -> Role {
        Role {
            role_id: RoleId(self.role_id),
            tenant_id: TenantId(self.tenant_id),
            name: self.name,
            created_at: self.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    user_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    email: String,
    status: String,
    created_at: DateTime<Utc>,
}

impl UserRow {
    fn into_user(self) -> User {
        User {
            user_id: UserId(self.user_id),
            tenant_id: TenantId(self.tenant_id),
            email: self.email,
            status: self.status,
            created_at: self.created_at,
        }
    }
}
