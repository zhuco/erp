use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportJob {
    pub job_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub job_type: String,
    pub params_json: Value,
    pub status: String,
    pub output_url: Option<String>,
    pub progress: i32,
    pub attempts: i32,
    pub last_error: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ExportJobCreate {
    pub tenant_id: TenantId,
    pub job_type: String,
    pub params_json: Value,
}

#[derive(Debug, Clone)]
pub struct ExportJobUpdate {
    pub status: Option<String>,
    pub output_url: Option<Option<String>>,
    pub progress: Option<i32>,
    pub last_error: Option<String>,
    pub params_json: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct ExportJobListFilters {
    pub status: Option<String>,
    pub job_type: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct ExportJobListResult {
    pub items: Vec<ExportJob>,
    pub total: i64,
}

pub struct ExportJobsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ExportJobsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_job(&self, input: ExportJobCreate) -> anyhow::Result<ExportJob> {
        #[derive(sqlx::FromRow)]
        struct Row {
            job_id: uuid::Uuid,
            tenant_id: uuid::Uuid,
            job_type: String,
            params_json: Value,
            status: String,
            output_url: Option<String>,
            progress: i32,
            attempts: i32,
            last_error: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into export_jobs(
                tenant_id, job_type, params_json, status, progress, attempts, last_error
            ) values ($1, $2, $3, 'PENDING', 0, 0, '')
            returning
                job_id, tenant_id, job_type, params_json, status,
                output_url, progress, attempts, last_error, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.job_type)
        .bind(&input.params_json)
        .fetch_one(self.pool)
        .await?;

        Ok(ExportJob {
            job_id: row.job_id,
            tenant_id: TenantId(row.tenant_id),
            job_type: row.job_type,
            params_json: row.params_json,
            status: row.status,
            output_url: row.output_url,
            progress: row.progress,
            attempts: row.attempts,
            last_error: row.last_error,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn get_job(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ExportJob>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            job_id: uuid::Uuid,
            tenant_id: uuid::Uuid,
            job_type: String,
            params_json: Value,
            status: String,
            output_url: Option<String>,
            progress: i32,
            attempts: i32,
            last_error: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select
                job_id, tenant_id, job_type, params_json, status,
                output_url, progress, attempts, last_error, created_at, updated_at
            from export_jobs
            where tenant_id = $1 and job_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(job_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ExportJob {
            job_id: row.job_id,
            tenant_id: TenantId(row.tenant_id),
            job_type: row.job_type,
            params_json: row.params_json,
            status: row.status,
            output_url: row.output_url,
            progress: row.progress,
            attempts: row.attempts,
            last_error: row.last_error,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn list_jobs(
        &self,
        tenant_id: TenantId,
        filters: ExportJobListFilters,
    ) -> anyhow::Result<ExportJobListResult> {
        let mut count_qb =
            QueryBuilder::<Postgres>::new("select count(*) from export_jobs where tenant_id = ");
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_list_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                job_id, tenant_id, job_type, params_json, status,
                output_url, progress, attempts, last_error, created_at, updated_at
            from export_jobs
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_list_filters(&mut qb, &filters);
        qb.push(" order by created_at desc, job_id desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);
        qb.push(" offset ");
        qb.push_bind(filters.offset);

        #[derive(sqlx::FromRow)]
        struct Row {
            job_id: uuid::Uuid,
            tenant_id: uuid::Uuid,
            job_type: String,
            params_json: Value,
            status: String,
            output_url: Option<String>,
            progress: i32,
            attempts: i32,
            last_error: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| ExportJob {
                job_id: row.job_id,
                tenant_id: TenantId(row.tenant_id),
                job_type: row.job_type,
                params_json: row.params_json,
                status: row.status,
                output_url: row.output_url,
                progress: row.progress,
                attempts: row.attempts,
                last_error: row.last_error,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(ExportJobListResult { items, total })
    }

    pub async fn claim_jobs(
        &self,
        limit: i64,
        max_attempts: i32,
    ) -> anyhow::Result<Vec<ExportJob>> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct Row {
            job_id: uuid::Uuid,
            tenant_id: uuid::Uuid,
            job_type: String,
            params_json: Value,
            output_url: Option<String>,
            attempts: i32,
            last_error: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select
                job_id, tenant_id, job_type, params_json,
                output_url, attempts, last_error, created_at, updated_at
            from export_jobs
            where status in ('PENDING', 'FAILED')
              and attempts < $1
            order by created_at asc
            limit $2
            for update skip locked
            "#,
        )
        .bind(max_attempts)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await?;

        for row in &rows {
            sqlx::query(
                r#"
                update export_jobs
                set status = 'RUNNING',
                    attempts = attempts + 1,
                    progress = 0,
                    updated_at = now()
                where job_id = $1
                "#,
            )
            .bind(row.job_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(rows
            .into_iter()
            .map(|row| ExportJob {
                job_id: row.job_id,
                tenant_id: TenantId(row.tenant_id),
                job_type: row.job_type,
                params_json: row.params_json,
                status: "RUNNING".to_string(),
                output_url: row.output_url,
                progress: 0,
                attempts: row.attempts + 1,
                last_error: row.last_error,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect())
    }

    pub async fn update_job(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
        input: ExportJobUpdate,
    ) -> anyhow::Result<Option<ExportJob>> {
        let mut qb = QueryBuilder::<Postgres>::new("update export_jobs set ");
        let mut separated = qb.separated(", ");

        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }
        if let Some(output_url) = input.output_url {
            separated.push("output_url = ");
            separated.push_bind(output_url);
        }
        if let Some(progress) = input.progress {
            separated.push("progress = ");
            separated.push_bind(progress);
        }
        if let Some(last_error) = input.last_error {
            separated.push("last_error = ");
            separated.push_bind(last_error);
        }
        if let Some(params_json) = input.params_json {
            separated.push("params_json = ");
            separated.push_bind(params_json);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and job_id = ");
        qb.push_bind(job_id);
        qb.push(
            r#"
            returning
                job_id, tenant_id, job_type, params_json, status,
                output_url, progress, attempts, last_error, created_at, updated_at
            "#,
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            job_id: uuid::Uuid,
            tenant_id: uuid::Uuid,
            job_type: String,
            params_json: Value,
            status: String,
            output_url: Option<String>,
            progress: i32,
            attempts: i32,
            last_error: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| ExportJob {
            job_id: row.job_id,
            tenant_id: TenantId(row.tenant_id),
            job_type: row.job_type,
            params_json: row.params_json,
            status: row.status,
            output_url: row.output_url,
            progress: row.progress,
            attempts: row.attempts,
            last_error: row.last_error,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}

fn apply_list_filters<'a>(qb: &mut QueryBuilder<'a, Postgres>, filters: &'a ExportJobListFilters) {
    if let Some(status) = filters.status.as_deref() {
        qb.push(" and status = ");
        qb.push_bind(status);
    }
    if let Some(job_type) = filters.job_type.as_deref() {
        qb.push(" and job_type = ");
        qb.push_bind(job_type);
    }
}
