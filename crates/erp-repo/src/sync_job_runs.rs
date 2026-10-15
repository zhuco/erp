use chrono::{DateTime, Utc};
use erp_types::{ShopId, TenantId};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJobRun {
    pub run_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub job_id: uuid::Uuid,
    pub shop_id: ShopId,
    pub job_type: String,
    pub status: String,
    pub item_count: i32,
    pub duration_ms: i64,
    pub error: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SyncJobRunCreate {
    pub tenant_id: TenantId,
    pub job_id: uuid::Uuid,
    pub shop_id: ShopId,
    pub job_type: String,
    pub status: String,
    pub item_count: i32,
    pub duration_ms: i64,
    pub error: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SyncJobRunListFilters {
    pub job_id: Option<uuid::Uuid>,
    pub shop_id: Option<ShopId>,
    pub shop_ids: Option<Vec<ShopId>>,
    pub job_type: Option<String>,
    pub status: Option<String>,
    pub started_from: Option<DateTime<Utc>>,
    pub started_to: Option<DateTime<Utc>>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct SyncJobRunListResult {
    pub items: Vec<SyncJobRun>,
    pub total: i64,
}

pub struct SyncJobRunsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> SyncJobRunsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_run(&self, input: SyncJobRunCreate) -> anyhow::Result<SyncJobRun> {
        let row: SyncJobRunRow = sqlx::query_as(
            r#"
            insert into sync_job_runs(
                tenant_id, job_id, shop_id, job_type, status, item_count, duration_ms, error,
                started_at, finished_at
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            returning
                run_id, tenant_id, job_id, shop_id, job_type, status, item_count, duration_ms,
                error, started_at, finished_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.job_id)
        .bind(uuid::Uuid::from(input.shop_id))
        .bind(&input.job_type)
        .bind(&input.status)
        .bind(input.item_count)
        .bind(input.duration_ms)
        .bind(&input.error)
        .bind(input.started_at)
        .bind(input.finished_at)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_run())
    }

    pub async fn list_runs(
        &self,
        tenant_id: TenantId,
        filters: SyncJobRunListFilters,
    ) -> anyhow::Result<SyncJobRunListResult> {
        let mut total_qb =
            QueryBuilder::<Postgres>::new("select count(*) from sync_job_runs where tenant_id = ");
        total_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut total_qb, &filters);
        let total: i64 = total_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                run_id, tenant_id, job_id, shop_id, job_type, status, item_count, duration_ms,
                error, started_at, finished_at
            from sync_job_runs
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut qb, &filters);
        qb.push(" order by started_at desc, run_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        let rows: Vec<SyncJobRunRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows.into_iter().map(SyncJobRunRow::into_run).collect();
        Ok(SyncJobRunListResult { items, total })
    }
}

fn apply_filters<'a>(qb: &mut QueryBuilder<'a, Postgres>, filters: &'a SyncJobRunListFilters) {
    if let Some(job_id) = filters.job_id {
        qb.push(" and job_id = ");
        qb.push_bind(job_id);
    }
    if let Some(shop_id) = filters.shop_id {
        qb.push(" and shop_id = ");
        qb.push_bind(uuid::Uuid::from(shop_id));
    } else if let Some(shop_ids) = &filters.shop_ids
        && !shop_ids.is_empty()
    {
        qb.push(" and shop_id = any(");
        qb.push_bind(
            shop_ids
                .iter()
                .map(|id| uuid::Uuid::from(*id))
                .collect::<Vec<_>>(),
        );
        qb.push(")");
    }
    if let Some(job_type) = &filters.job_type {
        qb.push(" and job_type = ");
        qb.push_bind(job_type);
    }
    if let Some(status) = &filters.status {
        qb.push(" and status = ");
        qb.push_bind(status);
    }
    if let Some(started_from) = filters.started_from {
        qb.push(" and started_at >= ");
        qb.push_bind(started_from);
    }
    if let Some(started_to) = filters.started_to {
        qb.push(" and started_at <= ");
        qb.push_bind(started_to);
    }
}

#[derive(sqlx::FromRow)]
struct SyncJobRunRow {
    run_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    job_id: uuid::Uuid,
    shop_id: uuid::Uuid,
    job_type: String,
    status: String,
    item_count: i32,
    duration_ms: i64,
    error: String,
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
}

impl SyncJobRunRow {
    fn into_run(self) -> SyncJobRun {
        SyncJobRun {
            run_id: self.run_id,
            tenant_id: TenantId(self.tenant_id),
            job_id: self.job_id,
            shop_id: ShopId(self.shop_id),
            job_type: self.job_type,
            status: self.status,
            item_count: self.item_count,
            duration_ms: self.duration_ms,
            error: self.error,
            started_at: self.started_at,
            finished_at: self.finished_at,
        }
    }
}
