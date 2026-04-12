use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerBlacklistEntry {
    pub blacklist_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub key_type: String,
    pub key_value: String,
    pub reason: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CustomerBlacklistCreate {
    pub tenant_id: TenantId,
    pub key_type: String,
    pub key_value: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct CustomerBlacklistListFilters {
    pub key_type: Option<String>,
    pub keyword: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct CustomerBlacklistListResult {
    pub items: Vec<CustomerBlacklistEntry>,
    pub total: i64,
}

pub struct CustomerBlacklistRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> CustomerBlacklistRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_entries(
        &self,
        tenant_id: TenantId,
        filters: CustomerBlacklistListFilters,
    ) -> anyhow::Result<CustomerBlacklistListResult> {
        let keyword = filters
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));

        let total: i64 = match (&filters.key_type, &keyword) {
            (Some(key_type), Some(keyword)) => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from customer_blacklist
                    where tenant_id = $1 and key_type = $2 and key_value ilike $3
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(key_type)
                .bind(keyword)
                .fetch_one(self.pool)
                .await?
            }
            (Some(key_type), None) => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from customer_blacklist
                    where tenant_id = $1 and key_type = $2
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(key_type)
                .fetch_one(self.pool)
                .await?
            }
            (None, Some(keyword)) => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from customer_blacklist
                    where tenant_id = $1 and key_value ilike $2
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(keyword)
                .fetch_one(self.pool)
                .await?
            }
            (None, None) => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from customer_blacklist
                    where tenant_id = $1
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .fetch_one(self.pool)
                .await?
            }
        };

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select blacklist_id, key_type, key_value, reason, created_at, updated_at
            from customer_blacklist
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(key_type) = filters.key_type {
            qb.push(" and key_type = ");
            qb.push_bind(key_type);
        }
        if let Some(keyword) = keyword.as_ref() {
            qb.push(" and key_value ilike ");
            qb.push_bind(keyword);
        }

        qb.push(" order by updated_at desc, blacklist_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        let rows: Vec<BlacklistRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_entry(tenant_id))
            .collect();

        Ok(CustomerBlacklistListResult { items, total })
    }

    pub async fn create_entry(
        &self,
        input: CustomerBlacklistCreate,
    ) -> anyhow::Result<CustomerBlacklistEntry> {
        let row: BlacklistRow = sqlx::query_as(
            r#"
            insert into customer_blacklist(tenant_id, key_type, key_value, reason)
            values ($1, $2, $3, $4)
            on conflict (tenant_id, key_type, key_value)
            do update set reason = excluded.reason, updated_at = now()
            returning blacklist_id, key_type, key_value, reason, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.key_type)
        .bind(&input.key_value)
        .bind(&input.reason)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_entry(input.tenant_id))
    }

    pub async fn delete_entry(
        &self,
        tenant_id: TenantId,
        blacklist_id: uuid::Uuid,
    ) -> anyhow::Result<Option<CustomerBlacklistEntry>> {
        let row: Option<BlacklistRow> = sqlx::query_as(
            r#"
            delete from customer_blacklist
            where tenant_id = $1 and blacklist_id = $2
            returning blacklist_id, key_type, key_value, reason, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(blacklist_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_entry(tenant_id)))
    }

    pub async fn find_entry(
        &self,
        tenant_id: TenantId,
        key_type: &str,
        key_value: &str,
    ) -> anyhow::Result<Option<CustomerBlacklistEntry>> {
        let row: Option<BlacklistRow> = sqlx::query_as(
            r#"
            select blacklist_id, key_type, key_value, reason, created_at, updated_at
            from customer_blacklist
            where tenant_id = $1 and key_type = $2 and key_value = $3
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(key_type)
        .bind(key_value)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_entry(tenant_id)))
    }
}

#[derive(sqlx::FromRow)]
struct BlacklistRow {
    blacklist_id: uuid::Uuid,
    key_type: String,
    key_value: String,
    reason: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl BlacklistRow {
    fn into_entry(self, tenant_id: TenantId) -> CustomerBlacklistEntry {
        CustomerBlacklistEntry {
            blacklist_id: self.blacklist_id,
            tenant_id,
            key_type: self.key_type,
            key_value: self.key_value,
            reason: self.reason,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
