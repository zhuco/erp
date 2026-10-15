use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) async fn apply_review_update(
    state: &AppState,
    ctx: &AuthContext,
    req_ctx: &RequestContext,
    tenant_id: TenantId,
    order_id: OrderId,
    action: ReviewAction,
    reason: Option<String>,
    version: Option<i32>,
    allow_frozen: bool,
    force_review: bool,
) -> Result<OrderReviewState, Response> {
    let orders_repo = OrdersRepo::new(state.db.pool());
    let shipping_info = match orders_repo
        .get_order_shipping_info(tenant_id, order_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return Err(not_found("order not found").into_response()),
        Err(e) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response());
        }
    };
    if let Err(resp) = ensure_shop_access(ctx, shipping_info.shop_id) {
        return Err(resp.into_response());
    }

    let state_row = match orders_repo
        .get_order_review_state(tenant_id, order_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return Err(not_found("order not found").into_response()),
        Err(e) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response());
        }
    };

    if !allow_frozen
        && state_row
            .exception_flags
            .iter()
            .any(|flag| flag == ORDER_FROZEN_FLAG)
    {
        return Err(bad_request("order frozen").into_response());
    }

    if let Some(version) = version {
        if version != state_row.version {
            return Err(conflict("version mismatch").into_response());
        }
    }

    if matches!(action, ReviewAction::Pass) {
        let has_invalid_goods = state_row.exception_flags.iter().any(|flag| {
            let normalized = flag.trim().to_ascii_uppercase();
            ORDER_INVALID_GOODS_KEYWORDS
                .iter()
                .any(|keyword| normalized.contains(keyword))
        });
        if has_invalid_goods {
            return Err(bad_request("order has invalid goods").into_response());
        }

        let review_check = match orders_repo
            .get_order_review_check(tenant_id, order_id)
            .await
        {
            Ok(Some(v)) => v,
            Ok(None) => return Err(not_found("order not found").into_response()),
            Err(e) => {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response());
            }
        };
        let item_policies = match orders_repo
            .list_order_item_stock_policies(tenant_id, order_id)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response());
            }
        };
        if item_policies.is_empty() {
            return Err(bad_request("order has no items").into_response());
        }
        let has_missing_sku = item_policies.iter().any(|item| {
            item.sku_id
                .as_deref()
                .map(|value| value.trim().is_empty())
                .unwrap_or(true)
                || !item.sku_exists
        });
        if has_missing_sku {
            return Err(bad_request("order items missing sku").into_response());
        }
        let logistics_company = review_check.logistics_company.and_then(non_empty);
        let tracking_no = review_check.tracking_no.and_then(non_empty);
        if logistics_company.is_none() || tracking_no.is_none() {
            return Err(bad_request("order logistics missing").into_response());
        }
        let shortage = review_check.inventory_shortage
            || state_row
                .exception_flags
                .iter()
                .any(|flag| flag == "INVENTORY_SHORTAGE");
        if shortage && !item_policies.iter().all(|item| item.allow_negative_stock) {
            return Err(bad_request("inventory shortage").into_response());
        }
    }

    let reason_flag = if matches!(action, ReviewAction::Pass) {
        None
    } else {
        reason.as_deref()
    };
    let mut outcome = match apply_review_decision(
        state_row.oms_status,
        action,
        &state_row.exception_flags,
        reason_flag,
        ReviewMode::Manual,
    ) {
        Ok(v) => v,
        Err(e) => return Err(bad_request(&e).into_response()),
    };

    if force_review {
        outcome
            .exception_flags
            .retain(|flag| flag != ORDER_FROZEN_FLAG);
    }

    let exception_status = derive_exception_status(&outcome.exception_flags);
    let order_status = derive_order_status(
        outcome.oms_status,
        outcome.review_status,
        state_row.fulfillment_status,
        exception_status,
    );

    let updated = match orders_repo
        .update_order_review_state(
            tenant_id,
            order_id,
            OrderReviewUpdate {
                oms_status: outcome.oms_status,
                order_status,
                review_status: outcome.review_status,
                fulfillment_status: state_row.fulfillment_status,
                exception_status,
                exception_flags: outcome.exception_flags.clone(),
                expected_version: Some(state_row.version),
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response());
        }
    };
    let Some(updated) = updated else {
        return Err(conflict("review update conflict").into_response());
    };

    upsert_order_review_record(
        state,
        tenant_id,
        order_id,
        updated.review_status,
        Some(ctx.user_id),
        Some(Utc::now()),
        reason.clone(),
    )
    .await;

    let action_label = match action {
        ReviewAction::Pass => "OMS.REVIEW_PASS",
        ReviewAction::Reject => "OMS.REVIEW_REJECT",
        ReviewAction::Flag => "OMS.REVIEW_FLAG",
    };
    record_audit_log(
        state,
        ctx,
        req_ctx,
        action_label,
        "ORDER",
        Some(order_id.to_string()),
        Some(serde_json::json!({
            "oms_status": state_row.oms_status,
            "review_status": state_row.review_status,
            "exception_flags": state_row.exception_flags,
        })),
        Some(serde_json::json!({
            "oms_status": updated.oms_status,
            "review_status": updated.review_status,
            "exception_flags": updated.exception_flags,
            "reason": reason,
            "force_review": force_review,
        })),
    )
    .await;

    record_order_state_logs(
        state,
        req_ctx,
        OrderStateLogInput {
            tenant_id,
            order_id,
            action_code: action_label.to_string(),
            from_status: state_row.order_status,
            to_status: updated.order_status,
            operator_id: Some(ctx.user_id),
            reason: reason.clone(),
            request_payload: serde_json::json!({
                "reason": reason.clone(),
                "force_review": force_review,
            }),
            response_payload: serde_json::json!({
                "oms_status": updated.oms_status.to_string(),
                "order_status": updated.order_status.to_string(),
                "review_status": updated.review_status.to_string(),
                "exception_flags": updated.exception_flags,
            }),
        },
    )
    .await;

    let operation_action = match action {
        ReviewAction::Pass => OP_ACTION_OMS_REVIEW_PASS,
        ReviewAction::Reject => OP_ACTION_OMS_REVIEW_REJECT,
        ReviewAction::Flag => OP_ACTION_OMS_REVIEW_FLAG,
    };
    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: operation_action.to_string(),
            ref_type: Some("ORDER".to_string()),
            ref_id: Some(order_id.to_string()),
            metadata: Some(serde_json::json!({
                "reason": reason,
                "review_status": updated.review_status.to_string(),
                "exception_flags": updated.exception_flags,
                "force_review": force_review,
            })),
        })
        .await;

    let _ = orders_repo
        .upsert_exception_tickets(tenant_id, order_id, &updated.exception_flags)
        .await;

    if outcome.review_status.is_approved() {
        let payload = serde_json::json!({
            "tenant_id": tenant_id,
            "order_id": order_id,
            "shop_id": shipping_info.shop_id,
            "platform": shipping_info.platform.to_string(),
            "review_status": outcome.review_status.to_string(),
            "oms_status": outcome.oms_status.to_string(),
            "exception_flags": outcome.exception_flags,
        });
        let outbox_repo = OutboxRepo::new(state.db.pool());
        let _ = outbox_repo
            .enqueue(tenant_id, "ORDER_REVIEWED", &payload)
            .await;
    }

    Ok(updated)
}

