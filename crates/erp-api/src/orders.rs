use super::*;
use erp_domain::{OrderItemProjection, OrderProjection};
use erp_repo::{
    order_action_logs::{OrderActionLogCreate, OrderActionLogsRepo},
    order_recheck_batches::{
        OmsRecheckBatchItemCreate, OmsRecheckBatchJobCreate, OmsRecheckBatchRepo,
    },
    order_status_history::{OrderStatusHistoryCreate, OrderStatusHistoryRepo},
    orders::{
        OrderExceptionTicketUpdate, OrderReviewState, OrderSplitPlanChild, OrderSplitPlanItem,
        OrdersRepo,
    },
};
use std::collections::HashSet;

mod dto;
mod exception_workbench;
mod fulfillment;
mod intake;
mod review_control;
mod settlement;
use dto::*;
use exception_workbench::*;
use intake::*;
use review_control::*;
use settlement::*;

const ORDER_FROZEN_FLAG: &str = "ORDER_FROZEN";
const ORDER_MERGED_FLAG: &str = "ORDER_MERGED";
const ORDER_SPLIT_FLAG: &str = "ORDER_SPLIT";
const ORDER_INVALID_GOODS_KEYWORDS: [&str; 4] = [
    "MAPPING_MISSING",
    "INVALID_GOODS",
    "INVALID_ITEM",
    "MISSING_SKU",
];
const ORDER_FORCE_REVIEW_PASSWORD_KEY: &str = "orders.force_review_password";
const ORDER_REVIEW_RECORD_PENDING: &str = "PENDING";
const ORDER_REVIEW_RECORD_APPROVED: &str = "APPROVED";
const ORDER_REVIEW_RECORD_REJECTED: &str = "REJECTED";
const OMS_EXCEPTION_ACTION_CLAIM: &str = "OMS.EXCEPTION_CLAIM";
const OMS_EXCEPTION_ACTION_ASSIGN: &str = "OMS.EXCEPTION_ASSIGN";
const OMS_EXCEPTION_ACTION_ESCALATE: &str = "OMS.EXCEPTION_ESCALATE";
const OMS_EXCEPTION_ACTION_HOLD: &str = "OMS.EXCEPTION_HOLD";
const OMS_EXCEPTION_ACTION_REJECT: &str = "OMS.EXCEPTION_REJECT";
const OMS_EXCEPTION_ACTION_RESOLVE: &str = "OMS.EXCEPTION_RESOLVE";
const OMS_EXCEPTION_SLA_DUE_SOON_MINUTES: i64 = 5;
const OMS_EXCEPTION_SLA_HIGH_PRIORITY_MINUTES: i64 = 15;
const OMS_EXCEPTION_SLA_MEDIUM_PRIORITY_MINUTES: i64 = 30;
const OMS_EXCEPTION_SLA_LOW_PRIORITY_MINUTES: i64 = 120;
const OMS_EXCEPTION_LOG_LIMIT: i64 = 100;
const OMS_SETTLEMENT_RULE_KEY: &str = "oms.order_settlement.rules";
const OMS_SETTLEMENT_ACTION_LOCK: &str = "OMS.SETTLEMENT_LOCK";
const OMS_SETTLEMENT_ACTION_UNLOCK: &str = "OMS.SETTLEMENT_UNLOCK";
const OMS_SETTLEMENT_ACTION_EXPORT: &str = "OMS.SETTLEMENT_EXPORT";
const OMS_RECONCILIATION_ACTION_SETTLE: &str = "OMS.RECONCILIATION_SETTLE";
const OMS_REVIEW_SETTINGS_KEY: &str = "oms.review.settings.v1";

fn review_record_status(review_status: OrderReviewStatus) -> &'static str {
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

async fn upsert_order_review_record(
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

struct OrderStateLogInput {
    tenant_id: TenantId,
    order_id: OrderId,
    action_code: String,
    from_status: OrderMainStatus,
    to_status: OrderMainStatus,
    operator_id: Option<UserId>,
    reason: Option<String>,
    request_payload: serde_json::Value,
    response_payload: serde_json::Value,
}

async fn record_order_state_logs(
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

fn trim_to_option(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn parse_optional_user_id(raw: Option<&str>, field_name: &str) -> Result<Option<UserId>, Response> {
    let Some(raw) = trim_to_option(raw) else {
        return Ok(None);
    };
    match UserId::from_str(&raw) {
        Ok(value) => Ok(Some(value)),
        Err(_) => Err((StatusCode::BAD_REQUEST, format!("bad {field_name}")).into_response()),
    }
}

fn dedup_string_values(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            result.push(value);
        }
    }
    result
}

fn json_array_to_strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item.as_str().map(str::trim))
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/tenants/:tenant_id/orders",
            get(list_orders).post(create_manual_order),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/download",
            post(download_orders),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/new",
            post(create_manual_order),
        )
        .route("/v1/tenants/:tenant_id/orders/merge", post(merge_orders))
        .route(
            "/v1/tenants/:tenant_id/orders/:order_id/split",
            post(split_order),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/:order_id",
            get(get_order_detail).put(edit_order),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/:order_id/chain",
            get(get_order_chain),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/:order_id/receiver",
            patch(update_order_receiver),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/:order_id/force-review",
            post(force_review_order),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/:order_id/review",
            post(review_order),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/:order_id/recheck",
            post(recheck_order),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/:order_id/submit",
            post(submit_order),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/:order_id/cancel",
            post(cancel_order),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/:order_id/intercept",
            post(intercept_order),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/:order_id/items/rematch",
            post(rematch_order_items),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/:order_id/return-to-download",
            post(return_order_to_download),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/:order_id/rollback-review",
            post(rollback_order_to_review),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/:order_id/review/requeue",
            post(requeue_order_review),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/:order_id/exceptions/handle",
            post(handle_order_exception),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/exceptions",
            get(list_order_exceptions).post(handle_order_exception_pool),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/exceptions/stats",
            get(get_order_exception_stats),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/exceptions/:order_id",
            get(get_order_exception_detail),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/exceptions/:order_id/claim",
            post(claim_order_exception),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/exceptions/:order_id/assign",
            post(assign_order_exception),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/exceptions/:order_id/escalate",
            post(escalate_order_exception),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/exceptions/:order_id/resolve",
            post(resolve_order_exception),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/exceptions/:order_id/hold",
            post(hold_order_exception),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/exceptions/:order_id/reject",
            post(reject_order_exception),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/settlements/rules",
            get(get_order_settlement_rule).put(update_order_settlement_rule),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/settlements",
            get(list_order_settlements),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/settlements/:order_id",
            get(get_order_settlement_detail),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/settlements/:order_id/lock",
            post(lock_order_settlement),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/settlements/:order_id/unlock",
            post(unlock_order_settlement),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/settlements/:order_id/export",
            post(export_order_settlement),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/reconciliations",
            get(list_order_reconciliations),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/reconciliations/:batch_id",
            get(get_order_reconciliation_detail),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/reconciliations/:batch_id/settle",
            post(settle_order_reconciliation),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/:order_id/tags",
            post(add_order_tags),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/auto-tags/apply",
            post(apply_auto_tags),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/review-hits/export",
            post(export_order_review_hits),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/:order_id/notes",
            post(add_order_note),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/:order_id/raw",
            get(get_order_raw),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/:order_id/ship",
            post(ship_order),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/:order_id/shipment-preprocess",
            post(preview_shipment_preprocess),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders",
            get(list_orders).post(create_manual_order),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/download",
            post(download_orders),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/new",
            post(create_manual_order),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/merge",
            post(merge_orders),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id/split",
            post(split_order),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id",
            get(get_order_detail).put(edit_order),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id/chain",
            get(get_order_chain),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id/receiver",
            patch(update_order_receiver),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id/force-review",
            post(force_review_order),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id/review",
            post(review_order),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id/recheck",
            post(recheck_order),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id/submit",
            post(submit_order),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id/submit-shipment",
            post(submit_order_shipment),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id/cancel",
            post(cancel_order),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id/intercept",
            post(intercept_order),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id/items/rematch",
            post(rematch_order_items),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id/return-to-download",
            post(return_order_to_download),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id/rollback-review",
            post(rollback_order_to_review),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id/review/requeue",
            post(requeue_order_review),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/exceptions",
            get(list_order_exceptions).post(handle_order_exception_pool),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/exceptions/stats",
            get(get_order_exception_stats),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/exceptions/:order_id",
            get(get_order_exception_detail),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/exceptions/:order_id/claim",
            post(claim_order_exception),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/exceptions/:order_id/assign",
            post(assign_order_exception),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/exceptions/:order_id/escalate",
            post(escalate_order_exception),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/exceptions/:order_id/resolve",
            post(resolve_order_exception),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/exceptions/:order_id/hold",
            post(hold_order_exception),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/exceptions/:order_id/reject",
            post(reject_order_exception),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/settlements/rules",
            get(get_order_settlement_rule).put(update_order_settlement_rule),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/settlements",
            get(list_order_settlements),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/settlements/:order_id",
            get(get_order_settlement_detail),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/settlements/:order_id/lock",
            post(lock_order_settlement),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/settlements/:order_id/unlock",
            post(unlock_order_settlement),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/settlements/:order_id/export",
            post(export_order_settlement),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/reconciliations",
            get(list_order_reconciliations),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/reconciliations/:batch_id",
            get(get_order_reconciliation_detail),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/reconciliations/:batch_id/settle",
            post(settle_order_reconciliation),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id/tags",
            post(add_order_tags),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id/notes",
            post(add_order_note),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id/allocate",
            post(allocate_oms_order),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id/resend",
            post(resend_oms_order),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id/release",
            post(release_oms_order),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id/shipments",
            get(list_oms_order_shipments).post(create_oms_shipments),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id/shipment-preprocess",
            post(preview_shipment_preprocess),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/shipments",
            get(list_oms_shipments),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/shipments/:shipment_id",
            get(get_oms_shipment),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/shipments/:shipment_id/ship",
            post(ship_oms_shipment),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/shipments/:shipment_id/cancel-request",
            post(request_oms_shipment_cancel),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/review-rules",
            get(list_review_rules).post(create_review_rule),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/review-rules/:rule_id",
            get(get_review_rule)
                .put(update_review_rule)
                .delete(delete_review_rule),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/review-settings",
            get(get_review_settings).put(update_review_settings),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/recheck-queue",
            get(list_recheck_queue),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/recheck-queue/batch-review",
            post(batch_review_recheck_queue),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/exception-reasons",
            get(list_exception_reasons),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/customer-blacklist",
            get(list_customer_blacklist).post(create_customer_blacklist),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/customer-blacklist/:blacklist_id",
            delete(delete_customer_blacklist),
        )
}

