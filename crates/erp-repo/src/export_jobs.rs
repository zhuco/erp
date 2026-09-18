use chrono::{DateTime, Utc};
use erp_grid_core::SortRule;
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

#[derive(Debug, thiserror::Error)]
pub enum ProductExportCreateError {
    #[error("product export request identity conflicts with an existing job")]
    Conflict,
    #[error("invalid product export request: {0}")]
    Invalid(String),
    #[error("product selection is unavailable")]
    Unavailable,
    #[error(transparent)]
    Storage(#[from] anyhow::Error),
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
        .await
        .map_err(anyhow::Error::from)?;

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

    /// Creates an actor-bound product export whose request identity is also its
    /// durable job identity. An exact retry returns the original job; reuse of
    /// the identity for another actor, tenant, type, or query is rejected.
    pub async fn create_product_job_idempotent(
        &self,
        tenant_id: TenantId,
        actor_user_id: erp_types::UserId,
        request_id: uuid::Uuid,
        normalized_query: Value,
    ) -> Result<ExportJob, ProductExportCreateError> {
        let params_json = serde_json::json!({
            "request_id": request_id,
            "actor_user_id": uuid::Uuid::from(actor_user_id),
            "query": normalized_query,
        });
        let mut tx = self.pool.begin().await.map_err(anyhow::Error::from)?;
        let inserted = sqlx::query(
            r#"
            insert into export_jobs(
                job_id, tenant_id, job_type, params_json,
                status, progress, attempts, last_error
            ) values ($1, $2, 'PRODUCT_EXPORT', $3, 'PENDING', 0, 0, '')
            on conflict (job_id) do nothing
            "#,
        )
        .bind(request_id)
        .bind(uuid::Uuid::from(tenant_id))
        .bind(&params_json)
        .execute(&mut *tx)
        .await
        .map_err(anyhow::Error::from)?
        .rows_affected()
            == 1;

        let row: Option<(
            uuid::Uuid, uuid::Uuid, String, Value, String, Option<String>,
            i32, i32, String, DateTime<Utc>, DateTime<Utc>,
        )> = sqlx::query_as(
            "select job_id, tenant_id, job_type, params_json, status, output_url, progress, attempts, last_error, created_at, updated_at from export_jobs where job_id = $1 for share",
        )
        .bind(request_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(anyhow::Error::from)?;
        let Some(row) = row else {
            return Err(ProductExportCreateError::Conflict);
        };
        let job = ExportJob {
            job_id: row.0,
            tenant_id: TenantId(row.1),
            job_type: row.2,
            params_json: row.3,
            status: row.4,
            output_url: row.5,
            progress: row.6,
            attempts: row.7,
            last_error: row.8,
            created_at: row.9,
            updated_at: row.10,
        };
        if job.tenant_id != tenant_id
            || job.job_type != "PRODUCT_EXPORT"
            || job.params_json != params_json
        {
            return Err(ProductExportCreateError::Conflict);
        }
        if inserted {
            sqlx::query("insert into audit_logs(tenant_id,user_id,action,entity_type,entity_id,before_json,after_json,ip,ua) values($1,$2,'EXPORT.JOB.CREATE','EXPORT_JOB',$3,null,$4,null,null)")
                .bind(uuid::Uuid::from(tenant_id))
                .bind(uuid::Uuid::from(actor_user_id))
                .bind(request_id.to_string())
                .bind(serde_json::to_value(&job).map_err(anyhow::Error::from)?)
                .execute(&mut *tx)
                .await
                .map_err(anyhow::Error::from)?;
        }
        tx.commit().await.map_err(anyhow::Error::from)?;
        Ok(job)
    }

    /// Atomically accepts an export of a frozen selection and copies its final
    /// membership into job-owned rows. Exact retries do not depend on the
    /// short-lived selection token remaining available.
    pub async fn create_product_selection_job_idempotent(
        &self,
        tenant_id: TenantId,
        actor_user_id: erp_types::UserId,
        request_id: uuid::Uuid,
        token: uuid::Uuid,
        excluded: &[uuid::Uuid],
        sorts: &[SortRule],
    ) -> Result<ExportJob, ProductExportCreateError> {
        const MAX_EXCLUSIONS: usize = 10_000;
        if request_id.is_nil() || token.is_nil() {
            return Err(ProductExportCreateError::Invalid(
                "request and selection IDs must be non-nil".into(),
            ));
        }
        if excluded.len() > MAX_EXCLUSIONS {
            return Err(ProductExportCreateError::Invalid(
                "too many exclusions".into(),
            ));
        }
        let mut canonical_excluded = excluded.to_vec();
        canonical_excluded.sort_unstable();
        if canonical_excluded.iter().any(uuid::Uuid::is_nil)
            || canonical_excluded.windows(2).any(|ids| ids[0] == ids[1])
        {
            return Err(ProductExportCreateError::Invalid(
                "exclusions must be unique, non-nil product IDs".into(),
            ));
        }
        crate::product_selection::validate_sorts(sorts)
            .map_err(|error| ProductExportCreateError::Invalid(error.to_string()))?;

        let params_json = serde_json::json!({
            "schema": 1,
            "request_id": request_id,
            "actor_user_id": uuid::Uuid::from(actor_user_id),
            "selection": {
                "token": token,
                "excluded": canonical_excluded,
                "sorts": sorts,
            },
        });
        let mut tx = self.pool.begin().await.map_err(anyhow::Error::from)?;
        sqlx::query("set local statement_timeout = '30s'")
            .execute(&mut *tx)
            .await
            .map_err(anyhow::Error::from)?;
        let (job, inserted) =
            insert_and_load_product_job(&mut tx, tenant_id, request_id, &params_json).await?;
        if !inserted {
            tx.commit().await.map_err(anyhow::Error::from)?;
            return Ok(job);
        }

        let selection_total: Option<i64> = sqlx::query_scalar(
            "select total from grid_product_selections where token=$1 and tenant_id=$2 and actor_user_id=$3 and expires_at > clock_timestamp() for share",
        )
        .bind(token)
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(actor_user_id))
        .fetch_optional(&mut *tx)
        .await
        .map_err(anyhow::Error::from)?;
        let Some(selection_total) = selection_total else {
            return Err(ProductExportCreateError::Unavailable);
        };
        let mut excluded_members = 0_i64;
        if !canonical_excluded.is_empty() {
            let members: i64 = sqlx::query_scalar(
                "select count(*) from grid_product_selection_items where selection_token=$1 and product_id=any($2)",
            )
            .bind(token)
            .bind(&canonical_excluded)
            .fetch_one(&mut *tx)
            .await
            .map_err(anyhow::Error::from)?;
            if usize::try_from(members).ok() != Some(canonical_excluded.len()) {
                return Err(ProductExportCreateError::Invalid(
                    "an excluded product is outside the frozen selection".into(),
                ));
            }
            excluded_members = members;
        }
        let expected_total = selection_total
            .checked_sub(excluded_members)
            .ok_or_else(|| {
                ProductExportCreateError::Invalid("exclusions exceed selection".into())
            })?;
        sqlx::query(
            "insert into grid_product_selection_exports(job_id,expected_total) values($1,$2)",
        )
        .bind(request_id)
        .bind(expected_total)
        .execute(&mut *tx)
        .await
        .map_err(anyhow::Error::from)?;

        let mut copy = QueryBuilder::<Postgres>::new(
            "insert into grid_product_selection_export_items(job_id,ordinal,product_id,data) select ",
        );
        copy.push_bind(request_id)
            .push(", (row_number() over (order by ");
        append_frozen_selection_ordering(&mut copy, sorts)?;
        copy.push(") - 1)::bigint, product_id, data from grid_product_selection_items where selection_token=")
            .push_bind(token);
        if !canonical_excluded.is_empty() {
            copy.push(" and not(product_id=any(")
                .push_bind(&canonical_excluded)
                .push("))");
        }
        copy.push(" order by ");
        append_frozen_selection_ordering(&mut copy, sorts)?;
        let copied = copy
            .build()
            .execute(&mut *tx)
            .await
            .map_err(anyhow::Error::from)?
            .rows_affected();
        if copied != u64::try_from(expected_total).map_err(anyhow::Error::from)? {
            return Err(ProductExportCreateError::Invalid(
                "frozen selection membership count is inconsistent".into(),
            ));
        }

        insert_product_export_audit(&mut tx, tenant_id, actor_user_id, &job).await?;
        tx.commit().await.map_err(anyhow::Error::from)?;
        Ok(job)
    }

    /// Creates an export from explicit cross-page checked product IDs. The
    /// accepted rows are copied into the same job-owned snapshot used by
    /// frozen-selection exports, so later product changes cannot alter output.
    pub async fn create_product_checked_job_idempotent(
        &self,
        tenant_id: TenantId,
        actor_user_id: erp_types::UserId,
        request_id: uuid::Uuid,
        ids: &[uuid::Uuid],
        sorts: &[SortRule],
    ) -> Result<ExportJob, ProductExportCreateError> {
        const MAX_CHECKED_IDS: usize = 100_000;
        if request_id.is_nil() {
            return Err(ProductExportCreateError::Invalid(
                "request ID must be non-nil".into(),
            ));
        }
        if ids.is_empty() || ids.len() > MAX_CHECKED_IDS {
            return Err(ProductExportCreateError::Invalid(
                "checked product IDs must contain between 1 and 100000 items".into(),
            ));
        }
        let mut canonical_ids = ids.to_vec();
        canonical_ids.sort_unstable();
        if canonical_ids.iter().any(uuid::Uuid::is_nil)
            || canonical_ids.windows(2).any(|pair| pair[0] == pair[1])
        {
            return Err(ProductExportCreateError::Invalid(
                "checked product IDs must be unique and non-nil".into(),
            ));
        }
        crate::product_selection::validate_sorts(sorts)
            .map_err(|error| ProductExportCreateError::Invalid(error.to_string()))?;

        let params_json = serde_json::json!({
            "schema": 1,
            "request_id": request_id,
            "actor_user_id": uuid::Uuid::from(actor_user_id),
            "checked": {
                "ids": canonical_ids,
                "sorts": sorts,
            },
        });
        let mut tx = self.pool.begin().await.map_err(anyhow::Error::from)?;
        sqlx::query("set local statement_timeout = '30s'")
            .execute(&mut *tx)
            .await
            .map_err(anyhow::Error::from)?;
        let (job, inserted) =
            insert_and_load_product_job(&mut tx, tenant_id, request_id, &params_json).await?;
        if !inserted {
            tx.commit().await.map_err(anyhow::Error::from)?;
            return Ok(job);
        }

        let expected_total = i64::try_from(canonical_ids.len()).map_err(anyhow::Error::from)?;
        sqlx::query(
            "insert into grid_product_selection_exports(job_id,expected_total) values($1,$2)",
        )
        .bind(request_id)
        .bind(expected_total)
        .execute(&mut *tx)
        .await
        .map_err(anyhow::Error::from)?;

        let mut copy = QueryBuilder::<Postgres>::new(
            "insert into grid_product_selection_export_items(job_id,ordinal,product_id,data) select ",
        );
        copy.push_bind(request_id)
            .push(", (row_number() over (order by ");
        append_snapshot_ordering(&mut copy, sorts, "product_id desc", "product_id desc")?;
        copy.push(") - 1)::bigint, product_id, data from (select product_id, jsonb_build_object('product_id',product_id,'tenant_id',tenant_id,'product_code',product_code,'name',name,'brand',brand,'category',category,'status',status,'created_at',created_at,'updated_at',updated_at) data from products where tenant_id=")
            .push_bind(uuid::Uuid::from(tenant_id))
            .push(" and status<>'DELETED' and product_id=any(")
            .push_bind(&canonical_ids)
            .push(")) checked_products order by ");
        append_snapshot_ordering(&mut copy, sorts, "product_id desc", "product_id desc")?;
        let copied = copy
            .build()
            .execute(&mut *tx)
            .await
            .map_err(anyhow::Error::from)?
            .rows_affected();
        if copied != u64::try_from(expected_total).map_err(anyhow::Error::from)? {
            return Err(ProductExportCreateError::Invalid(
                "one or more checked products are unavailable".into(),
            ));
        }

        insert_product_export_audit(&mut tx, tenant_id, actor_user_id, &job).await?;
        tx.commit().await.map_err(anyhow::Error::from)?;
        Ok(job)
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

        // Failed jobs at their retry limit can no longer consume their held
        // selection. Successful jobs are released by update_job below.
        sqlx::query(
            "delete from grid_product_selection_exports i using export_jobs j where j.job_id=i.job_id and (j.status='DONE' or (j.status='FAILED' and j.attempts >= $1))",
        )
        .bind(max_attempts)
        .execute(&mut *tx)
        .await?;

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
            separated.push_bind_unseparated(status);
        }
        if let Some(output_url) = input.output_url {
            separated.push("output_url = ");
            separated.push_bind_unseparated(output_url);
        }
        if let Some(progress) = input.progress {
            separated.push("progress = ");
            separated.push_bind_unseparated(progress);
        }
        if let Some(last_error) = input.last_error {
            separated.push("last_error = ");
            separated.push_bind_unseparated(last_error);
        }
        if let Some(params_json) = input.params_json {
            separated.push("params_json = ");
            separated.push_bind_unseparated(params_json);
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
        let job = row.map(|row| ExportJob {
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
        });
        if job.as_ref().is_some_and(|job| {
            job.status == "DONE"
                && job.job_type == "PRODUCT_EXPORT"
                && (job.params_json.get("selection").is_some()
                    || job.params_json.get("checked").is_some())
        }) {
            sqlx::query("delete from grid_product_selection_exports where job_id=$1")
                .bind(job_id)
                .execute(self.pool)
                .await?;
        }
        Ok(job)
    }
}

async fn insert_and_load_product_job(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    request_id: uuid::Uuid,
    params_json: &Value,
) -> Result<(ExportJob, bool), ProductExportCreateError> {
    let inserted = sqlx::query(
        "insert into export_jobs(job_id,tenant_id,job_type,params_json,status,progress,attempts,last_error) values($1,$2,'PRODUCT_EXPORT',$3,'PENDING',0,0,'') on conflict(job_id) do nothing",
    )
    .bind(request_id)
    .bind(uuid::Uuid::from(tenant_id))
    .bind(params_json)
    .execute(&mut **tx)
    .await
    .map_err(anyhow::Error::from)?
    .rows_affected()
        == 1;
    let row: Option<(
        uuid::Uuid,
        uuid::Uuid,
        String,
        Value,
        String,
        Option<String>,
        i32,
        i32,
        String,
        DateTime<Utc>,
        DateTime<Utc>,
    )> = sqlx::query_as(
        "select job_id,tenant_id,job_type,params_json,status,output_url,progress,attempts,last_error,created_at,updated_at from export_jobs where job_id=$1 for share",
    )
    .bind(request_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(anyhow::Error::from)?;
    let Some(row) = row else {
        return Err(ProductExportCreateError::Conflict);
    };
    let job = ExportJob {
        job_id: row.0,
        tenant_id: TenantId(row.1),
        job_type: row.2,
        params_json: row.3,
        status: row.4,
        output_url: row.5,
        progress: row.6,
        attempts: row.7,
        last_error: row.8,
        created_at: row.9,
        updated_at: row.10,
    };
    if job.tenant_id != tenant_id
        || job.job_type != "PRODUCT_EXPORT"
        || job.params_json != *params_json
    {
        return Err(ProductExportCreateError::Conflict);
    }
    Ok((job, inserted))
}

async fn insert_product_export_audit(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    actor_user_id: erp_types::UserId,
    job: &ExportJob,
) -> Result<(), ProductExportCreateError> {
    sqlx::query("insert into audit_logs(tenant_id,user_id,action,entity_type,entity_id,before_json,after_json,ip,ua) values($1,$2,'EXPORT.JOB.CREATE','EXPORT_JOB',$3,null,$4,null,null)")
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(actor_user_id))
        .bind(job.job_id.to_string())
        .bind(serde_json::to_value(job).map_err(anyhow::Error::from)?)
        .execute(&mut **tx)
        .await
        .map_err(anyhow::Error::from)?;
    Ok(())
}

fn append_frozen_selection_ordering<'a>(
    query: &mut QueryBuilder<'a, Postgres>,
    sorts: &[SortRule],
) -> Result<(), ProductExportCreateError> {
    append_snapshot_ordering(query, sorts, "ordinal asc", "product_id asc")
}