pub(super) async fn review_order(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OrderReviewRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::OmsOrderReview.as_str()) {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    let action = match req.action.to_uppercase().as_str() {
        "PASS" => ReviewAction::Pass,
        "REJECT" => ReviewAction::Reject,
        "FLAG" => ReviewAction::Flag,
        _ => return bad_request("bad action").into_response(),
    };

    match apply_review_update(
        &state,
        &ctx,
        &req_ctx,
        tenant_id,
        order_id,
        action,
        req.reason.clone(),
        req.version,
        false,
        false,
    )
    .await
    {
        Ok(updated) => (StatusCode::OK, Json(updated)).into_response(),
        Err(resp) => resp,
    }
}

pub(super) async fn force_review_order(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OrderForceReviewRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::OmsOrderReview.as_str()) {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    let password = req.password.trim();
    if password.is_empty() {
        return bad_request("password required").into_response();
    }

    let setting = match fetch_tenant_setting_value(
        state.db.pool(),
        tenant_id,
        ORDER_FORCE_REVIEW_PASSWORD_KEY,
    )
    .await
    {
        Ok(v) => v,
        Err(err) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
        }
    };
    let Some(expected) = setting.and_then(|value| parse_string_setting(&value)) else {
        return bad_request("force review password not set").into_response();
    };
    if expected != password {
        return bad_request("password invalid").into_response();
    }

    match apply_review_update(
        &state,
        &ctx,
        &req_ctx,
        tenant_id,
        order_id,
        ReviewAction::Pass,
        req.reason.clone(),
        req.version,
        true,
        true,
    )
    .await
    {
        Ok(updated) => (StatusCode::OK, Json(updated)).into_response(),
        Err(resp) => resp,
    }
}

