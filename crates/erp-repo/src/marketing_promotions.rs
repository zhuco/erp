use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketingPromotion {
    pub promotion_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub promo_type: String,
    pub status: String,
    pub priority: i32,
    pub exclusive: bool,
    pub scope_type: String,
    pub scope_json: Value,
    pub rule_json: Value,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct MarketingPromotionListFilters {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub promo_type: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct MarketingPromotionListResult {
    pub items: Vec<MarketingPromotion>,
    pub total: i64,
}

pub struct MarketingPromotionCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub promo_type: String,
    pub status: String,
    pub priority: i32,
    pub exclusive: bool,
    pub scope_type: String,
    pub scope_json: Value,
    pub rule_json: Value,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub created_by: Option<String>,
}

pub struct MarketingPromotionUpdate {
    pub name: Option<String>,
    pub promo_type: Option<String>,
    pub status: Option<String>,
    pub priority: Option<i32>,
    pub exclusive: Option<bool>,
    pub scope_type: Option<String>,
    pub scope_json: Option<Value>,
    pub rule_json: Option<Value>,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
}

pub struct MarketingPromotionsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> MarketingPromotionsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_promotions(
        &self,
        tenant_id: TenantId,
        filters: MarketingPromotionListFilters,
    ) -> anyhow::Result<MarketingPromotionListResult> {
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
                    from marketing_promotions
                    where tenant_id = $1
                      and name ilike $2
                      and ($3::text is null or status = $3)
                      and ($4::text is null or promo_type = $4)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(keyword)
                .bind(filters.status.as_deref())
                .bind(filters.promo_type.as_deref())
                .fetch_one(self.pool)
                .await?
            }
            None => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from marketing_promotions
                    where tenant_id = $1
                      and ($2::text is null or status = $2)
                      and ($3::text is null or promo_type = $3)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(filters.status.as_deref())
                .bind(filters.promo_type.as_deref())
                .fetch_one(self.pool)
                .await?
            }
        };

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select promotion_id, name, promo_type, status, priority, exclusive, scope_type,
                   scope_json, rule_json, start_at, end_at, created_by, created_at, updated_at
            from marketing_promotions
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
        if let Some(promo_type) = &filters.promo_type {
            qb.push(" and promo_type = ");
            qb.push_bind(promo_type);
        }

        qb.push(" order by start_at desc nulls last, updated_at desc, promotion_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            promotion_id: uuid::Uuid,
            name: String,
            promo_type: String,
            status: String,
            priority: i32,
            exclusive: bool,
            scope_type: String,
            scope_json: Value,
            rule_json: Value,
            start_at: Option<DateTime<Utc>>,
            end_at: Option<DateTime<Utc>>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| MarketingPromotion {
                promotion_id: row.promotion_id,
                tenant_id,
                name: row.name,
                promo_type: row.promo_type,
                status: row.status,
                priority: row.priority,
                exclusive: row.exclusive,
                scope_type: row.scope_type,
                scope_json: row.scope_json,
                rule_json: row.rule_json,
                start_at: row.start_at,
                end_at: row.end_at,
                created_by: row.created_by,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(MarketingPromotionListResult { items, total })
    }

    pub async fn create_promotion(
        &self,
        input: MarketingPromotionCreate,
    ) -> anyhow::Result<MarketingPromotion> {
        #[derive(sqlx::FromRow)]
        struct Row {
            promotion_id: uuid::Uuid,
            name: String,
            promo_type: String,
            status: String,
            priority: i32,
            exclusive: bool,
            scope_type: String,
            scope_json: Value,
            rule_json: Value,
            start_at: Option<DateTime<Utc>>,
            end_at: Option<DateTime<Utc>>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into marketing_promotions(
                tenant_id, name, promo_type, status, priority, exclusive, scope_type, scope_json,
                rule_json, start_at, end_at, created_by
            ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            returning promotion_id, name, promo_type, status, priority, exclusive, scope_type,
                      scope_json, rule_json, start_at, end_at, created_by, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(&input.promo_type)
        .bind(&input.status)
        .bind(input.priority)
        .bind(input.exclusive)
        .bind(&input.scope_type)
        .bind(&input.scope_json)
        .bind(&input.rule_json)
        .bind(input.start_at)
        .bind(input.end_at)
        .bind(&input.created_by)
        .fetch_one(self.pool)
        .await?;

        Ok(MarketingPromotion {
            promotion_id: row.promotion_id,
            tenant_id: input.tenant_id,
            name: row.name,
            promo_type: row.promo_type,
            status: row.status,
            priority: row.priority,
            exclusive: row.exclusive,
            scope_type: row.scope_type,
            scope_json: row.scope_json,
            rule_json: row.rule_json,
            start_at: row.start_at,
            end_at: row.end_at,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_promotion(
        &self,
        tenant_id: TenantId,
        promotion_id: uuid::Uuid,
        input: MarketingPromotionUpdate,
    ) -> anyhow::Result<Option<MarketingPromotion>> {
        let mut qb = QueryBuilder::<Postgres>::new("update marketing_promotions set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(promo_type) = input.promo_type {
            separated.push("promo_type = ");
            separated.push_bind(promo_type);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }
        if let Some(priority) = input.priority {
            separated.push("priority = ");
            separated.push_bind(priority);
        }
        if let Some(exclusive) = input.exclusive {
            separated.push("exclusive = ");
            separated.push_bind(exclusive);
        }
        if let Some(scope_type) = input.scope_type {
            separated.push("scope_type = ");
            separated.push_bind(scope_type);
        }
        if let Some(scope_json) = input.scope_json {
            separated.push("scope_json = ");
            separated.push_bind(scope_json);
        }
        if let Some(rule_json) = input.rule_json {
            separated.push("rule_json = ");
            separated.push_bind(rule_json);
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
        qb.push(" and promotion_id = ");
        qb.push_bind(promotion_id);
        qb.push(
            " returning promotion_id, name, promo_type, status, priority, exclusive, scope_type,
              scope_json, rule_json, start_at, end_at, created_by, created_at, updated_at",
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            promotion_id: uuid::Uuid,
            name: String,
            promo_type: String,
            status: String,
            priority: i32,
            exclusive: bool,
            scope_type: String,
            scope_json: Value,
            rule_json: Value,
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

        Ok(Some(MarketingPromotion {
            promotion_id: row.promotion_id,
            tenant_id,
            name: row.name,
            promo_type: row.promo_type,
            status: row.status,
            priority: row.priority,
            exclusive: row.exclusive,
            scope_type: row.scope_type,
            scope_json: row.scope_json,
            rule_json: row.rule_json,
            start_at: row.start_at,
            end_at: row.end_at,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_promotion(
        &self,
        tenant_id: TenantId,
        promotion_id: uuid::Uuid,
    ) -> anyhow::Result<Option<MarketingPromotion>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            promotion_id: uuid::Uuid,
            name: String,
            promo_type: String,
            status: String,
            priority: i32,
            exclusive: bool,
            scope_type: String,
            scope_json: Value,
            rule_json: Value,
            start_at: Option<DateTime<Utc>>,
            end_at: Option<DateTime<Utc>>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from marketing_promotions
            where tenant_id = $1 and promotion_id = $2
            returning promotion_id, name, promo_type, status, priority, exclusive, scope_type,
                      scope_json, rule_json, start_at, end_at, created_by, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(promotion_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| MarketingPromotion {
            promotion_id: row.promotion_id,
            tenant_id,
            name: row.name,
            promo_type: row.promo_type,
            status: row.status,
            priority: row.priority,
            exclusive: row.exclusive,
            scope_type: row.scope_type,
            scope_json: row.scope_json,
            rule_json: row.rule_json,
            start_at: row.start_at,
            end_at: row.end_at,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
