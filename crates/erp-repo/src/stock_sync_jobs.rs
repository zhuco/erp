use chrono::{DateTime, Utc};
use erp_types::{ShopId, TenantId};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockSyncJob {
    pub job_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub shop_id: ShopId,
    pub sku_id: uuid::Uuid,
    pub warehouse_id: uuid::Uuid,
    pub target_qty: i64,
    pub status: String,
    pub attempts: i32,
    pub last_error: Option<String>,
    pub success_count: i32,
    pub failure_count: i32,
    pub last_duration_ms: i32,
    pub last_success_at: Option<DateTime<Utc>>,
    pub next_retry_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct StockSyncJobListFilters {
    pub shop_id: Option<ShopId>,
    pub sku_id: Option<uuid::Uuid>,
    pub warehouse_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct StockSyncJobListResult {
    pub items: Vec<StockSyncJob>,
    pub total: i64,
}

pub struct StockSyncJobUpsert {
    pub tenant_id: TenantId,
    pub shop_id: ShopId,
    pub sku_id: uuid::Uuid,
    pub warehouse_id: uuid::Uuid,
    pub target_qty: i64,
    pub next_retry_at: DateTime<Utc>,
}

pub struct StockSyncJobsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> StockSyncJobsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_jobs(
        &self,
        tenant_id: TenantId,
        filters: StockSyncJobListFilters,
    ) -> anyhow::Result<StockSyncJobListResult> {
        let mut total_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from stock_sync_jobs where tenant_id = ",
        );
        total_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut total_qb, &filters);
        let total: i64 = total_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                job_id, tenant_id, shop_id, sku_id, warehouse_id, target_qty,
                status, attempts, last_error, success_count, failure_count, last_duration_ms,
                last_success_at, next_retry_at, created_at, updated_at
            from stock_sync_jobs
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut qb, &filters);
        qb.push(" order by created_at desc, job_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        let rows: Vec<StockSyncJobRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows.into_iter().map(StockSyncJobRow::into_job).collect();

        Ok(StockSyncJobListResult { items, total })
    }

    pub async fn upsert_pending_job(
        &self,
        input: StockSyncJobUpsert,
    ) -> anyhow::Result<StockSyncJob> {
        let row: StockSyncJobRow = sqlx::query_as(
            r#"
            insert into stock_sync_jobs(
                tenant_id, shop_id, sku_id, warehouse_id, target_qty, status, next_retry_at
            ) values ($1,$2,$3,$4,$5,'PENDING', $6)
            on conflict (tenant_id, shop_id, sku_id, warehouse_id)
                where status = 'PENDING'
            do update set
                target_qty = excluded.target_qty,
                next_retry_at = excluded.next_retry_at,
                last_error = null,
                updated_at = now()
            returning
                job_id, tenant_id, shop_id, sku_id, warehouse_id, target_qty,
                status, attempts, last_error, success_count, failure_count, last_duration_ms,
                last_success_at, next_retry_at, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(uuid::Uuid::from(input.shop_id))
        .bind(input.sku_id)
        .bind(input.warehouse_id)
        .bind(input.target_qty)
        .bind(input.next_retry_at)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_job())
    }

    pub async fn try_mark_sending(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
        expected_updated_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<StockSyncJob>> {
        let row: Option<StockSyncJobRow> = sqlx::query_as(
            r#"
            update stock_sync_jobs
            set status = 'SENDING',
                updated_at = now()
            where tenant_id = $1
              and job_id = $2
              and status in ('PENDING', 'FAILED')
              and updated_at = $3
              and next_retry_at <= $4
            returning
                job_id, tenant_id, shop_id, sku_id, warehouse_id, target_qty,
                status, attempts, last_error, success_count, failure_count, last_duration_ms,
                last_success_at, next_retry_at, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(job_id)
        .bind(expected_updated_at)
        .bind(now)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(StockSyncJobRow::into_job))
    }

    pub async fn list_jobs_for_run(&self, limit: i64) -> anyhow::Result<Vec<StockSyncJob>> {
        let rows: Vec<StockSyncJobRow> = sqlx::query_as(
            r#"
            select
                job_id, tenant_id, shop_id, sku_id, warehouse_id, target_qty,
                status, attempts, last_error, success_count, failure_count, last_duration_ms,
                last_success_at, next_retry_at, created_at, updated_at
            from stock_sync_jobs
            where status in ('PENDING', 'FAILED')
              and next_retry_at <= now()
            order by next_retry_at asc, updated_at asc, job_id asc
            limit $1
            "#,
        )
        .bind(limit)
        .fetch_all(self.pool)
        .await?;

        Ok(rows.into_iter().map(StockSyncJobRow::into_job).collect())
    }

    pub async fn mark_sent(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
        duration_ms: i32,
    ) -> anyhow::Result<Option<StockSyncJob>> {
        let row: Option<StockSyncJobRow> = sqlx::query_as(
            r#"
            update stock_sync_jobs
            set status = 'SENT',
                attempts = attempts + 1,
                success_count = success_count + 1,
                last_duration_ms = $3,
                last_error = null,
                last_success_at = now(),
                next_retry_at = now(),
                updated_at = now()
            where tenant_id = $1 and job_id = $2 and status = 'SENDING'
            returning
                job_id, tenant_id, shop_id, sku_id, warehouse_id, target_qty,
                status, attempts, last_error, success_count, failure_count, last_duration_ms,
                last_success_at, next_retry_at, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(job_id)
        .bind(duration_ms)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(StockSyncJobRow::into_job))
    }

    pub async fn mark_failed(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
        last_error: String,
        status: &str,
        next_retry_at: DateTime<Utc>,
        duration_ms: i32,
    ) -> anyhow::Result<Option<StockSyncJob>> {
        let row: Option<StockSyncJobRow> = sqlx::query_as(
            r#"
            update stock_sync_jobs
            set status = $3,
                attempts = attempts + 1,
                failure_count = failure_count + 1,
                last_duration_ms = $6,
                last_error = $4,
                next_retry_at = $5,
                updated_at = now()
            where tenant_id = $1 and job_id = $2 and status = 'SENDING'
            returning
                job_id, tenant_id, shop_id, sku_id, warehouse_id, target_qty,
                status, attempts, last_error, success_count, failure_count, last_duration_ms,
                last_success_at, next_retry_at, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(job_id)
        .bind(status)
        .bind(last_error)
        .bind(next_retry_at)
        .bind(duration_ms)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(StockSyncJobRow::into_job))
    }

    pub async fn retry_failed_job(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
    ) -> anyhow::Result<Option<StockSyncJob>> {
        let row: Option<StockSyncJobRow> = sqlx::query_as(
            r#"
            update stock_sync_jobs
            set status = 'PENDING',
                last_error = null,
                next_retry_at = now(),
                updated_at = now()
            where tenant_id = $1 and job_id = $2 and status in ('FAILED', 'DLQ')
            returning
                job_id, tenant_id, shop_id, sku_id, warehouse_id, target_qty,
                status, attempts, last_error, success_count, failure_count, last_duration_ms,
                last_success_at, next_retry_at, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(job_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(StockSyncJobRow::into_job))
    }

    pub async fn request_job_run(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
    ) -> anyhow::Result<Option<StockSyncJob>> {
        let row: Option<StockSyncJobRow> = sqlx::query_as(
            r#"
            update stock_sync_jobs
            set status = 'PENDING',
                last_error = null,
                next_retry_at = now(),
                updated_at = now()
            where tenant_id = $1 and job_id = $2 and status <> 'SENDING'
            returning
                job_id, tenant_id, shop_id, sku_id, warehouse_id, target_qty,
                status, attempts, last_error, success_count, failure_count, last_duration_ms,
                last_success_at, next_retry_at, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(job_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(StockSyncJobRow::into_job))
    }
}

#[derive(sqlx::FromRow)]
struct StockSyncJobRow {
    job_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    shop_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    target_qty: i64,
    status: String,
    attempts: i32,
    last_error: Option<String>,
    success_count: i32,
    failure_count: i32,
    last_duration_ms: i32,
    last_success_at: Option<DateTime<Utc>>,
    next_retry_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl StockSyncJobRow {
    fn into_job(self) -> StockSyncJob {
        StockSyncJob {
            job_id: self.job_id,
            tenant_id: TenantId(self.tenant_id),
            shop_id: ShopId(self.shop_id),
            sku_id: self.sku_id,
            warehouse_id: self.warehouse_id,
            target_qty: self.target_qty,
            status: self.status,
            attempts: self.attempts,
            last_error: self.last_error,
            success_count: self.success_count,
            failure_count: self.failure_count,
            last_duration_ms: self.last_duration_ms,
            last_success_at: self.last_success_at,
            next_retry_at: self.next_retry_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

fn apply_filters<'a>(qb: &mut QueryBuilder<'a, Postgres>, filters: &'a StockSyncJobListFilters) {
    if let Some(shop_id) = filters.shop_id {
        qb.push(" and shop_id = ");
        qb.push_bind(uuid::Uuid::from(shop_id));
    }
    if let Some(sku_id) = filters.sku_id {
        qb.push(" and sku_id = ");
        qb.push_bind(sku_id);
    }
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(status) = &filters.status {
        qb.push(" and status = ");
        qb.push_bind(status);
    }
}