async fn list_orders(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<OmsOrderListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }

    let parse_csv = |raw: &str| {
        raw.split(',')
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
    };
    let platform_raw = query.platform.as_deref().and_then(non_empty);
    let (platform, platforms) = match platform_raw.as_deref() {
        Some(raw) => {
            let values = parse_csv(raw);
            if values.len() > 1 {
                let mut parsed = Vec::with_capacity(values.len());
                for value in values {
                    match Platform::from_str(value.as_str()) {
                        Ok(parsed_value) => parsed.push(parsed_value),
                        Err(_) => return bad_request("bad platform").into_response(),
                    }
                }
                (None, Some(parsed))
            } else {
                match Platform::from_str(values[0].as_str()) {
                    Ok(parsed_value) => (Some(parsed_value), None),
                    Err(_) => return bad_request("bad platform").into_response(),
                }
            }
        }
        None => (None, None),
    };
    let shop_id_raw = query.shop_id.as_deref().and_then(non_empty);
    let (shop_id, shop_ids_filter) = match shop_id_raw.as_deref() {
        Some(raw) => {
            let values = parse_csv(raw);
            if values.len() > 1 {
                let mut parsed = Vec::with_capacity(values.len());
                for value in values {
                    match ShopId::from_str(value.as_str()) {
                        Ok(parsed_value) => {
                            if let Err(resp) = ensure_shop_access(&ctx, parsed_value) {
                                return resp.into_response();
                            }
                            parsed.push(parsed_value);
                        }
                        Err(_) => return bad_request("bad shop_id").into_response(),
                    }
                }
                (None, Some(parsed))
            } else {
                match ShopId::from_str(values[0].as_str()) {
                    Ok(parsed_value) => {
                        if let Err(resp) = ensure_shop_access(&ctx, parsed_value) {
                            return resp.into_response();
                        }
                        (Some(parsed_value), None)
                    }
                    Err(_) => return bad_request("bad shop_id").into_response(),
                }
            }
        }
        None => (None, None),
    };
    let oms_status_raw = query.oms_status.as_deref().and_then(non_empty);
    let (oms_status, oms_statuses) = match oms_status_raw.as_deref() {
        Some(raw) => {
            let values = parse_csv(raw);
            if values.len() > 1 {
                let mut parsed = Vec::with_capacity(values.len());
                for value in values {
                    match OmsOrderStatus::from_str(value.as_str()) {
                        Ok(parsed_value) => parsed.push(parsed_value),
                        Err(_) => return bad_request("bad oms_status").into_response(),
                    }
                }
                (None, Some(parsed))
            } else {
                match OmsOrderStatus::from_str(values[0].as_str()) {
                    Ok(parsed_value) => (Some(parsed_value), None),
                    Err(_) => return bad_request("bad oms_status").into_response(),
                }
            }
        }
        None => (None, None),
    };
    let order_status = match query.order_status.as_deref() {
        Some(raw) => match OrderMainStatus::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad order_status").into_response(),
        },
        None => None,
    };
    let review_status = match query.review_status.as_deref() {
        Some(raw) => match OrderReviewStatus::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad review_status").into_response(),
        },
        None => None,
    };
    let fulfillment_status = match query.fulfillment_status.as_deref() {
        Some(raw) => match OrderFulfillmentStatus::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad fulfillment_status").into_response(),
        },
        None => None,
    };
    let exception_status = match query.exception_status.as_deref() {
        Some(raw) => match OrderExceptionStatus::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad exception_status").into_response(),
        },
        None => None,
    };
    let payment_status = match query.payment_status.as_deref() {
        Some(raw) => match OrderPaymentStatus::from_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad payment_status").into_response(),
        },
        None => None,
    };
    let exception_flags = query
        .exception_flags
        .as_deref()
        .map(|raw| {
            raw.split(',')
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    let tags = query
        .tags
        .as_deref()
        .map(|raw| {
            raw.split(',')
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    if !tags.is_empty() {
        let enabled =
            match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::TagSystem).await {
                Ok(v) => v,
                Err(resp) => return resp.into_response(),
            };
        if !enabled {
            return feature_flag_forbidden(FeatureFlagKey::TagSystem).into_response();
        }
    }

    let parse_dt = |raw: &str, label: &str| -> Result<DateTime<Utc>, Response> {
        DateTime::parse_from_rfc3339(raw)
            .map(|value| value.with_timezone(&Utc))
            .map_err(|_| bad_request(&format!("bad {label}")).into_response())
    };
    let created_from = match query.created_from.as_deref() {
        Some(raw) => match parse_dt(raw, "created_from") {
            Ok(v) => Some(v),
            Err(resp) => return resp.into_response(),
        },
        None => None,
    };
    let created_to = match query.created_to.as_deref() {
        Some(raw) => match parse_dt(raw, "created_to") {
            Ok(v) => Some(v),
            Err(resp) => return resp.into_response(),
        },
        None => None,
    };
    let updated_from = match query.updated_from.as_deref() {
        Some(raw) => match parse_dt(raw, "updated_from") {
            Ok(v) => Some(v),
            Err(resp) => return resp.into_response(),
        },
        None => None,
    };
    let updated_to = match query.updated_to.as_deref() {
        Some(raw) => match parse_dt(raw, "updated_to") {
            Ok(v) => Some(v),
            Err(resp) => return resp.into_response(),
        },
        None => None,
    };
    let parse_i64 = |raw: &str, label: &str| -> Result<i64, Response> {
        let value = raw
            .parse::<i64>()
            .map_err(|_| bad_request(&format!("bad {label}")).into_response())?;
        if value < 0 {
            return Err(bad_request(&format!("{label} must be non-negative")).into_response());
        }
        Ok(value)
    };
    let parse_f64 = |raw: &str, label: &str| -> Result<f64, Response> {
        let value = raw
            .parse::<f64>()
            .map_err(|_| bad_request(&format!("bad {label}")).into_response())?;
        if value < 0.0 {
            return Err(bad_request(&format!("{label} must be non-negative")).into_response());
        }
        Ok(value)
    };
    let ship_warehouse_raw = query.ship_warehouse_id.as_deref().and_then(non_empty);
    let (ship_warehouse_id, ship_warehouse_ids) = match ship_warehouse_raw.as_deref() {
        Some(raw) => {
            let values = parse_csv(raw);
            if values.len() > 1 {
                let mut parsed = Vec::with_capacity(values.len());
                for value in values {
                    match uuid::Uuid::parse_str(value.as_str()) {
                        Ok(parsed_value) => parsed.push(parsed_value),
                        Err(_) => return bad_request("bad ship_warehouse_id").into_response(),
                    }
                }
                (None, Some(parsed))
            } else {
                match uuid::Uuid::parse_str(values[0].as_str()) {
                    Ok(parsed_value) => (Some(parsed_value), None),
                    Err(_) => return bad_request("bad ship_warehouse_id").into_response(),
                }
            }
        }
        None => (None, None),
    };
    let platform_order_id = query.platform_order_id.as_deref().and_then(non_empty);
    let item_keyword = query.item_keyword.as_deref().and_then(non_empty);
    let total_amount_cent = match query.total_amount_cent.as_deref().and_then(non_empty) {
        Some(raw) => match parse_i64(raw.as_str(), "total_amount_cent") {
            Ok(value) => Some(value),
            Err(resp) => return resp.into_response(),
        },
        None => None,
    };
    let goods_qty = match query.goods_qty.as_deref().and_then(non_empty) {
        Some(raw) => match parse_i64(raw.as_str(), "goods_qty") {
            Ok(value) => Some(value),
            Err(resp) => return resp.into_response(),
        },
        None => None,
    };
    let customer_account = query.customer_account.as_deref().and_then(non_empty);
    let receiver_name = query.receiver_name.as_deref().and_then(non_empty);
    let receiver_phone = query.receiver_phone.as_deref().and_then(non_empty);
    let receiver_address = query.receiver_address.as_deref().and_then(non_empty);
    let remark = query.remark.as_deref().and_then(non_empty);
    let order_source = query.order_source.as_deref().and_then(non_empty);
    let logistics_company = query.logistics_company.as_deref().and_then(non_empty);
    let estimated_postage_cent = match query.estimated_postage_cent.as_deref().and_then(non_empty) {
        Some(raw) => match parse_i64(raw.as_str(), "estimated_postage_cent") {
            Ok(value) => Some(value),
            Err(resp) => return resp.into_response(),
        },
        None => None,
    };
    let estimated_profit_cent = match query.estimated_profit_cent.as_deref().and_then(non_empty) {
        Some(raw) => match parse_i64(raw.as_str(), "estimated_profit_cent") {
            Ok(value) => Some(value),
            Err(resp) => return resp.into_response(),
        },
        None => None,
    };
    let estimated_profit_rate = match query.estimated_profit_rate.as_deref().and_then(non_empty) {
        Some(raw) => match parse_f64(raw.as_str(), "estimated_profit_rate") {
            Ok(value) => Some(value),
            Err(resp) => return resp.into_response(),
        },
        None => None,
    };

    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let filters = OmsOrderListFilters {
        platform,
        platforms,
        shop_id,
        shop_ids: shop_scope_filter(&ctx),
        shop_ids_filter,
        oms_status,
        oms_statuses,
        order_status,
        review_status,
        fulfillment_status,
        exception_status,
        payment_status,
        exception_flags,
        tags,
        tagged: query.tagged,
        created_from,
        created_to,
        updated_from,
        updated_to,
        ship_warehouse_id,
        ship_warehouse_ids,
        platform_order_id,
        item_keyword,
        total_amount_cent,
        goods_qty,
        customer_account,
        receiver_name,
        receiver_phone,
        receiver_address,
        remark,
        order_source,
        logistics_company,
        estimated_postage_cent,
        estimated_profit_cent,
        estimated_profit_rate,
        keyword: query.keyword,
        offset,
        limit,
    };

    let orders_repo = OrdersRepo::new(state.db.pool());
    match orders_repo.list_orders(tenant_id, filters).await {
        Ok(result) => (StatusCode::OK, Json(result)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    }
}

async fn submit_order_shipment(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OmsSubmitShipmentRequest>,
) -> impl IntoResponse {
    fulfillment::submit_order_shipment(
        State(state),
        Path((tenant_id, order_id)),
        Extension(ctx),
        Extension(req_ctx),
        Json(req),
    )
    .await
}

async fn get_order_detail(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    let orders_repo = OrdersRepo::new(state.db.pool());
    let detail = match orders_repo.get_order_detail(tenant_id, order_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if let Err(resp) = ensure_shop_access(&ctx, detail.order.shop_id) {
        return resp.into_response();
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let audit_logs = match audit_repo
        .list_logs(
            tenant_id,
            erp_repo::audit_logs::AuditLogListFilters {
                action: None,
                entity_type: Some("ORDER".to_string()),
                entity_id: Some(order_id.to_string()),
                user_id: None,
                created_from: None,
                created_to: None,
                cursor: None,
                limit: 50,
            },
        )
        .await
    {
        Ok(result) => result.items,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = OmsOrderDetailResponse {
        order: detail.order,
        items: detail.items,
        review_hits: detail.review_hits,
        tags: detail.tags,
        notes: detail.notes,
        audit_logs,
    };

    (StatusCode::OK, Json(response)).into_response()
}

async fn get_order_chain(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.read") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "aftersales.read") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.inbounds.read") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    let orders_repo = OrdersRepo::new(state.db.pool());
    let detail = match orders_repo.get_order_detail(tenant_id, order_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if let Err(resp) = ensure_shop_access(&ctx, detail.order.shop_id) {
        return resp.into_response();
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let audit_logs = match audit_repo
        .list_logs(
            tenant_id,
            erp_repo::audit_logs::AuditLogListFilters {
                action: None,
                entity_type: Some("ORDER".to_string()),
                entity_id: Some(order_id.to_string()),
                user_id: None,
                created_from: None,
                created_to: None,
                cursor: None,
                limit: 100,
            },
        )
        .await
    {
        Ok(result) => result.items,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let order_response = OmsOrderDetailResponse {
        order: detail.order,
        items: detail.items,
        review_hits: detail.review_hits,
        tags: detail.tags,
        notes: detail.notes,
        audit_logs,
    };

    let fulfillment_repo = FulfillmentRepo::new(state.db.pool());
    let shipment_result = match fulfillment_repo
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
    let mut shipments = Vec::new();
    for shipment in shipment_result.items {
        let shipment_items =
            match load_oms_shipment_items(state.db.pool(), tenant_id, shipment.shipment_id).await {
                Ok(v) => v,
                Err(resp) => return resp,
            };
        shipments.push(oms_shipment_summary(
            uuid::Uuid::from(tenant_id),
            shipment,
            Some(shipment_items),
        ));
    }

    let after_sales_repo = AfterSalesRepo::new(state.db.pool());
    let after_sales_list = match after_sales_repo
        .list_after_sales(
            tenant_id,
            AfterSalesListFilters {
                status: None,
                after_sales_type: None,
                order_id: Some(order_id),
                created_from: None,
                created_to: None,
                offset: 0,
                limit: 200,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let mut after_sales = Vec::new();
    let mut after_sales_ids = Vec::new();
    for entry in after_sales_list.items {
        let detail = match after_sales_repo.get_detail(tenant_id, entry.id).await {
            Ok(Some(v)) => v,
            Ok(None) => continue,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        let audit_logs = match load_after_sales_audit_logs(
            state.db.pool(),
            tenant_id,
            detail.after_sales.id,
            detail.refund_doc.as_ref().map(|doc| doc.id),
            100,
        )
        .await
        {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        after_sales_ids.push(detail.after_sales.id);
        after_sales.push(after_sales_detail_view(detail, audit_logs));
    }

    let inbounds_repo = InboundsRepo::new(state.db.pool());
    let mut inbounds = Vec::new();
    let mut seen_doc_ids = HashSet::new();
    for after_sales_id in after_sales_ids {
        let docs = match inbounds_repo
            .list_docs(
                tenant_id,
                InboundDocListFilters {
                    status: None,
                    inbound_status: None,
                    receipt_status: None,
                    inbound_qc_status: None,
                    putaway_status: None,
                    doc_type: None,
                    warehouse_id: None,
                    doc_no: None,
                    doc_no_keyword: None,
                    ref_type: Some("AFTER_SALES".to_string()),
                    ref_id: Some(after_sales_id.to_string()),
                    supplier_id: None,
                    sku_id: None,
                    created_from: None,
                    created_to: None,
                    offset: 0,
                    limit: 200,
                },
            )
            .await
        {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        for doc in docs.items {
            if seen_doc_ids.insert(doc.doc_id) {
                inbounds.push(inbound_doc_summary_response(doc));
            }
        }
    }

    inbounds.sort_by(|a, b| {
        b.updated_at
            .cmp(&a.updated_at)
            .then_with(|| b.doc_id.cmp(&a.doc_id))
    });

    let response = OmsOrderChainResponse {
        order: order_response,
        shipments,
        after_sales,
        inbounds,
    };
    (StatusCode::OK, Json(response)).into_response()
}

async fn update_order_receiver(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OrderReceiverUpdateRequest>,
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
    let receiver_name = req.receiver_name.trim();
    if receiver_name.is_empty() {
        return bad_request("receiver_name required").into_response();
    }
    let receiver_phone = req.receiver_phone.trim();
    if receiver_phone.is_empty() {
        return bad_request("receiver_phone required").into_response();
    }
    let receiver_address = req.receiver_address.trim();
    if receiver_address.is_empty() {
        return bad_request("receiver_address required").into_response();
    }

    let orders_repo = OrdersRepo::new(state.db.pool());
    let detail = match orders_repo.get_order_detail(tenant_id, order_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if let Err(resp) = ensure_shop_access(&ctx, detail.order.shop_id) {
        return resp.into_response();
    }

    if matches!(
        detail.order.oms_status,
        OmsOrderStatus::Shipped | OmsOrderStatus::Done | OmsOrderStatus::Cancelled
    ) {
        return bad_request("order already shipped").into_response();
    }
    if detail.order.review_status.is_approved() {
        return bad_request("reviewed order only allows logistics update").into_response();
    }

    let before_json = serde_json::json!({
        "receiver_name": detail.order.receiver_name,
        "receiver_phone": detail.order.receiver_phone,
        "receiver_address": detail.order.receiver_address,
    });

    let updated = match orders_repo
        .update_order_receiver(
            tenant_id,
            order_id,
            OrderReceiverUpdate {
                receiver_name: receiver_name.to_string(),
                receiver_phone: receiver_phone.to_string(),
                receiver_address: receiver_address.to_string(),
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    if !updated {
        return not_found("order not found").into_response();
    }

    let updated_detail = match orders_repo.get_order_detail(tenant_id, order_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "OMS.ORDER_RECEIVER_UPDATE".to_string(),
            entity_type: "ORDER".to_string(),
            entity_id: Some(order_id.to_string()),
            before_json: Some(before_json),
            after_json: Some(serde_json::json!({
                "receiver_name": receiver_name,
                "receiver_phone": receiver_phone,
                "receiver_address": receiver_address,
            })),
            ip: None,
            ua: None,
        })
        .await;

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: "OMS.ORDER_RECEIVER_UPDATE".to_string(),
            ref_type: Some("ORDER".to_string()),
            ref_id: Some(order_id.to_string()),
            metadata: Some(serde_json::json!({
                "receiver_name": receiver_name,
                "receiver_phone": receiver_phone,
                "receiver_address": receiver_address,
            })),
        })
        .await;

    let audit_logs = match audit_repo
        .list_logs(
            tenant_id,
            erp_repo::audit_logs::AuditLogListFilters {
                action: None,
                entity_type: Some("ORDER".to_string()),
                entity_id: Some(order_id.to_string()),
                user_id: None,
                created_from: None,
                created_to: None,
                cursor: None,
                limit: 50,
            },
        )
        .await
    {
        Ok(result) => result.items,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = OmsOrderDetailResponse {
        order: updated_detail.order,
        items: updated_detail.items,
        review_hits: updated_detail.review_hits,
        tags: updated_detail.tags,
        notes: updated_detail.notes,
        audit_logs,
    };

    (StatusCode::OK, Json(response)).into_response()
}

async fn edit_order(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OrderEditRequest>,
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
    let before_detail = match orders_repo.get_order_detail(tenant_id, order_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if let Err(resp) = ensure_shop_access(&ctx, before_detail.order.shop_id) {
        return resp.into_response();
    }

    if matches!(
        before_detail.order.oms_status,
        OmsOrderStatus::Shipped | OmsOrderStatus::Done | OmsOrderStatus::Cancelled
    ) {
        return bad_request("order already shipped").into_response();
    }

    let receiver_name = req.receiver_name.and_then(non_empty);
    let receiver_phone = req.receiver_phone.and_then(non_empty);
    let receiver_tel = req.receiver_tel.and_then(non_empty);
    let receiver_address = req.receiver_address.and_then(non_empty);
    let logistics_company = req.logistics_company.and_then(non_empty);
    let tracking_no = req.tracking_no.and_then(non_empty);
    let order_type = req.order_type.and_then(non_empty);
    let sales_channel = req.sales_channel.and_then(non_empty);
    let order_source = req.order_source.and_then(non_empty);
    let salesperson = req.salesperson.and_then(non_empty);
    let ship_warehouse_id = match req.ship_warehouse_id.as_deref().and_then(non_empty) {
        Some(raw) => match uuid::Uuid::parse_str(raw.as_str()) {
            Ok(value) => Some(value),
            Err(_) => return bad_request("bad ship_warehouse_id").into_response(),
        },
        None => None,
    };
    let shipment_no = req.shipment_no.and_then(non_empty);
    let estimated_weight_gram = req.estimated_weight_gram;
    if let Some(value) = estimated_weight_gram {
        if value < 0 {
            return bad_request("estimated_weight_gram must be non-negative").into_response();
        }
    }
    let package_weight_gram = req.package_weight_gram;
    if let Some(value) = package_weight_gram {
        if value < 0 {
            return bad_request("package_weight_gram must be non-negative").into_response();
        }
    }
    let customer_account = req.customer_account.and_then(non_empty);
    let paid_amount_cent = req.paid_amount_cent;
    if let Some(value) = paid_amount_cent {
        if value < 0 {
            return bad_request("paid_amount_cent must be non-negative").into_response();
        }
    }
    let subsidy_amount_cent = req.subsidy_amount_cent;
    if let Some(value) = subsidy_amount_cent {
        if value < 0 {
            return bad_request("subsidy_amount_cent must be non-negative").into_response();
        }
    }
    let receivable_amount_cent = req.receivable_amount_cent;
    if let Some(value) = receivable_amount_cent {
        if value < 0 {
            return bad_request("receivable_amount_cent must be non-negative").into_response();
        }
    }
    let other_fee_cent = req.other_fee_cent;
    if let Some(value) = other_fee_cent {
        if value < 0 {
            return bad_request("other_fee_cent must be non-negative").into_response();
        }
    }
    let estimated_postage_cent = req.estimated_postage_cent;
    if let Some(value) = estimated_postage_cent {
        if value < 0 {
            return bad_request("estimated_postage_cent must be non-negative").into_response();
        }
    }
    let invoice_type = req.invoice_type.and_then(non_empty);
    let invoice_title = req.invoice_title.and_then(non_empty);
    let payment_status = req.payment_status.and_then(non_empty);
    let buyer_message = req.buyer_message.and_then(non_empty);
    let seller_remark = req.seller_remark.and_then(non_empty);
    let freeze_reason = req.freeze_reason.and_then(non_empty);
    let cancel_reason = req.cancel_reason.and_then(non_empty);
    let exception_reason = req.exception_reason.and_then(non_empty);
    let extra_remark = req.extra_remark.and_then(non_empty);

    let item_inputs = match req.items {
        Some(items) => {
            if items.is_empty() {
                return bad_request("items required").into_response();
            }
            let mut mapped = Vec::with_capacity(items.len());
            for item in items {
                if item.qty <= 0 {
                    return bad_request("item qty must be positive").into_response();
                }
                if item.price_cent < 0 {
                    return bad_request("item price_cent must be non-negative").into_response();
                }
                mapped.push(OrderEditItemInput {
                    line_id: item.line_id.and_then(non_empty),
                    sku_id: item.sku_id.and_then(non_empty),
                    title: item.title.and_then(non_empty),
                    qty: item.qty,
                    price_cent: item.price_cent,
                });
            }
            Some(mapped)
        }
        None => None,
    };

    let has_detail_change = receiver_name.is_some()
        || receiver_phone.is_some()
        || receiver_tel.is_some()
        || receiver_address.is_some()
        || order_type.is_some()
        || sales_channel.is_some()
        || order_source.is_some()
        || salesperson.is_some()
        || customer_account.is_some()
        || paid_amount_cent.is_some()
        || subsidy_amount_cent.is_some()
        || receivable_amount_cent.is_some()
        || other_fee_cent.is_some()
        || estimated_postage_cent.is_some()
        || invoice_type.is_some()
        || invoice_title.is_some()
        || payment_status.is_some()
        || buyer_message.is_some()
        || seller_remark.is_some()
        || freeze_reason.is_some()
        || cancel_reason.is_some()
        || exception_reason.is_some()
        || extra_remark.is_some()
        || item_inputs.as_ref().is_some_and(|items| !items.is_empty());
    let has_logistics_change = logistics_company.is_some()
        || tracking_no.is_some()
        || ship_warehouse_id.is_some()
        || shipment_no.is_some()
        || estimated_weight_gram.is_some()
        || package_weight_gram.is_some();

    if !has_detail_change && !has_logistics_change {
        return bad_request("no fields to update").into_response();
    }
    if before_detail.order.review_status.is_approved() && has_detail_change {
        return bad_request("reviewed order only allows logistics update").into_response();
    }

    let log_items = item_inputs.clone();
    let log_items_json = log_items.as_ref().map(|items| {
        items
            .iter()
            .map(|item| {
                serde_json::json!({
                    "line_id": item.line_id,
                    "sku_id": item.sku_id,
                    "title": item.title,
                    "qty": item.qty,
                    "price_cent": item.price_cent,
                })
            })
            .collect::<Vec<_>>()
    });
    let edit_input = OrderEditInput {
        receiver_name: receiver_name.clone(),
        receiver_phone: receiver_phone.clone(),
        receiver_tel: receiver_tel.clone(),
        receiver_address: receiver_address.clone(),
        logistics_company: logistics_company.clone(),
        tracking_no: tracking_no.clone(),
        order_type: order_type.clone(),
        sales_channel: sales_channel.clone(),
        order_source: order_source.clone(),
        salesperson: salesperson.clone(),
        ship_warehouse_id,
        shipment_no: shipment_no.clone(),
        estimated_weight_gram,
        package_weight_gram,
        customer_account: customer_account.clone(),
        paid_amount_cent,
        subsidy_amount_cent,
        receivable_amount_cent,
        other_fee_cent,
        estimated_postage_cent,
        invoice_type: invoice_type.clone(),
        invoice_title: invoice_title.clone(),
        payment_status: payment_status.clone(),
        buyer_message: buyer_message.clone(),
        seller_remark: seller_remark.clone(),
        freeze_reason: freeze_reason.clone(),
        cancel_reason: cancel_reason.clone(),
        exception_reason: exception_reason.clone(),
        extra_remark: extra_remark.clone(),
        items: item_inputs,
    };

    let updated = match orders_repo
        .update_order_edit(tenant_id, order_id, edit_input)
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    if updated.is_none() {
        return not_found("order not found").into_response();
    }

    let detail = match orders_repo.get_order_detail(tenant_id, order_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let before_items: Vec<serde_json::Value> = before_detail
        .items
        .iter()
        .map(|item| {
            serde_json::json!({
                "id": item.id,
                "platform_sku_id": item.platform_sku_id,
                "outer_sku_id": item.outer_sku_id,
                "title": item.title,
                "quantity": item.quantity,
                "price_cent": item.price_cent,
            })
        })
        .collect();

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "OMS.ORDER_EDIT".to_string(),
            entity_type: "ORDER".to_string(),
            entity_id: Some(order_id.to_string()),
            before_json: Some(serde_json::json!({
                "receiver_name": before_detail.order.receiver_name,
                "receiver_phone": before_detail.order.receiver_phone,
                "receiver_tel": before_detail.order.receiver_tel,
                "receiver_address": before_detail.order.receiver_address,
                "logistics_company": before_detail.order.logistics_company,
                "tracking_no": before_detail.order.tracking_no,
                "order_type": before_detail.order.order_type,
                "sales_channel": before_detail.order.sales_channel,
                "order_source": before_detail.order.order_source,
                "salesperson": before_detail.order.salesperson,
                "ship_warehouse_id": before_detail.order.ship_warehouse_id,
                "shipment_no": before_detail.order.shipment_no,
                "estimated_weight_gram": before_detail.order.estimated_weight_gram,
                "package_weight_gram": before_detail.order.package_weight_gram,
                "customer_account": before_detail.order.customer_account,
                "paid_amount_cent": before_detail.order.paid_amount_cent,
                "subsidy_amount_cent": before_detail.order.subsidy_amount_cent,
                "receivable_amount_cent": before_detail.order.receivable_amount_cent,
                "other_fee_cent": before_detail.order.other_fee_cent,
                "estimated_postage_cent": before_detail.order.estimated_postage_cent,
                "invoice_type": before_detail.order.invoice_type,
                "invoice_title": before_detail.order.invoice_title,
                "payment_status": before_detail.order.payment_status,
                "buyer_message": before_detail.order.buyer_message,
                "seller_remark": before_detail.order.seller_remark,
                "freeze_reason": before_detail.order.freeze_reason,
                "cancel_reason": before_detail.order.cancel_reason,
                "exception_reason": before_detail.order.exception_reason,
                "extra_remark": before_detail.order.extra_remark,
                "items": before_items,
            })),
            after_json: Some(serde_json::json!({
                "receiver_name": receiver_name,
                "receiver_phone": receiver_phone,
                "receiver_tel": receiver_tel,
                "receiver_address": receiver_address,
                "logistics_company": logistics_company,
                "tracking_no": tracking_no,
                "order_type": order_type,
                "sales_channel": sales_channel,
                "order_source": order_source,
                "salesperson": salesperson,
                "ship_warehouse_id": ship_warehouse_id,
                "shipment_no": shipment_no,
                "estimated_weight_gram": estimated_weight_gram,
                "package_weight_gram": package_weight_gram,
                "customer_account": customer_account,
                "paid_amount_cent": paid_amount_cent,
                "subsidy_amount_cent": subsidy_amount_cent,
                "receivable_amount_cent": receivable_amount_cent,
                "other_fee_cent": other_fee_cent,
                "estimated_postage_cent": estimated_postage_cent,
                "invoice_type": invoice_type,
                "invoice_title": invoice_title,
                "payment_status": payment_status,
                "buyer_message": buyer_message,
                "seller_remark": seller_remark,
                "freeze_reason": freeze_reason,
                "cancel_reason": cancel_reason,
                "exception_reason": exception_reason,
                "extra_remark": extra_remark,
                "items": log_items_json,
            })),
            ip: None,
            ua: None,
        })
        .await;

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: "OMS.ORDER_EDIT".to_string(),
            ref_type: Some("ORDER".to_string()),
            ref_id: Some(order_id.to_string()),
            metadata: Some(serde_json::json!({
                "receiver_name": receiver_name,
                "receiver_phone": receiver_phone,
                "receiver_address": receiver_address,
                "logistics_company": logistics_company,
                "tracking_no": tracking_no,
                "shipment_no": shipment_no,
                "items": log_items_json,
            })),
        })
        .await;

    let audit_logs = match audit_repo
        .list_logs(
            tenant_id,
            erp_repo::audit_logs::AuditLogListFilters {
                action: None,
                entity_type: Some("ORDER".to_string()),
                entity_id: Some(order_id.to_string()),
                user_id: None,
                created_from: None,
                created_to: None,
                cursor: None,
                limit: 50,
            },
        )
        .await
    {
        Ok(v) => v.items,
        Err(_) => vec![],
    };

    let response = OmsOrderDetailResponse {
        order: detail.order,
        items: detail.items,
        review_hits: detail.review_hits,
        tags: detail.tags,
        notes: detail.notes,
        audit_logs,
    };

    (StatusCode::OK, Json(response)).into_response()
}

#[allow(clippy::too_many_arguments)]
async fn apply_review_update(
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

async fn review_order(
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

async fn force_review_order(
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

async fn recheck_order(
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

async fn submit_order(
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

async fn cancel_order(
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

async fn intercept_order(
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

async fn merge_orders(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OrderMergeRequest>,
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

    let OrderMergeRequest {
        order_ids: raw_order_ids,
        target_order_id,
        force,
    } = req;
    let force_merge = force.unwrap_or(false);

    let mut order_ids = raw_order_ids
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    order_ids.sort();
    order_ids.dedup();
    if order_ids.len() < 2 {
        return bad_request("order_ids required").into_response();
    }

    let parsed_ids = order_ids
        .iter()
        .map(|raw| OrderId::from_str(raw))
        .collect::<Result<Vec<_>, _>>();
    let order_ids = match parsed_ids {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    if let Some(target_raw) = target_order_id.as_deref() {
        if !order_ids.iter().any(|id| id.to_string() == target_raw) {
            return bad_request("target_order_id not in order_ids").into_response();
        }
    }

    let orders_repo = OrdersRepo::new(state.db.pool());
    let mut details = Vec::with_capacity(order_ids.len());
    for order_id in &order_ids {
        let detail = match orders_repo.get_order_detail(tenant_id, *order_id).await {
            Ok(Some(v)) => v,
            Ok(None) => return not_found("order not found").into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        if let Err(resp) = ensure_shop_access(&ctx, detail.order.shop_id) {
            return resp.into_response();
        }
        details.push(detail);
    }

    let mut identity_key: Option<(String, String, String, uuid::Uuid)> = None;
    for detail in &details {
        let order = &detail.order;
        if !order.review_status.is_pending() {
            return bad_request("order review_status not pending").into_response();
        }
        if matches!(
            order.oms_status,
            OmsOrderStatus::Shipped | OmsOrderStatus::Done | OmsOrderStatus::Cancelled
        ) {
            return bad_request("order status not mergeable").into_response();
        }
        let customer_key = order
            .customer_id
            .map(|value| value.to_string())
            .or_else(|| order.customer_name.clone())
            .unwrap_or_default();
        let phone_key = order.receiver_phone.clone().unwrap_or_default();
        let address_key = order.receiver_address.clone().unwrap_or_default();
        let shop_id = uuid::Uuid::from(order.shop_id);
        if let Some((customer, phone, address, shop)) = &identity_key {
            if (*customer != customer_key || *phone != phone_key || *address != address_key)
                && !force_merge
            {
                return bad_request("merge condition mismatch").into_response();
            }
            if *shop != shop_id {
                return bad_request("orders from different shops").into_response();
            }
        } else {
            identity_key = Some((customer_key, phone_key, address_key, shop_id));
        }
    }

    let target_order_id = if let Some(target_raw) = target_order_id.as_deref() {
        OrderId::from_str(target_raw).unwrap_or(order_ids[0])
    } else {
        details
            .iter()
            .min_by_key(|detail| detail.order.order_created_at)
            .map(|detail| detail.order.id)
            .unwrap_or(order_ids[0])
    };

    let mut merged_items = Vec::new();
    for order_id in &order_ids {
        let items = match orders_repo
            .list_order_edit_items(tenant_id, *order_id)
            .await
        {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        for item in items {
            merged_items.push(OrderEditItemInput {
                line_id: None,
                sku_id: item.sku_id,
                title: item.title,
                qty: item.qty,
                price_cent: item.price_cent,
            });
        }
    }

    if merged_items.is_empty() {
        return bad_request("merge items empty").into_response();
    }

    let update_result = orders_repo
        .update_order_edit(
            tenant_id,
            target_order_id,
            OrderEditInput {
                receiver_name: None,
                receiver_phone: None,
                receiver_tel: None,
                receiver_address: None,
                logistics_company: None,
                tracking_no: None,
                order_type: None,
                sales_channel: None,
                order_source: None,
                salesperson: None,
                ship_warehouse_id: None,
                shipment_no: None,
                estimated_weight_gram: None,
                package_weight_gram: None,
                customer_account: None,
                paid_amount_cent: None,
                subsidy_amount_cent: None,
                receivable_amount_cent: None,
                other_fee_cent: None,
                estimated_postage_cent: None,
                invoice_type: None,
                invoice_title: None,
                payment_status: None,
                buyer_message: None,
                seller_remark: None,
                freeze_reason: None,
                cancel_reason: None,
                exception_reason: None,
                extra_remark: None,
                items: Some(merged_items),
            },
        )
        .await;
    match update_result {
        Ok(Some(())) => {}
        Ok(None) => return not_found("target order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    }

    let total_amount_cent = details
        .iter()
        .map(|detail| detail.order.total_amount_cent)
        .sum::<i64>();
    let sum_optional = |values: Vec<Option<i64>>| -> Option<i64> {
        let mut sum = 0;
        let mut has_value = false;
        for v in values.into_iter().flatten() {
            sum += v;
            has_value = true;
        }
        if has_value {
            Some(sum)
        } else {
            None
        }
    };
    let goods_amount_cent = sum_optional(
        details
            .iter()
            .map(|detail| detail.order.goods_amount_cent)
            .collect(),
    );
    let postage_amount_cent = sum_optional(
        details
            .iter()
            .map(|detail| detail.order.postage_amount_cent)
            .collect(),
    );
    let discount_amount_cent = sum_optional(
        details
            .iter()
            .map(|detail| detail.order.discount_amount_cent)
            .collect(),
    );
    let _ = orders_repo
        .update_order_amounts(
            tenant_id,
            target_order_id,
            total_amount_cent,
            goods_amount_cent,
            postage_amount_cent,
            discount_amount_cent,
        )
        .await;

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let mut merged_order_ids = Vec::new();
    for detail in &details {
        if detail.order.id == target_order_id {
            continue;
        }
        let state_row = match orders_repo
            .get_order_review_state(tenant_id, detail.order.id)
            .await
        {
            Ok(Some(v)) => v,
            Ok(None) => continue,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        let mut next_flags = state_row.exception_flags.clone();
        if !next_flags.iter().any(|flag| flag == ORDER_MERGED_FLAG) {
            next_flags.push(ORDER_MERGED_FLAG.to_string());
        }
        let exception_status = OrderExceptionStatus::Closed;
        let order_status = OrderMainStatus::Cancelled;
        let updated = orders_repo
            .update_order_review_state(
                tenant_id,
                detail.order.id,
                OrderReviewUpdate {
                    oms_status: OmsOrderStatus::Cancelled,
                    order_status,
                    review_status: OrderReviewStatus::Rejected,
                    fulfillment_status: state_row.fulfillment_status,
                    exception_status,
                    exception_flags: next_flags.clone(),
                    expected_version: Some(state_row.version),
                },
            )
            .await;
        match updated {
            Ok(Some(_)) => {}
            Ok(None) => return conflict("merge update conflict").into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }

        upsert_order_review_record(
            &state,
            tenant_id,
            detail.order.id,
            OrderReviewStatus::Rejected,
            Some(ctx.user_id),
            Some(Utc::now()),
            None,
        )
        .await;

        record_order_state_logs(
            &state,
            &req_ctx,
            OrderStateLogInput {
                tenant_id,
                order_id: detail.order.id,
                action_code: "OMS.ORDER_MERGE_SOURCE".to_string(),
                from_status: state_row.order_status,
                to_status: order_status,
                operator_id: Some(ctx.user_id),
                reason: None,
                request_payload: serde_json::json!({
                    "target_order_id": target_order_id.to_string(),
                    "force": force_merge,
                }),
                response_payload: serde_json::json!({
                    "oms_status": OmsOrderStatus::Cancelled.to_string(),
                    "order_status": order_status.to_string(),
                    "review_status": OrderReviewStatus::Rejected.to_string(),
                    "exception_flags": next_flags.clone(),
                }),
            },
        )
        .await;
        merged_order_ids.push(detail.order.id.to_string());

        record_audit_log(
            &state,
            &ctx,
            &req_ctx,
            "OMS.ORDER_MERGE_SOURCE",
            "ORDER",
            Some(detail.order.id.to_string()),
            Some(serde_json::json!({
                "oms_status": state_row.oms_status,
                "review_status": state_row.review_status,
                "exception_flags": state_row.exception_flags,
            })),
            Some(serde_json::json!({
                "oms_status": OmsOrderStatus::Cancelled.to_string(),
                "review_status": OrderReviewStatus::Rejected.to_string(),
                "exception_flags": next_flags,
                "target_order_id": target_order_id.to_string(),
            })),
        )
        .await;

        let _ = logs_repo
            .insert(OperationLogCreate {
                tenant_id,
                actor_type: "USER".to_string(),
                actor_user_id: Some(ctx.user_id),
                action: OP_ACTION_OMS_ORDER_MERGE_SOURCE.to_string(),
                ref_type: Some("ORDER".to_string()),
                ref_id: Some(detail.order.id.to_string()),
                metadata: Some(serde_json::json!({
                    "target_order_id": target_order_id.to_string(),
                    "force": force_merge,
                })),
            })
            .await;
    }

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "OMS.ORDER_MERGE_TARGET",
        "ORDER",
        Some(target_order_id.to_string()),
        None,
        Some(serde_json::json!({
            "merged_order_ids": merged_order_ids.clone(),
            "force": force_merge,
        })),
    )
    .await;

    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_OMS_ORDER_MERGE_TARGET.to_string(),
            ref_type: Some("ORDER".to_string()),
            ref_id: Some(target_order_id.to_string()),
            metadata: Some(serde_json::json!({
                "merged_order_ids": merged_order_ids.clone(),
                "force": force_merge,
            })),
        })
        .await;

    let response = OrderMergeResponse {
        target_order_id: target_order_id.to_string(),
        merged_order_ids,
    };
    (StatusCode::OK, Json(response)).into_response()
}

async fn split_order(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OrderSplitRequest>,
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
    let detail = match orders_repo.get_order_detail(tenant_id, order_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if let Err(resp) = ensure_shop_access(&ctx, detail.order.shop_id) {
        return resp.into_response();
    }
    if !detail.order.review_status.is_pending() {
        return bad_request("order not pending review").into_response();
    }
    if matches!(
        detail.order.oms_status,
        OmsOrderStatus::Shipped | OmsOrderStatus::Done | OmsOrderStatus::Cancelled
    ) {
        return bad_request("order already shipped").into_response();
    }
    if detail
        .order
        .exception_flags
        .iter()
        .any(|flag| flag == ORDER_SPLIT_FLAG)
    {
        return bad_request("order already split").into_response();
    }
    if detail.items.is_empty() {
        return bad_request("order has no items").into_response();
    }

    let plan = if let Some(children) = req.children {
        if children.len() < 2 {
            return bad_request("split children required").into_response();
        }
        let mut parsed = Vec::with_capacity(children.len());
        for child in children {
            if child.items.is_empty() {
                return bad_request("split child empty").into_response();
            }
            let items = child
                .items
                .into_iter()
                .map(|item| OrderSplitPlanItem {
                    line_id: item.line_id.trim().to_string(),
                    qty: item.qty,
                })
                .collect::<Vec<_>>();
            parsed.push(OrderSplitPlanChild { items });
        }
        parsed
    } else {
        let Some(split_count) = req.split_count else {
            return bad_request("split_count required").into_response();
        };
        if split_count < 2 {
            return bad_request("split_count invalid").into_response();
        }
        let split_count = split_count as usize;
        let mut children = vec![OrderSplitPlanChild { items: Vec::new() }; split_count];
        for item in &detail.items {
            let mut remaining = item.quantity;
            for (idx, child) in children.iter_mut().enumerate() {
                if remaining <= 0 {
                    break;
                }
                let slots_left = (split_count - idx) as i32;
                let qty = if idx == split_count - 1 {
                    remaining
                } else {
                    (remaining + slots_left - 1) / slots_left
                };
                if qty > 0 {
                    child.items.push(OrderSplitPlanItem {
                        line_id: item.id.clone(),
                        qty,
                    });
                    remaining -= qty;
                }
            }
        }
        children
    };

    let mut item_qty_map = std::collections::HashMap::new();
    for item in &detail.items {
        item_qty_map.insert(item.id.clone(), item.quantity);
    }
    let mut plan_qty_map: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    for child in &plan {
        if child.items.is_empty() {
            return bad_request("split child empty").into_response();
        }
        for item in &child.items {
            if item.line_id.trim().is_empty() {
                return bad_request("split item line_id required").into_response();
            }
            if item.qty <= 0 {
                return bad_request("split item qty invalid").into_response();
            }
            if !item_qty_map.contains_key(&item.line_id) {
                return bad_request("split item not found").into_response();
            }
            *plan_qty_map.entry(item.line_id.clone()).or_insert(0) += item.qty;
        }
    }
    for (line_id, qty) in &item_qty_map {
        if plan_qty_map.get(line_id) != Some(qty) {
            return bad_request("split qty mismatch").into_response();
        }
    }
    if plan_qty_map.len() != item_qty_map.len() {
        return bad_request("split plan missing items").into_response();
    }

    let split_count = plan.len() as i32;
    let split_result = match orders_repo.split_order(tenant_id, order_id, plan).await {
        Ok(v) => v,
        Err(err) => {
            let message = err.to_string();
            if message.contains("order") || message.contains("split") {
                return bad_request(&message).into_response();
            }
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
        }
    };

    let child_order_ids = split_result
        .child_orders
        .iter()
        .map(|child| child.order_id.to_string())
        .collect::<Vec<_>>();

    let mut next_flags = detail.order.exception_flags.clone();
    if !next_flags.iter().any(|flag| flag == ORDER_SPLIT_FLAG) {
        next_flags.push(ORDER_SPLIT_FLAG.to_string());
    }

    record_order_state_logs(
        &state,
        &req_ctx,
        OrderStateLogInput {
            tenant_id,
            order_id,
            action_code: "OMS.ORDER_SPLIT_SOURCE".to_string(),
            from_status: detail.order.order_status,
            to_status: OrderMainStatus::Cancelled,
            operator_id: Some(ctx.user_id),
            reason: None,
            request_payload: serde_json::json!({
                "split_count": split_count,
                "child_order_ids": child_order_ids.clone(),
            }),
            response_payload: serde_json::json!({
                "oms_status": OmsOrderStatus::Cancelled.to_string(),
                "order_status": OrderMainStatus::Cancelled.to_string(),
                "review_status": OrderReviewStatus::Rejected.to_string(),
                "exception_flags": next_flags.clone(),
            }),
        },
    )
    .await;

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "OMS.ORDER_SPLIT_SOURCE",
        "ORDER",
        Some(order_id.to_string()),
        Some(serde_json::json!({
            "oms_status": detail.order.oms_status.to_string(),
            "review_status": detail.order.review_status.to_string(),
            "exception_flags": detail.order.exception_flags,
        })),
        Some(serde_json::json!({
            "oms_status": OmsOrderStatus::Cancelled.to_string(),
            "review_status": OrderReviewStatus::Rejected.to_string(),
            "exception_flags": next_flags,
            "child_order_ids": child_order_ids.clone(),
        })),
    )
    .await;

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_OMS_ORDER_SPLIT_SOURCE.to_string(),
            ref_type: Some("ORDER".to_string()),
            ref_id: Some(order_id.to_string()),
            metadata: Some(serde_json::json!({
                "child_order_ids": child_order_ids.clone(),
                "split_count": split_count,
            })),
        })
        .await;

    for child in &split_result.child_orders {
        record_audit_log(
            &state,
            &ctx,
            &req_ctx,
            "OMS.ORDER_SPLIT_CHILD",
            "ORDER",
            Some(child.order_id.to_string()),
            None,
            Some(serde_json::json!({
                "source_order_id": order_id.to_string(),
                "items": child.items.iter().map(|item| serde_json::json!({
                    "line_id": item.line_id,
                    "sku_id": item.sku_id,
                    "title": item.title,
                    "qty": item.qty,
                })).collect::<Vec<_>>(),
            })),
        )
        .await;

        let _ = logs_repo
            .insert(OperationLogCreate {
                tenant_id,
                actor_type: "USER".to_string(),
                actor_user_id: Some(ctx.user_id),
                action: OP_ACTION_OMS_ORDER_SPLIT_CHILD.to_string(),
                ref_type: Some("ORDER".to_string()),
                ref_id: Some(child.order_id.to_string()),
                metadata: Some(serde_json::json!({
                    "source_order_id": order_id.to_string(),
                })),
            })
            .await;
    }

    let response = OrderSplitResponse {
        source_order_id: order_id.to_string(),
        split_count,
        created_at: Utc::now(),
        child_orders: split_result
            .child_orders
            .into_iter()
            .map(|child| OrderSplitChildResponse {
                order_id: child.order_id.to_string(),
                items: child
                    .items
                    .into_iter()
                    .map(|item| OrderSplitItemResponse {
                        line_id: item.line_id,
                        sku_id: item.sku_id,
                        title: item.title,
                        qty: item.qty,
                        price_cent: item.price_cent,
                    })
                    .collect(),
            })
            .collect(),
    };

    (StatusCode::OK, Json(response)).into_response()
}

async fn get_order_settlement_rule(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }

    match load_order_settlement_rule(&state, tenant_id).await {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response(),
    }
}

async fn update_order_settlement_rule(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OrderSettlementRuleUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }

    let Some(stack_mode) = normalize_order_settlement_stack_mode(&req.stack_mode) else {
        return bad_request("bad stack_mode").into_response();
    };
    if !(0..=100).contains(&req.member_discount_rate)
        || !(0..=100).contains(&req.promo_discount_rate)
        || !(0..=100).contains(&req.max_discount_rate)
    {
        return bad_request("discount rate out of range").into_response();
    }

    let repo = SettingsRepo::new(state.db.pool());
    let value_json = serde_json::json!({
        "stack_mode": stack_mode,
        "member_discount_rate": req.member_discount_rate,
        "promo_discount_rate": req.promo_discount_rate,
        "max_discount_rate": req.max_discount_rate,
    });
    match repo
        .upsert_setting(SettingUpsert {
            tenant_id,
            scope_type: "TENANT".to_string(),
            scope_id: None,
            key: OMS_SETTLEMENT_RULE_KEY.to_string(),
            value_json,
            expected_version: req.expected_version,
        })
        .await
    {
        Ok(setting) => (
            StatusCode::OK,
            Json(OrderSettlementRuleResponse {
                stack_mode,
                member_discount_rate: req.member_discount_rate,
                promo_discount_rate: req.promo_discount_rate,
                max_discount_rate: req.max_discount_rate,
                version: setting.version,
                updated_at: Some(setting.updated_at),
            }),
        )
            .into_response(),
        Err(error) => {
            if error.to_string().contains("version conflict") {
                return conflict("settlement rule version conflict").into_response();
            }
            (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response()
        }
    }
}

async fn list_order_settlements(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<OrderSettlementListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }

    let created_from = match parse_datetime_optional(query.created_from.as_deref(), "created_from")
    {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    let created_to = match parse_datetime_optional(query.created_to.as_deref(), "created_to") {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    if let (Some(from), Some(to)) = (created_from, created_to) {
        if from > to {
            return bad_request("created_from after created_to").into_response();
        }
    }

    let shop_id = match query.shop_id.as_deref().and_then(non_empty) {
        Some(value) => match ShopId::from_str(&value) {
            Ok(parsed) => {
                if let Err(resp) = ensure_shop_access(&ctx, parsed) {
                    return resp.into_response();
                }
                Some(uuid::Uuid::from(parsed))
            }
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };
    let status = match query.status.as_deref().and_then(non_empty) {
        Some(value) => Some(normalize_order_settlement_status(Some(value.as_str()))),
        None => None,
    };
    let filters = OrderSettlementFilters {
        platform: query.platform.as_deref().and_then(non_empty),
        shop_id,
        keyword: query.keyword.as_deref().and_then(non_empty),
        status,
        created_from,
        created_to,
    };
    let offset = query.offset.unwrap_or(0).max(0) as usize;
    let limit = query.limit.unwrap_or(50).clamp(1, 200) as usize;

    let rule = match load_order_settlement_rule(&state, tenant_id).await {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response()
        }
    };
    let rows = match list_order_settlement_rows(&state, tenant_id, &filters).await {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response()
        }
    };
    let accessible_rows = rows
        .into_iter()
        .filter(|row| ensure_shop_access(&ctx, ShopId(row.shop_id)).is_ok())
        .collect::<Vec<_>>();
    let mut summary = OrderSettlementListSummaryResponse {
        total_orders: 0,
        open_orders: 0,
        locked_orders: 0,
        exported_orders: 0,
        total_amount_cent: 0,
        receivable_amount_cent: 0,
    };
    for row in &accessible_rows {
        let preview = build_order_settlement_preview(row.total_amount_cent, &rule);
        summary.total_orders += 1;
        summary.total_amount_cent += row.total_amount_cent;
        summary.receivable_amount_cent += preview.receivable_amount_cent;
        match normalize_order_settlement_status(row.settlement_status.as_deref()).as_str() {
            "LOCKED" => summary.locked_orders += 1,
            "EXPORTED" => summary.exported_orders += 1,
            _ => summary.open_orders += 1,
        }
    }

    let paged_rows = accessible_rows
        .iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect::<Vec<_>>();
    let mut items = Vec::with_capacity(paged_rows.len());
    for row in paged_rows {
        let logs = match load_order_settlement_logs(&state, tenant_id, OrderId(row.order_id)).await
        {
            Ok(value) => value,
            Err(error) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response()
            }
        };
        items.push(build_order_settlement_summary_response(&row, &rule, &logs));
    }

    (
        StatusCode::OK,
        Json(OrderSettlementListResponse {
            total: accessible_rows.len() as i64,
            items,
            summary,
        }),
    )
        .into_response()
}

async fn get_order_settlement_detail(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    let Some(row) = (match load_order_settlement_row(&state, tenant_id, order_id).await {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response()
        }
    }) else {
        return not_found("order settlement not found").into_response();
    };
    if let Err(resp) = ensure_shop_access(&ctx, ShopId(row.shop_id)) {
        return resp.into_response();
    }
    let rule = match load_order_settlement_rule(&state, tenant_id).await {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response()
        }
    };
    match build_order_settlement_detail_response(&state, tenant_id, order_id, &rule).await {
        Ok(Some(value)) => (StatusCode::OK, Json(value)).into_response(),
        Ok(None) => not_found("order settlement not found").into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response(),
    }
}

async fn execute_order_settlement_action(
    state: &AppState,
    ctx: &AuthContext,
    req_ctx: &RequestContext,
    tenant_id: TenantId,
    order_id: OrderId,
    action: &str,
    payload: OrderSettlementActionRequest,
) -> Result<OrderSettlementSummaryResponse, Response> {
    let Some(row) = load_order_settlement_row(state, tenant_id, order_id)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response())?
    else {
        return Err(not_found("order settlement not found").into_response());
    };
    if let Err(resp) = ensure_shop_access(ctx, ShopId(row.shop_id)) {
        return Err(resp.into_response());
    }

    let orders_repo = OrdersRepo::new(state.db.pool());
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

    let current_status = normalize_order_settlement_status(row.settlement_status.as_deref());
    let operator_id = parse_optional_user_id(payload.operator_id.as_deref(), "operator_id")?
        .unwrap_or(ctx.user_id);
    let operator_id_string = operator_id.to_string();
    let operator_name = trim_to_option(payload.operator_name.as_deref());
    let initial_review_status = row.review_status.clone();
    let initial_exception_flags = row.exception_flags.clone();
    let now = Utc::now();
    let (next_status, locked_at, export_job_id, action_code, message) = match action {
        "LOCK" => {
            if current_status == "EXPORTED" {
                return Err(conflict("exported settlement cannot be locked").into_response());
            }
            (
                "LOCKED".to_string(),
                Some(now),
                row.export_job_id.clone(),
                OMS_SETTLEMENT_ACTION_LOCK,
                "订单结算已锁定".to_string(),
            )
        }
        "UNLOCK" => {
            if current_status != "LOCKED" {
                return Err(conflict("only locked settlement can be unlocked").into_response());
            }
            (
                "OPEN".to_string(),
                None,
                row.export_job_id.clone(),
                OMS_SETTLEMENT_ACTION_UNLOCK,
                "订单结算已解锁".to_string(),
            )
        }
        "EXPORT" => {
            if current_status != "LOCKED" {
                return Err(conflict("only locked settlement can be exported").into_response());
            }
            let export_job_id = payload
                .export_job_id
                .as_deref()
                .and_then(non_empty)
                .ok_or_else(|| bad_request("export_job_id is required").into_response())?;
            (
                "EXPORTED".to_string(),
                row.locked_at.or(Some(now)),
                Some(export_job_id),
                OMS_SETTLEMENT_ACTION_EXPORT,
                "订单结算已导出".to_string(),
            )
        }
        _ => return Err(bad_request("bad action").into_response()),
    };

    upsert_order_settlement_record(
        state,
        tenant_id,
        order_id,
        &next_status,
        locked_at,
        export_job_id.clone(),
    )
    .await
    .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response())?;
    record_order_settlement_log(
        state,
        tenant_id,
        order_id,
        action_code,
        Some(operator_id.clone()),
        operator_name.clone(),
        export_job_id.clone(),
        message,
    )
    .await;

    let request_payload = serde_json::json!({
        "action": action,
        "operator_id": operator_id_string.clone(),
        "operator_name": operator_name.clone(),
        "export_job_id": export_job_id.clone(),
    });
    let response_payload = serde_json::json!({
        "current_settlement_status": current_status,
        "next_settlement_status": next_status,
        "export_job_id": export_job_id.clone(),
    });

    record_order_state_logs(
        state,
        req_ctx,
        OrderStateLogInput {
            tenant_id,
            order_id,
            action_code: action_code.to_string(),
            from_status: state_row.order_status,
            to_status: state_row.order_status,
            operator_id: Some(operator_id),
            reason: None,
            request_payload,
            response_payload,
        },
    )
    .await;

    record_audit_log(
        state,
        ctx,
        req_ctx,
        action_code,
        "ORDER",
        Some(order_id.to_string()),
        Some(serde_json::json!({
            "settlement_status": current_status,
            "review_status": initial_review_status,
            "exception_flags": initial_exception_flags,
        })),
        Some(serde_json::json!({
            "settlement_status": next_status,
            "export_job_id": export_job_id.clone(),
            "operator_id": operator_id_string.clone(),
            "operator_name": operator_name,
        })),
    )
    .await;

    let rule = load_order_settlement_rule(state, tenant_id)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response())?;
    let row = load_order_settlement_row(state, tenant_id, order_id)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response())?
        .ok_or_else(|| not_found("order settlement not found").into_response())?;
    let logs = load_order_settlement_logs(state, tenant_id, order_id)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response())?;
    Ok(build_order_settlement_summary_response(&row, &rule, &logs))
}

async fn lock_order_settlement(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(payload): Json<OrderSettlementActionRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad order_id").into_response(),
    };
    match execute_order_settlement_action(&state, &ctx, &req_ctx, tenant_id, order_id, "LOCK", payload).await
        {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(resp) => resp,
    }
}

async fn unlock_order_settlement(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(payload): Json<OrderSettlementActionRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad order_id").into_response(),
    };
    match execute_order_settlement_action(&state, &ctx, &req_ctx, tenant_id, order_id, "UNLOCK", payload).await
    {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(resp) => resp,
    }
}

async fn export_order_settlement(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(payload): Json<OrderSettlementActionRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad order_id").into_response(),
    };
    match execute_order_settlement_action(&state, &ctx, &req_ctx, tenant_id, order_id, "EXPORT", payload).await
    {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(resp) => resp,
    }
}

async fn list_order_reconciliations(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<OrderReconciliationListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }

    let created_from = match parse_datetime_optional(query.created_from.as_deref(), "created_from")
    {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    let created_to = match parse_datetime_optional(query.created_to.as_deref(), "created_to") {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    if let (Some(from), Some(to)) = (created_from, created_to) {
        if from > to {
            return bad_request("created_from after created_to").into_response();
        }
    }
    let shop_id = match query.shop_id.as_deref().and_then(non_empty) {
        Some(value) => match ShopId::from_str(&value) {
            Ok(parsed) => {
                if let Err(resp) = ensure_shop_access(&ctx, parsed) {
                    return resp.into_response();
                }
                Some(uuid::Uuid::from(parsed))
            }
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };
    let status_filter = query
        .status
        .as_deref()
        .and_then(non_empty)
        .map(|value| value.to_uppercase());
    let filters = OrderSettlementFilters {
        platform: query.platform.as_deref().and_then(non_empty),
        shop_id,
        keyword: query.keyword.as_deref().and_then(non_empty),
        status: None,
        created_from,
        created_to,
    };
    let offset = query.offset.unwrap_or(0).max(0) as usize;
    let limit = query.limit.unwrap_or(50).clamp(1, 200) as usize;

    let rule = match load_order_settlement_rule(&state, tenant_id).await {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response()
        }
    };
    let rows = match list_order_settlement_rows(&state, tenant_id, &filters).await {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response()
        }
    };
    let mut grouped: HashMap<(String, uuid::Uuid), Vec<OrderSettlementRow>> = HashMap::new();
    for row in rows {
        if ensure_shop_access(&ctx, ShopId(row.shop_id)).is_err() {
            continue;
        }
        grouped
            .entry((row.platform.clone(), row.shop_id))
            .or_default()
            .push(row);
    }

    let mut batches = Vec::new();
    for ((platform, shop_id), group_rows) in grouped {
        let period_start = filters
            .created_from
            .map(|value| value.date_naive())
            .unwrap_or_else(|| {
                group_rows
                    .iter()
                    .map(|row| row.order_created_at.date_naive())
                    .min()
                    .unwrap_or_else(|| Utc::now().date_naive())
            });
        let period_end = filters
            .created_to
            .map(|value| value.date_naive())
            .unwrap_or_else(|| {
                group_rows
                    .iter()
                    .map(|row| row.order_created_at.date_naive())
                    .max()
                    .unwrap_or_else(|| Utc::now().date_naive())
            });
        let input = OrderReconciliationBatchInput {
            batch_id: build_order_reconciliation_batch_id(
                &platform,
                shop_id,
                period_start,
                period_end,
            ),
            platform,
            shop_id,
            period_start,
            period_end,
        };
        let persisted =
            match load_order_reconciliation_persisted_row(&state, tenant_id, &input.batch_id).await
            {
                Ok(value) => value,
                Err(error) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response()
                }
            };
        let batch = build_order_reconciliation_batch(&group_rows, &rule, &input, persisted);
        if let Some(status) = status_filter.as_deref() {
            if batch.summary.status.to_uppercase() != status {
                continue;
            }
        }
        batches.push(batch);
    }

    batches.sort_by(|left, right| {
        right
            .summary
            .diff_cent
            .cmp(&left.summary.diff_cent)
            .then_with(|| right.summary.order_count.cmp(&left.summary.order_count))
            .then_with(|| left.summary.batch_id.cmp(&right.summary.batch_id))
    });

    let summary = OrderReconciliationListSummaryResponse {
        total_batches: batches.len() as i64,
        pending_batches: batches
            .iter()
            .filter(|item| item.summary.status == "PENDING")
            .count() as i64,
        done_batches: batches
            .iter()
            .filter(|item| item.summary.status == "DONE")
            .count() as i64,
        expected_total_cent: batches.iter().map(|item| item.summary.expected_cent).sum(),
        diff_total_cent: batches.iter().map(|item| item.summary.diff_cent).sum(),
    };
    let items = batches
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|item| item.summary)
        .collect::<Vec<_>>();

    (
        StatusCode::OK,
        Json(OrderReconciliationListResponse {
            total: summary.total_batches,
            summary,
            items,
        }),
    )
        .into_response()
}

async fn get_order_reconciliation_detail(
    State(state): State<AppState>,
    Path((tenant_id, batch_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let batch = match parse_order_reconciliation_batch_id(&batch_id) {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_shop_access(&ctx, ShopId(batch.shop_id)) {
        return resp.into_response();
    }

    let rule = match load_order_settlement_rule(&state, tenant_id).await {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response()
        }
    };
    let rows = match list_order_settlement_rows(
        &state,
        tenant_id,
        &OrderSettlementFilters {
            platform: Some(batch.platform.clone()),
            shop_id: Some(batch.shop_id),
            keyword: None,
            status: None,
            created_from: Some(
                batch
                    .period_start
                    .and_hms_opt(0, 0, 0)
                    .expect("date should build")
                    .and_utc(),
            ),
            created_to: Some(
                batch
                    .period_end
                    .and_hms_opt(23, 59, 59)
                    .expect("date should build")
                    .and_utc(),
            ),
        },
    )
    .await
    {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response()
        }
    };
    if rows.is_empty() {
        return not_found("order reconciliation not found").into_response();
    }
    let persisted =
        match load_order_reconciliation_persisted_row(&state, tenant_id, &batch.batch_id).await {
            Ok(value) => value,
            Err(error) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response()
            }
        };
    let batch = build_order_reconciliation_batch(&rows, &rule, &batch, persisted);
    (
        StatusCode::OK,
        Json(OrderReconciliationDetailResponse {
            batch_id: batch.summary.batch_id,
            platform: batch.summary.platform,
            shop_id: batch.summary.shop_id,
            period_start: batch.summary.period_start,
            period_end: batch.summary.period_end,
            order_count: batch.summary.order_count,
            expected_cent: batch.summary.expected_cent,
            actual_cent: batch.summary.actual_cent,
            diff_cent: batch.summary.diff_cent,
            status: batch.summary.status,
            reasons: batch.summary.reasons,
            settled_at: batch.summary.settled_at,
            note: batch.note,
            orders: batch.orders,
        }),
    )
        .into_response()
}

async fn settle_order_reconciliation(
    State(state): State<AppState>,
    Path((tenant_id, batch_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(payload): Json<OrderReconciliationActionRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let batch = match parse_order_reconciliation_batch_id(&batch_id) {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_shop_access(&ctx, ShopId(batch.shop_id)) {
        return resp.into_response();
    }
    let rows = match list_order_settlement_rows(
        &state,
        tenant_id,
        &OrderSettlementFilters {
            platform: Some(batch.platform.clone()),
            shop_id: Some(batch.shop_id),
            keyword: None,
            status: None,
            created_from: Some(
                batch
                    .period_start
                    .and_hms_opt(0, 0, 0)
                    .expect("date should build")
                    .and_utc(),
            ),
            created_to: Some(
                batch
                    .period_end
                    .and_hms_opt(23, 59, 59)
                    .expect("date should build")
                    .and_utc(),
            ),
        },
    )
    .await
    {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response()
        }
    };
    if rows.is_empty() {
        return not_found("order reconciliation not found").into_response();
    }
    let operator_id = match parse_optional_user_id(payload.operator_id.as_deref(), "operator_id") {
        Ok(value) => value.unwrap_or(ctx.user_id),
        Err(resp) => return resp,
    };
    let operator_name = trim_to_option(payload.operator_name.as_deref());
    let note = trim_to_option(payload.note.as_deref());
    let settled_at = Utc::now();
    if let Err(error) = sqlx::query(
        r#"
        insert into order_reconciliation_batches (
            tenant_id, batch_id, platform, shop_id, period_start, period_end,
            status, note, settled_at
        ) values ($1, $2, $3, $4, $5, $6, 'DONE', $7, $8)
        on conflict (tenant_id, batch_id)
        do update set
            status = 'DONE',
            note = excluded.note,
            settled_at = excluded.settled_at,
            updated_at = now()
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(&batch.batch_id)
    .bind(&batch.platform)
    .bind(batch.shop_id)
    .bind(batch.period_start)
    .bind(batch.period_end)
    .bind(note.clone())
    .bind(settled_at)
    .execute(state.db.pool())
    .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
    }
    record_order_reconciliation_log(
        &state,
        tenant_id,
        &batch,
        Some(operator_id),
        operator_name,
        note,
    )
    .await;

    let rule = match load_order_settlement_rule(&state, tenant_id).await {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response()
        }
    };
    let persisted =
        match load_order_reconciliation_persisted_row(&state, tenant_id, &batch.batch_id).await {
            Ok(value) => value,
            Err(error) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response()
            }
        };
    let batch = build_order_reconciliation_batch(&rows, &rule, &batch, persisted);
    (StatusCode::OK, Json(batch.summary)).into_response()
}

async fn handle_order_exception(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OrderExceptionHandleRequest>,
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

    match apply_order_exception_action(
        &state, &ctx, &req_ctx, tenant_id, order_id, req.action, req.flags, req.reason,
    )
    .await
    {
        Ok(updated) => (StatusCode::OK, Json(updated)).into_response(),
        Err(resp) => resp,
    }
}

async fn handle_order_exception_pool(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OrderExceptionPoolRequest>,
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
    let order_id = match OrderId::from_str(&req.order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    match apply_order_exception_action(
        &state, &ctx, &req_ctx, tenant_id, order_id, req.action, req.flags, req.reason,
    )
    .await
    {
        Ok(updated) => (StatusCode::OK, Json(updated)).into_response(),
        Err(resp) => resp,
    }
}

async fn get_order_exception_stats(
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
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }

    let repo = OrdersRepo::new(state.db.pool());
    let stats = match repo.get_exception_stats(tenant_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let codes = stats
        .reasons
        .iter()
        .map(|reason| reason.reason_code.clone())
        .collect::<Vec<_>>();
    let mut label_map = std::collections::HashMap::new();
    if !codes.is_empty() {
        let rows: Vec<(String, String)> = match sqlx::query_as(
            r#"
            select code, label
            from exception_reasons
            where tenant_id = $1 and code = any($2)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(&codes)
        .fetch_all(state.db.pool())
        .await
        {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        for (code, label) in rows {
            label_map.insert(code, label);
        }
    }

    let reasons = stats
        .reasons
        .into_iter()
        .map(|reason| OrderExceptionReasonStatResponse {
            reason_code: reason.reason_code.clone(),
            label: label_map.get(&reason.reason_code).cloned(),
            total: reason.total,
            open: reason.open,
            processed: reason.processed,
        })
        .collect::<Vec<_>>();

    let response = OrderExceptionStatsResponse {
        total: stats.total,
        open: stats.open,
        processed: stats.processed,
        reasons,
    };
    (StatusCode::OK, Json(response)).into_response()
}

#[allow(clippy::too_many_arguments)]
async fn apply_order_exception_action(
    state: &AppState,
    ctx: &AuthContext,
    req_ctx: &RequestContext,
    tenant_id: TenantId,
    order_id: OrderId,
    action: String,
    flags: Option<Vec<String>>,
    reason: Option<String>,
) -> Result<OrderReviewState, Response> {
    let action = action.to_uppercase();
    let orders_repo = OrdersRepo::new(state.db.pool());
    let state_row = match orders_repo
        .get_order_review_state(tenant_id, order_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return Err(not_found("order not found").into_response()),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()),
    };

    let mut next_flags = state_row.exception_flags.clone();
    let target_flags = flags
        .unwrap_or_default()
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    match action.as_str() {
        "RESOLVE" => {
            if target_flags.is_empty() {
                next_flags.clear();
            } else {
                next_flags.retain(|flag| !target_flags.contains(flag));
            }
        }
        "REJECT" => {}
        "REVOKE" | "REOPEN" => {
            for flag in target_flags.iter() {
                if !next_flags.contains(flag) {
                    next_flags.push(flag.clone());
                }
            }
        }
        "CLOSE" => {
            if target_flags.is_empty() {
                next_flags.clear();
            } else {
                next_flags.retain(|flag| !target_flags.contains(flag));
            }
        }
        _ => return Err(bad_request("bad action").into_response()),
    }

    let mut next_review_status = state_row.review_status;
    let mut next_oms_status = state_row.oms_status;
    let review_action = match action.as_str() {
        "RESOLVE" if state_row.review_status.is_pending() => Some(ReviewAction::Pass),
        "REJECT" => Some(ReviewAction::Reject),
        "REOPEN" | "REVOKE" => Some(ReviewAction::Flag),
        _ => None,
    };
    if let Some(review_action) = review_action {
        if let Ok(outcome) = apply_review_decision(
            state_row.oms_status,
            review_action,
            &next_flags,
            None,
            ReviewMode::Manual,
        ) {
            next_review_status = outcome.review_status;
            next_oms_status = outcome.oms_status;
            for flag in outcome.exception_flags {
                if !next_flags.contains(&flag) {
                    next_flags.push(flag);
                }
            }
        }
    }

    let mut next_exception_status = derive_exception_status(&next_flags);
    if action.as_str() == "RESOLVE" && next_flags.is_empty() {
        next_exception_status = OrderExceptionStatus::Released;
    }
    if action.as_str() == "CLOSE" {
        next_exception_status = OrderExceptionStatus::Closed;
    }
    let next_order_status = derive_order_status(
        next_oms_status,
        next_review_status,
        state_row.fulfillment_status,
        next_exception_status,
    );

    let updated = if next_flags == state_row.exception_flags
        && next_exception_status == state_row.exception_status
        && next_order_status == state_row.order_status
        && next_review_status == state_row.review_status
        && next_oms_status == state_row.oms_status
    {
        state_row.clone()
    } else {
        match orders_repo
            .update_order_review_state(
                tenant_id,
                order_id,
                OrderReviewUpdate {
                    oms_status: next_oms_status,
                    order_status: next_order_status,
                    review_status: next_review_status,
                    fulfillment_status: state_row.fulfillment_status,
                    exception_status: next_exception_status,
                    exception_flags: next_flags.clone(),
                    expected_version: Some(state_row.version),
                },
            )
            .await
        {
            Ok(Some(v)) => v,
            Ok(None) => return Err(conflict("exception update conflict").into_response()),
            Err(e) => {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response())
            }
        }
    };

    let target_codes = if target_flags.is_empty() {
        state_row.exception_flags.clone()
    } else {
        target_flags
    };
    if !target_codes.is_empty() {
        let (status, handled_at, handler_id) = match action.as_str() {
            "RESOLVE" => ("RESOLVED".to_string(), Some(Utc::now()), Some(ctx.user_id)),
            "REJECT" => ("REJECTED".to_string(), Some(Utc::now()), Some(ctx.user_id)),
            "CLOSE" => ("CLOSED".to_string(), Some(Utc::now()), Some(ctx.user_id)),
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

    let action_label = match action.as_str() {
        "RESOLVE" => "OMS.EXCEPTION_RESOLVE",
        "REJECT" => "OMS.EXCEPTION_REJECT",
        "CLOSE" => "OMS.EXCEPTION_CLOSE",
        _ => "OMS.EXCEPTION_REVOKE",
    };
    record_audit_log(
        state,
        ctx,
        req_ctx,
        action_label,
        "ORDER",
        Some(order_id.to_string()),
        Some(serde_json::json!({
            "exception_flags": state_row.exception_flags,
        })),
        Some(serde_json::json!({
            "exception_flags": updated.exception_flags,
            "reason": reason,
            "action": action,
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
                "action": action.clone(),
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

    let operation_action = match action.as_str() {
        "RESOLVE" => OP_ACTION_OMS_EXCEPTION_RESOLVE,
        "REJECT" => OP_ACTION_OMS_EXCEPTION_REJECT,
        _ => OP_ACTION_OMS_EXCEPTION_REVOKE,
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
                "before_flags": state_row.exception_flags,
                "after_flags": updated.exception_flags,
            })),
        })
        .await;

    if next_review_status != state_row.review_status {
        let reviewed_at = if review_record_status(next_review_status) == ORDER_REVIEW_RECORD_PENDING
        {
            None
        } else {
            Some(Utc::now())
        };
        upsert_order_review_record(
            state,
            tenant_id,
            order_id,
            next_review_status,
            Some(ctx.user_id),
            reviewed_at,
            reason.clone(),
        )
        .await;
    }

    Ok(updated)
}

async fn rematch_order_items(
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

async fn return_order_to_download(
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

async fn rollback_order_to_review(
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

    let fulfillment_repo = FulfillmentRepo::new(state.db.pool());
    let release_result = match fulfillment_repo
        .release_order_reservations_keep_order(tenant_id, order_id)
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
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
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()
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

async fn requeue_order_review(
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

async fn add_order_tags(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OrderTagsRequest>,
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
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::TagSystem).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::TagSystem).into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    let tags: Vec<String> = req
        .tags
        .into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect();
    if tags.is_empty() {
        return bad_request("tags required").into_response();
    }

    let orders_repo = OrdersRepo::new(state.db.pool());
    let created_tags = tags
        .iter()
        .map(|tag| OrderTagCreate {
            tag: tag.to_string(),
            created_by: Some(ctx.user_id),
        })
        .collect::<Vec<_>>();

    let result = match orders_repo
        .add_order_tags(tenant_id, order_id, created_tags)
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "OMS.ORDER_TAG_ADD".to_string(),
            entity_type: "ORDER".to_string(),
            entity_id: Some(order_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({ "tags": tags })),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(result)).into_response()
}

fn build_tag_context(detail: &erp_repo::orders::OmsOrderDetail) -> OrderReviewContext {
    let mut context = OrderReviewContext::new();
    context.insert_field(
        "total_cent",
        serde_json::Value::Number(detail.order.total_amount_cent.into()),
    );
    context.insert_field(
        "sku_count",
        serde_json::Value::Number(
            detail
                .items
                .iter()
                .map(|item| item.quantity as i64)
                .sum::<i64>()
                .into(),
        ),
    );
    context.insert_field(
        "sku_lines",
        serde_json::Value::Number((detail.items.len() as i64).into()),
    );
    if let Some(phone) = detail.order.receiver_phone.as_ref() {
        context.insert_field("receiver_phone", serde_json::Value::String(phone.clone()));
    }
    if let Some(name) = detail.order.receiver_name.as_ref() {
        context.insert_field("receiver_name", serde_json::Value::String(name.clone()));
    }
    context.insert_field(
        "platform",
        serde_json::Value::String(detail.order.platform.to_string()),
    );
    context.insert_field(
        "shop_id",
        serde_json::Value::String(detail.order.shop_id.to_string()),
    );
    context.insert_field(
        "platform_order_id",
        serde_json::Value::String(detail.order.platform_order_id.clone()),
    );
    if let Some(address) = detail.order.receiver_address.as_ref() {
        context.insert_field(
            "receiver_address",
            serde_json::Value::String(address.clone()),
        );
        if address.trim().is_empty() {
            context.insert_field("address_invalid", serde_json::Value::Bool(true));
        }
    } else {
        context.insert_field("address_invalid", serde_json::Value::Bool(true));
    }
    context
}

async fn apply_auto_tags(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OrderAutoTagApplyRequest>,
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
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::TagSystem).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::TagSystem).into_response();
    }
    if req.order_ids.is_empty() {
        return bad_request("order_ids required").into_response();
    }

    let tag_repo = OrderTagDefinitionsRepo::new(state.db.pool());
    let definitions = match tag_repo.list_auto_apply_tags(tenant_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if definitions.is_empty() {
        return (
            StatusCode::OK,
            Json(OrderAutoTagApplyResponse {
                tagged_orders: Vec::new(),
                total: 0,
            }),
        )
            .into_response();
    }

    let orders_repo = OrdersRepo::new(state.db.pool());
    let mut tagged_orders: Vec<String> = Vec::new();

    for raw_id in req.order_ids {
        let order_id = match OrderId::from_str(&raw_id) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let detail = match orders_repo.get_order_detail(tenant_id, order_id).await {
            Ok(Some(v)) => v,
            Ok(None) => continue,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };

        let context = build_tag_context(&detail);
        let existing_tags: std::collections::HashSet<String> =
            detail.tags.iter().map(|tag| tag.tag.clone()).collect();
        let mut matched_tags: Vec<String> = Vec::new();

        for definition in &definitions {
            if existing_tags.contains(&definition.name) {
                continue;
            }
            let conditions: Vec<OrderReviewCondition> =
                match serde_json::from_value(definition.conditions_json.clone()) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
            let matched = if conditions.is_empty() {
                true
            } else {
                evaluate_conditions(&context, &conditions)
            };
            if matched {
                matched_tags.push(definition.name.clone());
            }
        }

        if matched_tags.is_empty() {
            continue;
        }

        let created_tags = matched_tags
            .iter()
            .map(|tag| OrderTagCreate {
                tag: tag.to_string(),
                created_by: Some(ctx.user_id),
            })
            .collect::<Vec<_>>();
        if let Err(e) = orders_repo
            .add_order_tags(tenant_id, order_id, created_tags)
            .await
        {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
        }

        let audit_repo = AuditLogsRepo::new(state.db.pool());
        let _ = audit_repo
            .insert(AuditLogCreate {
                tenant_id,
                user_id: Some(ctx.user_id),
                action: "OMS.ORDER_TAG_AUTO_APPLY".to_string(),
                entity_type: "ORDER".to_string(),
                entity_id: Some(order_id.to_string()),
                before_json: None,
                after_json: Some(serde_json::json!({ "tags": matched_tags })),
                ip: None,
                ua: None,
            })
            .await;

        tagged_orders.push(order_id.to_string());
    }

    let response = OrderAutoTagApplyResponse {
        total: tagged_orders.len() as i64,
        tagged_orders,
    };
    (StatusCode::OK, Json(response)).into_response()
}

async fn export_order_review_hits(
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

async fn add_order_note(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OrderNoteRequest>,
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
    let note = req.note.trim();
    if note.is_empty() {
        return bad_request("note required").into_response();
    }

    let orders_repo = OrdersRepo::new(state.db.pool());
    let result = match orders_repo
        .add_order_note(
            tenant_id,
            order_id,
            OrderNoteCreate {
                note: note.to_string(),
                created_by: Some(ctx.user_id),
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "OMS.ORDER_NOTE_ADD".to_string(),
            entity_type: "ORDER".to_string(),
            entity_id: Some(order_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({ "note": note })),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(result)).into_response()
}

pub(super) async fn list_order_tag_definitions(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<TagDefinitionListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicOrderTagRead.as_str()) {
        return resp.into_response();
    }
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::TagSystem).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::TagSystem).into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let keyword = query
        .keyword
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = query
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());

    let repo = OrderTagDefinitionsRepo::new(state.db.pool());
    let result = match repo
        .list_tags(
            tenant_id,
            OrderTagDefinitionListFilters {
                keyword,
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

    let response = OrderTagDefinitionListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn create_order_tag_definition(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<TagDefinitionCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicOrderTagWrite.as_str()) {
        return resp.into_response();
    }
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::TagSystem).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::TagSystem).into_response();
    }

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name required").into_response();
    }
    let status = req
        .status
        .unwrap_or_else(|| "ACTIVE".to_string())
        .trim()
        .to_uppercase();
    if status.is_empty() {
        return bad_request("status required").into_response();
    }
    let conditions_json = req.conditions_json.unwrap_or_else(|| serde_json::json!([]));

    let repo = OrderTagDefinitionsRepo::new(state.db.pool());
    let tag = match repo
        .create_tag(OrderTagDefinitionCreate {
            tenant_id,
            name: name.to_string(),
            color: req.color.map(|value| value.trim().to_string()),
            priority: req.priority.unwrap_or(0),
            status,
            auto_apply: req.auto_apply.unwrap_or(false),
            conditions_json,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "ORDER_TAG_DEF.CREATE".to_string(),
            entity_type: "order_tag_definition".to_string(),
            entity_id: Some(tag.tag_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&tag).ok(),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(tag)).into_response()
}

pub(super) async fn update_order_tag_definition(
    State(state): State<AppState>,
    Path((tenant_id, tag_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<TagDefinitionUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let tag_id = match uuid::Uuid::parse_str(&tag_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tag_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicOrderTagWrite.as_str()) {
        return resp.into_response();
    }
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::TagSystem).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::TagSystem).into_response();
    }

    let name = req
        .name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let color = req
        .color
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = req
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());

    if name.is_none()
        && color.is_none()
        && req.priority.is_none()
        && status.is_none()
        && req.auto_apply.is_none()
        && req.conditions_json.is_none()
    {
        return bad_request("no fields to update").into_response();
    }

    let repo = OrderTagDefinitionsRepo::new(state.db.pool());
    let tag = match repo
        .update_tag(
            tenant_id,
            tag_id,
            OrderTagDefinitionUpdate {
                name,
                color,
                priority: req.priority,
                status,
                auto_apply: req.auto_apply,
                conditions_json: req.conditions_json,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order tag definition not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "ORDER_TAG_DEF.UPDATE".to_string(),
            entity_type: "order_tag_definition".to_string(),
            entity_id: Some(tag.tag_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&tag).ok(),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(tag)).into_response()
}

pub(super) async fn delete_order_tag_definition(
    State(state): State<AppState>,
    Path((tenant_id, tag_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let tag_id = match uuid::Uuid::parse_str(&tag_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tag_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicOrderTagWrite.as_str()) {
        return resp.into_response();
    }
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::TagSystem).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::TagSystem).into_response();
    }

    let repo = OrderTagDefinitionsRepo::new(state.db.pool());
    let usage = match repo.count_usage(tenant_id, tag_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if usage > 0 {
        return conflict("order tag in use").into_response();
    }

    let tag = match repo.delete_tag(tenant_id, tag_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order tag definition not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "ORDER_TAG_DEF.DELETE".to_string(),
            entity_type: "order_tag_definition".to_string(),
            entity_id: Some(tag.tag_id.to_string()),
            before_json: serde_json::to_value(&tag).ok(),
            after_json: None,
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(tag)).into_response()
}

pub(super) async fn list_sku_tag_definitions(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<TagDefinitionListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicSkuTagRead.as_str()) {
        return resp.into_response();
    }
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::TagSystem).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::TagSystem).into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let keyword = query
        .keyword
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = query
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());

    let repo = SkuTagDefinitionsRepo::new(state.db.pool());
    let result = match repo
        .list_tags(
            tenant_id,
            SkuTagDefinitionListFilters {
                keyword,
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

    let response = SkuTagDefinitionListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn create_sku_tag_definition(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<TagDefinitionCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicSkuTagWrite.as_str()) {
        return resp.into_response();
    }
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::TagSystem).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::TagSystem).into_response();
    }

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name required").into_response();
    }
    let status = req
        .status
        .unwrap_or_else(|| "ACTIVE".to_string())
        .trim()
        .to_uppercase();
    if status.is_empty() {
        return bad_request("status required").into_response();
    }

    let repo = SkuTagDefinitionsRepo::new(state.db.pool());
    let tag = match repo
        .create_tag(SkuTagDefinitionCreate {
            tenant_id,
            name: name.to_string(),
            color: req.color.map(|value| value.trim().to_string()),
            priority: req.priority.unwrap_or(0),
            status,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "SKU_TAG_DEF.CREATE".to_string(),
            entity_type: "sku_tag_definition".to_string(),
            entity_id: Some(tag.tag_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&tag).ok(),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(tag)).into_response()
}

