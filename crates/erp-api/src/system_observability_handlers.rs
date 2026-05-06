use super::*;

pub(crate) async fn list_audit_logs(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<AuditLogListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysAuditRead.as_str()) {
        return resp.into_response();
    }

    let cursor = match query.cursor.as_deref() {
        Some(raw) => match AuditLogCursor::decode(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad cursor").into_response(),
        },
        None => None,
    };
    let created_from = match query.created_from.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad created_from").into_response(),
        },
        None => None,
    };
    let created_to = match query.created_to.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad created_to").into_response(),
        },
        None => None,
    };
    let user_id = match query.user_id.as_deref() {
        Some(raw) => match UserId::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad user_id").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = AuditLogsRepo::new(state.db.pool());
    let result = match repo
        .list_logs(
            tenant_id,
            AuditLogListFilters {
                action: query.action.clone(),
                entity_type: query.entity_type.clone(),
                entity_id: query.entity_id.clone(),
                user_id,
                created_from,
                created_to,
                cursor,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = ListResponse {
        data: result.items,
        next_cursor: result.next_cursor.map(|c| c.encode()),
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn list_shipment_push_attempts(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ShipmentPushAttemptListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.read") {
        return resp.into_response();
    }

    let cursor = match query.cursor.as_deref() {
        Some(raw) => match ShipmentPushAttemptCursor::decode(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad cursor").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = ShipmentPushAttemptsRepo::new(state.db.pool());
    let result = match repo
        .list_attempts(
            tenant_id,
            ShipmentPushAttemptListFilters {
                status: query.status.clone(),
                platform_order_id: query.platform_order_id.clone(),
                cursor,
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
        .map(|attempt| ShipmentPushAttemptResponse {
            attempt_id: attempt.attempt_id.to_string(),
            outbox_id: attempt.outbox_id.to_string(),
            shipment_id: attempt.shipment_id.to_string(),
            shop_id: attempt.shop_id.to_string(),
            platform: attempt.platform,
            platform_order_id: attempt.platform_order_id,
            carrier_code: attempt.carrier_code,
            tracking_no: attempt.tracking_no,
            shipped_at: attempt.shipped_at,
            status: attempt.status,
            attempt: attempt.attempt,
            duration_ms: attempt.duration_ms,
            error_code: attempt.error_code,
            error_message: attempt.error_message,
            request_json: attempt.request_json,
            response_json: attempt.response_json,
            next_retry_at: attempt.next_retry_at,
            dead_letter_id: attempt.dead_letter_id.map(|id| id.to_string()),
            trigger: attempt.trigger,
            created_at: attempt.created_at,
        })
        .collect();

    let response = ListResponse {
        data: items,
        next_cursor: result.next_cursor.map(|c| c.encode()),
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn list_dead_letters(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<DeadLetterListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysDlqRead.as_str()) {
        return resp.into_response();
    }

    let cursor = match query.cursor.as_deref() {
        Some(raw) => match DeadLetterCursor::decode(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad cursor").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = DeadLettersRepo::new(state.db.pool());
    let result = match repo
        .list_dead_letters(
            tenant_id,
            DeadLetterListFilters {
                status: query.status.clone(),
                cursor,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = ListResponse {
        data: result.items,
        next_cursor: result.next_cursor.map(|c| c.encode()),
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn replay_dead_letter(
    State(state): State<AppState>,
    Path((tenant_id, dead_letter_id)): Path<(String, String)>,
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
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysDlqReplay.as_str()) {
        return resp.into_response();
    }
    let dead_letter_id = match uuid::Uuid::parse_str(&dead_letter_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad dead_letter_id").into_response(),
    };

    let repo = DeadLettersRepo::new(state.db.pool());
    let dead_letter = match repo.get_dead_letter(tenant_id, dead_letter_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("dead letter not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if dead_letter.status != "PENDING" {
        return conflict("dead letter not replayable").into_response();
    }

    let payload = match serde_json::to_value(DeadLetterReplayPayload {
        tenant_id,
        dead_letter_id,
    }) {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let outbox_repo = OutboxRepo::new(state.db.pool());
    let dedup_key = format!("dlq-replay:{dead_letter_id}");
    let outbox_id = match outbox_repo
        .enqueue_dedup_with_trace(
            tenant_id,
            DLQ_REPLAY_REQUESTED_EVENT,
            &payload,
            &dedup_key,
            Some(&req_ctx.trace_id),
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SYS.DLQ.REPLAY",
        "dead_letter",
        Some(dead_letter_id.to_string()),
        None,
        Some(serde_json::json!({ "outbox_id": outbox_id })),
    )
    .await;

    (StatusCode::OK, Json(OutboxRequeueResponse { outbox_id })).into_response()
}

pub(crate) async fn list_outbox_events(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<OutboxEventListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysDlqRead.as_str()) {
        return resp.into_response();
    }

    let cursor = match query.cursor.as_deref() {
        Some(raw) => match OutboxCursor::decode(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad cursor").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = OutboxRepo::new(state.db.pool());
    let result = match repo
        .list_outbox_events(
            tenant_id,
            OutboxListFilters {
                status: query.status.clone(),
                event_type: query.event_type.clone(),
                cursor,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = ListResponse {
        data: result.items,
        next_cursor: result.next_cursor.map(|c| c.encode()),
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn requeue_outbox_event(
    State(state): State<AppState>,
    Path((tenant_id, outbox_id)): Path<(String, String)>,
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
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysDlqReplay.as_str()) {
        return resp.into_response();
    }
    let outbox_id = match uuid::Uuid::parse_str(&outbox_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad outbox_id").into_response(),
    };

    let repo = OutboxRepo::new(state.db.pool());
    if let Err(e) = repo.requeue_outbox(tenant_id, outbox_id).await {
        let msg = e.to_string();
        if msg.contains("not found") {
            return not_found("outbox event not found").into_response();
        }
        if msg.contains("not requeueable") {
            return conflict("outbox status not requeueable").into_response();
        }
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SYS.OUTBOX.REQUEUE",
        "outbox_event",
        Some(outbox_id.to_string()),
        None,
        None,
    )
    .await;

    (StatusCode::OK, Json(OutboxRequeueResponse { outbox_id })).into_response()
}