pub(super) async fn recheck_order(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OrderRecheckRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.recheck") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    let orders_repo = OrdersRepo::new(state.db.pool());
    let shipping_info = match orders_repo
        .get_order_shipping_info(tenant_id, order_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if let Err(resp) = ensure_shop_access(&ctx, shipping_info.shop_id) {
        return resp.into_response();
    }

    let state_row = match orders_repo
        .get_order_review_state(tenant_id, order_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if !state_row.review_status.is_approved() {
        return bad_request("order not approved").into_response();
    }
    if state_row
        .exception_flags
        .iter()
        .any(|flag| flag == ORDER_FROZEN_FLAG)
    {
        return bad_request("order frozen").into_response();
    }

    let review_check = match orders_repo
        .get_order_review_check(tenant_id, order_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let item_policies = match orders_repo
        .list_order_item_stock_policies(tenant_id, order_id)
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if item_policies.is_empty() {
        return bad_request("order has no items").into_response();
    }
    let has_missing_sku = item_policies.iter().any(|item| {
        item.sku_id
            .as_deref()
            .map(|value| value.trim().is_empty())
            .unwrap_or(true)
            || !item.sku_exists
    });
    if has_missing_sku {
        return bad_request("order items missing sku").into_response();
    }
    let logistics_company = review_check.logistics_company.and_then(non_empty);
    let tracking_no = review_check.tracking_no.and_then(non_empty);
    if logistics_company.is_none() || tracking_no.is_none() {
        return bad_request("order logistics missing").into_response();
    }
    let shortage = review_check.inventory_shortage
        || state_row
            .exception_flags
            .iter()
            .any(|flag| flag == "INVENTORY_SHORTAGE");
    if shortage && !item_policies.iter().all(|item| item.allow_negative_stock) {
        return bad_request("inventory shortage").into_response();
    }

    let reason = req.reason.and_then(non_empty);
    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "OMS.ORDER_RECHECK",
        "ORDER",
        Some(order_id.to_string()),
        Some(serde_json::json!({
            "review_status": state_row.review_status.to_string(),
            "oms_status": state_row.oms_status.to_string(),
            "exception_flags": state_row.exception_flags,
        })),
        Some(serde_json::json!({
            "review_status": state_row.review_status.to_string(),
            "oms_status": state_row.oms_status.to_string(),
            "exception_flags": state_row.exception_flags,
            "reason": reason.clone(),
        })),
    )
    .await;
    record_order_state_logs(
        &state,
        &req_ctx,
        OrderStateLogInput {
            tenant_id,
            order_id,
            action_code: "OMS.ORDER_RECHECK".to_string(),
            from_status: state_row.order_status,
            to_status: state_row.order_status,
            operator_id: Some(ctx.user_id),
            reason: reason.clone(),
            request_payload: serde_json::json!({
                "reason": reason.clone(),
            }),
            response_payload: serde_json::json!({
                "oms_status": state_row.oms_status.to_string(),
                "order_status": state_row.order_status.to_string(),
                "review_status": state_row.review_status.to_string(),
                "exception_flags": state_row.exception_flags,
            }),
        },
    )
    .await;

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_OMS_REVIEW_PASS.to_string(),
            ref_type: Some("ORDER".to_string()),
            ref_id: Some(order_id.to_string()),
            metadata: Some(serde_json::json!({
                "recheck": true,
                "reason": reason,
            })),
        })
        .await;

    (StatusCode::OK, Json(state_row)).into_response()
}

