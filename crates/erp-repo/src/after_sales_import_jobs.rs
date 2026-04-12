use chrono::{DateTime, Utc};
use erp_types::TenantId;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

#[derive(Debug, Clone)]
pub struct AfterSalesImportJob {
    pub job_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub file_name: Option<String>,
    pub file_path: String,
    pub file_type: String,
    pub status: String,
    pub progress: i32,
    pub total_rows: i32,
    pub success_rows: i32,
    pub failed_rows: i32,
    pub attempts: i32,
    pub last_error: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AfterSalesImportJobCreate {
    pub tenant_id: TenantId,
    pub file_name: Option<String>,
    pub file_path: String,
    pub file_type: String,
}

#[derive(Debug, Clone)]
pub struct AfterSalesImportJobUpdate {
    pub status: Option<String>,
    pub progress: Option<i32>,
    pub total_rows: Option<i32>,
    pub success_rows: Option<i32>,
    pub failed_rows: Option<i32>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AfterSalesImportJobListFilters {
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct AfterSalesImportJobListResult {
    pub items: Vec<AfterSalesImportJob>,
    pub total: i64,
}

pub struct AfterSalesImportJobsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> AfterSalesImportJobsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_job(
        &self,
        input: AfterSalesImportJobCreate,
    ) -> anyhow::Result<AfterSalesImportJob> {
        #[derive(sqlx::FromRow)]
        struct Row {
            job_id: uuid::Uuid,
            tenant_id: uuid::Uuid,
            file_name: Option<String>,
            file_path: String,
            file_type: String,
            status: String,
            progress: i32,
            total_rows: i32,
            success_rows: i32,
            failed_rows: i32,
            attempts: i32,
            last_error: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into after_sales_import_jobs(
                tenant_id, file_name, file_path, file_type, status,
                progress, total_rows, success_rows, failed_rows, attempts, last_error
            ) values ($1,$2,$3,$4,'PENDING',0,0,0,0,0,'')
            returning
                job_id, tenant_id, file_name, file_path, file_type, status,
                progress, total_rows, success_rows, failed_rows, attempts, last_error,
                created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.file_name)
        .bind(input.file_path)
        .bind(input.file_type)
        .fetch_one(self.pool)
        .await?;

        Ok(AfterSalesImportJob {
            job_id: row.job_id,
            tenant_id: TenantId(row.tenant_id),
            file_name: row.file_name,
            file_path: row.file_path,
            file_type: row.file_type,
            status: row.status,
            progress: row.progress,
            total_rows: row.total_rows,
            success_rows: row.success_rows,
            failed_rows: row.failed_rows,
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
    ) -> anyhow::Result<Option<AfterSalesImportJob>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            job_id: uuid::Uuid,
            tenant_id: uuid::Uuid,
            file_name: Option<String>,
            file_path: String,
            file_type: String,
            status: String,
            progress: i32,
            total_rows: i32,
            success_rows: i32,
            failed_rows: i32,
            attempts: i32,
            last_error: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select
                job_id, tenant_id, file_name, file_path, file_type, status,
                progress, total_rows, success_rows, failed_rows, attempts, last_error,
                created_at, updated_at
            from after_sales_import_jobs
            where tenant_id = $1 and job_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(job_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| AfterSalesImportJob {
            job_id: row.job_id,
            tenant_id: TenantId(row.tenant_id),
            file_name: row.file_name,
            file_path: row.file_path,
            file_type: row.file_type,
            status: row.status,
            progress: row.progress,
            total_rows: row.total_rows,
            success_rows: row.success_rows,
            failed_rows: row.failed_rows,
            attempts: row.attempts,
            last_error: row.last_error,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn list_jobs(
        &self,
        tenant_id: TenantId,
        filters: AfterSalesImportJobListFilters,
    ) -> anyhow::Result<AfterSalesImportJobListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from after_sales_import_jobs where tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(status) = &filters.status {
            count_qb.push(" and status = ");
            count_qb.push_bind(status);
        }
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                job_id, tenant_id, file_name, file_path, file_type, status,
                progress, total_rows, success_rows, failed_rows, attempts, last_error,
                created_at, updated_at
            from after_sales_import_jobs
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(status) = &filters.status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        qb.push(" order by created_at desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);
        qb.push(" offset ");
        qb.push_bind(filters.offset);

        #[derive(sqlx::FromRow)]
        struct Row {
            job_id: uuid::Uuid,
            tenant_id: uuid::Uuid,
            file_name: Option<String>,
            file_path: String,
            file_type: String,
            status: String,
            progress: i32,
            total_rows: i32,
            success_rows: i32,
            failed_rows: i32,
            attempts: i32,
            last_error: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| AfterSalesImportJob {
                job_id: row.job_id,
                tenant_id: TenantId(row.tenant_id),
                file_name: row.file_name,
                file_path: row.file_path,
                file_type: row.file_type,
                status: row.status,
                progress: row.progress,
                total_rows: row.total_rows,
                success_rows: row.success_rows,
                failed_rows: row.failed_rows,
                attempts: row.attempts,
                last_error: row.last_error,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(AfterSalesImportJobListResult { items, total })
    }

    pub async fn claim_jobs(
        &self,
        limit: i64,
        max_attempts: i32,
    ) -> anyhow::Result<Vec<AfterSalesImportJob>> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct Row {
            job_id: uuid::Uuid,
            tenant_id: uuid::Uuid,
            file_name: Option<String>,
            file_path: String,
            file_type: String,
            attempts: i32,
            last_error: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select
                job_id, tenant_id, file_name, file_path, file_type,
                attempts, last_error, created_at, updated_at
            from after_sales_import_jobs
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
                update after_sales_import_jobs
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
            .map(|row| AfterSalesImportJob {
                job_id: row.job_id,
                tenant_id: TenantId(row.tenant_id),
                file_name: row.file_name,
                file_path: row.file_path,
                file_type: row.file_type,
                status: "RUNNING".to_string(),
                progress: 0,
                total_rows: 0,
                success_rows: 0,
                failed_rows: 0,
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
        input: AfterSalesImportJobUpdate,
    ) -> anyhow::Result<Option<AfterSalesImportJob>> {
        let mut qb = QueryBuilder::<Postgres>::new("update after_sales_import_jobs set ");
        let mut separated = qb.separated(", ");

        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }
        if let Some(progress) = input.progress {
            separated.push("progress = ");
            separated.push_bind(progress);
        }
        if let Some(total_rows) = input.total_rows {
            separated.push("total_rows = ");
            separated.push_bind(total_rows);
        }
        if let Some(success_rows) = input.success_rows {
            separated.push("success_rows = ");
            separated.push_bind(success_rows);
        }
        if let Some(failed_rows) = input.failed_rows {
            separated.push("failed_rows = ");
            separated.push_bind(failed_rows);
        }
        if let Some(last_error) = input.last_error {
            separated.push("last_error = ");
            separated.push_bind(last_error);
        }

        separated.push("updated_at = now()");
        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and job_id = ");
        qb.push_bind(job_id);
        qb.push(" returning job_id, tenant_id, file_name, file_path, file_type, status, progress, total_rows, success_rows, failed_rows, attempts, last_error, created_at, updated_at");

        #[derive(sqlx::FromRow)]
        struct Row {
            job_id: uuid::Uuid,
            tenant_id: uuid::Uuid,
            file_name: Option<String>,
            file_path: String,
            file_type: String,
            status: String,
            progress: i32,
            total_rows: i32,
            success_rows: i32,
            failed_rows: i32,
            attempts: i32,
            last_error: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| AfterSalesImportJob {
            job_id: row.job_id,
            tenant_id: TenantId(row.tenant_id),
            file_name: row.file_name,
            file_path: row.file_path,
            file_type: row.file_type,
            status: row.status,
            progress: row.progress,
            total_rows: row.total_rows,
            success_rows: row.success_rows,
            failed_rows: row.failed_rows,
            attempts: row.attempts,
            last_error: row.last_error,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
