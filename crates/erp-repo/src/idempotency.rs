use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde_json::Value;
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct IdempotencyKey {
    pub tenant_id: TenantId,
    pub key: String,
    pub route: String,
    pub request_hash: String,
    pub response_status: i32,
    pub response_body: Value,
    pub created_at: DateTime<Utc>,
}

pub struct IdempotencyKeyInsert {
    pub tenant_id: TenantId,
    pub key: String,
    pub route: String,
    pub request_hash: String,
    pub response_status: i32,
    pub response_body: Value,
}

pub struct IdempotencyRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> IdempotencyRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_key(
        &self,
        tenant_id: TenantId,
        route: &str,
        key: &str,
    ) -> anyhow::Result<Option<IdempotencyKey>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            tenant_id: uuid::Uuid,
            key: String,
            route: String,
            request_hash: String,
            response_status: i32,
            response_body: Value,
            created_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select tenant_id, "key", route, request_hash, response_status, response_body, created_at
            from idempotency_keys
            where tenant_id = $1 and route = $2 and "key" = $3
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(route)
        .bind(key)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| IdempotencyKey {
            tenant_id: TenantId::from(row.tenant_id),
            key: row.key,
            route: row.route,
            request_hash: row.request_hash,
            response_status: row.response_status,
            response_body: row.response_body,
            created_at: row.created_at,
        }))
    }

    pub async fn insert_key(&self, input: IdempotencyKeyInsert) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            insert into idempotency_keys(
                tenant_id, "key", route, request_hash, response_status, response_body
            ) values ($1, $2, $3, $4, $5, $6)
            on conflict (tenant_id, route, "key") do nothing
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.key)
        .bind(input.route)
        .bind(input.request_hash)
        .bind(input.response_status)
        .bind(input.response_body)
        .execute(self.pool)
        .await?;

        Ok(())
    }
}
