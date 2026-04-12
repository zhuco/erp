use chrono::{DateTime, NaiveDate, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketingCost {
    pub cost_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub channel_id: Option<uuid::Uuid>,
    pub amount_cent: i64,
    pub currency: String,
    pub allocation_rule: Value,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub status: String,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct MarketingCostListFilters {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub channel_id: Option<uuid::Uuid>,
    pub offset: i64,
    pub limit: i64,
}

pub struct MarketingCostListResult {
    pub items: Vec<MarketingCost>,
    pub total: i64,
}

pub struct MarketingCostCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub channel_id: Option<uuid::Uuid>,
    pub amount_cent: i64,
    pub currency: String,
    pub allocation_rule: Value,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub status: String,
    pub created_by: Option<String>,
}

pub struct MarketingCostUpdate {
    pub name: Option<String>,
    pub channel_id: Option<uuid::Uuid>,
    pub amount_cent: Option<i64>,
    pub currency: Option<String>,
    pub allocation_rule: Option<Value>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub status: Option<String>,
}

pub struct MarketingCostsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> MarketingCostsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_costs(
        &self,
        tenant_id: TenantId,
        filters: MarketingCostListFilters,
    ) -> anyhow::Result<MarketingCostListResult> {
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
                    from marketing_costs
                    where tenant_id = $1
                      and name ilike $2
                      and ($3::text is null or status = $3)
                      and ($4::uuid is null or channel_id = $4)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(keyword)
                .bind(filters.status.as_deref())
                .bind(filters.channel_id)
                .fetch_one(self.pool)
                .await?
            }
            None => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from marketing_costs
                    where tenant_id = $1
                      and ($2::text is null or status = $2)
                      and ($3::uuid is null or channel_id = $3)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(filters.status.as_deref())
                .bind(filters.channel_id)
                .fetch_one(self.pool)
                .await?
            }
        };

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select cost_id, name, channel_id, amount_cent, currency, allocation_rule,
                   period_start, period_end, status, created_by, created_at, updated_at
            from marketing_costs
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
        if let Some(channel_id) = filters.channel_id {
            qb.push(" and channel_id = ");
            qb.push_bind(channel_id);
        }

        qb.push(" order by period_start desc nulls last, updated_at desc, cost_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            cost_id: uuid::Uuid,
            name: String,
            channel_id: Option<uuid::Uuid>,
            amount_cent: i64,
            currency: String,
            allocation_rule: Value,
            period_start: Option<NaiveDate>,
            period_end: Option<NaiveDate>,
            status: String,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| MarketingCost {
                cost_id: row.cost_id,
                tenant_id,
                name: row.name,
                channel_id: row.channel_id,
                amount_cent: row.amount_cent,
                currency: row.currency,
                allocation_rule: row.allocation_rule,
                period_start: row.period_start,
                period_end: row.period_end,
                status: row.status,
                created_by: row.created_by,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(MarketingCostListResult { items, total })
    }

    pub async fn create_cost(&self, input: MarketingCostCreate) -> anyhow::Result<MarketingCost> {
        #[derive(sqlx::FromRow)]
        struct Row {
            cost_id: uuid::Uuid,
            name: String,
            channel_id: Option<uuid::Uuid>,
            amount_cent: i64,
            currency: String,
            allocation_rule: Value,
            period_start: Option<NaiveDate>,
            period_end: Option<NaiveDate>,
            status: String,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into marketing_costs(
                tenant_id, name, channel_id, amount_cent, currency, allocation_rule, period_start,
                period_end, status, created_by
            ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            returning cost_id, name, channel_id, amount_cent, currency, allocation_rule, period_start,
                      period_end, status, created_by, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(input.channel_id)
        .bind(input.amount_cent)
        .bind(&input.currency)
        .bind(&input.allocation_rule)
        .bind(input.period_start)
        .bind(input.period_end)
        .bind(&input.status)
        .bind(&input.created_by)
        .fetch_one(self.pool)
        .await?;

        Ok(MarketingCost {
            cost_id: row.cost_id,
            tenant_id: input.tenant_id,
            name: row.name,
            channel_id: row.channel_id,
            amount_cent: row.amount_cent,
            currency: row.currency,
            allocation_rule: row.allocation_rule,
            period_start: row.period_start,
            period_end: row.period_end,
            status: row.status,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_cost(
        &self,
        tenant_id: TenantId,
        cost_id: uuid::Uuid,
        input: MarketingCostUpdate,
    ) -> anyhow::Result<Option<MarketingCost>> {
        let mut qb = QueryBuilder::<Postgres>::new("update marketing_costs set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(channel_id) = input.channel_id {
            separated.push("channel_id = ");
            separated.push_bind(channel_id);
        }
        if let Some(amount_cent) = input.amount_cent {
            separated.push("amount_cent = ");
            separated.push_bind(amount_cent);
        }
        if let Some(currency) = input.currency {
            separated.push("currency = ");
            separated.push_bind(currency);
        }
        if let Some(allocation_rule) = input.allocation_rule {
            separated.push("allocation_rule = ");
            separated.push_bind(allocation_rule);
        }
        if let Some(period_start) = input.period_start {
            separated.push("period_start = ");
            separated.push_bind(period_start);
        }
        if let Some(period_end) = input.period_end {
            separated.push("period_end = ");
            separated.push_bind(period_end);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and cost_id = ");
        qb.push_bind(cost_id);
        qb.push(
            " returning cost_id, name, channel_id, amount_cent, currency, allocation_rule,
              period_start, period_end, status, created_by, created_at, updated_at",
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            cost_id: uuid::Uuid,
            name: String,
            channel_id: Option<uuid::Uuid>,
            amount_cent: i64,
            currency: String,
            allocation_rule: Value,
            period_start: Option<NaiveDate>,
            period_end: Option<NaiveDate>,
            status: String,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(MarketingCost {
            cost_id: row.cost_id,
            tenant_id,
            name: row.name,
            channel_id: row.channel_id,
            amount_cent: row.amount_cent,
            currency: row.currency,
            allocation_rule: row.allocation_rule,
            period_start: row.period_start,
            period_end: row.period_end,
            status: row.status,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_cost(
        &self,
        tenant_id: TenantId,
        cost_id: uuid::Uuid,
    ) -> anyhow::Result<Option<MarketingCost>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            cost_id: uuid::Uuid,
            name: String,
            channel_id: Option<uuid::Uuid>,
            amount_cent: i64,
            currency: String,
            allocation_rule: Value,
            period_start: Option<NaiveDate>,
            period_end: Option<NaiveDate>,
            status: String,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from marketing_costs
            where tenant_id = $1 and cost_id = $2
            returning cost_id, name, channel_id, amount_cent, currency, allocation_rule,
                      period_start, period_end, status, created_by, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(cost_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| MarketingCost {
            cost_id: row.cost_id,
            tenant_id,
            name: row.name,
            channel_id: row.channel_id,
            amount_cent: row.amount_cent,
            currency: row.currency,
            allocation_rule: row.allocation_rule,
            period_start: row.period_start,
            period_end: row.period_end,
            status: row.status,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
