use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketingCoupon {
    pub coupon_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub code_prefix: Option<String>,
    pub discount_type: String,
    pub discount_value: f64,
    pub max_usage: i32,
    pub per_user_limit: i32,
    pub status: String,
    pub rules_json: Value,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct MarketingCouponListFilters {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub discount_type: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct MarketingCouponListResult {
    pub items: Vec<MarketingCoupon>,
    pub total: i64,
}

pub struct MarketingCouponCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub code_prefix: Option<String>,
    pub discount_type: String,
    pub discount_value: f64,
    pub max_usage: i32,
    pub per_user_limit: i32,
    pub status: String,
    pub rules_json: Value,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub created_by: Option<String>,
}

pub struct MarketingCouponUpdate {
    pub name: Option<String>,
    pub code_prefix: Option<String>,
    pub discount_type: Option<String>,
    pub discount_value: Option<f64>,
    pub max_usage: Option<i32>,
    pub per_user_limit: Option<i32>,
    pub status: Option<String>,
    pub rules_json: Option<Value>,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
}

pub struct MarketingCouponsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> MarketingCouponsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_coupons(
        &self,
        tenant_id: TenantId,
        filters: MarketingCouponListFilters,
    ) -> anyhow::Result<MarketingCouponListResult> {
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
                    from marketing_coupons
                    where tenant_id = $1
                      and name ilike $2
                      and ($3::text is null or status = $3)
                      and ($4::text is null or discount_type = $4)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(keyword)
                .bind(filters.status.as_deref())
                .bind(filters.discount_type.as_deref())
                .fetch_one(self.pool)
                .await?
            }
            None => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from marketing_coupons
                    where tenant_id = $1
                      and ($2::text is null or status = $2)
                      and ($3::text is null or discount_type = $3)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(filters.status.as_deref())
                .bind(filters.discount_type.as_deref())
                .fetch_one(self.pool)
                .await?
            }
        };

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select coupon_id, name, code_prefix, discount_type, discount_value, max_usage,
                   per_user_limit, status, rules_json, start_at, end_at, created_by, created_at,
                   updated_at
            from marketing_coupons
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
        if let Some(discount_type) = &filters.discount_type {
            qb.push(" and discount_type = ");
            qb.push_bind(discount_type);
        }

        qb.push(" order by updated_at desc, coupon_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            coupon_id: uuid::Uuid,
            name: String,
            code_prefix: Option<String>,
            discount_type: String,
            discount_value: f64,
            max_usage: i32,
            per_user_limit: i32,
            status: String,
            rules_json: Value,
            start_at: Option<DateTime<Utc>>,
            end_at: Option<DateTime<Utc>>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| MarketingCoupon {
                coupon_id: row.coupon_id,
                tenant_id,
                name: row.name,
                code_prefix: row.code_prefix,
                discount_type: row.discount_type,
                discount_value: row.discount_value,
                max_usage: row.max_usage,
                per_user_limit: row.per_user_limit,
                status: row.status,
                rules_json: row.rules_json,
                start_at: row.start_at,
                end_at: row.end_at,
                created_by: row.created_by,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(MarketingCouponListResult { items, total })
    }

    pub async fn create_coupon(
        &self,
        input: MarketingCouponCreate,
    ) -> anyhow::Result<MarketingCoupon> {
        #[derive(sqlx::FromRow)]
        struct Row {
            coupon_id: uuid::Uuid,
            name: String,
            code_prefix: Option<String>,
            discount_type: String,
            discount_value: f64,
            max_usage: i32,
            per_user_limit: i32,
            status: String,
            rules_json: Value,
            start_at: Option<DateTime<Utc>>,
            end_at: Option<DateTime<Utc>>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into marketing_coupons(
                tenant_id, name, code_prefix, discount_type, discount_value, max_usage,
                per_user_limit, status, rules_json, start_at, end_at, created_by
            ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            returning coupon_id, name, code_prefix, discount_type, discount_value, max_usage,
                      per_user_limit, status, rules_json, start_at, end_at, created_by, created_at,
                      updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(&input.code_prefix)
        .bind(&input.discount_type)
        .bind(input.discount_value)
        .bind(input.max_usage)
        .bind(input.per_user_limit)
        .bind(&input.status)
        .bind(&input.rules_json)
        .bind(input.start_at)
        .bind(input.end_at)
        .bind(&input.created_by)
        .fetch_one(self.pool)
        .await?;

        Ok(MarketingCoupon {
            coupon_id: row.coupon_id,
            tenant_id: input.tenant_id,
            name: row.name,
            code_prefix: row.code_prefix,
            discount_type: row.discount_type,
            discount_value: row.discount_value,
            max_usage: row.max_usage,
            per_user_limit: row.per_user_limit,
            status: row.status,
            rules_json: row.rules_json,
            start_at: row.start_at,
            end_at: row.end_at,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_coupon(
        &self,
        tenant_id: TenantId,
        coupon_id: uuid::Uuid,
        input: MarketingCouponUpdate,
    ) -> anyhow::Result<Option<MarketingCoupon>> {
        let mut qb = QueryBuilder::<Postgres>::new("update marketing_coupons set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(code_prefix) = input.code_prefix {
            separated.push("code_prefix = ");
            separated.push_bind(code_prefix);
        }
        if let Some(discount_type) = input.discount_type {
            separated.push("discount_type = ");
            separated.push_bind(discount_type);
        }
        if let Some(discount_value) = input.discount_value {
            separated.push("discount_value = ");
            separated.push_bind(discount_value);
        }
        if let Some(max_usage) = input.max_usage {
            separated.push("max_usage = ");
            separated.push_bind(max_usage);
        }
        if let Some(per_user_limit) = input.per_user_limit {
            separated.push("per_user_limit = ");
            separated.push_bind(per_user_limit);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }
        if let Some(rules_json) = input.rules_json {
            separated.push("rules_json = ");
            separated.push_bind(rules_json);
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
        qb.push(" and coupon_id = ");
        qb.push_bind(coupon_id);
        qb.push(
            " returning coupon_id, name, code_prefix, discount_type, discount_value, max_usage,
              per_user_limit, status, rules_json, start_at, end_at, created_by, created_at,
              updated_at",
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            coupon_id: uuid::Uuid,
            name: String,
            code_prefix: Option<String>,
            discount_type: String,
            discount_value: f64,
            max_usage: i32,
            per_user_limit: i32,
            status: String,
            rules_json: Value,
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

        Ok(Some(MarketingCoupon {
            coupon_id: row.coupon_id,
            tenant_id,
            name: row.name,
            code_prefix: row.code_prefix,
            discount_type: row.discount_type,
            discount_value: row.discount_value,
            max_usage: row.max_usage,
            per_user_limit: row.per_user_limit,
            status: row.status,
            rules_json: row.rules_json,
            start_at: row.start_at,
            end_at: row.end_at,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_coupon(
        &self,
        tenant_id: TenantId,
        coupon_id: uuid::Uuid,
    ) -> anyhow::Result<Option<MarketingCoupon>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            coupon_id: uuid::Uuid,
            name: String,
            code_prefix: Option<String>,
            discount_type: String,
            discount_value: f64,
            max_usage: i32,
            per_user_limit: i32,
            status: String,
            rules_json: Value,
            start_at: Option<DateTime<Utc>>,
            end_at: Option<DateTime<Utc>>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from marketing_coupons
            where tenant_id = $1 and coupon_id = $2
            returning coupon_id, name, code_prefix, discount_type, discount_value, max_usage,
                      per_user_limit, status, rules_json, start_at, end_at, created_by, created_at,
                      updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(coupon_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| MarketingCoupon {
            coupon_id: row.coupon_id,
            tenant_id,
            name: row.name,
            code_prefix: row.code_prefix,
            discount_type: row.discount_type,
            discount_value: row.discount_value,
            max_usage: row.max_usage,
            per_user_limit: row.per_user_limit,
            status: row.status,
            rules_json: row.rules_json,
            start_at: row.start_at,
            end_at: row.end_at,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
