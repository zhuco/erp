use chrono::{DateTime, NaiveDate, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryCostingRun {
    pub costing_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub method: String,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub status: String,
    pub variance_cent: i64,
    pub result_json: Value,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct InventoryCostingRunListFilters {
    pub status: Option<String>,
    pub method: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct InventoryCostingRunListResult {
    pub items: Vec<InventoryCostingRun>,
    pub total: i64,
}

pub struct InventoryCostingRunCreate {
    pub tenant_id: TenantId,
    pub method: String,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub status: String,
    pub variance_cent: i64,
    pub result_json: Value,
    pub created_by: Option<String>,
}

pub struct InventoryCostingRunUpdate {
    pub method: Option<String>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub status: Option<String>,
    pub variance_cent: Option<i64>,
    pub result_json: Option<Value>,
}

pub struct InventoryCostingRunsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> InventoryCostingRunsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_runs(
        &self,
        tenant_id: TenantId,
        filters: InventoryCostingRunListFilters,
    ) -> anyhow::Result<InventoryCostingRunListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from inventory_costing_runs
            where tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(status) = &filters.status {
            count_qb.push(" and status = ");
            count_qb.push_bind(status);
        }
        if let Some(method) = &filters.method {
            count_qb.push(" and method = ");
            count_qb.push_bind(method);
        }
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select costing_id, method, period_start, period_end, status, variance_cent, result_json,
                   created_by, created_at, updated_at
            from inventory_costing_runs
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(status) = &filters.status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        if let Some(method) = &filters.method {
            qb.push(" and method = ");
            qb.push_bind(method);
        }
        qb.push(" order by created_at desc, costing_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            costing_id: uuid::Uuid,
            method: String,
            period_start: Option<NaiveDate>,
            period_end: Option<NaiveDate>,
            status: String,
            variance_cent: i64,
            result_json: Value,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| InventoryCostingRun {
                costing_id: row.costing_id,
                tenant_id,
                method: row.method,
                period_start: row.period_start,
                period_end: row.period_end,
                status: row.status,
                variance_cent: row.variance_cent,
                result_json: row.result_json,
                created_by: row.created_by,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(InventoryCostingRunListResult { items, total })
    }

    pub async fn create_run(
        &self,
        input: InventoryCostingRunCreate,
    ) -> anyhow::Result<InventoryCostingRun> {
        #[derive(sqlx::FromRow)]
        struct Row {
            costing_id: uuid::Uuid,
            method: String,
            period_start: Option<NaiveDate>,
            period_end: Option<NaiveDate>,
            status: String,
            variance_cent: i64,
            result_json: Value,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into inventory_costing_runs(
                tenant_id, method, period_start, period_end, status, variance_cent, result_json,
                created_by
            ) values ($1,$2,$3,$4,$5,$6,$7,$8)
            returning costing_id, method, period_start, period_end, status, variance_cent, result_json,
                      created_by, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.method)
        .bind(input.period_start)
        .bind(input.period_end)
        .bind(&input.status)
        .bind(input.variance_cent)
        .bind(&input.result_json)
        .bind(&input.created_by)
        .fetch_one(self.pool)
        .await?;

        Ok(InventoryCostingRun {
            costing_id: row.costing_id,
            tenant_id: input.tenant_id,
            method: row.method,
            period_start: row.period_start,
            period_end: row.period_end,
            status: row.status,
            variance_cent: row.variance_cent,
            result_json: row.result_json,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_run(
        &self,
        tenant_id: TenantId,
        costing_id: uuid::Uuid,
        input: InventoryCostingRunUpdate,
    ) -> anyhow::Result<Option<InventoryCostingRun>> {
        let mut qb = QueryBuilder::<Postgres>::new("update inventory_costing_runs set ");
        let mut separated = qb.separated(", ");

        if let Some(method) = input.method {
            separated.push("method = ");
            separated.push_bind(method);
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
        if let Some(variance_cent) = input.variance_cent {
            separated.push("variance_cent = ");
            separated.push_bind(variance_cent);
        }
        if let Some(result_json) = input.result_json {
            separated.push("result_json = ");
            separated.push_bind(result_json);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and costing_id = ");
        qb.push_bind(costing_id);
        qb.push(
            r#"
            returning costing_id, method, period_start, period_end, status, variance_cent,
                      result_json, created_by, created_at, updated_at
            "#,
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            costing_id: uuid::Uuid,
            method: String,
            period_start: Option<NaiveDate>,
            period_end: Option<NaiveDate>,
            status: String,
            variance_cent: i64,
            result_json: Value,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| InventoryCostingRun {
            costing_id: row.costing_id,
            tenant_id,
            method: row.method,
            period_start: row.period_start,
            period_end: row.period_end,
            status: row.status,
            variance_cent: row.variance_cent,
            result_json: row.result_json,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
