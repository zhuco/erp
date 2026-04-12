use chrono::{DateTime, Utc};
use erp_types::{TenantId, UserId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserShortcut {
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub shortcuts_json: Value,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UserShortcutUpsert {
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub shortcuts_json: Value,
}

pub struct UserShortcutsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> UserShortcutsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_shortcuts(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> anyhow::Result<Option<UserShortcut>> {
        let row: Option<UserShortcutRow> = sqlx::query_as(
            r#"
            select tenant_id, user_id, shortcuts_json, updated_at, created_at
            from user_shortcuts
            where tenant_id = $1 and user_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(user_id))
        .fetch_optional(self.pool)
        .await?;
        Ok(row.map(UserShortcutRow::into_shortcut))
    }

    pub async fn upsert_shortcuts(
        &self,
        input: UserShortcutUpsert,
    ) -> anyhow::Result<UserShortcut> {
        let row: UserShortcutRow = sqlx::query_as(
            r#"
            insert into user_shortcuts (tenant_id, user_id, shortcuts_json)
            values ($1, $2, $3)
            on conflict (tenant_id, user_id)
            do update set
                shortcuts_json = excluded.shortcuts_json,
                updated_at = now()
            returning tenant_id, user_id, shortcuts_json, updated_at, created_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(uuid::Uuid::from(input.user_id))
        .bind(input.shortcuts_json)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_shortcut())
    }
}

#[derive(sqlx::FromRow)]
struct UserShortcutRow {
    tenant_id: uuid::Uuid,
    user_id: uuid::Uuid,
    shortcuts_json: Value,
    updated_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl UserShortcutRow {
    fn into_shortcut(self) -> UserShortcut {
        UserShortcut {
            tenant_id: TenantId(self.tenant_id),
            user_id: UserId(self.user_id),
            shortcuts_json: self.shortcuts_json,
            updated_at: self.updated_at,
            created_at: self.created_at,
        }
    }
}
