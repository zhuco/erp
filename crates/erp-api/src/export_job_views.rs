use super::*;

pub(crate) fn export_job_response(job: ExportJobRecord) -> ExportJobResponse {
    ExportJobResponse {
        job_id: job.job_id,
        tenant_id: uuid::Uuid::from(job.tenant_id),
        job_type: job.job_type,
        params: job.params_json,
        status: job.status,
        output_url: job.output_url,
        progress: job.progress,
        attempts: job.attempts,
        last_error: job.last_error,
        created_at: job.created_at,
        updated_at: job.updated_at,
    }
}

#[allow(clippy::result_large_err)]
pub(crate) fn normalize_duplicate_strategy(raw: Option<&str>) -> Result<String, Response> {
    let normalized = raw
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty());
    match normalized {
        None => Ok("skip".to_string()),
        Some(value) if value == "skip" || value == "overwrite" => Ok(value),
        Some(_) => Err(bad_request("invalid duplicate_strategy").into_response()),
    }
}

pub(crate) fn build_sku_import_job_response(job: ExportJobRecord) -> SkuImportJobResponse {
    let params = job.params_json.as_object();
    let total_rows = params
        .and_then(|map| map.get("total_rows"))
        .and_then(|value| value.as_i64())
        .unwrap_or(0);
    let failed_rows = params
        .and_then(|map| map.get("failed_rows"))
        .and_then(|value| value.as_i64())
        .unwrap_or(0);
    let success_rows = params
        .and_then(|map| map.get("success_rows"))
        .and_then(|value| value.as_i64())
        .unwrap_or_else(|| total_rows.saturating_sub(failed_rows));
    let file_name = params
        .and_then(|map| map.get("file_name"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());
    let file_type = params
        .and_then(|map| map.get("file_type"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());
    let duplicate_strategy = params
        .and_then(|map| map.get("duplicate_strategy"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());

    let failure_log_url = if failed_rows > 0 {
        let path = sku_import_failure_file_path(job.tenant_id, job.job_id);
        if path.exists() {
            Some(format!(
                "/v1/tenants/{}/skus/import/jobs/{}/failures/file",
                uuid::Uuid::from(job.tenant_id),
                job.job_id
            ))
        } else {
            None
        }
    } else {
        None
    };

    SkuImportJobResponse {
        job_id: job.job_id,
        status: job.status,
        progress: job.progress,
        total_rows,
        success_rows,
        failed_rows,
        file_name,
        file_type,
        duplicate_strategy,
        failure_log_url,
        created_at: job.created_at,
        updated_at: job.updated_at,
    }
}
