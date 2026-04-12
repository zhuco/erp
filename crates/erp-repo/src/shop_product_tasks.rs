use chrono::{DateTime, Utc};
use erp_types::{Platform, ShopId, TenantId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopProductTask {
    pub task_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub task_type: String,
    pub status: String,
    pub shop_id: Option<ShopId>,
    pub platform: Option<Platform>,
    pub total_count: i32,
    pub success_count: i32,
    pub failure_count: i32,
    pub error: String,
    pub payload_json: Value,
    pub result_json: Value,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ShopProductTaskCreate {
    pub tenant_id: TenantId,
    pub task_type: String,
    pub shop_id: Option<ShopId>,
    pub platform: Option<Platform>,
    pub payload_json: Value,
}

#[derive(Debug, Clone)]
pub struct ShopProductTaskUpdate {
    pub status: Option<String>,
    pub total_count: Option<i32>,
    pub success_count: Option<i32>,
    pub failure_count: Option<i32>,
    pub error: Option<String>,
    pub result_json: Option<Value>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct ShopProductTaskListFilters {
    pub task_type: Option<String>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopProductTaskListResult {
    pub items: Vec<ShopProductTask>,
    pub total: i64,
}

#[derive(sqlx::FromRow)]
struct ShopProductTaskRow {
    task_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    task_type: String,
    status: String,
    shop_id: Option<uuid::Uuid>,
    platform: Option<String>,
    total_count: i32,
    success_count: i32,
    failure_count: i32,
    error: String,
    payload_json: Value,
    result_json: Value,
    started_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

pub struct ShopProductTasksRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ShopProductTasksRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_task(
        &self,
        input: ShopProductTaskCreate,
    ) -> anyhow::Result<ShopProductTask> {
        let row: ShopProductTaskRow = sqlx::query_as(
            r#"
            insert into shop_product_tasks(
                tenant_id, task_type, status, shop_id, platform,
                total_count, success_count, failure_count, error,
                payload_json, result_json, started_at, finished_at
            ) values ($1,$2,'RUNNING',$3,$4,0,0,0,'',$5,'{}',now(),null)
            returning
                task_id, tenant_id, task_type, status, shop_id, platform,
                total_count, success_count, failure_count, error, payload_json,
                result_json, started_at, finished_at, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.task_type)
        .bind(input.shop_id.map(uuid::Uuid::from))
        .bind(input.platform.map(|platform| platform.to_string()))
        .bind(&input.payload_json)
        .fetch_one(self.pool)
        .await?;

        map_task_row(row)
    }

    pub async fn update_task(
        &self,
        tenant_id: TenantId,
        task_id: uuid::Uuid,
        input: ShopProductTaskUpdate,
    ) -> anyhow::Result<Option<ShopProductTask>> {
        let mut qb = QueryBuilder::<Postgres>::new("update shop_product_tasks set ");
        let mut separated = qb.separated(", ");

        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }
        if let Some(total_count) = input.total_count {
            separated.push("total_count = ");
            separated.push_bind(total_count);
        }
        if let Some(success_count) = input.success_count {
            separated.push("success_count = ");
            separated.push_bind(success_count);
        }
        if let Some(failure_count) = input.failure_count {
            separated.push("failure_count = ");
            separated.push_bind(failure_count);
        }
        if let Some(error) = input.error {
            separated.push("error = ");
            separated.push_bind(error);
        }
        if let Some(result_json) = input.result_json {
            separated.push("result_json = ");
            separated.push_bind(result_json);
        }
        if let Some(finished_at) = input.finished_at {
            separated.push("finished_at = ");
            separated.push_bind(finished_at);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and task_id = ");
        qb.push_bind(task_id);
        qb.push(
            r#"
            returning
                task_id, tenant_id, task_type, status, shop_id, platform,
                total_count, success_count, failure_count, error, payload_json,
                result_json, started_at, finished_at, created_at, updated_at
            "#,
        );

        let row: Option<ShopProductTaskRow> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(match row {
            Some(row) => Some(map_task_row(row)?),
            None => None,
        })
    }

    pub async fn list_tasks(
        &self,
        tenant_id: TenantId,
        filters: ShopProductTaskListFilters,
    ) -> anyhow::Result<ShopProductTaskListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from shop_product_tasks where tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_task_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                task_id, tenant_id, task_type, status, shop_id, platform,
                total_count, success_count, failure_count, error, payload_json,
                result_json, started_at, finished_at, created_at, updated_at
            from shop_product_tasks
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_task_filters(&mut qb, &filters);
        qb.push(" order by created_at desc, task_id desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);
        qb.push(" offset ");
        qb.push_bind(filters.offset);

        let rows: Vec<ShopProductTaskRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(map_task_row(row)?);
        }

        Ok(ShopProductTaskListResult { items, total })
    }
}

fn map_task_row(row: ShopProductTaskRow) -> anyhow::Result<ShopProductTask> {
    let platform = match row.platform.as_deref() {
        Some(raw) => Some(Platform::from_str(raw).map_err(|e| anyhow::anyhow!(e))?),
        None => None,
    };
    Ok(ShopProductTask {
        task_id: row.task_id,
        tenant_id: TenantId(row.tenant_id),
        task_type: row.task_type,
        status: row.status,
        shop_id: row.shop_id.map(ShopId),
        platform,
        total_count: row.total_count,
        success_count: row.success_count,
        failure_count: row.failure_count,
        error: row.error,
        payload_json: row.payload_json,
        result_json: row.result_json,
        started_at: row.started_at,
        finished_at: row.finished_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

fn apply_task_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a ShopProductTaskListFilters,
) {
    if let Some(task_type) = filters.task_type.as_deref() {
        qb.push(" and task_type = ");
        qb.push_bind(task_type);
    }
    if let Some(status) = filters.status.as_deref() {
        qb.push(" and status = ");
        qb.push_bind(status);
    }
}