pub(super) async fn submit_order(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OrderSubmitRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.submit") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    let orders_repo = OrdersRepo::new(state.db.pool());
    let state_row = match orders_repo
        .get_order_review_state(tenant_id, order_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if !state_row.review_status.is_approved() {
        return bad_request("order not approved").into_response();
    }

    handle_oms_allocation(
        &state,
        &ctx,
        &req_ctx,
        &tenant_id.to_string(),
        &order_id.to_string(),
        req.carrier_code,
    )
    .await
}

pub(super) async fn cancel_order(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OrderCancelRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    let orders_repo = OrdersRepo::new(state.db.pool());
    let state_row = match orders_repo
        .get_order_review_state(tenant_id, order_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    if !state_row.review_status.is_pending() {
        return bad_request("order not pending review").into_response();
    }
    if matches!(
        state_row.oms_status,
        OmsOrderStatus::Shipped | OmsOrderStatus::Done | OmsOrderStatus::Cancelled
    ) {
        return bad_request("order already shipped").into_response();
    }

    let reason = req.reason.and_then(non_empty);
    let exception_status = OrderExceptionStatus::Closed;
    let order_status = OrderMainStatus::Cancelled;
    let updated = match orders_repo
        .update_order_review_state(
            tenant_id,
            order_id,
            OrderReviewUpdate {
                oms_status: OmsOrderStatus::Cancelled,
                order_status,
                review_status: OrderReviewStatus::Rejected,
                fulfillment_status: state_row.fulfillment_status,
                exception_status,
                exception_flags: state_row.exception_flags.clone(),
                expected_version: Some(state_row.version),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return conflict("review update conflict").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    upsert_order_review_record(
        &state,
        tenant_id,
        order_id,
        updated.review_status,
        Some(ctx.user_id),
        Some(Utc::now()),
        reason.clone(),
    )
    .await;

    let _ = orders_repo
        .update_order_cancel_info(tenant_id, order_id, reason.clone())
        .await;

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "OMS.ORDER_CANCEL",
        "ORDER",
        Some(order_id.to_string()),
        Some(serde_json::json!( {
            "oms_status": state_row.oms_status,
            "review_status": state_row.review_status,
            "exception_flags": state_row.exception_flags,
        })),
        Some(serde_json::json!( {
            "oms_status": updated.oms_status,
            "review_status": updated.review_status,
            "exception_flags": updated.exception_flags,
            "reason": reason.clone(),
        })),
    )
    .await;

    record_order_state_logs(
        &state,
        &req_ctx,
        OrderStateLogInput {
            tenant_id,
            order_id,
            action_code: "OMS.ORDER_CANCEL".to_string(),
            from_status: state_row.order_status,
            to_status: updated.order_status,
            operator_id: Some(ctx.user_id),
            reason: reason.clone(),
            request_payload: serde_json::json!({
                "reason": reason.clone(),
            }),
            response_payload: serde_json::json!({
                "oms_status": updated.oms_status.to_string(),
                "order_status": updated.order_status.to_string(),
                "review_status": updated.review_status.to_string(),
                "exception_flags": updated.exception_flags,
            }),
        },
    )
    .await;

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_OMS_ORDER_CANCEL.to_string(),
            ref_type: Some("ORDER".to_string()),
            ref_id: Some(order_id.to_string()),
            metadata: Some(serde_json::json!({
                "oms_status": updated.oms_status.clone(),
                "review_status": updated.review_status.clone(),
                "reason": reason,
            })),
        })
        .await;

    (StatusCode::OK, Json(updated)).into_response()
}

pub(super) async fn intercept_order(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OrderCancelRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    let orders_repo = OrdersRepo::new(state.db.pool());
    let state_row = match orders_repo
        .get_order_review_state(tenant_id, order_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    if matches!(
        state_row.order_status,
        OrderMainStatus::Shipped
            | OrderMainStatus::Signed
            | OrderMainStatus::Completed
            | OrderMainStatus::Cancelled
    ) {
        return bad_request("order already shipped").into_response();
    }

    let reason = req.reason.and_then(non_empty);
    let updated = match orders_repo
        .update_order_review_state(
            tenant_id,
            order_id,
            OrderReviewUpdate {
                oms_status: OmsOrderStatus::Cancelled,
                order_status: OrderMainStatus::Cancelled,
                review_status: OrderReviewStatus::Rejected,
                fulfillment_status: state_row.fulfillment_status,
                exception_status: OrderExceptionStatus::Closed,
                exception_flags: state_row.exception_flags.clone(),
                expected_version: Some(state_row.version),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return conflict("review update conflict").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    upsert_order_review_record(
        &state,
        tenant_id,
        order_id,
        updated.review_status,
        Some(ctx.user_id),
        Some(Utc::now()),
        reason.clone(),
    )
    .await;

    let _ = orders_repo
        .update_order_cancel_info(tenant_id, order_id, reason.clone())
        .await;

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "OMS.ORDER_INTERCEPT",
        "ORDER",
        Some(order_id.to_string()),
        Some(serde_json::json!( {
            "oms_status": state_row.oms_status,
            "review_status": state_row.review_status,
            "exception_flags": state_row.exception_flags,
        })),
        Some(serde_json::json!( {
            "oms_status": updated.oms_status,
            "review_status": updated.review_status,
            "exception_flags": updated.exception_flags,
            "reason": reason.clone(),
        })),
    )
    .await;

    record_order_state_logs(
        &state,
        &req_ctx,
        OrderStateLogInput {
            tenant_id,
            order_id,
            action_code: "OMS.ORDER_INTERCEPT".to_string(),
            from_status: state_row.order_status,
            to_status: updated.order_status,
            operator_id: Some(ctx.user_id),
            reason: reason.clone(),
            request_payload: serde_json::json!({
                "reason": reason.clone(),
            }),
            response_payload: serde_json::json!({
                "oms_status": updated.oms_status.to_string(),
                "order_status": updated.order_status.to_string(),
                "review_status": updated.review_status.to_string(),
                "exception_flags": updated.exception_flags,
            }),
        },
    )
    .await;

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_OMS_ORDER_CANCEL.to_string(),
            ref_type: Some("ORDER".to_string()),
            ref_id: Some(order_id.to_string()),
            metadata: Some(serde_json::json!({
                "oms_status": updated.oms_status.clone(),
                "review_status": updated.review_status.clone(),
                "reason": reason,
            })),
        })
        .await;

    (StatusCode::OK, Json(updated)).into_response()
}

pub(super) async fn rollback_order_to_review(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OrderRollbackReviewRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    let orders_repo = OrdersRepo::new(state.db.pool());
    let state_row = match orders_repo
        .get_order_review_state(tenant_id, order_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if let Err(err) = ensure_order_review_reopen_allowed(
        state_row.oms_status,
        state_row.order_status,
        state_row.fulfillment_status,
    ) {
        return bad_request(err).into_response();
    }

    let inventory_enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::InventoryWrite).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !inventory_enabled {
        return feature_flag_forbidden(FeatureFlagKey::InventoryWrite).into_response();
    }

    let mut next_flags = state_row.exception_flags.clone();
    next_flags.retain(|flag| flag != "REVIEW_REJECTED" && flag != "REVIEW_FLAGGED");

    let fulfillment_repo =
        FulfillmentRepo::new(state.db.pool()).with_actor(ctx.fulfillment_actor(&req_ctx));
    let release_result = match fulfillment_repo
        .release_order_reservations_keep_order(tenant_id, order_id)
        .await
    {
        Ok(v) => v,
        Err(e) => return crate::inventory_views::inventory_error_response(e).into_response(),
    };

    let wms_repo = WmsRepo::new(state.db.pool());
    let cancelled_outbounds = match wms_repo
        .cancel_outbounds_for_order(tenant_id, order_id)
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let shipments = match fulfillment_repo
        .list_oms_shipments(
            tenant_id,
            OmsShipmentListFilters {
                order_id: Some(uuid::Uuid::from(order_id)),
                status: None,
                warehouse_id: None,
                shipment_types: vec![
                    SHIPMENT_TYPE_OMS.to_string(),
                    SHIPMENT_TYPE_EXCHANGE.to_string(),
                    SHIPMENT_TYPE_RESEND.to_string(),
                ],
                limit: 200,
                offset: 0,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let mut released_waybills = 0_i64;
    if !shipments.items.is_empty() {
        let waybills_repo = WaybillsRepo::new(state.db.pool());
        for shipment in shipments.items {
            match waybills_repo
                .release_by_shipment(tenant_id, shipment.shipment_id)
                .await
            {
                Ok(Some(_)) => released_waybills += 1,
                Ok(None) => {}
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
                }
            };
        }
    }

    let next_exception_status = if next_flags.is_empty() {
        OrderExceptionStatus::Released
    } else {
        derive_exception_status(&next_flags)
    };
    let next_order_status = derive_order_status(
        OmsOrderStatus::ReviewPending,
        OrderReviewStatus::RolledBack,
        OrderFulfillmentStatus::NotAllocated,
        next_exception_status,
    );

    let updated = match orders_repo
        .update_order_review_state(
            tenant_id,
            order_id,
            OrderReviewUpdate {
                oms_status: OmsOrderStatus::ReviewPending,
                order_status: next_order_status,
                review_status: OrderReviewStatus::RolledBack,
                fulfillment_status: OrderFulfillmentStatus::NotAllocated,
                exception_status: next_exception_status,
                exception_flags: next_flags.clone(),
                expected_version: Some(state_row.version),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => match orders_repo
            .get_order_review_state(tenant_id, order_id)
            .await
        {
            Ok(Some(current))
                if current.oms_status == OmsOrderStatus::ReviewPending
                    && current.review_status == OrderReviewStatus::RolledBack =>
            {
                current
            }
            Ok(_) => return conflict("review update conflict").into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        },
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let reason = req.reason.and_then(non_empty);
    upsert_order_review_record(
        &state,
        tenant_id,
        order_id,
        updated.review_status,
        None,
        None,
        reason.clone(),
    )
    .await;

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "OMS.ORDER_ROLLBACK_REVIEW",
        "ORDER",
        Some(order_id.to_string()),
        Some(serde_json::json!({
            "review_status": state_row.review_status,
            "exception_flags": state_row.exception_flags,
        })),
        Some(serde_json::json!({
            "review_status": updated.review_status,
            "exception_flags": updated.exception_flags,
            "reason": reason,
            "released_reservations": release_result.released,
            "cancelled_outbounds": cancelled_outbounds.len(),
            "released_waybills": released_waybills,
        })),
    )
    .await;

    record_order_state_logs(
        &state,
        &req_ctx,
        OrderStateLogInput {
            tenant_id,
            order_id,
            action_code: "OMS.ORDER_ROLLBACK_REVIEW".to_string(),
            from_status: state_row.order_status,
            to_status: updated.order_status,
            operator_id: Some(ctx.user_id),
            reason: reason.clone(),
            request_payload: serde_json::json!({
                "reason": reason.clone(),
                "released_reservations": release_result.released,
                "cancelled_outbounds": cancelled_outbounds.len(),
                "released_waybills": released_waybills,
            }),
            response_payload: serde_json::json!({
                "oms_status": updated.oms_status.to_string(),
                "order_status": updated.order_status.to_string(),
                "review_status": updated.review_status.to_string(),
                "exception_flags": updated.exception_flags,
            }),
        },
    )
    .await;

    (StatusCode::OK, Json(updated)).into_response()
}

pub(super) async fn requeue_order_review(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
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
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    let orders_repo = OrdersRepo::new(state.db.pool());
    let state_row = match orders_repo
        .get_order_review_state(tenant_id, order_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if let Err(err) = ensure_order_review_reopen_allowed(
        state_row.oms_status,
        state_row.order_status,
        state_row.fulfillment_status,
    ) {
        return bad_request(err).into_response();
    }

    let updated_items = match orders_repo.rematch_order_items(tenant_id, order_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let mapping_missing = match orders_repo.has_mapping_missing(tenant_id, order_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let mut next_flags = state_row.exception_flags.clone();
    next_flags.retain(|flag| flag != "REVIEW_REJECTED" && flag != "REVIEW_FLAGGED");
    if mapping_missing {
        if !next_flags.iter().any(|flag| flag == "MAPPING_MISSING") {
            next_flags.push("MAPPING_MISSING".to_string());
        }
    } else {
        next_flags.retain(|flag| flag != "MAPPING_MISSING");
    }

    let next_exception_status = derive_exception_status(&next_flags);
    let next_order_status = derive_order_status(
        OmsOrderStatus::ReviewPending,
        OrderReviewStatus::ManualReview,
        state_row.fulfillment_status,
        next_exception_status,
    );
    let updated = match orders_repo
        .update_order_review_state(
            tenant_id,
            order_id,
            OrderReviewUpdate {
                oms_status: OmsOrderStatus::ReviewPending,
                order_status: next_order_status,
                review_status: OrderReviewStatus::ManualReview,
                fulfillment_status: state_row.fulfillment_status,
                exception_status: next_exception_status,
                exception_flags: next_flags.clone(),
                expected_version: Some(state_row.version),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return conflict("review update conflict").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    upsert_order_review_record(
        &state,
        tenant_id,
        order_id,
        updated.review_status,
        None,
        None,
        None,
    )
    .await;

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "OMS.ORDER_REQUEUE_REVIEW",
        "ORDER",
        Some(order_id.to_string()),
        Some(serde_json::json!({
            "review_status": state_row.review_status,
            "exception_flags": state_row.exception_flags,
        })),
        Some(serde_json::json!({
            "review_status": updated.review_status,
            "exception_flags": updated.exception_flags,
            "mapping_missing": mapping_missing,
            "updated_items": updated_items,
        })),
    )
    .await;

    record_order_state_logs(
        &state,
        &req_ctx,
        OrderStateLogInput {
            tenant_id,
            order_id,
            action_code: "OMS.ORDER_REQUEUE_REVIEW".to_string(),
            from_status: state_row.order_status,
            to_status: updated.order_status,
            operator_id: Some(ctx.user_id),
            reason: None,
            request_payload: serde_json::json!({
                "mapping_missing": mapping_missing,
                "updated_items": updated_items,
            }),
            response_payload: serde_json::json!({
                "oms_status": updated.oms_status.to_string(),
                "order_status": updated.order_status.to_string(),
                "review_status": updated.review_status.to_string(),
                "exception_flags": updated.exception_flags,
            }),
        },
    )
    .await;

    (StatusCode::OK, Json(updated)).into_response()
}

pub(super) async fn export_order_review_hits(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OrderReviewHitsExportRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "exports.write") {
        return resp.into_response();
    }
    if req.order_ids.is_empty() {
        return bad_request("order_ids required").into_response();
    }

    let repo = ExportJobsRepo::new(state.db.pool());
    let job = match repo
        .create_job(ExportJobCreateInput {
            tenant_id,
            job_type: "ORDER_REVIEW_HITS_EXPORT".to_string(),
            params_json: serde_json::json!({ "order_ids": req.order_ids }),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let orders_repo = OrdersRepo::new(state.db.pool());
    let mut rows: Vec<Vec<String>> = Vec::new();
    rows.push(vec![
        "??ID".to_string(),
        "?????".to_string(),
        "????".to_string(),
        "??".to_string(),
        "????".to_string(),
        "??".to_string(),
        "??".to_string(),
    ]);

    for raw_id in &job.params_json["order_ids"]
        .as_array()
        .cloned()
        .unwrap_or_default()
    {
        let Some(raw) = raw_id.as_str() else {
            continue;
        };
        let order_id = match OrderId::from_str(raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let detail = match orders_repo.get_order_detail(tenant_id, order_id).await {
            Ok(Some(v)) => v,
            Ok(None) => continue,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        for hit in detail.review_hits {
            rows.push(vec![
                order_id.to_string(),
                detail.order.platform_order_id.clone(),
                hit.rule_name,
                hit.action,
                if hit.hit { "??" } else { "???" }.to_string(),
                hit.message,
                hit.created_at.to_rfc3339(),
            ]);
        }
    }

    let csv = rows
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|cell| format!("\"{}\"", cell.replace('"', "\"\"")))
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let file_path = export_job_file_path(tenant_id, job.job_id);
    if let Some(parent) = file_path.parent() {
        if let Err(err) = tokio::fs::create_dir_all(parent).await {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
        }
    }
    if let Err(err) = tokio::fs::write(&file_path, format!("\u{FEFF}{csv}")).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    let output_url = Some(format!("/exports/jobs/{}/file", job.job_id));
    let updated = match repo
        .update_job(
            tenant_id,
            job.job_id,
            ExportJobUpdate {
                status: Some("DONE".to_string()),
                output_url: Some(output_url.clone()),
                progress: Some(100),
                last_error: None,
                params_json: None,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => job,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "ORDER.REVIEW_HITS_EXPORT".to_string(),
            entity_type: "EXPORT_JOB".to_string(),
            entity_id: Some(updated.job_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({ "output_url": output_url })),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(export_job_response(updated))).into_response()
}
