use super::*;
use erp_domain::{OrderItemProjection, OrderProjection};
use erp_repo::{
    order_recheck_batches::{
        OmsRecheckBatchItemCreate, OmsRecheckBatchJobCreate, OmsRecheckBatchRepo,
    },
    orders::{OrderExceptionTicketUpdate, OrderReviewState, OrdersRepo},
};

mod annotations;
mod details;
mod dto;
mod exception_workbench;
mod fulfillment;
mod intake;
mod recovery;
mod review_config;
mod review_control;
mod review_flow;
mod settlement;
mod shared;
mod split_merge;
pub(super) use annotations::{
    create_order_tag_definition, create_sku_tag_definition, delete_order_tag_definition,
    delete_sku_tag_definition, list_order_tag_definitions, list_sku_tag_definitions,
    update_order_tag_definition, update_sku_tag_definition,
};
use details::*;
use dto::*;
use exception_workbench::*;
use fulfillment::{
    allocate_oms_order, create_oms_shipments, get_oms_shipment, get_order_raw,
    handle_oms_allocation, list_oms_order_shipments, list_oms_shipments, release_oms_order,
    request_oms_shipment_cancel, resend_oms_order, ship_oms_shipment, ship_order,
    submit_order_shipment,
};
use intake::*;
use review_config::*;
use review_control::*;
use review_flow::*;
use settlement::*;
use shared::*;
use split_merge::*;

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
            post(recovery::rematch_order_items),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/:order_id/return-to-download",
            post(recovery::return_order_to_download),
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
            post(annotations::add_order_tags),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/auto-tags/apply",
            post(annotations::apply_auto_tags),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/review-hits/export",
            post(review_flow::export_order_review_hits),
        )
        .route(
            "/v1/tenants/:tenant_id/orders/:order_id/notes",
            post(annotations::add_order_note),
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
            post(recovery::rematch_order_items),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id/return-to-download",
            post(recovery::return_order_to_download),
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
            post(annotations::add_order_tags),
        )
        .route(
            "/v1/tenants/:tenant_id/oms/orders/:order_id/notes",
            post(annotations::add_order_note),
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Extension,
        body::Body,
        http::{Request, StatusCode},
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
