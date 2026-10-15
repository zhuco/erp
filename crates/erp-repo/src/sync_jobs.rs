use chrono::{DateTime, Utc};
use erp_types::{ShopId, TenantId};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJob {
    pub job_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub shop_id: ShopId,
    pub job_type: String,
    pub cursor: Option<String>,
    pub watermark: Option<DateTime<Utc>>,
    pub status: String,
    pub last_error: String,
    pub last_success_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct SyncJobListFilters {
    pub shop_id: Option<ShopId>,
    pub shop_ids: Option<Vec<ShopId>>,
    pub job_type: Option<String>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct SyncJobListResult {
    pub items: Vec<SyncJob>,
    pub total: i64,
}

pub struct SyncJobUpsert {
    pub tenant_id: TenantId,
    pub shop_id: ShopId,
    pub job_type: String,
    pub cursor: Option<String>,
    pub watermark: Option<DateTime<Utc>>,
    pub status: Option<String>,
    pub last_error: Option<String>,
}

pub struct SyncJobsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> SyncJobsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_job(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
    ) -> anyhow::Result<Option<SyncJob>> {
        let row: Option<SyncJobRow> = sqlx::query_as(
            r#"
            select job_id, shop_id, job_type, cursor, watermark, status, last_error,
                   last_success_at, created_at, updated_at
            from sync_jobs
            where tenant_id = $1 and job_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(job_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_job(tenant_id)))
    }

    pub async fn list_jobs(
        &self,
        tenant_id: TenantId,
        filters: SyncJobListFilters,
    ) -> anyhow::Result<SyncJobListResult> {
        let shop_id = filters.shop_id;
        let shop_ids = filters.shop_ids.clone();
        let job_type = filters.job_type.clone();
        let status = filters.status.clone();
        if shop_id.is_none()
            && let Some(ids) = &shop_ids
            && ids.is_empty()
        {
            return Ok(SyncJobListResult {
                items: vec![],
                total: 0,
            });
        }
        let mut total_qb =
            QueryBuilder::<Postgres>::new("select count(*) from sync_jobs where tenant_id = ");
        total_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(shop_id) = shop_id {
            total_qb.push(" and shop_id = ");
            total_qb.push_bind(uuid::Uuid::from(shop_id));
        } else if let Some(shop_ids) = &shop_ids {
            total_qb.push(" and shop_id = any(");
            total_qb.push_bind(
                shop_ids
                    .iter()
                    .map(|id| uuid::Uuid::from(*id))
                    .collect::<Vec<_>>(),
            );
            total_qb.push(")");
        }
        if let Some(job_type) = &job_type {
            total_qb.push(" and job_type = ");
            total_qb.push_bind(job_type);
        }
        if let Some(status) = &status {
            total_qb.push(" and status = ");
            total_qb.push_bind(status);
        }
        let total: i64 = total_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select job_id, shop_id, job_type, cursor, watermark, status, last_error,
                   last_success_at, created_at, updated_at
            from sync_jobs
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(shop_id) = shop_id {
            qb.push(" and shop_id = ");
            qb.push_bind(uuid::Uuid::from(shop_id));
        } else if let Some(shop_ids) = shop_ids {
            qb.push(" and shop_id = any(");
            qb.push_bind(
                shop_ids
                    .iter()
                    .map(|id| uuid::Uuid::from(*id))
                    .collect::<Vec<_>>(),
            );
            qb.push(")");
        }
        if let Some(job_type) = job_type {
            qb.push(" and job_type = ");
            qb.push_bind(job_type);
        }
        if let Some(status) = status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }

        qb.push(" order by updated_at desc, job_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        let rows: Vec<SyncJobRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_job(tenant_id))
            .collect();

        Ok(SyncJobListResult { items, total })
    }

    pub async fn upsert_job(&self, input: SyncJobUpsert) -> anyhow::Result<SyncJob> {
        let status = input.status.unwrap_or_else(|| "PENDING".to_string());
        let last_error = input.last_error.unwrap_or_default();
        let row: SyncJobRow = sqlx::query_as(
            r#"
            insert into sync_jobs (tenant_id, shop_id, job_type, cursor, watermark, status, last_error)
            values ($1,$2,$3,$4,$5,$6,$7)
            on conflict (tenant_id, shop_id, job_type)
            do update set
              cursor = excluded.cursor,
              watermark = excluded.watermark,
              status = excluded.status,
              last_error = excluded.last_error,
              updated_at = now()
            returning job_id, shop_id, job_type, cursor, watermark, status, last_error,
                      last_success_at, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(uuid::Uuid::from(input.shop_id))
        .bind(&input.job_type)
        .bind(input.cursor)
        .bind(input.watermark)
        .bind(status)
        .bind(last_error)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_job(input.tenant_id))
    }

    pub async fn try_mark_running(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
    ) -> anyhow::Result<Option<SyncJob>> {
        let row: Option<SyncJobRow> = sqlx::query_as(
            r#"
            update sync_jobs
            set status = 'RUNNING', updated_at = now()
            where tenant_id = $1 and job_id = $2 and status <> 'RUNNING'
            returning job_id, shop_id, job_type, cursor, watermark, status, last_error,
                      last_success_at, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(job_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_job(tenant_id)))
    }

    pub async fn update_job_result(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
        status: &str,
        cursor: Option<String>,
        watermark: Option<DateTime<Utc>>,
        last_error: Option<String>,
    ) -> anyhow::Result<Option<SyncJob>> {
        let row: Option<SyncJobRow> = sqlx::query_as(
            r#"
            update sync_jobs
            set status = $3,
                cursor = $4,
                watermark = $5,
                last_error = $6,
                last_success_at = case when $3 = 'SUCCESS' then now() else last_success_at end,
                updated_at = now()
            where tenant_id = $1 and job_id = $2
            returning job_id, shop_id, job_type, cursor, watermark, status, last_error,
                      last_success_at, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(job_id)
        .bind(status)
        .bind(cursor)
        .bind(watermark)
        .bind(last_error.unwrap_or_default())
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_job(tenant_id)))
    }

    pub async fn update_job_status(
        &self,
        tenant_id: TenantId,
        shop_id: ShopId,
        job_type: &str,
        status: &str,
        last_error: Option<String>,
    ) -> anyhow::Result<Option<SyncJob>> {
        let row: Option<SyncJobRow> = sqlx::query_as(
            r#"
            update sync_jobs
            set status = $4,
                last_error = $5,
                updated_at = now()
            where tenant_id = $1
              and shop_id = $2
              and job_type = $3
            returning job_id, shop_id, job_type, cursor, watermark, status, last_error,
                      last_success_at, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(shop_id))
        .bind(job_type)
        .bind(status)
        .bind(last_error.unwrap_or_default())
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_job(tenant_id)))
    }

    pub async fn list_jobs_for_run(&self, limit: i64) -> anyhow::Result<Vec<SyncJob>> {
        let rows: Vec<SyncJobRow> = sqlx::query_as(
            r#"
            select job_id, tenant_id, shop_id, job_type, cursor, watermark, status, last_error,
                   last_success_at, created_at, updated_at
            from sync_jobs
            where status <> 'DISABLED'
            order by updated_at asc
            limit $1
            "#,
        )
        .bind(limit)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let tenant_id = TenantId(row.tenant_id);
                row.into_job(tenant_id)
            })
            .collect())
    }
}

#[derive(sqlx::FromRow)]
struct SyncJobRow {
    job_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    shop_id: uuid::Uuid,
    job_type: String,
    cursor: Option<String>,
    watermark: Option<DateTime<Utc>>,
    status: String,
    last_error: String,
    last_success_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl SyncJobRow {
    fn into_job(self, tenant_id: TenantId) -> SyncJob {
        SyncJob {
            job_id: self.job_id,
            tenant_id,
            shop_id: ShopId(self.shop_id),
            job_type: self.job_type,
            cursor: self.cursor,
            watermark: self.watermark,
            status: self.status,
            last_error: self.last_error,
            last_success_at: self.last_success_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
