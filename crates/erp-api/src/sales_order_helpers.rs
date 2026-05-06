use super::*;

pub(crate) fn derive_delivery_status(outbound_status: &str, ship_status: &str) -> String {
    if ship_status == "SHIPPED" || outbound_status == "SHIPPED" {
        "SHIPPED".to_string()
    } else {
        "NOT_SHIPPED".to_string()
    }
}

pub(crate) fn format_sales_order_status(label: &str, value: Option<&str>) -> Option<String> {
    value.map(|value| format!("{label}:{value}"))
}

pub(crate) fn append_status_flags(flags: &[String], additions: &[&str]) -> Vec<String> {
    let mut next = flags.to_vec();
    for addition in additions {
        if !next.iter().any(|flag| flag == addition) {
            next.push(addition.to_string());
        }
    }
    next
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn record_sales_order_state_logs(
    state: &AppState,
    req_ctx: &RequestContext,
    tenant_id: TenantId,
    sales_order_id: uuid::Uuid,
    action_code: &str,
    from_status: Option<String>,
    to_status: Option<String>,
    operator_id: Option<UserId>,
    reason: Option<String>,
    request_payload: serde_json::Value,
    response_payload: serde_json::Value,
) {
    let Some(to_status) = to_status else {
        return;
    };

    let request_payload_clone = request_payload.clone();
    let response_payload_clone = response_payload.clone();
    let request_json = serde_json::json!({
        "trace_id": req_ctx.trace_id,
        "request_id": req_ctx.request_id,
        "payload": request_payload,
    });
    let response_json = serde_json::json!({
        "payload": response_payload,
    });
    let action_repo = OrderActionLogsRepo::new(state.db.pool());
    let _ = action_repo
        .insert(OrderActionLogCreate {
            tenant_id,
            order_id: OrderId::from(sales_order_id),
            action_code: action_code.to_string(),
            idempotency_key: req_ctx.request_id.clone(),
            status: Some("SUCCESS".to_string()),
            operator_id,
            request_json: Some(request_json),
            response_json: Some(response_json),
            error_message: None,
        })
        .await;

    let history_repo = OrderStatusHistoryRepo::new(state.db.pool());
    let history_reason = reason.clone();
    let payload = serde_json::json!({
        "trace_id": req_ctx.trace_id,
        "request_id": req_ctx.request_id,
        "reason": reason,
        "request": request_payload_clone,
        "response": response_payload_clone,
    });
    let _ = history_repo
        .insert(OrderStatusHistoryCreate {
            tenant_id,
            order_id: OrderId::from(sales_order_id),
            from_status,
            to_status,
            action_code: Some(action_code.to_string()),
            operator_id,
            reason: history_reason,
            payload: Some(payload),
        })
        .await;
}
