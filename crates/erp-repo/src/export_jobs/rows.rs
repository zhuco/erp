//! Named database fields keep export receipts independent of column positions.
use super::ExportJob;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde_json::Value;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
pub(super) struct ExportJobRow {
    job_id: Uuid,
    tenant_id: Uuid,
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
impl From<ExportJobRow> for ExportJob {
    fn from(row: ExportJobRow) -> Self {
        Self {
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
        }
    }
}
