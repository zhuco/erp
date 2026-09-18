use super::*;

pub(super) fn default_oms_review_settings() -> OmsReviewSettingsPayload {
    OmsReviewSettingsPayload {
        review_enabled: true,
        auto_review_enabled: true,
        recheck_enabled: true,
        default_unmatched_action: "MANUAL_REVIEW".to_string(),
        manual_order_force_recheck: false,
    }
}

pub(super) fn parse_oms_review_settings_value(value: &Value) -> Option<OmsReviewSettingsPayload> {
    if let Ok(settings) = serde_json::from_value::<OmsReviewSettingsPayload>(value.clone()) {
        return Some(settings);
    }
    if let Some(nested) = value.get("value") {
        if let Ok(settings) = serde_json::from_value::<OmsReviewSettingsPayload>(nested.clone()) {
            return Some(settings);
        }
    }
    None
}

pub(super) async fn get_review_settings(
    State(state): State<AppState>,
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
    if let Err(resp) = ensure_permission(&ctx, "orders.review.settings.read") {
        return resp.into_response();
    }

    let repo = SettingsRepo::new(state.db.pool());
    let setting = match repo
        .list_settings(
            tenant_id,
            SettingListFilters {
                scope_type: Some("TENANT".to_string()),
                scope_id: None,
                key: Some(OMS_REVIEW_SETTINGS_KEY.to_string()),
            },
        )
        .await
    {
        Ok(mut rows) => rows.drain(..).next(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let (payload, updated_at) = match setting {
        Some(setting) => (
            parse_oms_review_settings_value(&setting.value_json)
                .unwrap_or_else(default_oms_review_settings),
            setting.updated_at,
        ),
        None => (default_oms_review_settings(), Utc::now()),
    };

    let response = OmsReviewSettingsResponse {
        tenant_id: tenant_id.to_string(),
        review_enabled: payload.review_enabled,
        auto_review_enabled: payload.auto_review_enabled,
        recheck_enabled: payload.recheck_enabled,
        default_unmatched_action: payload.default_unmatched_action,
        manual_order_force_recheck: payload.manual_order_force_recheck,
        updated_at,
    };

    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn update_review_settings(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OmsReviewSettingsPayload>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.review.settings.write") {
        return resp.into_response();
    }
    let action = req.default_unmatched_action.trim().to_ascii_uppercase();
    if !matches!(
        action.as_str(),
        "MANUAL_REVIEW" | "PASS" | "REJECT" | "FLAG"
    ) {
        return bad_request("bad default_unmatched_action").into_response();
    }

    let payload = OmsReviewSettingsPayload {
        review_enabled: req.review_enabled,
        auto_review_enabled: req.auto_review_enabled,
        recheck_enabled: req.recheck_enabled,
        default_unmatched_action: action,
        manual_order_force_recheck: req.manual_order_force_recheck,
    };

    let repo = SettingsRepo::new(state.db.pool());
    let setting = match repo
        .upsert_setting(SettingUpsert {
            tenant_id,
            scope_type: "TENANT".to_string(),
            scope_id: None,
            key: OMS_REVIEW_SETTINGS_KEY.to_string(),
            value_json: serde_json::to_value(&payload).unwrap_or_default(),
            expected_version: None,
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
        "OMS.REVIEW_SETTINGS_UPDATE",
        "ORDER_REVIEW_SETTINGS",
        Some(setting.setting_id.to_string()),
        None,
        Some(serde_json::json!({
            "review_enabled": payload.review_enabled,
            "auto_review_enabled": payload.auto_review_enabled,
            "recheck_enabled": payload.recheck_enabled,
            "default_unmatched_action": payload.default_unmatched_action,
            "manual_order_force_recheck": payload.manual_order_force_recheck,
        })),
    )
    .await;

    let response = OmsReviewSettingsResponse {
        tenant_id: tenant_id.to_string(),
        review_enabled: payload.review_enabled,
        auto_review_enabled: payload.auto_review_enabled,
        recheck_enabled: payload.recheck_enabled,
        default_unmatched_action: payload.default_unmatched_action,
        manual_order_force_recheck: payload.manual_order_force_recheck,
        updated_at: setting.updated_at,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn list_recheck_queue(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<OmsRecheckQueueQuery>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.recheck.read") {
        return resp.into_response();
    }

    let platform = match query.platform.as_deref().and_then(non_empty) {
        Some(raw) => match Platform::from_str(&raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad platform").into_response(),
        },
        None => None,
    };
    let shop_id = match query.shop_id.as_deref().and_then(non_empty) {
        Some(raw) => match ShopId::from_str(&raw) {
            Ok(v) => {
                if let Err(resp) = ensure_shop_access(&ctx, v) {
                    return resp.into_response();
                }
                Some(v)
            }
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };

    let order_source = query.order_source.as_deref().and_then(non_empty);
    let item_keyword = query.sku_id.as_deref().and_then(non_empty);
    let _sales_channel = query.sales_channel.as_deref().and_then(non_empty);
    let _min_profit_rate_bp = query
        .min_estimated_profit_rate_bp
        .as_deref()
        .and_then(non_empty);
    let _max_profit_rate_bp = query
        .max_estimated_profit_rate_bp
        .as_deref()
        .and_then(non_empty);
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let filters = OmsOrderListFilters {
        platform,
        platforms: None,
        shop_id,
        shop_ids: shop_scope_filter(&ctx),
        shop_ids_filter: None,
        oms_status: None,
        oms_statuses: None,
        order_status: None,
        review_status: Some(OrderReviewStatus::ManualReview),
        fulfillment_status: None,
        exception_status: None,
        payment_status: None,
        exception_flags: Vec::new(),
        tags: Vec::new(),
        tagged: None,
        created_from: None,
        created_to: None,
        updated_from: None,
        updated_to: None,
        ship_warehouse_id: None,
        ship_warehouse_ids: warehouse_scope_filter(&ctx),
        platform_order_id: None,
        item_keyword,
        total_amount_cent: None,
        total_amount_min_cent: None,
        total_amount_max_cent: None,
        goods_qty: None,
        customer_account: None,
        receiver_name: None,
        receiver_phone: None,
        receiver_address: None,
        remark: None,
        order_source,
        logistics_company: None,
        estimated_postage_cent: None,
        estimated_profit_cent: None,
        estimated_profit_rate: None,
        keyword: None,
        offset,
        limit,
    };

    let orders_repo = OrdersRepo::new(state.db.pool());
    let result = match orders_repo.list_orders(tenant_id, filters).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "OMS.RECHECK_QUEUE_LIST",
        "ORDER_RECHECK_QUEUE",
        None,
        None,
        Some(serde_json::json!({
            "offset": offset,
            "limit": limit,
            "total": result.total,
        })),
    )
    .await;

    (StatusCode::OK, Json(result)).into_response()
}

pub(super) async fn batch_review_recheck_queue(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OmsRecheckBatchReviewRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.recheck.write") {
        return resp.into_response();
    }
    if req.order_ids.is_empty() {
        return bad_request("order_ids required").into_response();
    }

    let action = req.action.trim().to_ascii_uppercase();
    if !matches!(action.as_str(), "APPROVE" | "REJECT" | "FLAG") {
        return bad_request("bad action").into_response();
    }
    let review_action = match action.as_str() {
        "APPROVE" => ReviewAction::Pass,
        "REJECT" => ReviewAction::Reject,
        "FLAG" => ReviewAction::Flag,
        _ => return bad_request("bad action").into_response(),
    };

    let total = req.order_ids.len() as i64;
    let batch_repo = OmsRecheckBatchRepo::new(state.db.pool());
    let batch_job = match batch_repo
        .create_job(OmsRecheckBatchJobCreate {
            tenant_id,
            action: action.clone(),
            reason: req.reason.clone(),
            requested_by: Some(ctx.user_id),
            total_count: total,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let orders_repo = OrdersRepo::new(state.db.pool());
    let mut accepted = 0_i64;
    let mut rejected = 0_i64;
    let mut seen_order_ids = std::collections::HashSet::<String>::new();
    for raw in &req.order_ids {
        let raw_order_id = raw.trim().to_string();
        if raw_order_id.is_empty() {
            rejected += 1;
            let _ = batch_repo
                .insert_item(OmsRecheckBatchItemCreate {
                    tenant_id,
                    job_id: batch_job.job_id,
                    order_id: None,
                    raw_order_id,
                    result: "REJECTED".to_string(),
                    error_code: Some("INVALID_ORDER_ID".to_string()),
                    error_message: Some("order_id is empty".to_string()),
                    before_json: None,
                    after_json: None,
                })
                .await;
            continue;
        }
        let dedup_key = raw_order_id.to_ascii_lowercase();
        if !seen_order_ids.insert(dedup_key) {
            rejected += 1;
            let _ = batch_repo
                .insert_item(OmsRecheckBatchItemCreate {
                    tenant_id,
                    job_id: batch_job.job_id,
                    order_id: None,
                    raw_order_id,
                    result: "REJECTED".to_string(),
                    error_code: Some("DUPLICATE_ORDER_ID".to_string()),
                    error_message: Some("duplicate order_id in request".to_string()),
                    before_json: None,
                    after_json: None,
                })
                .await;
            continue;
        }

        let order_id = match OrderId::from_str(&raw_order_id) {
            Ok(v) => v,
            Err(_) => {
                rejected += 1;
                let _ = batch_repo
                    .insert_item(OmsRecheckBatchItemCreate {
                        tenant_id,
                        job_id: batch_job.job_id,
                        order_id: None,
                        raw_order_id,
                        result: "REJECTED".to_string(),
                        error_code: Some("INVALID_ORDER_ID".to_string()),
                        error_message: Some("invalid UUID format".to_string()),
                        before_json: None,
                        after_json: None,
                    })
                    .await;
                continue;
            }
        };

        let before_state = match orders_repo
            .get_order_review_state(tenant_id, order_id)
            .await
        {
            Ok(Some(v)) => v,
            Ok(None) => {
                rejected += 1;
                let _ = batch_repo
                    .insert_item(OmsRecheckBatchItemCreate {
                        tenant_id,
                        job_id: batch_job.job_id,
                        order_id: Some(order_id),
                        raw_order_id,
                        result: "REJECTED".to_string(),
                        error_code: Some("ORDER_NOT_FOUND".to_string()),
                        error_message: Some("order not found".to_string()),
                        before_json: None,
                        after_json: None,
                    })
                    .await;
                continue;
            }
            Err(e) => {
                rejected += 1;
                let _ = batch_repo
                    .insert_item(OmsRecheckBatchItemCreate {
                        tenant_id,
                        job_id: batch_job.job_id,
                        order_id: Some(order_id),
                        raw_order_id,
                        result: "REJECTED".to_string(),
                        error_code: Some("STATE_LOAD_FAILED".to_string()),
                        error_message: Some(e.to_string()),
                        before_json: None,
                        after_json: None,
                    })
                    .await;
                continue;
            }
        };

        if before_state.review_status != OrderReviewStatus::ManualReview {
            rejected += 1;
            let _ = batch_repo
                .insert_item(OmsRecheckBatchItemCreate {
                    tenant_id,
                    job_id: batch_job.job_id,
                    order_id: Some(order_id),
                    raw_order_id,
                    result: "REJECTED".to_string(),
                    error_code: Some("NOT_IN_RECHECK_QUEUE".to_string()),
                    error_message: Some("order review status is not MANUAL_REVIEW".to_string()),
                    before_json: Some(serde_json::json!({
                        "review_status": before_state.review_status.to_string(),
                        "oms_status": before_state.oms_status.to_string(),
                        "order_status": before_state.order_status.to_string(),
                    })),
                    after_json: None,
                })
                .await;
            continue;
        }

        match apply_review_update(
            &state,
            &ctx,
            &req_ctx,
            tenant_id,
            order_id,
            review_action,
            req.reason.clone(),
            Some(before_state.version),
            false,
            false,
        )
        .await
        {
            Ok(updated) => {
                accepted += 1;
                let _ = batch_repo
                    .insert_item(OmsRecheckBatchItemCreate {
                        tenant_id,
                        job_id: batch_job.job_id,
                        order_id: Some(order_id),
                        raw_order_id,
                        result: "ACCEPTED".to_string(),
                        error_code: None,
                        error_message: None,
                        before_json: Some(serde_json::json!({
                            "review_status": before_state.review_status.to_string(),
                            "oms_status": before_state.oms_status.to_string(),
                            "order_status": before_state.order_status.to_string(),
                            "version": before_state.version,
                        })),
                        after_json: Some(serde_json::json!({
                            "review_status": updated.review_status.to_string(),
                            "oms_status": updated.oms_status.to_string(),
                            "order_status": updated.order_status.to_string(),
                            "version": updated.version,
                        })),
                    })
                    .await;
            }
            Err(resp) => {
                rejected += 1;
                let error_code = match resp.status() {
                    StatusCode::BAD_REQUEST => "INVALID_ARGUMENT",
                    StatusCode::FORBIDDEN => "FORBIDDEN",
                    StatusCode::NOT_FOUND => "ORDER_NOT_FOUND",
                    StatusCode::CONFLICT => "STATE_VERSION_CONFLICT",
                    _ => "REVIEW_UPDATE_FAILED",
                };
                let _ = batch_repo
                    .insert_item(OmsRecheckBatchItemCreate {
                        tenant_id,
                        job_id: batch_job.job_id,
                        order_id: Some(order_id),
                        raw_order_id,
                        result: "REJECTED".to_string(),
                        error_code: Some(error_code.to_string()),
                        error_message: Some(format!("http status {}", resp.status().as_u16())),
                        before_json: Some(serde_json::json!({
                            "review_status": before_state.review_status.to_string(),
                            "oms_status": before_state.oms_status.to_string(),
                            "order_status": before_state.order_status.to_string(),
                            "version": before_state.version,
                        })),
                        after_json: None,
                    })
                    .await;
            }
        }
    }
    let _ = batch_repo
        .complete_job(tenant_id, batch_job.job_id, accepted, rejected, "COMPLETED")
        .await;

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "OMS.RECHECK_QUEUE_BATCH_REVIEW",
        "ORDER_RECHECK_QUEUE",
        None,
        None,
        Some(serde_json::json!({
            "job_id": batch_job.job_id,
            "total": total,
            "accepted": accepted,
            "rejected": rejected,
            "action": action,
            "reason": req.reason,
        })),
    )
    .await;

    let response = OmsRecheckBatchReviewResponse {
        total,
        accepted,
        rejected,
        action,
        reason: req.reason,
    };
    (StatusCode::OK, Json(response)).into_response()
}
