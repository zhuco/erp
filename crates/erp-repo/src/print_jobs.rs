use anyhow::anyhow;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use erp_types::{TenantId, UserId};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use std::collections::HashMap;

fn month_start(dt: DateTime<Utc>) -> NaiveDate {
    let d = dt.date_naive();
    NaiveDate::from_ymd_opt(d.year(), d.month(), 1).expect("valid date")
}

fn outbox_payload(job_id: uuid::Uuid, device_fingerprint: Option<&str>) -> Value {
    let mut map = Map::new();
    map.insert("job_id".to_string(), Value::String(job_id.to_string()));
    if let Some(device_fingerprint) = device_fingerprint
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        map.insert(
            "device_fingerprint".to_string(),
            Value::String(device_fingerprint.to_string()),
        );
    }
    Value::Object(map)
}

fn resolve_dispatch_status(status: &str) -> &str {
    match status {
        "PENDING" => "QUEUED",
        "RUNNING" => "DISPATCHED",
        "DONE" => "PRINTED",
        "FAILED" | "CANCELLED" => "FAILED",
        _ => "QUEUED",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintJob {
    pub job_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub job_type: Option<String>,
    pub ref_type: Option<String>,
    pub ref_id: Option<uuid::Uuid>,
    pub template_id: Option<uuid::Uuid>,
    pub template_version: Option<i32>,
    pub requested_by_user_id: Option<UserId>,
    pub status: String,
    pub dispatch_status: String,
    pub payload: Value,
    pub output_url: Option<String>,
    pub attempts: i32,
    pub last_error: String,
    pub device_fingerprint: Option<String>,
    pub device_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub events: Vec<PrintJobEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintJobEvent {
    pub event_id: uuid::Uuid,
    pub job_id: uuid::Uuid,
    pub status: String,
    pub actor_type: String,
    pub actor_user_id: Option<UserId>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

struct PrintJobEventInput {
    tenant_id: TenantId,
    job_id: uuid::Uuid,
    status: String,
    actor_type: String,
    actor_user_id: Option<UserId>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PrintJobCursor {
    pub created_at: DateTime<Utc>,
    pub job_id: uuid::Uuid,
}

impl PrintJobCursor {
    pub fn encode(&self) -> String {
        format!("{}|{}", self.created_at.to_rfc3339(), self.job_id)
    }

    pub fn decode(raw: &str) -> anyhow::Result<Self> {
        let (ts, id) = raw
            .split_once('|')
            .ok_or_else(|| anyhow!("invalid cursor"))?;
        let created_at = DateTime::parse_from_rfc3339(ts)
            .map_err(|_| anyhow!("invalid cursor"))?
            .with_timezone(&Utc);
        let job_id = uuid::Uuid::parse_str(id).map_err(|_| anyhow!("invalid cursor"))?;
        Ok(Self { created_at, job_id })
    }
}

pub struct PrintJobListFilters {
    pub job_type: Option<String>,
    pub ref_type: Option<String>,
    pub ref_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub job_id: Option<uuid::Uuid>,
    pub cursor: Option<PrintJobCursor>,
    pub limit: i64,
}

pub struct PrintJobListResult {
    pub items: Vec<PrintJob>,
    pub next_cursor: Option<PrintJobCursor>,
}

pub struct PrintJobOutput {
    pub status: String,
    pub output_url: Option<String>,
    pub output_content: Option<String>,
    pub content_format: Option<String>,
}

pub struct PrintJobCreate {
    pub tenant_id: TenantId,
    pub requested_by_user_id: UserId,
    pub job_type: String,
    pub ref_type: String,
    pub ref_id: uuid::Uuid,
    pub template_id: uuid::Uuid,
    pub device_fingerprint: Option<String>,
    pub payload: Value,
    pub trace_id: Option<String>,
}

pub struct PrintJobUpdate {
    pub status: Option<String>,
    pub payload: Option<Value>,
    pub output_url: Option<String>,
    pub output_content: Option<String>,
    pub attempts: Option<i32>,
    pub last_error: Option<String>,
    pub actor_user_id: Option<UserId>,
}

pub struct PrintJobsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> PrintJobsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_job(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
    ) -> anyhow::Result<Option<PrintJob>> {
        let mut result = self
            .list_jobs(
                tenant_id,
                PrintJobListFilters {
                    job_type: None,
                    ref_type: None,
                    ref_id: None,
                    status: None,
                    job_id: Some(job_id),
                    cursor: None,
                    limit: 1,
                },
            )
            .await?;
        Ok(result.items.pop())
    }

    pub async fn get_job_output(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
    ) -> anyhow::Result<Option<PrintJobOutput>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            status: String,
            output_url: Option<String>,
            output_content: Option<String>,
            content_format: Option<String>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select
                pj.status,
                pj.output_url,
                pj.output_content,
                pt.content_format
            from print_jobs pj
            left join print_templates pt
              on pt.tenant_id = pj.tenant_id
             and pt.template_id = pj.template_id
            where pj.tenant_id = $1 and pj.job_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(job_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| PrintJobOutput {
            status: row.status,
            output_url: row.output_url,
            output_content: row.output_content,
            content_format: row.content_format,
        }))
    }

    pub async fn list_jobs(
        &self,
        tenant_id: TenantId,
        filters: PrintJobListFilters,
    ) -> anyhow::Result<PrintJobListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                pj.job_id, pj.type, pj.ref_type, pj.ref_id, pj.template_id, pj.template_version,
                pj.requested_by_user_id, pj.status, pj.dispatch_status, pj.payload, pj.output_url, pj.attempts,
                pj.last_error, pj.created_at, pj.updated_at, pj.device_fingerprint,
                pd.name as device_name
            from print_jobs pj
            left join print_devices pd
              on pd.tenant_id = pj.tenant_id
             and pd.fingerprint = pj.device_fingerprint
            where pj.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(job_id) = filters.job_id {
            qb.push(" and pj.job_id = ");
            qb.push_bind(job_id);
        }
        if let Some(job_type) = filters.job_type {
            qb.push(" and pj.type = ");
            qb.push_bind(job_type);
        }
        if let Some(ref_type) = filters.ref_type {
            qb.push(" and pj.ref_type = ");
            qb.push_bind(ref_type);
        }
        if let Some(ref_id) = filters.ref_id {
            qb.push(" and pj.ref_id = ");
            qb.push_bind(ref_id);
        }
        if let Some(status) = filters.status {
            qb.push(" and pj.status = ");
            qb.push_bind(status);
        }
        if let Some(cursor) = filters.cursor {
            qb.push(" and (pj.created_at, pj.job_id) < (");
            qb.push_bind(cursor.created_at);
            qb.push(", ");
            qb.push_bind(cursor.job_id);
            qb.push(")");
        }

        qb.push(" order by pj.created_at desc, pj.job_id desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            job_id: uuid::Uuid,
            job_type: Option<String>,
            ref_type: Option<String>,
            ref_id: Option<uuid::Uuid>,
            template_id: Option<uuid::Uuid>,
            template_version: Option<i32>,
            requested_by_user_id: Option<uuid::Uuid>,
            status: String,
            dispatch_status: String,
            payload: Value,
            output_url: Option<String>,
            attempts: i32,
            last_error: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
            device_fingerprint: Option<String>,
            device_name: Option<String>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let job_ids: Vec<uuid::Uuid> = rows.iter().map(|row| row.job_id).collect();
        let mut events_by_job = self.fetch_job_events(tenant_id, &job_ids).await?;
        let next_cursor = rows.last().map(|row| PrintJobCursor {
            created_at: row.created_at,
            job_id: row.job_id,
        });

        let items = rows
            .into_iter()
            .map(|row| PrintJob {
                job_id: row.job_id,
                tenant_id,
                job_type: row.job_type,
                ref_type: row.ref_type,
                ref_id: row.ref_id,
                template_id: row.template_id,
                template_version: row.template_version,
                requested_by_user_id: row.requested_by_user_id.map(UserId),
                status: row.status,
                dispatch_status: row.dispatch_status,
                payload: row.payload,
                output_url: row.output_url,
                attempts: row.attempts,
                last_error: row.last_error,
                device_fingerprint: row.device_fingerprint,
                device_name: row.device_name,
                created_at: row.created_at,
                updated_at: row.updated_at,
                events: events_by_job.remove(&row.job_id).unwrap_or_default(),
            })
            .collect();

        Ok(PrintJobListResult { items, next_cursor })
    }

    pub async fn create_job(&self, input: PrintJobCreate) -> anyhow::Result<PrintJob> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        let template_version: i32 = sqlx::query_scalar(
            r#"
            select version
            from print_templates
            where tenant_id = $1 and template_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.template_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| anyhow!("template not found"))?;

        #[derive(sqlx::FromRow)]
        struct Row {
            job_id: uuid::Uuid,
            job_type: Option<String>,
            ref_type: Option<String>,
            ref_id: Option<uuid::Uuid>,
            template_id: Option<uuid::Uuid>,
            template_version: Option<i32>,
            requested_by_user_id: Option<uuid::Uuid>,
            status: String,
            dispatch_status: String,
            payload: Value,
            output_url: Option<String>,
            attempts: i32,
            last_error: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
            device_fingerprint: Option<String>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into print_jobs(
                tenant_id, type, ref_type, ref_id, template_id, template_version,
                requested_by_user_id, status, dispatch_status, payload, device_fingerprint
            ) values ($1, $2, $3, $4, $5, $6, $7, 'PENDING', 'QUEUED', $8, $9)
            returning
                job_id, type, ref_type, ref_id, template_id, template_version,
                requested_by_user_id, status, dispatch_status, payload, output_url, attempts,
                last_error, created_at, updated_at, device_fingerprint
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.job_type)
        .bind(&input.ref_type)
        .bind(input.ref_id)
        .bind(input.template_id)
        .bind(template_version)
        .bind(uuid::Uuid::from(input.requested_by_user_id))
        .bind(&input.payload)
        .bind(input.device_fingerprint.as_deref())
        .fetch_one(&mut *tx)
        .await?;

        let queued_event = self
            .insert_job_event(
                &mut tx,
                PrintJobEventInput {
                    tenant_id: input.tenant_id,
                    job_id: row.job_id,
                    status: "PENDING".to_string(),
                    actor_type: "USER".to_string(),
                    actor_user_id: Some(input.requested_by_user_id),
                    error: None,
                },
            )
            .await?;

        let payload = outbox_payload(row.job_id, row.device_fingerprint.as_deref());
        let now = Utc::now();
        let event_month = month_start(now);

        sqlx::query(
            r#"
            insert into outbox_events(tenant_id, event_month, event_type, payload, trace_id)
            values ($1, $2, 'PRINT_JOB_QUEUED', $3, $4)
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(event_month)
        .bind(&payload)
        .bind(input.trace_id.as_deref())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(PrintJob {
            job_id: row.job_id,
            tenant_id: input.tenant_id,
            job_type: row.job_type,
            ref_type: row.ref_type,
            ref_id: row.ref_id,
            template_id: row.template_id,
            template_version: row.template_version,
            requested_by_user_id: row.requested_by_user_id.map(UserId),
            status: row.status,
            dispatch_status: row.dispatch_status,
            payload: row.payload,
            output_url: row.output_url,
            attempts: row.attempts,
            last_error: row.last_error,
            device_fingerprint: row.device_fingerprint,
            device_name: None,
            created_at: row.created_at,
            updated_at: row.updated_at,
            events: vec![queued_event],
        })
    }

    pub async fn update_job(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
        input: PrintJobUpdate,
    ) -> anyhow::Result<Option<PrintJob>> {
        let PrintJobUpdate {
            status,
            payload,
            output_url,
            output_content,
            attempts,
            last_error,
            actor_user_id,
        } = input;
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct StatusRow {
            status: String,
        }

        let current: Option<StatusRow> = sqlx::query_as(
            r#"
            select status
            from print_jobs
            where tenant_id = $1 and job_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(job_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(current) = current else {
            return Ok(None);
        };

        let status_value = status.clone();
        let dispatch_status = status_value.as_deref().map(resolve_dispatch_status);
        let mut qb = QueryBuilder::<Postgres>::new("update print_jobs set ");
        let mut separated = qb.separated(", ");

        if let Some(status) = status {
            separated.push("status = ");
            separated.push_bind(status);
        }
        if let Some(dispatch_status) = dispatch_status {
            separated.push("dispatch_status = ");
            separated.push_bind(dispatch_status);
        }
        if let Some(payload) = payload {
            separated.push("payload = ");
            separated.push_bind(payload);
        }
        if let Some(output_url) = output_url {
            separated.push("output_url = ");
            separated.push_bind(output_url);
        }
        if let Some(output_content) = output_content {
            separated.push("output_content = ");
            separated.push_bind(output_content);
        }
        if let Some(attempts) = attempts {
            separated.push("attempts = ");
            separated.push_bind(attempts);
        }
        if let Some(last_error) = last_error.clone() {
            separated.push("last_error = ");
            separated.push_bind(last_error);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and job_id = ");
        qb.push_bind(job_id);
        qb.push(
            r#"
            returning
                job_id, type, ref_type, ref_id, template_id, template_version,
                requested_by_user_id, status, dispatch_status, payload, output_url, attempts,
                last_error, created_at, updated_at, device_fingerprint
            "#,
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            job_id: uuid::Uuid,
            job_type: Option<String>,
            ref_type: Option<String>,
            ref_id: Option<uuid::Uuid>,
            template_id: Option<uuid::Uuid>,
            template_version: Option<i32>,
            requested_by_user_id: Option<uuid::Uuid>,
            status: String,
            dispatch_status: String,
            payload: Value,
            output_url: Option<String>,
            attempts: i32,
            last_error: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
            device_fingerprint: Option<String>,
        }

        let row: Row = qb.build_query_as().fetch_one(&mut *tx).await?;
        if let Some(new_status) = status_value.as_deref() {
            if new_status != current.status {
                let actor_type = if actor_user_id.is_some() {
                    "USER"
                } else {
                    "SYSTEM"
                };
                self.insert_job_event(
                    &mut tx,
                    PrintJobEventInput {
                        tenant_id,
                        job_id,
                        status: new_status.to_string(),
                        actor_type: actor_type.to_string(),
                        actor_user_id,
                        error: last_error,
                    },
                )
                .await?;
            }
        }

        tx.commit().await?;
        let mut events_by_job = self.fetch_job_events(tenant_id, &[row.job_id]).await?;
        let device_name =
            resolve_device_name(self.pool, tenant_id, row.device_fingerprint.as_deref()).await?;

        Ok(Some(PrintJob {
            job_id: row.job_id,
            tenant_id,
            job_type: row.job_type,
            ref_type: row.ref_type,
            ref_id: row.ref_id,
            template_id: row.template_id,
            template_version: row.template_version,
            requested_by_user_id: row.requested_by_user_id.map(UserId),
            status: row.status,
            dispatch_status: row.dispatch_status,
            payload: row.payload,
            output_url: row.output_url,
            attempts: row.attempts,
            last_error: row.last_error,
            device_fingerprint: row.device_fingerprint,
            device_name,
            created_at: row.created_at,
            updated_at: row.updated_at,
            events: events_by_job.remove(&row.job_id).unwrap_or_default(),
        }))
    }

    pub async fn claim_pending_jobs_for_device(
        &self,
        tenant_id: TenantId,
        device_fingerprint: &str,
        limit: i64,
    ) -> anyhow::Result<Vec<PrintJob>> {
        let fingerprint = device_fingerprint.trim();
        if fingerprint.is_empty() {
            return Ok(Vec::new());
        }

        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct Row {
            job_id: uuid::Uuid,
            job_type: Option<String>,
            ref_type: Option<String>,
            ref_id: Option<uuid::Uuid>,
            template_id: Option<uuid::Uuid>,
            template_version: Option<i32>,
            requested_by_user_id: Option<uuid::Uuid>,
            payload: Value,
            output_url: Option<String>,
            attempts: i32,
            last_error: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
            device_fingerprint: Option<String>,
            device_name: Option<String>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select
                pj.job_id, pj.type, pj.ref_type, pj.ref_id, pj.template_id, pj.template_version,
                pj.requested_by_user_id, pj.payload, pj.output_url, pj.attempts, pj.last_error,
                pj.created_at, pj.updated_at, pj.device_fingerprint,
                pd.name as device_name
            from print_jobs pj
            left join print_devices pd
              on pd.tenant_id = pj.tenant_id
             and pd.fingerprint = pj.device_fingerprint
            where pj.tenant_id = $1
              and pj.status = 'PENDING'
              and pj.device_fingerprint = $2
            order by pj.created_at asc, pj.job_id asc
            limit $3
            for update skip locked
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(fingerprint)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await?;

        if rows.is_empty() {
            tx.commit().await?;
            return Ok(Vec::new());
        }

        let job_ids: Vec<uuid::Uuid> = rows.iter().map(|row| row.job_id).collect();
        sqlx::query(
            r#"
            update print_jobs
            set status = 'RUNNING',
                dispatch_status = 'DISPATCHED',
                attempts = attempts + 1,
                updated_at = now()
            where tenant_id = $1 and job_id = any($2)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(&job_ids)
        .execute(&mut *tx)
        .await?;

        for job_id in &job_ids {
            sqlx::query(
                r#"
                insert into print_job_events(
                    tenant_id, job_id, status, actor_type, actor_user_id, error
                ) values ($1, $2, 'RUNNING', 'DEVICE', null, null)
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(job_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        let jobs = rows
            .into_iter()
            .map(|row| PrintJob {
                job_id: row.job_id,
                tenant_id,
                job_type: row.job_type,
                ref_type: row.ref_type,
                ref_id: row.ref_id,
                template_id: row.template_id,
                template_version: row.template_version,
                requested_by_user_id: row.requested_by_user_id.map(UserId),
                status: "RUNNING".to_string(),
                dispatch_status: "DISPATCHED".to_string(),
                payload: row.payload,
                output_url: row.output_url,
                attempts: row.attempts.saturating_add(1),
                last_error: row.last_error,
                device_fingerprint: row.device_fingerprint,
                device_name: row.device_name,
                created_at: row.created_at,
                updated_at: row.updated_at,
                events: Vec::new(),
            })
            .collect();

        Ok(jobs)
    }

    pub async fn retry_failed_job(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        #[derive(sqlx::FromRow)]
        struct Row {
            status: String,
            device_fingerprint: Option<String>,
        }

        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        let row: Option<Row> = sqlx::query_as(
            r#"
            select status, device_fingerprint
            from print_jobs
            where tenant_id = $1 and job_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(job_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            return Err(anyhow!("print job not found"));
        };

        if row.status != "FAILED" {
            return Err(anyhow!("print job status not retryable"));
        }

        let outbox_exists: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            select outbox_id
            from outbox_events
            where tenant_id = $1
              and event_type = 'PRINT_JOB_QUEUED'
              and payload->>'job_id' = $2
              and status in ('PENDING','FAILED','DISPATCHING')
            limit 1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(job_id.to_string())
        .fetch_optional(&mut *tx)
        .await?;

        if outbox_exists.is_none() {
            let payload = outbox_payload(job_id, row.device_fingerprint.as_deref());
            let now = Utc::now();
            let event_month = month_start(now);
            let trace_id: Option<String> = None;

            sqlx::query(
                r#"
                insert into outbox_events(tenant_id, event_month, event_type, payload, trace_id)
                values ($1, $2, 'PRINT_JOB_QUEUED', $3, $4)
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(event_month)
            .bind(payload)
            .bind(trace_id)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query(
            r#"
            update print_jobs
            set status = 'PENDING',
                dispatch_status = 'QUEUED',
                updated_at = now()
            where tenant_id = $1 and job_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(job_id)
        .execute(&mut *tx)
        .await?;

        self.insert_job_event(
            &mut tx,
            PrintJobEventInput {
                tenant_id,
                job_id,
                status: "PENDING".to_string(),
                actor_type: "SYSTEM".to_string(),
                actor_user_id: None,
                error: None,
            },
        )
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn dispatch_job(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        #[derive(sqlx::FromRow)]
        struct Row {
            status: String,
            device_fingerprint: Option<String>,
        }

        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        let row: Option<Row> = sqlx::query_as(
            r#"
            select status, device_fingerprint
            from print_jobs
            where tenant_id = $1 and job_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(job_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            return Err(anyhow!("print job not found"));
        };

        if matches!(row.status.as_str(), "DONE" | "RUNNING") {
            return Err(anyhow!("print job status not dispatchable"));
        }

        if row.status == "FAILED" {
            sqlx::query(
                r#"
                update print_jobs
                set status = 'PENDING',
                    dispatch_status = 'QUEUED',
                    updated_at = now()
                where tenant_id = $1 and job_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(job_id)
            .execute(&mut *tx)
            .await?;

            self.insert_job_event(
                &mut tx,
                PrintJobEventInput {
                    tenant_id,
                    job_id,
                    status: "PENDING".to_string(),
                    actor_type: "SYSTEM".to_string(),
                    actor_user_id: None,
                    error: None,
                },
            )
            .await?;
        }

        let outbox_exists: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            select outbox_id
            from outbox_events
            where tenant_id = $1
              and event_type = 'PRINT_JOB_QUEUED'
              and payload->>'job_id' = $2
              and status in ('PENDING','FAILED','DISPATCHING')
            limit 1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(job_id.to_string())
        .fetch_optional(&mut *tx)
        .await?;

        if outbox_exists.is_none() {
            let payload = outbox_payload(job_id, row.device_fingerprint.as_deref());
            let now = Utc::now();
            let event_month = month_start(now);
            let trace_id: Option<String> = None;

            sqlx::query(
                r#"
                insert into outbox_events(tenant_id, event_month, event_type, payload, trace_id)
                values ($1, $2, 'PRINT_JOB_QUEUED', $3, $4)
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(event_month)
            .bind(payload)
            .bind(trace_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn insert_job_event(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        input: PrintJobEventInput,
    ) -> anyhow::Result<PrintJobEvent> {
        let PrintJobEventInput {
            tenant_id,
            job_id,
            status,
            actor_type,
            actor_user_id,
            error,
        } = input;
        #[derive(sqlx::FromRow)]
        struct EventRow {
            event_id: uuid::Uuid,
            job_id: uuid::Uuid,
            status: String,
            actor_type: String,
            actor_user_id: Option<uuid::Uuid>,
            error: Option<String>,
            created_at: DateTime<Utc>,
        }

        let row: EventRow = sqlx::query_as(
            r#"
            insert into print_job_events(
                tenant_id, job_id, status, actor_type, actor_user_id, error
            ) values ($1, $2, $3, $4, $5, $6)
            returning
                event_id, job_id, status, actor_type, actor_user_id, error, created_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(job_id)
        .bind(status)
        .bind(actor_type)
        .bind(actor_user_id.map(uuid::Uuid::from))
        .bind(error)
        .fetch_one(&mut **tx)
        .await?;

        Ok(PrintJobEvent {
            event_id: row.event_id,
            job_id: row.job_id,
            status: row.status,
            actor_type: row.actor_type,
            actor_user_id: row.actor_user_id.map(UserId),
            error: row.error,
            created_at: row.created_at,
        })
    }

    async fn fetch_job_events(
        &self,
        tenant_id: TenantId,
        job_ids: &[uuid::Uuid],
    ) -> anyhow::Result<HashMap<uuid::Uuid, Vec<PrintJobEvent>>> {
        if job_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let job_ids = job_ids.to_vec();

        #[derive(sqlx::FromRow)]
        struct EventRow {
            event_id: uuid::Uuid,
            job_id: uuid::Uuid,
            status: String,
            actor_type: String,
            actor_user_id: Option<uuid::Uuid>,
            error: Option<String>,
            created_at: DateTime<Utc>,
        }

        let rows: Vec<EventRow> = sqlx::query_as(
            r#"
            select
                event_id, job_id, status, actor_type, actor_user_id, error, created_at
            from print_job_events
            where tenant_id = $1 and job_id = any($2)
            order by created_at asc, event_id asc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(&job_ids)
        .fetch_all(self.pool)
        .await?;

        let mut events_by_job: HashMap<uuid::Uuid, Vec<PrintJobEvent>> = HashMap::new();
        for row in rows {
            events_by_job
                .entry(row.job_id)
                .or_default()
                .push(PrintJobEvent {
                    event_id: row.event_id,
                    job_id: row.job_id,
                    status: row.status,
                    actor_type: row.actor_type,
                    actor_user_id: row.actor_user_id.map(UserId),
                    error: row.error,
                    created_at: row.created_at,
                });
        }

        Ok(events_by_job)
    }
}

async fn resolve_device_name(
    pool: &PgPool,
    tenant_id: TenantId,
    fingerprint: Option<&str>,
) -> anyhow::Result<Option<String>> {
    let Some(fingerprint) = fingerprint else {
        return Ok(None);
    };
    let name: Option<String> = sqlx::query_scalar(
        r#"
        select name
        from print_devices
        where tenant_id = $1 and fingerprint = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(fingerprint)
    .fetch_optional(pool)
    .await?;
    Ok(name)
}
