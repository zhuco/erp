use chrono::{DateTime, Utc};
use erp_types::{OrderId, TenantId, UserId};
use serde_json::Value;
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct OmsRecheckBatchJobCreate {
    pub tenant_id: TenantId,
    pub action: String,
    pub reason: Option<String>,
    pub requested_by: Option<UserId>,
    pub total_count: i64,
}

#[derive(Debug, Clone)]
pub struct OmsRecheckBatchJob {
    pub job_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub action: String,
    pub reason: Option<String>,
    pub status: String,
    pub total_count: i64,
    pub accepted_count: i64,
    pub rejected_count: i64,
    pub requested_by: Option<UserId>,
    pub requested_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OmsRecheckBatchItemCreate {
    pub tenant_id: TenantId,
    pub job_id: uuid::Uuid,
    pub order_id: Option<OrderId>,
    pub raw_order_id: String,
    pub result: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub before_json: Option<Value>,
    pub after_json: Option<Value>,
}

pub struct OmsRecheckBatchRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> OmsRecheckBatchRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_job(
        &self,
        input: OmsRecheckBatchJobCreate,
    ) -> anyhow::Result<OmsRecheckBatchJob> {
        #[derive(sqlx::FromRow)]
        struct Row {
            job_id: uuid::Uuid,
            tenant_id: uuid::Uuid,
            action: String,
            reason: Option<String>,
            status: String,
            total_count: i64,
            accepted_count: i64,
            rejected_count: i64,
            requested_by: Option<uuid::Uuid>,
            requested_at: DateTime<Utc>,
            completed_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into oms_recheck_batch_jobs(
                tenant_id, action, reason, status, total_count, requested_by
            ) values ($1, $2, $3, 'RUNNING', $4, $5)
            returning
                job_id, tenant_id, action, reason, status, total_count,
                accepted_count, rejected_count, requested_by, requested_at,
                completed_at, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.action)
        .bind(input.reason)
        .bind(input.total_count)
        .bind(input.requested_by.map(uuid::Uuid::from))
        .fetch_one(self.pool)
        .await?;

        Ok(OmsRecheckBatchJob {
            job_id: row.job_id,
            tenant_id: TenantId::from(row.tenant_id),
            action: row.action,
            reason: row.reason,
            status: row.status,
            total_count: row.total_count,
            accepted_count: row.accepted_count,
            rejected_count: row.rejected_count,
            requested_by: row.requested_by.map(UserId),
            requested_at: row.requested_at,
            completed_at: row.completed_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn insert_item(&self, input: OmsRecheckBatchItemCreate) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            insert into oms_recheck_batch_items(
                job_id, tenant_id, order_id, raw_order_id, result,
                error_code, error_message, before_json, after_json
            ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(input.job_id)
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.order_id.map(uuid::Uuid::from))
        .bind(input.raw_order_id)
        .bind(input.result)
        .bind(input.error_code)
        .bind(input.error_message)
        .bind(input.before_json)
        .bind(input.after_json)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn complete_job(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
        accepted_count: i64,
        rejected_count: i64,
        status: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            update oms_recheck_batch_jobs
            set accepted_count = $3,
                rejected_count = $4,
                status = $5,
                completed_at = now(),
                updated_at = now()
            where tenant_id = $1 and job_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(job_id)
        .bind(accepted_count)
        .bind(rejected_count)
        .bind(status)
        .execute(self.pool)
        .await?;

        Ok(())
    }
}
