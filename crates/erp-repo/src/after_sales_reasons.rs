use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone)]
pub struct AfterSalesReason {
    pub tenant_id: TenantId,
    pub reason_code: String,
    pub label: String,
    pub category: Option<String>,
    pub status: String,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct AfterSalesReasonListFilters {
    pub category: Option<String>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct AfterSalesReasonListResult {
    pub items: Vec<AfterSalesReason>,
    pub total: i64,
}

pub struct AfterSalesReasonUpsert {
    pub tenant_id: TenantId,
    pub reason_code: String,
    pub label: String,
    pub category: Option<String>,
    pub status: String,
    pub sort_order: i32,
}

pub struct AfterSalesReasonsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> AfterSalesReasonsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_reasons(
        &self,
        tenant_id: TenantId,
        filters: AfterSalesReasonListFilters,
    ) -> anyhow::Result<AfterSalesReasonListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                reason_code, label, category, status, sort_order,
                created_at, updated_at
            from after_sales_reasons
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(category) = filters.category.as_ref() {
            qb.push(" and category = ");
            qb.push_bind(category);
        }
        if let Some(status) = filters.status.as_ref() {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        qb.push(" order by sort_order asc, updated_at desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);
        qb.push(" offset ");
        qb.push_bind(filters.offset);

        let rows: Vec<AfterSalesReasonRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_reason(tenant_id))
            .collect();

        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from after_sales_reasons where tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(category) = filters.category.as_ref() {
            count_qb.push(" and category = ");
            count_qb.push_bind(category);
        }
        if let Some(status) = filters.status.as_ref() {
            count_qb.push(" and status = ");
            count_qb.push_bind(status);
        }
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        Ok(AfterSalesReasonListResult { items, total })
    }

    pub async fn get_reason(
        &self,
        tenant_id: TenantId,
        reason_code: &str,
    ) -> anyhow::Result<Option<AfterSalesReason>> {
        let row = sqlx::query_as::<_, AfterSalesReasonRow>(
            r#"
            select
                reason_code, label, category, status, sort_order,
                created_at, updated_at
            from after_sales_reasons
            where tenant_id = $1 and reason_code = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(reason_code)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_reason(tenant_id)))
    }

    pub async fn upsert_reason(
        &self,
        input: AfterSalesReasonUpsert,
    ) -> anyhow::Result<AfterSalesReason> {
        if input.reason_code.trim().is_empty() {
            return Err(anyhow!("reason_code required"));
        }
        if input.label.trim().is_empty() {
            return Err(anyhow!("label required"));
        }
        let row: AfterSalesReasonRow = sqlx::query_as(
            r#"
            insert into after_sales_reasons(
                tenant_id, reason_code, label, category, status, sort_order
            ) values ($1,$2,$3,$4,$5,$6)
            on conflict (tenant_id, reason_code)
            do update set
                label = excluded.label,
                category = excluded.category,
                status = excluded.status,
                sort_order = excluded.sort_order,
                updated_at = now()
            returning
                reason_code, label, category, status, sort_order,
                created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.reason_code.trim())
        .bind(input.label.trim())
        .bind(input.category.as_deref())
        .bind(input.status)
        .bind(input.sort_order)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_reason(input.tenant_id))
    }

    pub async fn delete_reason(
        &self,
        tenant_id: TenantId,
        reason_code: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            delete from after_sales_reasons
            where tenant_id = $1 and reason_code = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(reason_code)
        .execute(self.pool)
        .await?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct AfterSalesReasonRow {
    reason_code: String,
    label: String,
    category: Option<String>,
    status: String,
    sort_order: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl AfterSalesReasonRow {
    fn into_reason(self, tenant_id: TenantId) -> AfterSalesReason {
        AfterSalesReason {
            tenant_id,
            reason_code: self.reason_code,
            label: self.label,
            category: self.category,
            status: self.status,
            sort_order: self.sort_order,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
