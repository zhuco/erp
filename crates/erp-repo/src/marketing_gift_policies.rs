use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketingGiftPolicy {
    pub policy_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub status: String,
    pub strategy: String,
    pub probability: Option<f64>,
    pub scope_json: Value,
    pub gift_pool: Value,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct MarketingGiftPolicyListFilters {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct MarketingGiftPolicyListResult {
    pub items: Vec<MarketingGiftPolicy>,
    pub total: i64,
}

pub struct MarketingGiftPolicyCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub status: String,
    pub strategy: String,
    pub probability: Option<f64>,
    pub scope_json: Value,
    pub gift_pool: Value,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub created_by: Option<String>,
}

pub struct MarketingGiftPolicyUpdate {
    pub name: Option<String>,
    pub status: Option<String>,
    pub strategy: Option<String>,
    pub probability: Option<f64>,
    pub scope_json: Option<Value>,
    pub gift_pool: Option<Value>,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
}

pub struct MarketingGiftPoliciesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> MarketingGiftPoliciesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_policies(
        &self,
        tenant_id: TenantId,
        filters: MarketingGiftPolicyListFilters,
    ) -> anyhow::Result<MarketingGiftPolicyListResult> {
        let keyword = filters
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));

        let total: i64 = match &keyword {
            Some(keyword) => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from marketing_gift_policies
                    where tenant_id = $1
                      and name ilike $2
                      and ($3::text is null or status = $3)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(keyword)
                .bind(filters.status.as_deref())
                .fetch_one(self.pool)
                .await?
            }
            None => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from marketing_gift_policies
                    where tenant_id = $1
                      and ($2::text is null or status = $2)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(filters.status.as_deref())
                .fetch_one(self.pool)
                .await?
            }
        };

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select policy_id, name, status, strategy, probability, scope_json, gift_pool, start_at,
                   end_at, created_by, created_at, updated_at
            from marketing_gift_policies
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(keyword) = &keyword {
            qb.push(" and name ilike ");
            qb.push_bind(keyword);
        }
        if let Some(status) = &filters.status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }

        qb.push(" order by updated_at desc, policy_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            policy_id: uuid::Uuid,
            name: String,
            status: String,
            strategy: String,
            probability: Option<f64>,
            scope_json: Value,
            gift_pool: Value,
            start_at: Option<DateTime<Utc>>,
            end_at: Option<DateTime<Utc>>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| MarketingGiftPolicy {
                policy_id: row.policy_id,
                tenant_id,
                name: row.name,
                status: row.status,
                strategy: row.strategy,
                probability: row.probability,
                scope_json: row.scope_json,
                gift_pool: row.gift_pool,
                start_at: row.start_at,
                end_at: row.end_at,
                created_by: row.created_by,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(MarketingGiftPolicyListResult { items, total })
    }

    pub async fn create_policy(
        &self,
        input: MarketingGiftPolicyCreate,
    ) -> anyhow::Result<MarketingGiftPolicy> {
        #[derive(sqlx::FromRow)]
        struct Row {
            policy_id: uuid::Uuid,
            name: String,
            status: String,
            strategy: String,
            probability: Option<f64>,
            scope_json: Value,
            gift_pool: Value,
            start_at: Option<DateTime<Utc>>,
            end_at: Option<DateTime<Utc>>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into marketing_gift_policies(
                tenant_id, name, status, strategy, probability, scope_json, gift_pool, start_at,
                end_at, created_by
            ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            returning policy_id, name, status, strategy, probability, scope_json, gift_pool, start_at,
                      end_at, created_by, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(&input.status)
        .bind(&input.strategy)
        .bind(input.probability)
        .bind(&input.scope_json)
        .bind(&input.gift_pool)
        .bind(input.start_at)
        .bind(input.end_at)
        .bind(&input.created_by)
        .fetch_one(self.pool)
        .await?;

        Ok(MarketingGiftPolicy {
            policy_id: row.policy_id,
            tenant_id: input.tenant_id,
            name: row.name,
            status: row.status,
            strategy: row.strategy,
            probability: row.probability,
            scope_json: row.scope_json,
            gift_pool: row.gift_pool,
            start_at: row.start_at,
            end_at: row.end_at,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_policy(
        &self,
        tenant_id: TenantId,
        policy_id: uuid::Uuid,
        input: MarketingGiftPolicyUpdate,
    ) -> anyhow::Result<Option<MarketingGiftPolicy>> {
        let mut qb = QueryBuilder::<Postgres>::new("update marketing_gift_policies set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }
        if let Some(strategy) = input.strategy {
            separated.push("strategy = ");
            separated.push_bind(strategy);
        }
        if let Some(probability) = input.probability {
            separated.push("probability = ");
            separated.push_bind(probability);
        }
        if let Some(scope_json) = input.scope_json {
            separated.push("scope_json = ");
            separated.push_bind(scope_json);
        }
        if let Some(gift_pool) = input.gift_pool {
            separated.push("gift_pool = ");
            separated.push_bind(gift_pool);
        }
        if let Some(start_at) = input.start_at {
            separated.push("start_at = ");
            separated.push_bind(start_at);
        }
        if let Some(end_at) = input.end_at {
            separated.push("end_at = ");
            separated.push_bind(end_at);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and policy_id = ");
        qb.push_bind(policy_id);
        qb.push(
            " returning policy_id, name, status, strategy, probability, scope_json, gift_pool,
              start_at, end_at, created_by, created_at, updated_at",
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            policy_id: uuid::Uuid,
            name: String,
            status: String,
            strategy: String,
            probability: Option<f64>,
            scope_json: Value,
            gift_pool: Value,
            start_at: Option<DateTime<Utc>>,
            end_at: Option<DateTime<Utc>>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(MarketingGiftPolicy {
            policy_id: row.policy_id,
            tenant_id,
            name: row.name,
            status: row.status,
            strategy: row.strategy,
            probability: row.probability,
            scope_json: row.scope_json,
            gift_pool: row.gift_pool,
            start_at: row.start_at,
            end_at: row.end_at,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_policy(
        &self,
        tenant_id: TenantId,
        policy_id: uuid::Uuid,
    ) -> anyhow::Result<Option<MarketingGiftPolicy>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            policy_id: uuid::Uuid,
            name: String,
            status: String,
            strategy: String,
            probability: Option<f64>,
            scope_json: Value,
            gift_pool: Value,
            start_at: Option<DateTime<Utc>>,
            end_at: Option<DateTime<Utc>>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from marketing_gift_policies
            where tenant_id = $1 and policy_id = $2
            returning policy_id, name, status, strategy, probability, scope_json, gift_pool, start_at,
                      end_at, created_by, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(policy_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| MarketingGiftPolicy {
            policy_id: row.policy_id,
            tenant_id,
            name: row.name,
            status: row.status,
            strategy: row.strategy,
            probability: row.probability,
            scope_json: row.scope_json,
            gift_pool: row.gift_pool,
            start_at: row.start_at,
            end_at: row.end_at,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