pub(super) async fn update_sku_tag_definition(
    State(state): State<AppState>,
    Path((tenant_id, tag_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<TagDefinitionUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let tag_id = match uuid::Uuid::parse_str(&tag_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tag_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicSkuTagWrite.as_str()) {
        return resp.into_response();
    }
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::TagSystem).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::TagSystem).into_response();
    }

    let name = req
        .name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let color = req
        .color
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = req
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());

    if name.is_none() && color.is_none() && req.priority.is_none() && status.is_none() {
        return bad_request("no fields to update").into_response();
    }

    let repo = SkuTagDefinitionsRepo::new(state.db.pool());
    let tag = match repo
        .update_tag(
            tenant_id,
            tag_id,
            SkuTagDefinitionUpdate {
                name,
                color,
                priority: req.priority,
                status,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("sku tag definition not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "SKU_TAG_DEF.UPDATE".to_string(),
            entity_type: "sku_tag_definition".to_string(),
            entity_id: Some(tag.tag_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&tag).ok(),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(tag)).into_response()
}

pub(super) async fn delete_sku_tag_definition(
    State(state): State<AppState>,
    Path((tenant_id, tag_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let tag_id = match uuid::Uuid::parse_str(&tag_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tag_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicSkuTagWrite.as_str()) {
        return resp.into_response();
    }
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::TagSystem).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::TagSystem).into_response();
    }

    let repo = SkuTagDefinitionsRepo::new(state.db.pool());
    let tag = match repo.delete_tag(tenant_id, tag_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("sku tag definition not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "SKU_TAG_DEF.DELETE".to_string(),
            entity_type: "sku_tag_definition".to_string(),
            entity_id: Some(tag.tag_id.to_string()),
            before_json: serde_json::to_value(&tag).ok(),
            after_json: None,
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(tag)).into_response()
}

async fn list_review_rules(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<OrderReviewRuleListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }

    let filters = OrderReviewRuleListFilters {
        enabled: query.enabled,
        offset: query.offset.unwrap_or(0).max(0),
        limit: query.limit.unwrap_or(50).clamp(1, 200),
    };
    let repo = OrderReviewRulesRepo::new(state.db.pool());
    let result = match repo.list_rules(tenant_id, filters).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = OrderReviewRuleListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

async fn get_review_rule(
    State(state): State<AppState>,
    Path((tenant_id, rule_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let rule_id = match uuid::Uuid::parse_str(&rule_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad rule_id").into_response(),
    };

    let repo = OrderReviewRulesRepo::new(state.db.pool());
    match repo.get_rule(tenant_id, rule_id).await {
        Ok(Some(rule)) => (StatusCode::OK, Json(rule)).into_response(),
        Ok(None) => not_found("rule not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    }
}

async fn create_review_rule(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OrderReviewRuleRequest>,
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
    let conditions_json = req.conditions_json.unwrap_or_else(|| serde_json::json!([]));
    let repo = OrderReviewRulesRepo::new(state.db.pool());
    let rule = match repo
        .create_rule(OrderReviewRuleCreate {
            tenant_id,
            name: req.name,
            enabled: req.enabled.unwrap_or(true),
            priority: req.priority.unwrap_or(0),
            conditions_json,
            action: req.action,
            reason: req.reason.unwrap_or_default(),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "OMS.REVIEW_RULE_CREATE".to_string(),
            entity_type: "ORDER_REVIEW_RULE".to_string(),
            entity_id: Some(rule.rule_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::to_value(&rule).unwrap_or_default()),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(rule)).into_response()
}

async fn update_review_rule(
    State(state): State<AppState>,
    Path((tenant_id, rule_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OrderReviewRuleUpdateRequest>,
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
    let rule_id = match uuid::Uuid::parse_str(&rule_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad rule_id").into_response(),
    };

    let repo = OrderReviewRulesRepo::new(state.db.pool());
    let before = match repo.get_rule(tenant_id, rule_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let Some(before) = before else {
        return not_found("rule not found").into_response();
    };

    let updated = match repo
        .update_rule(
            tenant_id,
            rule_id,
            OrderReviewRuleUpdate {
                name: req.name,
                enabled: req.enabled,
                priority: req.priority,
                conditions_json: req.conditions_json,
                action: req.action,
                reason: req.reason,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let Some(updated) = updated else {
        return not_found("rule not found").into_response();
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "OMS.REVIEW_RULE_UPDATE".to_string(),
            entity_type: "ORDER_REVIEW_RULE".to_string(),
            entity_id: Some(rule_id.to_string()),
            before_json: Some(serde_json::to_value(before).unwrap_or_default()),
            after_json: Some(serde_json::to_value(&updated).unwrap_or_default()),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(updated)).into_response()
}

async fn delete_review_rule(
    State(state): State<AppState>,
    Path((tenant_id, rule_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
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
    let rule_id = match uuid::Uuid::parse_str(&rule_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad rule_id").into_response(),
    };

    let repo = OrderReviewRulesRepo::new(state.db.pool());
    let deleted = match repo.delete_rule(tenant_id, rule_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let Some(deleted) = deleted else {
        return not_found("rule not found").into_response();
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "OMS.REVIEW_RULE_DELETE".to_string(),
            entity_type: "ORDER_REVIEW_RULE".to_string(),
            entity_id: Some(rule_id.to_string()),
            before_json: Some(serde_json::to_value(&deleted).unwrap_or_default()),
            after_json: None,
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(deleted)).into_response()
}

async fn list_exception_reasons(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ExceptionReasonListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }

    let repo = ExceptionReasonsRepo::new(state.db.pool());
    let result = match repo
        .list_reasons(
            tenant_id,
            ExceptionReasonListFilters {
                category: query.category,
                offset: query.offset.unwrap_or(0).max(0),
                limit: query.limit.unwrap_or(100).clamp(1, 500),
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = ExceptionReasonListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

async fn list_customer_blacklist(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<CustomerBlacklistListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }

    let repo = CustomerBlacklistRepo::new(state.db.pool());
    let result = match repo
        .list_entries(
            tenant_id,
            CustomerBlacklistListFilters {
                key_type: query.key_type,
                keyword: query.keyword,
                offset: query.offset.unwrap_or(0).max(0),
                limit: query.limit.unwrap_or(100).clamp(1, 500),
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = CustomerBlacklistListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

async fn create_customer_blacklist(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CustomerBlacklistRequest>,
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

    let repo = CustomerBlacklistRepo::new(state.db.pool());
    let entry = match repo
        .create_entry(CustomerBlacklistCreate {
            tenant_id,
            key_type: req.key_type,
            key_value: req.key_value,
            reason: req.reason.unwrap_or_default(),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "OMS.BLACKLIST_CREATE".to_string(),
            entity_type: "CUSTOMER_BLACKLIST".to_string(),
            entity_id: Some(entry.blacklist_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::to_value(&entry).unwrap_or_default()),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(entry)).into_response()
}

async fn delete_customer_blacklist(
    State(state): State<AppState>,
    Path((tenant_id, blacklist_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
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
    let blacklist_id = match uuid::Uuid::parse_str(&blacklist_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad blacklist_id").into_response(),
    };

    let repo = CustomerBlacklistRepo::new(state.db.pool());
    let entry = match repo.delete_entry(tenant_id, blacklist_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let Some(entry) = entry else {
        return not_found("blacklist entry not found").into_response();
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "OMS.BLACKLIST_DELETE".to_string(),
            entity_type: "CUSTOMER_BLACKLIST".to_string(),
            entity_id: Some(entry.blacklist_id.to_string()),
            before_json: Some(serde_json::to_value(&entry).unwrap_or_default()),
            after_json: None,
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(entry)).into_response()
}

async fn get_order_raw(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, "bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read_raw") {
        return resp.into_response();
    }

    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, "bad order_id").into_response(),
    };

    let orders_repo = OrdersRepo::new(state.db.pool());
    let detail = match orders_repo.get_order_detail(tenant_id, order_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if let Err(resp) = ensure_shop_access(&ctx, detail.order.shop_id) {
        return resp.into_response();
    }
    match orders_repo.get_order_raw(tenant_id, order_id).await {
        Ok(Some(raw)) => (StatusCode::OK, Json(raw)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    }
}

async fn ship_order(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    headers: HeaderMap,
    Json(req): Json<ShipOrderRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };
    let warehouse_id = match req.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };

    let idempotency_key = headers
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| order_id.to_string());

    let _ = req.print_label;
    let shipment = match handle_shipment_request(
        &state,
        &ctx,
        &req_ctx,
        ShipmentRequestArgs {
            tenant_id,
            order_id,
            warehouse_id,
            carrier_code: req.carrier_code,
            tracking_no: req.tracking_no,
            weight_gram: req.weight_gram,
            serials: req.serials.unwrap_or_default(),
            idempotency_key,
        },
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    (StatusCode::OK, Json(shipment)).into_response()
}

async fn allocate_oms_order(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OmsAllocationRequest>,
) -> impl IntoResponse {
    handle_oms_allocation(
        &state,
        &ctx,
        &req_ctx,
        &tenant_id,
        &order_id,
        req.carrier_code,
    )
    .await
}

async fn create_oms_shipments(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OmsAllocationRequest>,
) -> impl IntoResponse {
    handle_oms_allocation(
        &state,
        &ctx,
        &req_ctx,
        &tenant_id,
        &order_id,
        req.carrier_code,
    )
    .await
}

async fn resend_oms_order(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OmsResendRequest>,
) -> impl IntoResponse {
    handle_oms_resend(
        &state,
        &ctx,
        &req_ctx,
        &tenant_id,
        &order_id,
        req.carrier_code,
        req.origin_shipment_id,
    )
    .await
}

async fn release_oms_order(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    handle_oms_release(&state, &ctx, &req_ctx, &tenant_id, &order_id).await
}

async fn request_oms_shipment_cancel(
    State(state): State<AppState>,
    Path((tenant_id, shipment_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OmsShipmentCancelRequest>,
) -> impl IntoResponse {
    handle_oms_cancel_request(&state, &ctx, &req_ctx, &tenant_id, &shipment_id, req.reason).await
}

async fn list_oms_shipments(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<OmsShipmentListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    fulfillment::list_oms_shipments(State(state), Path(tenant_id), Query(query), Extension(ctx))
        .await
}

async fn list_oms_order_shipments(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    fulfillment::list_oms_order_shipments(State(state), Path((tenant_id, order_id)), Extension(ctx))
        .await
}

async fn get_oms_shipment(
    State(state): State<AppState>,
    Path((tenant_id, shipment_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    fulfillment::get_oms_shipment(State(state), Path((tenant_id, shipment_id)), Extension(ctx))
        .await
}

async fn ship_oms_shipment(
    State(state): State<AppState>,
    Path((tenant_id, shipment_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OmsShipRequest>,
) -> impl IntoResponse {
    fulfillment::ship_oms_shipment(
        State(state),
        Path((tenant_id, shipment_id)),
        Extension(ctx),
        Extension(req_ctx),
        Json(req),
    )
    .await
}

async fn handle_oms_allocation(
    state: &AppState,
    ctx: &AuthContext,
    req_ctx: &RequestContext,
    tenant_id: &str,
    order_id: &str,
    carrier_code: Option<String>,
) -> Response {
    fulfillment::handle_oms_allocation(state, ctx, req_ctx, tenant_id, order_id, carrier_code).await
}

async fn handle_oms_resend(
    state: &AppState,
    ctx: &AuthContext,
    req_ctx: &RequestContext,
    tenant_id: &str,
    order_id: &str,
    carrier_code: Option<String>,
    origin_shipment_id: Option<String>,
) -> Response {
    fulfillment::handle_oms_resend(
        state,
        ctx,
        req_ctx,
        tenant_id,
        order_id,
        carrier_code,
        origin_shipment_id,
    )
    .await
}

async fn handle_oms_release(
    state: &AppState,
    ctx: &AuthContext,
    req_ctx: &RequestContext,
    tenant_id: &str,
    order_id: &str,
) -> Response {
    fulfillment::handle_oms_release(state, ctx, req_ctx, tenant_id, order_id).await
}

async fn handle_oms_cancel_request(
    state: &AppState,
    ctx: &AuthContext,
    req_ctx: &RequestContext,
    tenant_id: &str,
    shipment_id: &str,
    reason: Option<String>,
) -> Response {
    fulfillment::handle_oms_cancel_request(state, ctx, req_ctx, tenant_id, shipment_id, reason)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        Extension,
    };
    use tower::ServiceExt;

    #[test]
    fn dto_module_preserves_order_download_last_time_alias() {
        let request: super::dto::OrderDownloadRequest = serde_json::from_value(serde_json::json!({
            "shop_id": "shop-1",
            "last_time": "2026-04-14T08:00:00Z"
        }))
        .expect("order download request should deserialize");

        assert_eq!(request.shop_id, "shop-1");
        assert_eq!(request.watermark.as_deref(), Some("2026-04-14T08:00:00Z"));
    }

    #[test]
    fn intake_module_normalizes_download_request_cursor_and_watermark() {
        let normalized = intake::normalize_download_request(&OrderDownloadRequest {
            shop_id: "shop-1".to_string(),
            cursor: Some("  CURSOR-001  ".to_string()),
            watermark: Some("2026-04-14T08:00:00+08:00".to_string()),
        })
        .expect("download request should normalize");

        assert_eq!(normalized.cursor.as_deref(), Some("CURSOR-001"));
        assert_eq!(
            normalized.watermark,
            Some(
                DateTime::parse_from_rfc3339("2026-04-14T08:00:00+08:00")
                    .expect("watermark should parse")
                    .with_timezone(&Utc)
            )
        );
    }

    #[test]
    fn intake_module_builds_manual_order_items_with_defaults() {
        let normalized = intake::normalize_manual_order_items(&[
            ManualOrderItemRequest {
                line_id: None,
                sku_id: Some("  SKU-001  ".to_string()),
                title: Some("  手工订单商品  ".to_string()),
                qty: 2,
                price_cent: 500,
                amount_cent: None,
                pay_amount_cent: None,
                discount_cent: Some(50),
            },
            ManualOrderItemRequest {
                line_id: Some("  CUSTOM-LINE  ".to_string()),
                sku_id: None,
                title: Some("第二行".to_string()),
                qty: 1,
                price_cent: 700,
                amount_cent: Some(650),
                pay_amount_cent: Some(600),
                discount_cent: None,
            },
        ])
        .expect("manual order items should normalize");

        assert_eq!(normalized.computed_total_cent, 1650);
        assert_eq!(normalized.items.len(), 2);
        assert_eq!(normalized.items[0].line_id, "LINE-1");
        assert_eq!(normalized.items[0].sku_id.as_deref(), Some("SKU-001"));
        assert_eq!(normalized.items[0].title.as_deref(), Some("手工订单商品"));
        assert_eq!(normalized.items[0].amount_cent, 1000);
        assert_eq!(normalized.items[0].pay_amount_cent, Some(1000));
        assert_eq!(normalized.items[1].line_id, "CUSTOM-LINE");
        assert_eq!(normalized.items[1].amount_cent, 650);
        assert_eq!(normalized.items[1].pay_amount_cent, Some(600));
    }

    #[test]
    fn fulfillment_module_derives_allocation_status_from_shortage_count() {
        assert_eq!(fulfillment::derive_allocation_status(0), "ALLOCATED");
        assert_eq!(fulfillment::derive_allocation_status(3), "SHORTAGE");
    }

    #[test]
    fn fulfillment_module_parses_shipped_at_and_converts_to_utc() {
        let fallback = DateTime::parse_from_rfc3339("2026-04-14T00:00:00Z")
            .expect("fallback time should parse")
            .with_timezone(&Utc);
        let parsed = fulfillment::parse_shipped_at(Some("2026-04-14T08:30:00+08:00"), fallback)
            .expect("shipped_at should parse");

        assert_eq!(
            parsed,
            DateTime::parse_from_rfc3339("2026-04-14T08:30:00+08:00")
                .expect("expected shipped_at should parse")
                .with_timezone(&Utc)
        );
    }

    fn build_orders_test_router() -> Router {
        let db = Db::connect_lazy("postgres://postgres@localhost/postgres")
            .expect("lazy db should be constructible");
        let jwt = JwtService::new("test-secret".to_string());
        let state = AppState::new(db, Arc::new(jwt));
        let auth_ctx = AuthContext {
            tenant_id: TenantId(uuid::Uuid::new_v4()),
            user_id: UserId(uuid::Uuid::new_v4()),
            perms: vec!["orders.read".to_string(), "orders.write".to_string()],
            is_admin: false,
            shop_ids: Vec::new(),
            warehouse_ids: Vec::new(),
            merchant_ids: Vec::new(),
            organization_ids: Vec::new(),
        };
        let req_ctx = RequestContext {
            request_id: "req-test".to_string(),
            trace_id: "trace-test".to_string(),
            ip: None,
            user_agent: None,
        };
        router()
            .layer(Extension(auth_ctx))
            .layer(Extension(req_ctx))
            .with_state(state)
    }

    #[tokio::test]
    async fn exception_pool_routes_validate_path_before_db_access() {
        let app = build_orders_test_router();
        let cases = [
            (
                "GET",
                "/v1/tenants/not-a-uuid/oms/orders/exceptions",
                StatusCode::BAD_REQUEST,
            ),
            (
                "GET",
                "/v1/tenants/not-a-uuid/oms/orders/exceptions/not-a-uuid",
                StatusCode::BAD_REQUEST,
            ),
            (
                "POST",
                "/v1/tenants/not-a-uuid/oms/orders/exceptions/not-a-uuid/claim",
                StatusCode::BAD_REQUEST,
            ),
            (
                "POST",
                "/v1/tenants/not-a-uuid/oms/orders/exceptions/not-a-uuid/assign",
                StatusCode::BAD_REQUEST,
            ),
            (
                "POST",
                "/v1/tenants/not-a-uuid/oms/orders/exceptions/not-a-uuid/escalate",
                StatusCode::BAD_REQUEST,
            ),
            (
                "POST",
                "/v1/tenants/not-a-uuid/oms/orders/exceptions/not-a-uuid/resolve",
                StatusCode::BAD_REQUEST,
            ),
            (
                "POST",
                "/v1/tenants/not-a-uuid/oms/orders/exceptions/not-a-uuid/hold",
                StatusCode::BAD_REQUEST,
            ),
            (
                "POST",
                "/v1/tenants/not-a-uuid/oms/orders/exceptions/not-a-uuid/reject",
                StatusCode::BAD_REQUEST,
            ),
            (
                "GET",
                "/v1/tenants/not-a-uuid/oms/orders/settlements",
                StatusCode::BAD_REQUEST,
            ),
            (
                "GET",
                "/v1/tenants/not-a-uuid/oms/orders/settlements/rules",
                StatusCode::BAD_REQUEST,
            ),
            (
                "GET",
                "/v1/tenants/not-a-uuid/oms/orders/settlements/not-a-uuid",
                StatusCode::BAD_REQUEST,
            ),
            (
                "POST",
                "/v1/tenants/not-a-uuid/oms/orders/settlements/not-a-uuid/lock",
                StatusCode::BAD_REQUEST,
            ),
            (
                "POST",
                "/v1/tenants/not-a-uuid/oms/orders/settlements/not-a-uuid/unlock",
                StatusCode::BAD_REQUEST,
            ),
            (
                "POST",
                "/v1/tenants/not-a-uuid/oms/orders/settlements/not-a-uuid/export",
                StatusCode::BAD_REQUEST,
            ),
            (
                "GET",
                "/v1/tenants/not-a-uuid/oms/orders/reconciliations",
                StatusCode::BAD_REQUEST,
            ),
            (
                "GET",
                "/v1/tenants/not-a-uuid/oms/orders/reconciliations/bad-batch-id",
                StatusCode::BAD_REQUEST,
            ),
            (
                "POST",
                "/v1/tenants/not-a-uuid/oms/orders/reconciliations/bad-batch-id/settle",
                StatusCode::BAD_REQUEST,
            ),
            (
                "GET",
                "/v1/tenants/not-a-uuid/oms/review-settings",
                StatusCode::BAD_REQUEST,
            ),
            (
                "PUT",
                "/v1/tenants/not-a-uuid/oms/review-settings",
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (
                "GET",
                "/v1/tenants/not-a-uuid/oms/orders/recheck-queue",
                StatusCode::BAD_REQUEST,
            ),
            (
                "POST",
                "/v1/tenants/not-a-uuid/oms/orders/recheck-queue/batch-review",
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (
                "POST",
                "/v1/tenants/not-a-uuid/oms/orders/not-a-uuid/submit-shipment",
                StatusCode::BAD_REQUEST,
            ),
        ];

        for (method, uri, expected_status) in cases {
            let request = Request::builder()
                .uri(uri)
                .method(method)
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap();
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(response.status(), expected_status, "{method} {uri}");
        }
    }

    #[test]
    fn exception_pool_sla_marks_active_overdue_and_resolved_normal() {
        let created_at = Utc::now() - chrono::Duration::minutes(45);
        let active = exception_workbench::select_order_exception_tickets(
            &[exception_workbench::OrderExceptionTicketRow {
                reason_code: "MAPPING_MISSING".to_string(),
                status: "OPEN".to_string(),
                handler_id: None,
                created_at,
                updated_at: created_at,
            }],
            &[],
        );
        let (active_status, active_remaining_minutes) =
            exception_workbench::derive_order_exception_sla(&active);
        assert_eq!(active_status, "OVERDUE");
        assert!(active_remaining_minutes < 0);

        let resolved = exception_workbench::select_order_exception_tickets(
            &[exception_workbench::OrderExceptionTicketRow {
                reason_code: "MAPPING_MISSING".to_string(),
                status: "RESOLVED".to_string(),
                handler_id: Some(uuid::Uuid::new_v4()),
                created_at,
                updated_at: Utc::now(),
            }],
            &[],
        );
        let (resolved_status, resolved_remaining_minutes) =
            exception_workbench::derive_order_exception_sla(&resolved);
        assert_eq!(resolved_status, "NORMAL");
        assert!(resolved_remaining_minutes >= 0);
    }

    #[test]
    fn settlement_module_normalizes_stack_mode_and_status() {
        assert_eq!(
            settlement::normalize_order_settlement_stack_mode(" priority ").as_deref(),
            Some("PRIORITY")
        );
        assert_eq!(
            settlement::normalize_order_settlement_stack_mode("invalid"),
            None
        );
        assert_eq!(
            settlement::normalize_order_settlement_status(Some("locked")),
            "LOCKED"
        );
        assert_eq!(settlement::normalize_order_settlement_status(None), "OPEN");
    }

    #[test]
    fn settlement_module_round_trips_reconciliation_batch_id() {
        let shop_id = uuid::Uuid::new_v4();
        let period_start = NaiveDate::from_ymd_opt(2026, 4, 1).expect("valid start date");
        let period_end = NaiveDate::from_ymd_opt(2026, 4, 14).expect("valid end date");
        let batch_id = settlement::build_order_reconciliation_batch_id(
            "TAOBAO",
            shop_id,
            period_start,
            period_end,
        );

        let parsed = settlement::parse_order_reconciliation_batch_id(&batch_id)
            .expect("batch id should round-trip");

        assert_eq!(parsed.batch_id, batch_id);
        assert_eq!(parsed.platform, "TAOBAO");
        assert_eq!(parsed.shop_id, shop_id);
        assert_eq!(parsed.period_start, period_start);
        assert_eq!(parsed.period_end, period_end);
    }

    #[test]
    fn review_control_module_provides_default_review_settings() {
        let settings = review_control::default_oms_review_settings();

        assert!(settings.review_enabled);
        assert!(settings.auto_review_enabled);
        assert!(settings.recheck_enabled);
        assert_eq!(settings.default_unmatched_action, "MANUAL_REVIEW");
        assert!(!settings.manual_order_force_recheck);
    }

    #[test]
    fn review_control_module_parses_nested_review_settings_payload() {
        let settings = review_control::parse_oms_review_settings_value(&serde_json::json!({
            "value": {
                "review_enabled": false,
                "auto_review_enabled": false,
                "recheck_enabled": true,
                "default_unmatched_action": "PASS",
                "manual_order_force_recheck": true
            }
        }))
        .expect("nested review settings should parse");

        assert!(!settings.review_enabled);
        assert!(!settings.auto_review_enabled);
        assert!(settings.recheck_enabled);
        assert_eq!(settings.default_unmatched_action, "PASS");
        assert!(settings.manual_order_force_recheck);
    }
}
