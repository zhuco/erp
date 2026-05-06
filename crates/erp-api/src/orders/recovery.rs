use super::*;

pub(super) async fn rematch_order_items(
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
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.write") {
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

    let updated_items = match orders_repo.rematch_order_items(tenant_id, order_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let mapping_missing = match orders_repo.has_mapping_missing(tenant_id, order_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let mut next_flags = state_row.exception_flags.clone();
    if mapping_missing {
        if !next_flags.iter().any(|flag| flag == "MAPPING_MISSING") {
            next_flags.push("MAPPING_MISSING".to_string());
        }
    } else {
        next_flags.retain(|flag| flag != "MAPPING_MISSING");
    }

    let next_exception_status = derive_exception_status(&next_flags);
    let next_order_status = derive_order_status(
        state_row.oms_status,
        state_row.review_status,
        state_row.fulfillment_status,
        next_exception_status,
    );
    let updated_state = if next_flags == state_row.exception_flags
        && next_exception_status == state_row.exception_status
        && next_order_status == state_row.order_status
    {
        state_row.clone()
    } else {
        match orders_repo
            .update_order_review_state(
                tenant_id,
                order_id,
                OrderReviewUpdate {
                    oms_status: state_row.oms_status,
                    order_status: next_order_status,
                    review_status: state_row.review_status,
                    fulfillment_status: state_row.fulfillment_status,
                    exception_status: next_exception_status,
                    exception_flags: next_flags.clone(),
                    expected_version: Some(state_row.version),
                },
            )
            .await
        {
            Ok(Some(v)) => v,
            Ok(None) => return conflict("rematch update conflict").into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }
    };

    let action = if mapping_missing {
        "OPEN".to_string()
    } else {
        "RESOLVE".to_string()
    };
    let target_flags = vec!["MAPPING_MISSING".to_string()];
    let target_codes = if target_flags.is_empty() {
        state_row.exception_flags.clone()
    } else {
        target_flags
    };
    if !target_codes.is_empty() {
        let (status, handled_at, handler_id) = match action.as_str() {
            "RESOLVE" => ("RESOLVED".to_string(), Some(Utc::now()), Some(ctx.user_id)),
            "REJECT" => ("REJECTED".to_string(), Some(Utc::now()), Some(ctx.user_id)),
            _ => ("OPEN".to_string(), None, None),
        };
        let updates = target_codes
            .into_iter()
            .map(|code| OrderExceptionTicketUpdate {
                reason_code: code,
                status: status.clone(),
                handler_id,
                handled_at,
            })
            .collect::<Vec<_>>();
        let _ = orders_repo
            .update_exception_ticket_status(tenant_id, order_id, &updates)
            .await;
    }

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "OMS.ORDER_ITEMS_REMATCH",
        "ORDER",
        Some(order_id.to_string()),
        Some(serde_json::json!({
            "exception_flags": state_row.exception_flags,
        })),
        Some(serde_json::json!({
            "exception_flags": updated_state.exception_flags,
            "mapping_missing": mapping_missing,
            "updated_items": updated_items,
        })),
    )
    .await;

    let detail = match orders_repo.get_order_detail(tenant_id, order_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(detail)).into_response()
}

pub(super) async fn return_order_to_download(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OrderReturnToDownloadRequest>,
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

    let reason = req.reason.and_then(non_empty);
    let outcome = match apply_review_decision(
        state_row.oms_status,
        ReviewAction::Reject,
        &state_row.exception_flags,
        reason.as_deref(),
        ReviewMode::Manual,
    ) {
        Ok(v) => v,
        Err(e) => return bad_request(&e).into_response(),
    };

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

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "OMS.ORDER_RETURN_TO_DOWNLOAD",
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
        })),
    )
    .await;

    record_order_state_logs(
        &state,
        &req_ctx,
        OrderStateLogInput {
            tenant_id,
            order_id,
            action_code: "OMS.ORDER_RETURN_TO_DOWNLOAD".to_string(),
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

    (StatusCode::OK, Json(updated)).into_response()
}