fn append_snapshot_ordering<'a>(
    query: &mut QueryBuilder<'a, Postgres>,
    sorts: &[SortRule],
    default_ordering: &'static str,
    stable_tie_breaker: &'static str,
) -> Result<(), ProductExportCreateError> {
    if sorts.is_empty() {
        query.push(default_ordering);
        return Ok(());
    }
    for sort in sorts {
        let expression = match sort.column.0 {
            0 => "lower(nullif(data ->> 'product_code', ''))",
            1 => "lower(nullif(data ->> 'name', ''))",
            2 => "lower(nullif(data ->> 'brand', ''))",
            3 => "lower(nullif(data ->> 'category', ''))",
            4 => "lower(nullif(data ->> 'status', ''))",
            5 => "(data ->> 'created_at')::timestamptz",
            6 => "(data ->> 'updated_at')::timestamptz",
            _ => {
                return Err(ProductExportCreateError::Invalid(
                    "unknown product selection sort column".into(),
                ));
            }
        };
        query.push(expression).push(if sort.descending {
            " desc nulls last, "
        } else {
            " asc nulls last, "
        });
    }
    query.push(stable_tie_breaker);
    Ok(())
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

#[cfg(test)]
mod product_export_tests {
    use super::*;
    use erp_types::UserId;

    #[tokio::test]
    async fn checked_export_rejects_invalid_identity_membership_and_sorts() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
            .unwrap();
        let repo = ExportJobsRepo::new(&pool);
        let tenant = TenantId(uuid::Uuid::from_u128(301));
        let actor = UserId(uuid::Uuid::from_u128(302));
        let id = uuid::Uuid::from_u128(303);
        for result in [
            repo.create_product_checked_job_idempotent(
                tenant,
                actor,
                uuid::Uuid::nil(),
                &[id],
                &[],
            )
            .await,
            repo.create_product_checked_job_idempotent(
                tenant,
                actor,
                uuid::Uuid::from_u128(304),
                &[],
                &[],
            )
            .await,
            repo.create_product_checked_job_idempotent(
                tenant,
                actor,
                uuid::Uuid::from_u128(305),
                &[id, id],
                &[],
            )
            .await,
            repo.create_product_checked_job_idempotent(
                tenant,
                actor,
                uuid::Uuid::from_u128(306),
                &[id],
                &[
                    SortRule {
                        column: erp_grid_core::ColumnId(1),
                        descending: false,
                    },
                    SortRule {
                        column: erp_grid_core::ColumnId(1),
                        descending: true,
                    },
                ],
            )
            .await,
        ] {
            assert!(matches!(result, Err(ProductExportCreateError::Invalid(_))));
        }
    }

    #[tokio::test]
    #[ignore = "Requires explicit ERP_GRID_TEST_DATABASE_URL; uses connection-local temporary tables"]
    async fn product_export_identity_and_audit_are_atomic() {
        let url =
            std::env::var("ERP_GRID_TEST_DATABASE_URL").expect("explicit test database required");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        sqlx::query("create temporary table export_jobs(job_id uuid primary key,tenant_id uuid not null,job_type text not null,params_json jsonb not null,status text not null default 'PENDING',output_url text,progress int not null default 0,attempts int not null default 0,last_error text not null default '',created_at timestamptz not null default now(),updated_at timestamptz not null default now())")
            .execute(&pool).await.unwrap();
        sqlx::query("create temporary table audit_logs(audit_id uuid primary key default gen_random_uuid(),tenant_id uuid not null,user_id uuid,action text not null,entity_type text not null,entity_id text,before_json jsonb,after_json jsonb,ip text,ua text,created_at timestamptz not null default now())")
            .execute(&pool).await.unwrap();
        let tenant = TenantId(uuid::Uuid::from_u128(1));
        let actor = UserId(uuid::Uuid::from_u128(2));
        let request = uuid::Uuid::from_u128(3);
        let query = serde_json::json!({"schema":1,"offset":0,"limit":100});
        let repo = ExportJobsRepo::new(&pool);

        let first = repo
            .create_product_job_idempotent(tenant, actor, request, query.clone())
            .await
            .unwrap();
        let retry = repo
            .create_product_job_idempotent(tenant, actor, request, query.clone())
            .await
            .unwrap();
        assert_eq!(first.job_id, retry.job_id);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("select count(*) from audit_logs")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        assert!(matches!(
            repo.create_product_job_idempotent(
                tenant,
                UserId(uuid::Uuid::from_u128(4)),
                request,
                query.clone()
            )
            .await,
            Err(ProductExportCreateError::Conflict)
        ));
        assert!(matches!(
            repo.create_product_job_idempotent(
                tenant,
                actor,
                request,
                serde_json::json!({"schema":1,"offset":0,"limit":100,"search":"other"})
            )
            .await,
            Err(ProductExportCreateError::Conflict)
        ));

        sqlx::query("drop table audit_logs")
            .execute(&pool)
            .await
            .unwrap();
        let rolled_back = uuid::Uuid::from_u128(5);
        assert!(matches!(
            repo.create_product_job_idempotent(tenant, actor, rolled_back, query)
                .await,
            Err(ProductExportCreateError::Storage(_))
        ));
        assert_eq!(
            sqlx::query_scalar::<_, i64>("select count(*) from export_jobs where job_id=$1")
                .bind(rolled_back)
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "Requires explicit ERP_GRID_TEST_DATABASE_URL; uses connection-local temporary tables"]
    async fn product_selection_export_is_durable_bound_and_complete() {
        let url =
            std::env::var("ERP_GRID_TEST_DATABASE_URL").expect("explicit test database required");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        sqlx::query("create temporary table export_jobs(job_id uuid primary key,tenant_id uuid not null,job_type text not null,params_json jsonb not null,status text not null,output_url text,progress int not null default 0,attempts int not null default 0,last_error text not null default '',created_at timestamptz not null default now(),updated_at timestamptz not null default now())").execute(&pool).await.unwrap();
        sqlx::query("create temporary table audit_logs(audit_id uuid primary key default gen_random_uuid(),tenant_id uuid not null,user_id uuid,action text not null,entity_type text not null,entity_id text,before_json jsonb,after_json jsonb,ip text,ua text,created_at timestamptz not null default now())").execute(&pool).await.unwrap();
        sqlx::query("create temporary table grid_product_selections(token uuid primary key,tenant_id uuid not null,actor_user_id uuid not null,total bigint not null,expires_at timestamptz not null)").execute(&pool).await.unwrap();
        sqlx::query("create temporary table grid_product_selection_items(selection_token uuid not null references grid_product_selections(token) on delete cascade,ordinal bigint not null,product_id uuid not null,data jsonb not null,primary key(selection_token,ordinal),unique(selection_token,product_id))").execute(&pool).await.unwrap();
        sqlx::query("create temporary table grid_product_selection_exports(job_id uuid primary key references export_jobs(job_id) on delete cascade,expected_total bigint not null)").execute(&pool).await.unwrap();
        sqlx::query("create temporary table grid_product_selection_export_items(job_id uuid not null references grid_product_selection_exports(job_id) on delete cascade,ordinal bigint not null,product_id uuid not null,data jsonb not null,primary key(job_id,ordinal),unique(job_id,product_id))").execute(&pool).await.unwrap();

        let tenant = TenantId(uuid::Uuid::from_u128(101));
        let actor = UserId(uuid::Uuid::from_u128(102));
        let other_actor = UserId(uuid::Uuid::from_u128(103));
        let token = uuid::Uuid::from_u128(104);
        let kept = uuid::Uuid::from_u128(105);
        let excluded = uuid::Uuid::from_u128(106);
        sqlx::query("insert into grid_product_selections values($1,$2,$3,2,clock_timestamp()+interval '10 minutes')")
            .bind(token).bind(uuid::Uuid::from(tenant)).bind(uuid::Uuid::from(actor)).execute(&pool).await.unwrap();
        for (ordinal, product_id, name) in [(0_i64, kept, "kept"), (1, excluded, "excluded")] {
            let data = serde_json::json!({"product_id":product_id,"tenant_id":uuid::Uuid::from(tenant),"product_code":name,"name":name,"brand":null,"category":null,"status":"ACTIVE","created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z"});
            sqlx::query("insert into grid_product_selection_items values($1,$2,$3,$4)")
                .bind(token)
                .bind(ordinal)
                .bind(product_id)
                .bind(data)
                .execute(&pool)
                .await
                .unwrap();
        }
        let repo = ExportJobsRepo::new(&pool);
        assert!(matches!(
            repo.create_product_selection_job_idempotent(
                tenant,
                other_actor,
                uuid::Uuid::from_u128(107),
                token,
                &[],
                &[]
            )
            .await,
            Err(ProductExportCreateError::Unavailable)
        ));
        let request = uuid::Uuid::from_u128(108);
        let first = repo
            .create_product_selection_job_idempotent(
                tenant,
                actor,
                request,
                token,
                &[excluded],
                &[],
            )
            .await
            .unwrap();
        let retry = repo
            .create_product_selection_job_idempotent(
                tenant,
                actor,
                request,
                token,
                &[excluded],
                &[],
            )
            .await
            .unwrap();
        assert_eq!(first.job_id, retry.job_id);
        sqlx::query("delete from grid_product_selections where token=$1")
            .bind(token)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("update export_jobs set status='RUNNING' where job_id=$1")
            .bind(request)
            .execute(&pool)
            .await
            .unwrap();
        let mut cursor =
            crate::product_grid_export::ProductExportCursor::open_selection(&pool, tenant, request)
                .await
                .unwrap();
        let rows = cursor.fetch_next(10).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].product_id, kept);
        cursor.finish().await.unwrap();
        sqlx::query("delete from grid_product_selection_export_items where job_id=$1")
            .bind(request)
            .execute(&pool)
            .await
            .unwrap();
        assert!(
            crate::product_grid_export::ProductExportCursor::open_selection(&pool, tenant, request)
                .await
                .is_err()
        );
        repo.update_job(
            tenant,
            request,
            ExportJobUpdate {
                status: Some("DONE".into()),
                output_url: None,
                progress: Some(100),
                last_error: None,
                params_json: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "select count(*) from grid_product_selection_exports where job_id=$1"
            )
            .bind(request)
            .fetch_one(&pool)
            .await
            .unwrap(),
            0
        );
        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "Requires explicit ERP_GRID_TEST_DATABASE_URL; uses connection-local temporary tables"]
    async fn checked_export_is_complete_idempotent_sorted_and_snapshot_bound() {
        let url =
            std::env::var("ERP_GRID_TEST_DATABASE_URL").expect("explicit test database required");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        sqlx::query("create temporary table export_jobs(job_id uuid primary key,tenant_id uuid not null,job_type text not null,params_json jsonb not null,status text not null,output_url text,progress int not null default 0,attempts int not null default 0,last_error text not null default '',created_at timestamptz not null default now(),updated_at timestamptz not null default now())").execute(&pool).await.unwrap();
        sqlx::query("create temporary table audit_logs(audit_id uuid primary key default gen_random_uuid(),tenant_id uuid not null,user_id uuid,action text not null,entity_type text not null,entity_id text,before_json jsonb,after_json jsonb,ip text,ua text,created_at timestamptz not null default now())").execute(&pool).await.unwrap();
        sqlx::query("create temporary table grid_product_selection_exports(job_id uuid primary key references export_jobs(job_id) on delete cascade,expected_total bigint not null)").execute(&pool).await.unwrap();
        sqlx::query("create temporary table grid_product_selection_export_items(job_id uuid not null references grid_product_selection_exports(job_id) on delete cascade,ordinal bigint not null,product_id uuid not null,data jsonb not null,primary key(job_id,ordinal),unique(job_id,product_id))").execute(&pool).await.unwrap();
        sqlx::query("create temporary table products(product_id uuid primary key,tenant_id uuid not null,product_code text,name text not null,brand text,category text,status text not null,created_at timestamptz not null,updated_at timestamptz not null)").execute(&pool).await.unwrap();

        let tenant = TenantId(uuid::Uuid::from_u128(201));
        let other_tenant = TenantId(uuid::Uuid::from_u128(202));
        let actor = UserId(uuid::Uuid::from_u128(203));
        let first_id = uuid::Uuid::from_u128(204);
        let second_id = uuid::Uuid::from_u128(205);
        let foreign_id = uuid::Uuid::from_u128(206);
        for (id, owner, code, name) in [
            (first_id, tenant, "A", "Alpha"),
            (second_id, tenant, "B", "Beta"),
            (foreign_id, other_tenant, "C", "Foreign"),
        ] {
            sqlx::query("insert into products values($1,$2,$3,$4,null,null,'ACTIVE','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')")
                .bind(id).bind(uuid::Uuid::from(owner)).bind(code).bind(name)
                .execute(&pool).await.unwrap();
        }

        let repo = ExportJobsRepo::new(&pool);
        let request = uuid::Uuid::from_u128(207);
        let sorts = [SortRule {
            column: erp_grid_core::ColumnId(1),
            descending: true,
        }];
        let created = repo
            .create_product_checked_job_idempotent(
                tenant,
                actor,
                request,
                &[first_id, second_id],
                &sorts,
            )
            .await
            .unwrap();
        let retry = repo
            .create_product_checked_job_idempotent(
                tenant,
                actor,
                request,
                &[second_id, first_id],
                &sorts,
            )
            .await
            .unwrap();
        assert_eq!(created.job_id, retry.job_id);
        assert!(created.params_json.get("checked").is_some());
        assert!(created.params_json.get("query").is_none());
        assert!(created.params_json.get("selection").is_none());
        assert_eq!(
            sqlx::query_scalar::<_, i64>("select count(*) from audit_logs")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );

        sqlx::query("update products set name='Changed' where product_id=$1")
            .bind(second_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("update export_jobs set status='RUNNING' where job_id=$1")
            .bind(request)
            .execute(&pool)
            .await
            .unwrap();
        let mut cursor =
            crate::product_grid_export::ProductExportCursor::open_selection(&pool, tenant, request)
                .await
                .unwrap();
        let rows = cursor.fetch_next(10).await.unwrap();
        assert_eq!(
            rows.iter().map(|row| row.product_id).collect::<Vec<_>>(),
            vec![second_id, first_id]
        );
        assert_eq!(rows[0].name, "Beta");
        cursor.finish().await.unwrap();

        let rejected = uuid::Uuid::from_u128(208);
        assert!(matches!(
            repo.create_product_checked_job_idempotent(
                tenant,
                actor,
                rejected,
                &[first_id, foreign_id],
                &[],
            )
            .await,
            Err(ProductExportCreateError::Invalid(_))
        ));
        assert_eq!(
            sqlx::query_scalar::<_, i64>("select count(*) from export_jobs where job_id=$1")
                .bind(rejected)
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        pool.close().await;
    }
}
