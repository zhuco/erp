use super::*;

pub(crate) async fn list_wms_waves(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<WmsWaveListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.read") {
        return resp.into_response();
    }

    let warehouse_id = match query.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = WmsRepo::new(state.db.pool());
    let result = match repo
        .list_waves(
            tenant_id,
            WaveListFilters {
                warehouse_id,
                status: query.status.filter(|v| !v.trim().is_empty()),
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let items = result.items.into_iter().map(wms_wave_response).collect();
    let response = WmsWaveListResponse {
        items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn get_wms_wave(
    State(state): State<AppState>,
    Path((tenant_id, wave_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let wave_id = match uuid::Uuid::parse_str(&wave_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad wave_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.read") {
        return resp.into_response();
    }

    let repo = WmsRepo::new(state.db.pool());
    let detail = match repo.get_wave(tenant_id, wave_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("wave not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = WmsWaveDetailResponse {
        wave: wms_wave_response(detail.wave),
        docs: detail
            .docs
            .into_iter()
            .map(wms_outbound_doc_response)
            .collect(),
        tasks: detail
            .tasks
            .into_iter()
            .map(wms_pick_task_response)
            .collect(),
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn ship_wms_wave(
    State(state): State<AppState>,
    Path((tenant_id, wave_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<WmsWaveShipRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let wave_id = match uuid::Uuid::parse_str(&wave_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad wave_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.wave_ship") {
        return resp.into_response();
    }

    let direct_ship = req.direct_ship.unwrap_or(false);
    if direct_ship {
        if let Err(resp) = ensure_permission(&ctx, "wms.direct_ship") {
            return resp.into_response();
        }
    }

    let shipped_at = match req.shipped_at.as_deref().and_then(non_empty) {
        Some(raw) => match DateTime::parse_from_rfc3339(&raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad shipped_at").into_response(),
        },
        None => None,
    };
    let continue_on_error = req.continue_on_error.unwrap_or(true);

    let repo = WmsRepo::new(state.db.pool());
    let detail = match repo.get_wave(tenant_id, wave_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("wave not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if let Err(resp) = ensure_warehouse_access(&ctx, detail.wave.warehouse_id) {
        return resp.into_response();
    }
    if detail.docs.is_empty() {
        return bad_request("wave has no outbound docs").into_response();
    }

    let inspect_enabled = match feature_flag_enabled(
        &state,
        tenant_id,
        None,
        FeatureFlagKey::WmsOutboundInspection,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let workflow_config =
        match fetch_warehouse_workflow_config(state.db.pool(), tenant_id, detail.wave.warehouse_id)
            .await
        {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };

    let allow_unpacked = direct_ship || !workflow_config.require_pack;
    let inspect_required = !direct_ship && inspect_enabled && workflow_config.require_inspection;

    let mut items = Vec::with_capacity(detail.docs.len());
    let mut success = 0_i64;
    let mut failed = 0_i64;
    for doc in &detail.docs {
        let result = repo
            .ship_outbound(OutboundShipInput {
                tenant_id,
                outbound_doc_id: doc.outbound_doc_id,
                shipped_at,
                allow_unpacked,
                inspect_required,
            })
            .await;

        match result {
            Ok(_) => {
                sync_shipment_serials_after_wms_ship(&state, tenant_id, doc.shipment_id).await;
                success += 1;
                items.push(WmsWaveShipDocResult {
                    outbound_doc_id: doc.outbound_doc_id,
                    status: "SHIPPED".to_string(),
                    message: None,
                });
                sync_sales_order_status_after_wms_ship(
                    &state,
                    &req_ctx,
                    tenant_id,
                    doc.outbound_doc_id,
                    ctx.user_id,
                    shipped_at,
                )
                .await;
            }
            Err(e) => {
                failed += 1;
                items.push(WmsWaveShipDocResult {
                    outbound_doc_id: doc.outbound_doc_id,
                    status: "FAILED".to_string(),
                    message: Some(e.to_string()),
                });
                if !continue_on_error {
                    break;
                }
            }
        }
    }

    let final_status = if failed == 0 {
        "SHIPPED"
    } else if success == 0 {
        "SHIP_FAILED"
    } else {
        "PARTIAL_SHIPPED"
    };
    if let Err(e) = sqlx::query(
        r#"
        update waves
        set status = $3,
            updated_at = now()
        where tenant_id = $1 and wave_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(wave_id)
    .bind(final_status)
    .execute(state.db.pool())
    .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: "WMS.WAVE_SHIPPED".to_string(),
            ref_type: Some("WAVE".to_string()),
            ref_id: Some(wave_id.to_string()),
            metadata: Some(serde_json::json!({
                "success": success,
                "failed": failed,
                "direct_ship": direct_ship,
                "continue_on_error": continue_on_error,
                "shipped_at": shipped_at,
            })),
        })
        .await
    {
        warn!(error = %e, "operation log insert failed");
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(e) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.WAVE_SHIPPED".to_string(),
            entity_type: "WAVE".to_string(),
            entity_id: Some(wave_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({
                "success": success,
                "failed": failed,
                "status": final_status,
                "direct_ship": direct_ship,
            })),
            ip: req_ctx.ip.clone(),
            ua: req_ctx.user_agent.clone(),
        })
        .await
    {
        warn!(error = %e, "audit log insert failed");
    }

    (
        StatusCode::OK,
        Json(WmsWaveShipResponse {
            wave_id,
            total: success + failed,
            success,
            failed,
            direct_ship,
            items,
        }),
    )
        .into_response()
}

pub(crate) async fn print_wms_wave(
    State(state): State<AppState>,
    Path((tenant_id, wave_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let wave_id = match uuid::Uuid::parse_str(&wave_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad wave_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.read") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "print_jobs.write") {
        return resp.into_response();
    }

    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::Printing).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::Printing).into_response();
    }

    let repo = WmsRepo::new(state.db.pool());
    let detail = match repo.get_wave(tenant_id, wave_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("wave not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let (template_id, device_fingerprint) =
        match resolve_print_mapping(&state, tenant_id, "PICK_LIST").await {
            Ok(v) => v,
            Err(resp) => return resp,
        };

    let payload = match serde_json::to_value(WmsWaveDetailResponse {
        wave: wms_wave_response(detail.wave),
        docs: detail
            .docs
            .into_iter()
            .map(wms_outbound_doc_response)
            .collect(),
        tasks: detail
            .tasks
            .into_iter()
            .map(wms_pick_task_response)
            .collect(),
    }) {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let job = match state
        .print_jobs_repo
        .create_job(PrintJobCreate {
            tenant_id,
            requested_by_user_id: ctx.user_id,
            job_type: "PICK_LIST".to_string(),
            ref_type: "WAVE".to_string(),
            ref_id: wave_id,
            template_id,
            device_fingerprint,
            payload,
            trace_id: Some(req_ctx.trace_id.clone()),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "PRINT_JOB.CREATE",
        "print_job",
        Some(job.job_id.to_string()),
        None,
        Some(serde_json::to_value(&job).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(job)).into_response()
}

pub(crate) async fn create_wms_wave(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WmsWaveCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.write") {
        return resp.into_response();
    }
    let wave_enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::WmsWave).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !wave_enabled {
        return feature_flag_forbidden(FeatureFlagKey::WmsWave).into_response();
    }

    let warehouse_id = match uuid::Uuid::parse_str(&req.warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    let workflow_config =
        match fetch_warehouse_workflow_config(state.db.pool(), tenant_id, warehouse_id).await {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
    if !workflow_config.enable_wave {
        return bad_request("wave disabled").into_response();
    }
    let cutoff_at = match req.cutoff_at.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => v.with_timezone(&Utc),
            Err(_) => return bad_request("bad cutoff_at").into_response(),
        },
        None => Utc::now(),
    };
    let max_docs = req.max_docs.unwrap_or(50).clamp(1, 500);
    let auto_pick = req.auto_pick.unwrap_or(false);
    let outbound_doc_ids = match req.outbound_doc_ids {
        Some(ids) => {
            if ids.is_empty() {
                return bad_request("outbound_doc_ids required").into_response();
            }
            let mut parsed = Vec::with_capacity(ids.len());
            for raw in ids {
                let id = match uuid::Uuid::parse_str(raw.trim()) {
                    Ok(v) => v,
                    Err(_) => return bad_request("bad outbound_doc_id").into_response(),
                };
                parsed.push(id);
            }
            Some(parsed)
        }
        None => None,
    };

    let repo = WmsRepo::new(state.db.pool());
    let require_audit =
        match fetch_tenant_setting_value(state.db.pool(), tenant_id, "inv.outbound_require_audit")
            .await
        {
            Ok(value) => value.as_ref().and_then(parse_bool_setting).unwrap_or(false),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };

    let wave_input = WaveCreateInput {
        tenant_id,
        warehouse_id,
        cutoff_at,
        max_docs,
        auto_pick,
        outbound_doc_ids,
    };

    let detail = if require_audit {
        match repo.create_wave_with_status(wave_input, "AUDITED").await {
            Ok(v) => v,
            Err(e) => return bad_request(&e.to_string()).into_response(),
        }
    } else {
        match repo.create_wave(wave_input).await {
            Ok(v) => v,
            Err(e) => return bad_request(&e.to_string()).into_response(),
        }
    };

    if let Err(e) = apply_pick_recommendation_for_wave(&state, tenant_id, detail.wave.wave_id).await
    {
        warn!(error = %e, "pick recommendation failed");
    }

    let detail = match repo.get_wave(tenant_id, detail.wave.wave_id).await {
        Ok(Some(v)) => v,
        Ok(None) => detail,
        Err(e) => {
            warn!(error = %e, "reload wave failed");
            detail
        }
    };

    let logs_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.WAVE_CREATED".to_string(),
            entity_type: "WAVE".to_string(),
            entity_id: Some(detail.wave.wave_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({
                "warehouse_id": detail.wave.warehouse_id,
                "doc_count": detail.docs.len(),
            })),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %e, "audit log insert failed");
    }

    let response = WmsWaveDetailResponse {
        wave: wms_wave_response(detail.wave),
        docs: detail
            .docs
            .into_iter()
            .map(wms_outbound_doc_response)
            .collect(),
        tasks: detail
            .tasks
            .into_iter()
            .map(wms_pick_task_response)
            .collect(),
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn run_wms_wave(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WmsWaveRunRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.write") {
        return resp.into_response();
    }

    let wave_enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::WmsWave).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !wave_enabled {
        return feature_flag_forbidden(FeatureFlagKey::WmsWave).into_response();
    }

    let warehouse_id = match uuid::Uuid::parse_str(&req.warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    let workflow_config =
        match fetch_warehouse_workflow_config(state.db.pool(), tenant_id, warehouse_id).await {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
    if !workflow_config.enable_wave {
        return bad_request("wave disabled").into_response();
    }

    let repo = WmsRepo::new(state.db.pool());
    let rule = match repo
        .list_wave_rules(
            tenant_id,
            erp_repo::wms::WaveRuleListFilters {
                warehouse_id: Some(warehouse_id),
                carrier_code: None,
                enabled: None,
                offset: 0,
                limit: 1,
            },
        )
        .await
    {
        Ok(v) => v.items.into_iter().next(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if rule.as_ref().map(|cfg| cfg.enabled) == Some(false) {
        let response = WmsWaveRunResponse {
            status: "disabled".to_string(),
            wave: None,
        };
        return (StatusCode::OK, Json(response)).into_response();
    }

    let cutoff_at = if let Some(raw) = req.cutoff_at.as_deref().and_then(non_empty) {
        match DateTime::parse_from_rfc3339(&raw) {
            Ok(v) => v.with_timezone(&Utc),
            Err(_) => return bad_request("bad cutoff_at").into_response(),
        }
    } else if let Some(time) = rule.as_ref().and_then(|cfg| cfg.cutoff_time) {
        resolve_wave_cutoff_at(time)
    } else {
        Utc::now()
    };

    let max_docs = req
        .max_docs
        .or_else(|| {
            rule.as_ref()
                .and_then(|cfg| cfg.max_orders.map(|v| v as i64))
        })
        .unwrap_or(50)
        .clamp(1, 500);
    let min_docs = req.min_docs.unwrap_or(1).clamp(1, 500);
    let auto_pick = req.auto_pick.unwrap_or(false);
    let max_qty = rule.as_ref().and_then(|cfg| cfg.max_qty);
    let carrier_code = rule
        .as_ref()
        .and_then(|cfg| cfg.carrier_code.as_deref())
        .and_then(non_empty);

    let require_audit =
        match fetch_tenant_setting_value(state.db.pool(), tenant_id, "inv.outbound_require_audit")
            .await
        {
            Ok(value) => value.as_ref().and_then(parse_bool_setting).unwrap_or(false),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
    let status = if require_audit { "AUDITED" } else { "DRAFT" };

    let candidates = match repo
        .list_wave_candidates(
            tenant_id,
            warehouse_id,
            status,
            cutoff_at,
            carrier_code.as_deref(),
            max_docs,
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let selected_ids = select_wave_candidates(candidates, max_docs, max_qty);
    if (selected_ids.len() as i64) < min_docs {
        let response = WmsWaveRunResponse {
            status: "empty".to_string(),
            wave: None,
        };
        return (StatusCode::OK, Json(response)).into_response();
    }

    let wave_input = WaveCreateInput {
        tenant_id,
        warehouse_id,
        cutoff_at,
        max_docs: selected_ids.len().max(1) as i64,
        auto_pick,
        outbound_doc_ids: Some(selected_ids),
    };

    let detail = if require_audit {
        repo.create_wave_with_status(wave_input, "AUDITED").await
    } else {
        repo.create_wave(wave_input).await
    };

    let detail = match detail {
        Ok(v) => v,
        Err(e) => {
            let message = e.to_string();
            if message.contains("no outbound docs available") {
                let response = WmsWaveRunResponse {
                    status: "empty".to_string(),
                    wave: None,
                };
                return (StatusCode::OK, Json(response)).into_response();
            }
            return bad_request(&message).into_response();
        }
    };

    if let Err(e) = apply_pick_recommendation_for_wave(&state, tenant_id, detail.wave.wave_id).await
    {
        warn!(error = %e, "pick recommendation failed");
    }

    let detail = match repo.get_wave(tenant_id, detail.wave.wave_id).await {
        Ok(Some(v)) => v,
        Ok(None) => detail,
        Err(e) => {
            warn!(error = %e, "reload wave failed");
            detail
        }
    };

    let response = WmsWaveDetailResponse {
        wave: wms_wave_response(detail.wave),
        docs: detail
            .docs
            .into_iter()
            .map(wms_outbound_doc_response)
            .collect(),
        tasks: detail
            .tasks
            .into_iter()
            .map(wms_pick_task_response)
            .collect(),
    };
    let response = WmsWaveRunResponse {
        status: "created".to_string(),
        wave: Some(response),
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) fn parse_wave_rule_time(raw: &str) -> Option<NaiveTime> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    NaiveTime::parse_from_str(trimmed, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(trimmed, "%H:%M"))
        .ok()
}

pub(crate) fn resolve_wave_cutoff_at(cutoff_time: NaiveTime) -> DateTime<Utc> {
    let now = Utc::now();
    let today = now.date_naive();
    let naive_cutoff = chrono::NaiveDateTime::new(today, cutoff_time);
    let mut cutoff = DateTime::<Utc>::from_naive_utc_and_offset(naive_cutoff, Utc);
    if cutoff > now {
        cutoff -= chrono::Duration::days(1);
    }
    cutoff
}

pub(crate) fn select_wave_candidates(
    candidates: Vec<erp_repo::wms::WaveCandidate>,
    max_docs: i64,
    max_qty: Option<i64>,
) -> Vec<uuid::Uuid> {
    let max_docs = max_docs.max(1) as usize;
    let mut selected = Vec::new();
    let mut total_qty: i64 = 0;
    for candidate in candidates {
        if selected.len() >= max_docs {
            break;
        }
        if let Some(limit) = max_qty {
            if !selected.is_empty() && total_qty + candidate.total_qty > limit {
                break;
            }
            if selected.is_empty() && candidate.total_qty > limit {
                selected.push(candidate.outbound_doc_id);
                break;
            }
            total_qty += candidate.total_qty;
        }
        selected.push(candidate.outbound_doc_id);
    }
    selected
}

pub(crate) async fn list_wms_wave_rules(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<WmsWaveRuleListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.read") {
        return resp.into_response();
    }

    let warehouse_id = match query.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = WmsRepo::new(state.db.pool());
    let result = match repo
        .list_wave_rules(
            tenant_id,
            WaveRuleListFilters {
                warehouse_id,
                carrier_code: query.carrier_code.and_then(non_empty),
                enabled: query.enabled,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let items = result
        .items
        .into_iter()
        .map(wms_wave_rule_response)
        .collect();
    let response = WmsWaveRuleListResponse {
        items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn get_wms_wave_rule(
    State(state): State<AppState>,
    Path((tenant_id, rule_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let rule_id = match uuid::Uuid::parse_str(&rule_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad rule_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.read") {
        return resp.into_response();
    }

    let repo = WmsRepo::new(state.db.pool());
    let rule = match repo.get_wave_rule(tenant_id, rule_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("wave rule not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(wms_wave_rule_response(rule))).into_response()
}

pub(crate) async fn create_wms_wave_rule(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WmsWaveRuleCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.write") {
        return resp.into_response();
    }

    let warehouse_id = match uuid::Uuid::parse_str(&req.warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    let cutoff_time = match req.cutoff_time.as_deref().and_then(non_empty) {
        Some(raw) => match parse_wave_rule_time(&raw) {
            Some(v) => Some(v),
            None => return bad_request("bad cutoff_time").into_response(),
        },
        None => None,
    };

    let repo = WmsRepo::new(state.db.pool());
    let rule = match repo
        .create_wave_rule(WaveRuleCreateInput {
            tenant_id,
            warehouse_id,
            carrier_code: req.carrier_code.and_then(non_empty),
            max_orders: req.max_orders,
            max_qty: req.max_qty,
            cutoff_time,
            strategy: req.strategy.and_then(non_empty),
            enabled: req.enabled.unwrap_or(true),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(wms_wave_rule_response(rule))).into_response()
}

pub(crate) async fn update_wms_wave_rule(
    State(state): State<AppState>,
    Path((tenant_id, rule_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WmsWaveRuleUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let rule_id = match uuid::Uuid::parse_str(&rule_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad rule_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.write") {
        return resp.into_response();
    }

    let cutoff_time = match req.cutoff_time.as_deref().and_then(non_empty) {
        Some(raw) => match parse_wave_rule_time(&raw) {
            Some(v) => Some(v),
            None => return bad_request("bad cutoff_time").into_response(),
        },
        None => None,
    };

    let repo = WmsRepo::new(state.db.pool());
    let rule = match repo
        .update_wave_rule(
            tenant_id,
            rule_id,
            WaveRuleUpdateInput {
                carrier_code: req.carrier_code.and_then(non_empty),
                max_orders: req.max_orders,
                max_qty: req.max_qty,
                cutoff_time,
                strategy: req.strategy.and_then(non_empty),
                enabled: req.enabled,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("wave rule not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(wms_wave_rule_response(rule))).into_response()
}

pub(crate) async fn delete_wms_wave_rule(
    State(state): State<AppState>,
    Path((tenant_id, rule_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let rule_id = match uuid::Uuid::parse_str(&rule_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad rule_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.write") {
        return resp.into_response();
    }

    let repo = WmsRepo::new(state.db.pool());
    let rule = match repo.get_wave_rule(tenant_id, rule_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("wave rule not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let deleted = match repo.delete_wave_rule(tenant_id, rule_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if !deleted {
        return not_found("wave rule not found").into_response();
    }

    (StatusCode::OK, Json(wms_wave_rule_response(rule))).into_response()
}

pub(crate) async fn list_wms_pick_tasks(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<WmsPickTaskListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.read") {
        return resp.into_response();
    }

    let warehouse_id = match query.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    let outbound_doc_id = match query.outbound_doc_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad outbound_doc_id").into_response(),
        },
        None => None,
    };
    let status = match query.status.as_deref() {
        Some(raw) => match PickStatus::from_str(raw) {
            Ok(v) => Some(v.to_string()),
            Err(_) => return bad_request("bad status").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = WmsRepo::new(state.db.pool());
    let result = match repo
        .list_pick_tasks(
            tenant_id,
            PickTaskListFilters {
                warehouse_id,
                outbound_doc_id,
                status,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let items = result
        .items
        .into_iter()
        .map(wms_pick_task_response)
        .collect();
    let response = WmsPickTaskListResponse {
        items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn confirm_wms_pick_task(
    State(state): State<AppState>,
    Path((tenant_id, pick_task_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WmsPickTaskConfirmRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let pick_task_id = match uuid::Uuid::parse_str(&pick_task_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad pick_task_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.write") {
        return resp.into_response();
    }

    let container_id = match req.container_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad container_id").into_response(),
        },
        None => None,
    };

    let repo = WmsRepo::new(state.db.pool());
    if let Err(e) = repo
        .confirm_pick_task(PickTaskConfirmInput {
            tenant_id,
            pick_task_id,
            picked_qty: req.picked_qty,
            container_id,
        })
        .await
    {
        return bad_request(&e.to_string()).into_response();
    }

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: "WMS.PICK_CONFIRMED".to_string(),
            ref_type: Some("PICK_TASK".to_string()),
            ref_id: Some(pick_task_id.to_string()),
            metadata: Some(serde_json::json!({
                "picked_qty": req.picked_qty,
                "container_id": container_id,
            })),
        })
        .await
    {
        warn!(error = %e, "operation log insert failed");
    }

    let logs_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.PICK_CONFIRMED".to_string(),
            entity_type: "PICK_TASK".to_string(),
            entity_id: Some(pick_task_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({
                "picked_qty": req.picked_qty,
                "container_id": container_id,
            })),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %e, "audit log insert failed");
    }

    StatusCode::NO_CONTENT.into_response()
}
