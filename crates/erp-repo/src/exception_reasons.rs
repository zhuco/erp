use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExceptionReason {
    pub tenant_id: TenantId,
    pub code: String,
    pub label: String,
    pub category: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ExceptionReasonListFilters {
    pub category: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct ExceptionReasonListResult {
    pub items: Vec<ExceptionReason>,
    pub total: i64,
}

pub struct ExceptionReasonsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ExceptionReasonsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_reasons(
        &self,
        tenant_id: TenantId,
        filters: ExceptionReasonListFilters,
    ) -> anyhow::Result<ExceptionReasonListResult> {
        let total: i64 = match filters.category.as_deref() {
            Some(category) => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from exception_reasons
                    where tenant_id = $1 and category = $2
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(category)
                .fetch_one(self.pool)
                .await?
            }
            None => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from exception_reasons
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
            select code, label, category, created_at, updated_at
            from exception_reasons
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(category) = filters.category {
            qb.push(" and category = ");
            qb.push_bind(category);
        }
        qb.push(" order by category asc, code asc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        let rows: Vec<ExceptionReasonRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_reason(tenant_id))
            .collect();

        Ok(ExceptionReasonListResult { items, total })
    }
}

#[derive(sqlx::FromRow)]
struct ExceptionReasonRow {
    code: String,
    label: String,
    category: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ExceptionReasonRow {
    fn into_reason(self, tenant_id: TenantId) -> ExceptionReason {
        ExceptionReason {
            tenant_id,
            code: self.code,
            label: self.label,
            category: self.category,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
