use super::*;
use sha2::Digest;

#[path = "../export_file_stream.rs"]
mod export_file_stream;

use export_file_stream::export_file_body;
#[path = "../inventory_export_access.rs"]
mod inventory_export_access;

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ProductExportParams {
    request_id: uuid::Uuid,
    query: Option<erp_grid_core::paged_query::PageQuery>,
    selection: Option<ProductExportSelection>,
    checked: Option<ProductExportChecked>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ProductExportSelection {
    token: uuid::Uuid,
    excluded: Vec<uuid::Uuid>,
    sorts: Vec<erp_grid_core::SortRule>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ProductExportChecked {
    ids: Vec<uuid::Uuid>,
    sorts: Vec<erp_grid_core::SortRule>,
}

fn normalize_product_export_params(params: Option<Value>) -> Result<ProductExportParams, String> {
    let mut params: ProductExportParams = serde_json::from_value(
        params.ok_or_else(|| "PRODUCT_EXPORT params are required".to_string())?,
    )
    .map_err(|_| "invalid PRODUCT_EXPORT params".to_string())?;
    if params.request_id.is_nil() {
        return Err("PRODUCT_EXPORT request_id must not be nil".to_string());
    }
    match (
        &mut params.query,
        &mut params.selection,
        &mut params.checked,
    ) {
        (Some(query), None, None) => {
            erp_repo::product_grid::validate(query).map_err(str::to_string)?;
            query.offset = 0;
            query.limit = 100;
        }
        (None, Some(selection), None) => {
            if selection.token.is_nil()
                || selection.excluded.len() > 10_000
                || selection.excluded.iter().any(uuid::Uuid::is_nil)
            {
                return Err("invalid product export selection".into());
            }
            selection.excluded.sort_unstable();
            if selection.excluded.windows(2).any(|ids| ids[0] == ids[1]) {
                return Err("duplicate product export exclusions".into());
            }
            erp_repo::product_selection::validate_sorts(&selection.sorts)
                .map_err(|error| error.to_string())?;
        }
        (None, None, Some(checked)) => {
            if checked.ids.is_empty()
                || checked.ids.len() > 100_000
                || checked.ids.iter().any(uuid::Uuid::is_nil)
            {
                return Err("checked export requires 1 to 100000 non-nil IDs".into());
            }
            checked.ids.sort_unstable();
            if checked.ids.windows(2).any(|ids| ids[0] == ids[1]) {
                return Err("duplicate checked export IDs".into());
            }
            erp_repo::product_selection::validate_sorts(&checked.sorts)
                .map_err(|error| error.to_string())?;
        }
        _ => return Err("provide exactly one product query, selection or checked source".into()),
    }
    Ok(params)
}

pub(crate) async fn get_dashboard_report(
    State(state): State<AppState>,
    Query(query): Query<ReportRangeQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    if let Err(resp) = ensure_permission(&ctx, "reports.read") {
        return resp.into_response();
    }

    let (start_at, end_at) = match parse_report_range(query.start_at, query.end_at) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let platform = match query.platform.as_deref() {
        Some(raw) => match Platform::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad platform").into_response(),
        },
        None => None,
    };
    let shop_id = match query.shop_id.as_deref() {
        Some(raw) => match ShopId::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };

    let repo = ReportsRepo::new(state.db.pool());
    let report = match repo
        .dashboard_report(
            ctx.tenant_id,
            DashboardReportFilters {
                start_at,
                end_at,
                platform,
                shop_id,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(report)).into_response()
}

pub(crate) async fn get_fulfillment_report(
    State(state): State<AppState>,
    Query(query): Query<FulfillmentReportQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    if let Err(resp) = ensure_permission(&ctx, "reports.read") {
        return resp.into_response();
    }

    let (start_at, end_at) = match parse_report_range(query.start_at, query.end_at) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let overdue_hours = query.overdue_hours.unwrap_or(48).max(1);

    let repo = ReportsRepo::new(state.db.pool());
    let report = match repo
        .fulfillment_report(
            ctx.tenant_id,
            FulfillmentReportFilters {
                start_at,
                end_at,
                overdue_hours,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(report)).into_response()
}

pub(crate) async fn get_inventory_report(
    State(state): State<AppState>,
    Query(query): Query<InventoryReportQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    if let Err(resp) = ensure_permission(&ctx, "reports.read") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.read") {
        return resp.into_response();
    }

    let (start_at, end_at) = match parse_report_range(query.start_at, query.end_at) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if (end_at - start_at).num_milliseconds() < 1 {
        return bad_request("report interval must be at least one millisecond").into_response();
    }
    let warehouse_id = match query.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    if let Some(warehouse_id) = warehouse_id
        && let Err(resp) = ensure_warehouse_access(&ctx, warehouse_id)
    {
        return resp.into_response();
    }
    if query.warning_quantity.is_some() && query.warning_threshold.is_some() {
        return bad_request("provide one warning quantity").into_response();
    }
    let warning_threshold_micros = match query.warning_quantity.as_deref() {
        Some(text) => match erp_domain::quantity::parse_qty(text) {
            Ok(value) if !value.is_negative() => i128::from(value.as_micros()),
            _ => return bad_request("bad warning quantity").into_response(),
        },
        None => match query.warning_threshold.unwrap_or(5) {
            value if value >= 0 => i128::from(value) * 1_000_000,
            _ => return bad_request("bad warning quantity").into_response(),
        },
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = ReportsRepo::new(state.db.pool());
    let report = match repo
        .inventory_report(
            ctx.tenant_id,
            InventoryReportFilters {
                start_at,
                end_at,
                warehouse_id,
                warehouse_ids: warehouse_scope_filter(&ctx),
                warning_threshold_micros,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return inventory_error_response(e).into_response(),
    };

    (StatusCode::OK, Json(report)).into_response()
}

pub(crate) async fn get_finance_report(
    State(state): State<AppState>,
    Query(query): Query<ReportRangeQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    if let Err(resp) = ensure_permission(&ctx, "reports.read") {
        return resp.into_response();
    }

    let (start_at, end_at) = match parse_report_range(query.start_at, query.end_at) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let platform = match query.platform.as_deref() {
        Some(raw) => match Platform::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad platform").into_response(),
        },
        None => None,
    };
    let shop_id = match query.shop_id.as_deref() {
        Some(raw) => match ShopId::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };

    let repo = ReportsRepo::new(state.db.pool());
    let report = match repo
        .finance_report(
            ctx.tenant_id,
            FinanceReportFilters {
                start_at,
                end_at,
                platform,
                shop_id,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(report)).into_response()
}

pub(crate) async fn get_stats_infra_health(
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["stats.read", "reports.read"]) {
        return resp.into_response();
    }

    let health = erp_analytics::detect_analytics_infra_health();

    (
        StatusCode::OK,
        Json(StatsInfraHealthResponse {
            analytics_backend: health.analytics_backend,
            clickhouse_ready: health.clickhouse_ready,
            sso_ready: health.sso_ready,
            checked_at: Utc::now(),
        }),
    )
        .into_response()
}

pub(crate) async fn get_stats_report(
    State(state): State<AppState>,
    Path((tenant_id, stats_type)): Path<(String, String)>,
    Query(query): Query<StatsReportQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission_any(&ctx, &["stats.read", "reports.read"]) {
        return resp.into_response();
    }

    let stats_type = stats_type.trim().to_lowercase();
    if !matches!(
        stats_type.as_str(),
        "sales"
            | "shipping"
            | "sales-daily"
            | "sales-trends"
            | "slow-moving"
            | "best-sellers"
            | "sales-cycle"
    ) {
        return bad_request("bad stats_type").into_response();
    }

    let (start_at, end_at) = match parse_report_range(query.start_at, query.end_at) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let compare_range = match (query.compare_start_at, query.compare_end_at) {
        (Some(start), Some(end)) => match parse_report_range(Some(start), Some(end)) {
            Ok(v) => Some(v),
            Err(resp) => return resp.into_response(),
        },
        (None, None) => None,
        _ => {
            return bad_request("compare_start_at and compare_end_at must be paired")
                .into_response();
        }
    };

    let metrics = parse_stats_metrics_csv(query.metrics.as_deref(), stats_type.as_str());
    let current = match compute_stats_metrics(
        state.db.pool(),
        tenant_id,
        stats_type.as_str(),
        start_at,
        end_at,
        &metrics,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let compare = match compare_range {
        Some((compare_start_at, compare_end_at)) => {
            match compute_stats_metrics(
                state.db.pool(),
                tenant_id,
                stats_type.as_str(),
                compare_start_at,
                compare_end_at,
                &metrics,
            )
            .await
            {
                Ok(v) => Some(v),
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
                }
            }
        }
        None => None,
    };
    let breakdown = match query.dimension.as_deref().and_then(non_empty) {
        Some(dimension) => {
            match compute_stats_breakdown(
                state.db.pool(),
                tenant_id,
                stats_type.as_str(),
                start_at,
                end_at,
                dimension.trim(),
                &metrics,
            )
            .await
            {
                Ok(v) if v.is_empty() => None,
                Ok(v) => Some(v),
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
                }
            }
        }
        None => None,
    };

    (
        StatusCode::OK,
        Json(StatsReportResponse {
            stats_type,
            current,
            compare,
            breakdown,
            updated_at: Utc::now(),
        }),
    )
        .into_response()
}

pub(crate) async fn list_export_jobs(
    State(state): State<AppState>,
    Query(query): Query<ExportJobListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    if let Err(resp) = ensure_permission(&ctx, "exports.read") {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = ExportJobsRepo::new(state.db.pool());
    let visibility = match inventory_export_access::visibility(&state, &ctx).await {
        Ok(visibility) => visibility,
        Err(response) => return response,
    };
    let result: ExportJobListResultRepo = match repo
        .list_jobs_visible_inventory(
            ctx.tenant_id,
            ExportJobListFiltersRepo {
                status: query.status.and_then(non_empty),
                job_type: query.job_type.and_then(non_empty),
                offset,
                limit,
            },
            visibility,
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = ExportJobListResponse {
        items: result.items.into_iter().map(export_job_response).collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_export_job(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ExportJobCreateRequest>,
) -> impl IntoResponse {
    if let Err(resp) = ensure_permission(&ctx, "exports.write") {
        return resp.into_response();
    }

    let job_type = req.job_type.trim().to_uppercase();
    if !matches!(
        job_type.as_str(),
        "ORDER_EXPORT" | "RECONCILE_EXPORT" | "INVENTORY_EXPORT" | "PRODUCT_EXPORT"
    ) {
        return bad_request("unsupported export type").into_response();
    }
    if job_type == "PRODUCT_EXPORT"
        && let Err(resp) = ensure_permission(&ctx, "skus.read")
    {
        return resp.into_response();
    }
    if job_type == "INVENTORY_EXPORT"
        && let Err(resp) = ensure_permission(&ctx, "inventory.read")
    {
        return resp.into_response();
    }

    let product_params = if job_type == "PRODUCT_EXPORT" {
        match normalize_product_export_params(req.params.clone()) {
            Ok(params) => Some(params),
            Err(message) => return bad_request(&message).into_response(),
        }
    } else {
        None
    };

    let read_only = match read_only_enabled(&state, ctx.tenant_id, None).await {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if read_only {
        return read_only_forbidden().into_response();
    }

    if job_type == "INVENTORY_EXPORT" {
        return inventory_export_access::create(&state, &ctx, req.params).await;
    }

    let repo = ExportJobsRepo::new(state.db.pool());
    let job = if job_type == "PRODUCT_EXPORT" {
        let Some(params) = product_params else {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "missing normalized product export parameters",
            )
                .into_response();
        };
        let created = if let Some(selection) = params.selection {
            repo.create_product_selection_job_idempotent(
                ctx.tenant_id,
                ctx.user_id,
                params.request_id,
                selection.token,
                &selection.excluded,
                &selection.sorts,
            )
            .await
        } else if let Some(checked) = params.checked {
            repo.create_product_checked_job_idempotent(
                ctx.tenant_id,
                ctx.user_id,
                params.request_id,
                &checked.ids,
                &checked.sorts,
            )
            .await
        } else {
            let normalized_query = match serde_json::to_value(&params.query) {
                Ok(query) => query,
                Err(error) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
                }
            };
            repo.create_product_job_idempotent(
                ctx.tenant_id,
                ctx.user_id,
                params.request_id,
                normalized_query,
            )
            .await
        };
        match created {
            Ok(job) => {
                return (StatusCode::OK, Json(export_job_response(job))).into_response();
            }
            Err(erp_repo::export_jobs::ProductExportCreateError::Conflict) => {
                return conflict("product export request identity conflict").into_response();
            }
            Err(erp_repo::export_jobs::ProductExportCreateError::Invalid(message)) => {
                return bad_request(&message).into_response();
            }
            Err(erp_repo::export_jobs::ProductExportCreateError::Unavailable) => {
                return (StatusCode::GONE, "product selection expired or unavailable")
                    .into_response();
            }
            Err(erp_repo::export_jobs::ProductExportCreateError::Storage(error)) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
            }
        }
    } else {
        let params = req.params.unwrap_or_else(|| serde_json::json!({}));
        match repo
            .create_job(ExportJobCreateInput {
                tenant_id: ctx.tenant_id,
                job_type: job_type.clone(),
                params_json: params,
            })
            .await
        {
            Ok(job) => job,
            Err(error) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
            }
        }
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let after_json = serde_json::to_value(&job).ok();
    if let Err(e) = audit_repo
        .insert(AuditLogCreate {
            tenant_id: ctx.tenant_id,
            user_id: Some(ctx.user_id),
            action: "EXPORT.JOB.CREATE".to_string(),
            entity_type: "EXPORT_JOB".to_string(),
            entity_id: Some(job.job_id.to_string()),
            before_json: None,
            after_json,
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    (StatusCode::OK, Json(export_job_response(job))).into_response()
}

pub(crate) async fn get_export_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    if let Err(resp) = ensure_permission(&ctx, "exports.read") {
        return resp.into_response();
    }

    let job_id = match uuid::Uuid::parse_str(&job_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad job_id").into_response(),
    };

    let repo = ExportJobsRepo::new(state.db.pool());
    let job = match repo.get_job(ctx.tenant_id, job_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("export job not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    if let Err(response) = inventory_export_access::authorize(&state, &ctx, &job).await {
        return response;
    }
    let output_url = match crate::export_job_views::visible_output_url(&state, &job).await {
        Ok(url) => url,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    let mut response = export_job_response(job);
    response.output_url = output_url;
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn download_export_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    if let Err(resp) = ensure_permission(&ctx, "exports.read") {
        return resp.into_response();
    }

    let job_id = match uuid::Uuid::parse_str(&job_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad job_id").into_response(),
    };

    let repo = ExportJobsRepo::new(state.db.pool());
    let job = match repo.get_job(ctx.tenant_id, job_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("export job not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let artifact = match inventory_export_access::authorize(&state, &ctx, &job).await {
        Ok(artifact) => artifact,
        Err(response) => return response,
    };
    if job.status != "DONE" {
        return bad_request("export not ready").into_response();
    }
    if let Err(response) = exit_manifest_matches_download(&state, ctx.tenant_id, &job).await {
        return response;
    }

    let file_path = export_job_file_path(job.tenant_id, job.job_id);
    if artifact.is_some() && inventory_export_access::check_path(&file_path).await.is_err() {
        return not_found("inventory export file unavailable").into_response();
    }
    let file_result = if let Some(receipt) = artifact.as_ref() {
        export_file_stream::export_file_body_checked(
            &file_path,
            Some((receipt.bytes, receipt.sha256.clone())),
        ).await
    } else {
        export_file_body(&file_path).await
    };
    let (file_body, content_len) = match file_result {
        Ok(file) => file,
        Err(_) => return not_found("export file missing").into_response(),
    };
    let mut headers = HeaderMap::new();
    if let Some(artifact) = artifact {
        if content_len != artifact.bytes {
            return conflict("inventory export file length mismatch").into_response();
        }
        let digest = match HeaderValue::from_str(&artifact.sha256) {
            Ok(digest) => digest,
            Err(_) => return conflict("inventory export receipt invalid").into_response(),
        };
        headers.insert("x-export-sha256", digest);
        headers.insert("x-export-schema", HeaderValue::from_static("2"));
    }
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/csv"));
    let content_len = match HeaderValue::from_str(&content_len.to_string()) {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    };
    headers.insert(header::CONTENT_LENGTH, content_len);
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    let filename = format!("export-{}.csv", job.job_id);
    let disposition = format!("attachment; filename=\"{filename}\"");
    let disposition = match HeaderValue::from_str(&disposition) {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
        }
    };
    headers.insert(header::CONTENT_DISPOSITION, disposition);

    (StatusCode::OK, headers, file_body).into_response()
}

async fn exit_manifest_matches_download(
    state: &AppState,
    tenant_id: TenantId,
    job: &erp_repo::export_jobs::ExportJob,
) -> Result<(), Response> {
    let item_no = job.job_id.to_string();
    let listed: Option<(Option<i64>, Option<String>, bool)> = sqlx::query_as(
        "select m.line_count, m.digest, exists(
            select 1 from exit_requests r
            where r.tenant_id = m.tenant_id and r.status = 'DELIVERED'
         )
         from exit_manifests m
         where m.tenant_id = $1 and m.item_kind = 'EXPORT_FILE' and m.item_no = $2",
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(&item_no)
    .fetch_optional(state.db.pool())
    .await
    .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response())?;
    let Some((expected_lines, expected_digest, delivered)) = listed else {
        return Ok(());
    };
    if !delivered {
        return Err(conflict("exit file is not delivered").into_response());
    }
    let (Some(expected_lines), Some(expected_digest)) = (expected_lines, expected_digest) else {
        return Err(conflict("delivered file does not match the exit manifest").into_response());
    };
    let file_path = export_job_file_path(tenant_id, job.job_id);
    let bytes = tokio::fs::read(&file_path)
        .await
        .map_err(|_| not_found("export file missing").into_response())?;
    let lines = bytes.iter().filter(|byte| **byte == b'\n').count() as i64;
    let digest = format!("{:x}", sha2::Sha256::digest(&bytes));
    erp_domain::exit::exit_file_matches(lines, &digest, expected_lines, &expected_digest)
        .map_err(|_| conflict("delivered file does not match the exit manifest").into_response())
}

#[cfg(test)]
mod export_stream_authorization_tests {
    use super::*;

    #[tokio::test]
    async fn export_stream_download_requires_exports_read_permission() {
        let db = Db::connect_lazy("postgres://postgres@localhost/postgres")
            .expect("lazy db should be constructible");
        let jwt = JwtService::new("test-secret".to_string());
        let state = AppState::new(db, Arc::new(jwt));
        let ctx = AuthContext {
            tenant_id: TenantId(uuid::Uuid::new_v4()),
            user_id: UserId(uuid::Uuid::new_v4()),
            perms: Vec::new(),
            is_admin: false,
            shop_ids: Vec::new(),
            warehouse_ids: Vec::new(),
            merchant_ids: Vec::new(),
            organization_ids: Vec::new(),
        session_id: None,
        };

        let response = download_export_job(
            State(state),
            Path(uuid::Uuid::new_v4().to_string()),
            Extension(ctx),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn product_export_params_are_strict_validated_and_normalized() {
        let request_id = uuid::Uuid::new_v4();
        let params = normalize_product_export_params(Some(serde_json::json!({
            "request_id": request_id,
            "query": {
                "schema": 1,
                "offset": 900,
                "limit": 500,
                "filters": {},
                "sorts": [],
                "search": "shirt",
                "expression": null
            }
        })))
        .unwrap();
        assert_eq!(params.request_id, request_id);
        let query = params.query.unwrap();
        assert_eq!((query.offset, query.limit), (0, 100));

        assert!(normalize_product_export_params(Some(serde_json::json!({
            "request_id": uuid::Uuid::nil(),
            "query": {"schema": 1, "offset": 0, "limit": 100, "filters": {}, "sorts": [], "search": null, "expression": null}
        }))).is_err());

        assert!(normalize_product_export_params(Some(serde_json::json!({
            "request_id": request_id,
            "query": {"schema": 1, "offset": 0, "limit": 100, "filters": {}, "sorts": [], "search": null, "expression": null},
            "unexpected": true
        }))).is_err());
        assert!(normalize_product_export_params(Some(serde_json::json!({
            "request_id": request_id,
            "query": {"schema": 2, "offset": 0, "limit": 100, "filters": {}, "sorts": [], "search": null, "expression": null}
        }))).is_err());
    }

    #[test]
    fn product_export_selection_params_require_one_source_and_unique_ids() {
        let token = uuid::Uuid::new_v4();
        let id = uuid::Uuid::new_v4();
        let valid = serde_json::json!({"request_id":uuid::Uuid::new_v4(),
            "selection":{"token":token,"excluded":[id],"sorts":[]}});
        assert!(normalize_product_export_params(Some(valid.clone())).is_ok());
        let mut duplicate = valid.clone();
        duplicate["selection"]["excluded"] = serde_json::json!([id, id]);
        assert!(normalize_product_export_params(Some(duplicate)).is_err());
        let mut nil = valid.clone();
        nil["selection"]["token"] = serde_json::json!(uuid::Uuid::nil());
        assert!(normalize_product_export_params(Some(nil)).is_err());
        let mut both = valid;
        both["query"] = serde_json::json!({"schema":1,"offset":0,"limit":100,
            "filters":{},"sorts":[],"search":null,"expression":null});
        assert!(normalize_product_export_params(Some(both)).is_err());
        assert!(
            normalize_product_export_params(Some(serde_json::json!({
            "request_id":uuid::Uuid::new_v4()})))
            .is_err()
        );
    }

    #[test]
    fn checked_export_params_reject_ambiguous_empty_duplicate_and_oversized_sources() {
        let request_id = uuid::Uuid::new_v4();
        let low = uuid::Uuid::from_u128(1);
        let high = uuid::Uuid::from_u128(2);
        let valid = serde_json::json!({"request_id":request_id,
            "checked":{"ids":[high,low],"sorts":[]}});
        let parsed = normalize_product_export_params(Some(valid.clone())).unwrap();
        assert_eq!(parsed.checked.unwrap().ids, vec![low, high]);
        for ids in [
            serde_json::json!([]),
            serde_json::json!([low, low]),
            serde_json::json!([uuid::Uuid::nil()]),
            serde_json::json!(vec![low; 100_001]),
        ] {
            let mut candidate = valid.clone();
            candidate["checked"]["ids"] = ids;
            assert!(normalize_product_export_params(Some(candidate)).is_err());
        }
        let mut mixed = valid;
        mixed["selection"] = serde_json::json!({"token":high,"excluded":[],"sorts":[]});
        assert!(normalize_product_export_params(Some(mixed)).is_err());
    }

    #[tokio::test]
    async fn product_export_requires_skus_read_in_addition_to_exports_write() {
        let db = Db::connect_lazy("postgres://postgres@localhost/postgres")
            .expect("lazy db should be constructible");
        let state = AppState::new(db, Arc::new(JwtService::new("test-secret".to_string())));
        let base_ctx = AuthContext {
            tenant_id: TenantId(uuid::Uuid::new_v4()),
            user_id: UserId(uuid::Uuid::new_v4()),
            perms: normalize_permissions(vec!["exports.write".to_string()]),
            is_admin: false,
            shop_ids: Vec::new(),
            warehouse_ids: Vec::new(),
            merchant_ids: Vec::new(),
            organization_ids: Vec::new(),
        session_id: None,
        };
        let response = create_export_job(
            State(state.clone()),
            Extension(base_ctx.clone()),
            Json(ExportJobCreateRequest {
                job_type: "PRODUCT_EXPORT".to_string(),
                params: Some(serde_json::json!({})),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let response = create_export_job(
            State(state),
            Extension(AuthContext {
                perms: normalize_permissions(vec![
                    "exports.write".to_string(),
                    "skus.read".to_string(),
                ]),
                ..base_ctx
            }),
            Json(ExportJobCreateRequest {
                job_type: "PRODUCT_EXPORT".to_string(),
                params: Some(serde_json::json!({})),
            }),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
