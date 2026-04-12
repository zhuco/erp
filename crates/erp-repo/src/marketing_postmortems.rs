use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketingPostmortem {
    pub postmortem_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub promotion_id: Option<uuid::Uuid>,
    pub channel_id: Option<uuid::Uuid>,
    pub summary: Option<String>,
    pub metrics_json: Value,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct MarketingPostmortemListFilters {
    pub keyword: Option<String>,
    pub promotion_id: Option<uuid::Uuid>,
    pub channel_id: Option<uuid::Uuid>,
    pub offset: i64,
    pub limit: i64,
}

pub struct MarketingPostmortemListResult {
    pub items: Vec<MarketingPostmortem>,
    pub total: i64,
}

pub struct MarketingPostmortemCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub promotion_id: Option<uuid::Uuid>,
    pub channel_id: Option<uuid::Uuid>,
    pub summary: Option<String>,
    pub metrics_json: Value,
    pub created_by: Option<String>,
}

pub struct MarketingPostmortemUpdate {
    pub name: Option<String>,
    pub promotion_id: Option<uuid::Uuid>,
    pub channel_id: Option<uuid::Uuid>,
    pub summary: Option<String>,
    pub metrics_json: Option<Value>,
}

pub struct MarketingPostmortemsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> MarketingPostmortemsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_postmortems(
        &self,
        tenant_id: TenantId,
        filters: MarketingPostmortemListFilters,
    ) -> anyhow::Result<MarketingPostmortemListResult> {
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
                    from marketing_postmortems
                    where tenant_id = $1
                      and name ilike $2
                      and ($3::uuid is null or promotion_id = $3)
                      and ($4::uuid is null or channel_id = $4)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(keyword)
                .bind(filters.promotion_id)
                .bind(filters.channel_id)
                .fetch_one(self.pool)
                .await?
            }
            None => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from marketing_postmortems
                    where tenant_id = $1
                      and ($2::uuid is null or promotion_id = $2)
                      and ($3::uuid is null or channel_id = $3)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(filters.promotion_id)
                .bind(filters.channel_id)
                .fetch_one(self.pool)
                .await?
            }
        };

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select postmortem_id, name, promotion_id, channel_id, summary, metrics_json, created_by,
                   created_at, updated_at
            from marketing_postmortems
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(keyword) = &keyword {
            qb.push(" and name ilike ");
            qb.push_bind(keyword);
        }
        if let Some(promotion_id) = filters.promotion_id {
            qb.push(" and promotion_id = ");
            qb.push_bind(promotion_id);
        }
        if let Some(channel_id) = filters.channel_id {
            qb.push(" and channel_id = ");
            qb.push_bind(channel_id);
        }

        qb.push(" order by created_at desc, postmortem_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            postmortem_id: uuid::Uuid,
            name: String,
            promotion_id: Option<uuid::Uuid>,
            channel_id: Option<uuid::Uuid>,
            summary: Option<String>,
            metrics_json: Value,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| MarketingPostmortem {
                postmortem_id: row.postmortem_id,
                tenant_id,
                name: row.name,
                promotion_id: row.promotion_id,
                channel_id: row.channel_id,
                summary: row.summary,
                metrics_json: row.metrics_json,
                created_by: row.created_by,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(MarketingPostmortemListResult { items, total })
    }

    pub async fn create_postmortem(
        &self,
        input: MarketingPostmortemCreate,
    ) -> anyhow::Result<MarketingPostmortem> {
        #[derive(sqlx::FromRow)]
        struct Row {
            postmortem_id: uuid::Uuid,
            name: String,
            promotion_id: Option<uuid::Uuid>,
            channel_id: Option<uuid::Uuid>,
            summary: Option<String>,
            metrics_json: Value,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into marketing_postmortems(
                tenant_id, name, promotion_id, channel_id, summary, metrics_json, created_by
            ) values ($1, $2, $3, $4, $5, $6, $7)
            returning postmortem_id, name, promotion_id, channel_id, summary, metrics_json,
                      created_by, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(input.promotion_id)
        .bind(input.channel_id)
        .bind(&input.summary)
        .bind(&input.metrics_json)
        .bind(&input.created_by)
        .fetch_one(self.pool)
        .await?;

        Ok(MarketingPostmortem {
            postmortem_id: row.postmortem_id,
            tenant_id: input.tenant_id,
            name: row.name,
            promotion_id: row.promotion_id,
            channel_id: row.channel_id,
            summary: row.summary,
            metrics_json: row.metrics_json,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_postmortem(
        &self,
        tenant_id: TenantId,
        postmortem_id: uuid::Uuid,
        input: MarketingPostmortemUpdate,
    ) -> anyhow::Result<Option<MarketingPostmortem>> {
        let mut qb = QueryBuilder::<Postgres>::new("update marketing_postmortems set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(promotion_id) = input.promotion_id {
            separated.push("promotion_id = ");
            separated.push_bind(promotion_id);
        }
        if let Some(channel_id) = input.channel_id {
            separated.push("channel_id = ");
            separated.push_bind(channel_id);
        }
        if let Some(summary) = input.summary {
            separated.push("summary = ");
            separated.push_bind(summary);
        }
        if let Some(metrics_json) = input.metrics_json {
            separated.push("metrics_json = ");
            separated.push_bind(metrics_json);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and postmortem_id = ");
        qb.push_bind(postmortem_id);
        qb.push(
            " returning postmortem_id, name, promotion_id, channel_id, summary, metrics_json,
              created_by, created_at, updated_at",
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            postmortem_id: uuid::Uuid,
            name: String,
            promotion_id: Option<uuid::Uuid>,
            channel_id: Option<uuid::Uuid>,
            summary: Option<String>,
            metrics_json: Value,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(MarketingPostmortem {
            postmortem_id: row.postmortem_id,
            tenant_id,
            name: row.name,
            promotion_id: row.promotion_id,
            channel_id: row.channel_id,
            summary: row.summary,
            metrics_json: row.metrics_json,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_postmortem(
        &self,
        tenant_id: TenantId,
        postmortem_id: uuid::Uuid,
    ) -> anyhow::Result<Option<MarketingPostmortem>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            postmortem_id: uuid::Uuid,
            name: String,
            promotion_id: Option<uuid::Uuid>,
            channel_id: Option<uuid::Uuid>,
            summary: Option<String>,
            metrics_json: Value,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from marketing_postmortems
            where tenant_id = $1 and postmortem_id = $2
            returning postmortem_id, name, promotion_id, channel_id, summary, metrics_json,
                      created_by, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(postmortem_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| MarketingPostmortem {
            postmortem_id: row.postmortem_id,
            tenant_id,
            name: row.name,
            promotion_id: row.promotion_id,
            channel_id: row.channel_id,
            summary: row.summary,
            metrics_json: row.metrics_json,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
