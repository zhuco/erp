use super::*;

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

    let (start_at, end_at) = match parse_report_range(query.start_at, query.end_at) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let warehouse_id = match query.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    let warning_threshold = query.warning_threshold.unwrap_or(5).max(0);
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
                warning_threshold,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
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
            clickhouse_endpoint: health.clickhouse_endpoint,
            sso_ready: health.sso_ready,
            sso_issuer: health.sso_issuer,
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
                .into_response()
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
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()
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
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()
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
    let result: ExportJobListResultRepo = match repo
        .list_jobs(
            ctx.tenant_id,
            ExportJobListFiltersRepo {
                status: query.status.and_then(non_empty),
                job_type: query.job_type.and_then(non_empty),
                offset,
                limit,
            },
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

    let read_only = match read_only_enabled(&state, ctx.tenant_id, None).await {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if read_only {
        return read_only_forbidden().into_response();
    }

    let job_type = req.job_type.trim().to_uppercase();
    if !matches!(
        job_type.as_str(),
        "ORDER_EXPORT" | "RECONCILE_EXPORT" | "INVENTORY_EXPORT"
    ) {
        return bad_request("unsupported export type").into_response();
    }

    let repo = ExportJobsRepo::new(state.db.pool());
    let params = req.params.unwrap_or_else(|| serde_json::json!({}));
    let job = match repo
        .create_job(ExportJobCreateInput {
            tenant_id: ctx.tenant_id,
            job_type: job_type.clone(),
            params_json: params.clone(),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
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

    (StatusCode::OK, Json(export_job_response(job))).into_response()
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

    if job.status != "DONE" {
        return bad_request("export not ready").into_response();
    }

    let file_path = export_job_file_path(job.tenant_id, job.job_id);
    let file_bytes = match tokio::fs::read(&file_path).await {
        Ok(bytes) => bytes,
        Err(_) => return not_found("export file missing").into_response(),
    };
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/csv"));
    let filename = format!("export-{}.csv", job.job_id);
    let disposition = format!("attachment; filename=\"{filename}\"");
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&disposition).unwrap_or_else(|_| HeaderValue::from_static("")),
    );

    (StatusCode::OK, headers, file_bytes).into_response()
}
