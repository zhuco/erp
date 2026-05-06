use super::*;
use erp_repo::{
    order_action_logs::{OrderActionLogCreate, OrderActionLogsRepo},
    order_reviews::{OrderReviewUpsert, OrderReviewsRepo},
    order_status_history::{OrderStatusHistoryCreate, OrderStatusHistoryRepo},
};
use std::collections::HashSet;
pub(super) fn review_record_status(review_status: OrderReviewStatus) -> &'static str {
    match review_status {
        OrderReviewStatus::AutoApproved | OrderReviewStatus::Approved => {
            ORDER_REVIEW_RECORD_APPROVED
        }
        OrderReviewStatus::Rejected => ORDER_REVIEW_RECORD_REJECTED,
        OrderReviewStatus::NotReviewed
        | OrderReviewStatus::ManualReview
        | OrderReviewStatus::RolledBack => ORDER_REVIEW_RECORD_PENDING,
    }
}

pub(super) async fn upsert_order_review_record(
    state: &AppState,
    tenant_id: TenantId,
    order_id: OrderId,
    review_status: OrderReviewStatus,
    reviewed_by: Option<UserId>,
    reviewed_at: Option<DateTime<Utc>>,
    notes: Option<String>,
) {
    let repo = OrderReviewsRepo::new(state.db.pool());
    let status = review_record_status(review_status).to_string();
    let notes = notes.unwrap_or_default();
    if let Err(err) = repo
        .upsert_review(OrderReviewUpsert {
            tenant_id,
            order_id,
            status,
            reviewed_by,
            reviewed_at,
            notes,
        })
        .await
    {
        tracing::warn!(
            tenant_id = %tenant_id,
            order_id = %order_id,
            error = %err,
            "failed to upsert order review record"
        );
    }
}

pub(super) struct OrderStateLogInput {
    pub(super) tenant_id: TenantId,
    pub(super) order_id: OrderId,
    pub(super) action_code: String,
    pub(super) from_status: OrderMainStatus,
    pub(super) to_status: OrderMainStatus,
    pub(super) operator_id: Option<UserId>,
    pub(super) reason: Option<String>,
    pub(super) request_payload: serde_json::Value,
    pub(super) response_payload: serde_json::Value,
}

pub(super) async fn record_order_state_logs(
    state: &AppState,
    req_ctx: &RequestContext,
    input: OrderStateLogInput,
) {
    let OrderStateLogInput {
        tenant_id,
        order_id,
        action_code,
        from_status,
        to_status,
        operator_id,
        reason,
        request_payload,
        response_payload,
    } = input;
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
            order_id,
            action_code: action_code.clone(),
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
            order_id,
            from_status: Some(from_status.to_string()),
            to_status: to_status.to_string(),
            action_code: Some(action_code),
            operator_id,
            reason: history_reason,
            payload: Some(payload),
        })
        .await;
}

pub(super) fn trim_to_option(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(super) fn parse_optional_user_id(
    raw: Option<&str>,
    field_name: &str,
) -> Result<Option<UserId>, Response> {
    let Some(raw) = trim_to_option(raw) else {
        return Ok(None);
    };
    match UserId::from_str(&raw) {
        Ok(value) => Ok(Some(value)),
        Err(_) => Err((StatusCode::BAD_REQUEST, format!("bad {field_name}")).into_response()),
    }
}

pub(super) fn dedup_string_values(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            result.push(value);
        }
    }
    result
}

pub(super) fn json_array_to_strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item.as_str().map(str::trim))
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}
