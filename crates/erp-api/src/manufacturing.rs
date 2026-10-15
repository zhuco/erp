//! Manufacturing route registration. Handlers validate; planning stays in domain.

mod atomic_effects;
mod material_documents;
mod work_order_catalog;
mod material_batch_options;
mod operation_reports;
use atomic_effects::commit_atomic_effect_route;

mod inspection_outcomes;
mod command_readback;
mod receipt_dispositions;
mod cost_response;
mod cost_center_binding;
mod cost_read;

use super::*;
use erp_domain::manufacturing::{BomItem, DatedDemand, Execution, MrpItem, RoutingStep};

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/tenants/:tenant_id/manufacturing/work-order-options",get(work_order_catalog::list))
        .route("/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/material-context",get(work_order_catalog::context))
        .route("/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/material-batch-options",get(material_batch_options::list))
        .route("/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/material-documents",post(material_documents::save).get(material_documents::list))
        .route("/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/material-documents/:document_id",get(material_documents::read))
        .route("/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/material-documents/:document_id/confirm",post(material_documents::confirm))
        .route("/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/material-documents/:document_id/source-review",get(material_documents::review_source))
        .route("/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/material-documents/:document_id/source-adoptions",post(material_documents::adopt_source))
        .route("/v1/tenants/:tenant_id/quality/receipt-dispositions/:receipt_id",get(receipt_dispositions::read))
        .route("/v1/tenants/:tenant_id/manufacturing/boms", post(save_bom))
        .route(
            "/v1/tenants/:tenant_id/manufacturing/bom-structures",
            post(save_bom_structure_route).get(read_bom_explode_page_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/bom-versions",
            post(publish_bom_version_route).get(read_work_order_bom_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/routings",
            post(save_routing_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/routing-changes",
            post(save_routing_change_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/operation-routes",
            post(save_operation_route_handler).get(read_work_order_route_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/work-centers",
            post(save_work_center_handler).get(read_work_center_span_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/production-plans",
            post(save_production_plan_handler),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/mrp-buckets",
            get(read_bucket_suggestion_route).post(save_bucket_mrp_handler),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/mrp-runs",
            get(read_mrp_calendar_route).post(plan_mrp_run),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/executions",
            post(save_execution_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/work-orders",
            get(read_work_order_release_route).post(save_work_order_route),
        )
        .route("/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/reporting-context",get(operation_reports::context))
        .route("/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/reports/commands/:request_id",get(operation_reports::read))
        .route(
            "/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/reports",
            get(read_operation_report_route).post(save_operation_report_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/atomic-effects",
            post(commit_atomic_effect_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/receipts",
            post(save_production_receipt_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/receipts/:receipt_id/release",
            post(release_production_receipt_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/reissues",
            post(reissue_semi_finished_route),
        )
        .route("/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/cost-center-binding",post(cost_center_binding::post))
        .route("/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/costs/:cost_run_id",get(cost_read::read))
        .route(
            "/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/costs",
            post(confirm_work_order_cost_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/cost-adjustments",
            post(save_late_cost_adjustment_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/cost-estimates",
            post(save_missing_fee_estimate_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/cost-variances",
            post(save_confirmed_cost_variance_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/actions",
            post(transition_work_order_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/materials",
            get(read_material_history_route).post(save_material_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/exceptions",
            post(save_work_order_exception_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/exception-closes",
            post(close_work_order_after_exceptions_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/subcontracts",
            post(save_subcontract_order_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/subcontracts/:subcontract_id/moves",
            post(save_subcontract_move_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/substitutes",
            post(save_substitute_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/unit-substitutes",
            post(save_unit_substitute_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/engineering-changes",
            post(apply_engineering_change_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/engineering-impacts",
            get(read_engineering_impact_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/co-products",
            post(save_co_product_split_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/disassembly-recoveries",
            post(save_disassembly_recovery_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/backflushes",
            post(save_backflush_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/board-refreshes",
            post(enqueue_board_refresh_route),
        )
        .route(
            "/v1/tenants/:tenant_id/tasks",
            post(enqueue_durable_task_route).get(read_durable_task_page_route),
        )
        .route(
            "/v1/tenants/:tenant_id/tasks/:task_id/claims",
            post(claim_durable_task_route),
        )
        .route(
            "/v1/tenants/:tenant_id/tasks/:task_id/heartbeats",
            post(heartbeat_durable_task_route),
        )
        .route(
            "/v1/tenants/:tenant_id/tasks/:task_id/failures",
            post(fail_durable_task_route),
        )
        .route(
            "/v1/tenants/:tenant_id/tasks/:task_id/cancellations",
            post(cancel_durable_task_route),
        )
        .route(
            "/v1/tenants/:tenant_id/tasks/:task_id/restores",
            post(restore_dead_task_route),
        )
        .route(
            "/v1/tenants/:tenant_id/work/admissions",
            post(admit_tenant_work_route),
        )
        .route(
            "/v1/tenants/:tenant_id/health/signals",
            post(record_health_signal_route),
        )
        .route(
            "/v1/tenants/:tenant_id/documents",
            post(open_versioned_document_route),
        )
        .route(
            "/v1/tenants/:tenant_id/documents/:document_id",
            get(read_conflict_input_route).post(save_versioned_document_route),
        )
        .route(
            "/v1/tenants/:tenant_id/access/grants",
            post(grant_access_route),
        )
        .route(
            "/v1/tenants/:tenant_id/access/objects",
            post(register_access_object_route),
        )
        .route(
            "/v1/tenants/:tenant_id/access/actions",
            post(authorize_audited_action_route),
        )
        .route(
            "/v1/tenants/:tenant_id/access/frozen-scopes",
            post(freeze_task_scope_route),
        )
        .route(
            "/v1/tenants/:tenant_id/access/frozen-scopes/:task_id/downloads",
            post(authorize_frozen_download_route),
        )
        .route(
            "/v1/tenants/:tenant_id/access/warehouses/:warehouse_id/revocations",
            post(revoke_warehouse_scope_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/work-orders/:work_order_id/board-snapshots",
            get(read_board_snapshot_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/board-refreshes/:task_id/completions",
            post(complete_board_refresh_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/board-refreshes/:task_id/claims",
            post(claim_board_refresh_route),
        )
        .route(
            "/v1/tenants/:tenant_id/quality/inspection-templates",
            post(save_inspection_template_route),
        )
        .route(
            "/v1/tenants/:tenant_id/quality/inspection-freezes",
            post(freeze_inspection_template_route),
        )
        .route(
            "/v1/tenants/:tenant_id/quality/inspection-freezes/:freeze_id/readings",
            post(save_frozen_inspection_reading_route).get(inspection_outcomes::history_route),
        )
        .route(
            "/v1/tenants/:tenant_id/quality/inspection-freezes/:freeze_id/reinspections",
            post(inspection_outcomes::reinspect_route),
        )
        .route(
            "/v1/tenants/:tenant_id/quality/inspection-freezes/:freeze_id/readings/:item_id/reviews",
            post(inspection_outcomes::review_route),
        )
        .route(
            "/v1/tenants/:tenant_id/quality/receipts/:receipt_id/releases",
            post(release_after_quality_route),
        )
        .route(
            "/v1/tenants/:tenant_id/quality/dispositions",
            post(save_cited_quality_disposition_route),
        )
        .route(
            "/v1/tenants/:tenant_id/quality/holds",
            post(save_quality_hold_route),
        )
        .route(
            "/v1/tenants/:tenant_id/quality/genealogy-edges",
            post(save_genealogy_edge_route).get(read_genealogy_keyset_route),
        )
        .route(
            "/v1/tenants/:tenant_id/quality/trace-exports",
            post(save_trace_export_scope_route),
        )
        .route(
            "/v1/tenants/:tenant_id/quality/sampling-rules",
            post(save_sampling_rule_route),
        )
        .route(
            "/v1/tenants/:tenant_id/quality/sampling-decisions",
            post(save_cited_sampling_decision_route),
        )
        .route(
            "/v1/tenants/:tenant_id/quality/sampling-trends",
            get(read_sampling_trend_route),
        )
        .route(
            "/v1/tenants/:tenant_id/quality/recalls",
            post(save_scoped_recall_route),
        )
        .route(
            "/v1/tenants/:tenant_id/quality/recall-tasks",
            post(enqueue_recall_route),
        )
        .route(
            "/v1/tenants/:tenant_id/quality/recall-tasks/:task_id/completions",
            post(complete_recall_route),
        )
        .route(
            "/v1/tenants/:tenant_id/quality/recalls/:recall_id/results",
            post(save_cited_recall_result_route),
        )
        .route(
            "/v1/tenants/:tenant_id/finance/document-balances",
            post(confirm_cited_document_balance_route),
        )
        .route(
            "/v1/tenants/:tenant_id/finance/opening-balances",
            post(save_opening_balance_route),
        )
        .route(
            "/v1/tenants/:tenant_id/finance/cash-entries",
            post(save_cash_entry_route),
        )
        .route(
            "/v1/tenants/:tenant_id/finance/tax-cost-rules",
            post(save_tax_cost_rule_route),
        )
        .route(
            "/v1/tenants/:tenant_id/finance/expense-allocations",
            post(save_cited_expense_allocation_route),
        )
        .route(
            "/v1/tenants/:tenant_id/finance/expense-adjustments",
            post(save_expense_adjustment_route),
        )
        .route(
            "/v1/tenants/:tenant_id/finance/basic-margins",
            get(read_basic_margin_route),
        )
        .route(
            "/v1/tenants/:tenant_id/finance/cost-facts",
            post(save_cost_fact_route),
        )
        .route(
            "/v1/tenants/:tenant_id/finance/cost-reconciliations",
            post(save_posted_cost_reconciliation_route),
        )
        .route(
            "/v1/tenants/:tenant_id/finance/cost-periods/closures",
            post(close_cost_period_route),
        )
        .route(
            "/v1/tenants/:tenant_id/finance/cost-period-adjustments",
            post(save_period_adjustment_route),
        )
        .route(
            "/v1/tenants/:tenant_id/finance/invoice-records",
            post(save_cited_invoice_route),
        )
        .route(
            "/v1/tenants/:tenant_id/finance/late-invoices",
            post(save_cited_late_invoice_route),
        )
        .route(
            "/v1/tenants/:tenant_id/finance/invoice-attachments",
            post(save_cited_invoice_attachment_route),
        )
        .route(
            "/v1/tenants/:tenant_id/finance/aging-snapshots",
            post(save_cited_aging_route).get(read_cited_aging_route),
        )
        .route(
            "/v1/tenants/:tenant_id/finance/standard-cost-snapshots",
            post(save_standard_cost_snapshot_route),
        )
        .route(
            "/v1/tenants/:tenant_id/finance/cited-cost-variances",
            post(save_cited_cost_variance_route),
        )
        .route(
            "/v1/tenants/:tenant_id/finance/operating-reports",
            post(save_operating_report_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manufacturing/reports",
            post(save_manufacturing_report_route),
        )
        .route(
            "/v1/tenants/:tenant_id/exports/cited-jobs",
            post(save_cited_export_route),
        )
        .route(
            "/v1/tenants/:tenant_id/exports/jobs/:job_id/downloads",
            post(authorize_export_download_route),
        )
        .route(
            "/v1/tenants/:tenant_id/exports/jobs/:job_id/read-cancellations",
            post(cancel_export_read_route),
        )
        .route(
            "/v1/tenants/:tenant_id/exports/jobs/:job_id/expirations",
            post(mark_export_expired_route),
        )
        .route(
            "/v1/tenants/:tenant_id/tasks/exceptions",
            post(save_task_exception_route),
        )
        .route(
            "/v1/tenants/:tenant_id/tasks/exceptions/:exception_id/actions",
            post(act_on_task_exception_route),
        )
        .route(
            "/v1/tenants/:tenant_id/tasks/exceptions/:exception_id/effects",
            post(save_task_effect_route),
        )
        .route(
            "/v1/tenants/:tenant_id/tasks/exceptions/:exception_id/cancellations",
            post(cancel_committed_task_route),
        )
        .route(
            "/v1/tenants/:tenant_id/tasks/effects",
            get(read_task_effect_route),
        )
        .route(
            "/v1/tenants/:tenant_id/tasks/dead-letters",
            post(post_dead_letter_once_route).get(read_dead_letter_posting_route),
        )
        .route(
            "/v1/tenants/:tenant_id/reports/saved-views",
            post(save_report_view_route),
        )
        .route(
            "/v1/tenants/:tenant_id/reports/field-catalogs",
            post(save_report_field_catalog_route),
        )
        .route(
            "/v1/tenants/:tenant_id/reports/saved-views/:view_id/stale-fields",
            post(mark_cited_report_stale_route),
        )
        .route(
            "/v1/tenants/:tenant_id/reports/saved-views/:view_id/restores",
            post(restore_cited_report_pair_route),
        )
        .route(
            "/v1/tenants/:tenant_id/reports/saved-views/:view_id/trends",
            post(save_saved_view_trend_route),
        )
        .route(
            "/v1/tenants/:tenant_id/reports/saved-views/:view_id/shares",
            post(share_saved_report_view_route).get(read_shared_report_view_route),
        )
        .route(
            "/v1/tenants/:tenant_id/finance/rebuilt-balances",
            get(read_rebuilt_balances_route),
        )
        .route(
            "/v1/tenants/:tenant_id/commands",
            post(open_command_route),
        )
        .route(
            "/v1/tenants/:tenant_id/commands/:request_id",
            get(read_command_route).post(advance_command_route),
        )
        .route(
            "/v1/tenants/:tenant_id/commands/:request_id/restarts",
            get(read_restarted_command_route),
        )
        .route(
            "/v1/tenants/:tenant_id/commands/:request_id/window-closes",
            get(read_window_close_route).post(record_window_close_route),
        )
        .route(
            "/v1/tenants/:tenant_id/commands/:request_id/lost-lookups",
            post(lookup_lost_command_route),
        )
}

#[derive(Debug, Deserialize)]
struct BomLineRequest {
    parent: String,
    component: String,
    qty_per: String,
}

#[derive(Debug, Deserialize)]
struct BomSaveRequest {
    request_id: String,
    product_code: String,
    items: Vec<BomLineRequest>,
}

#[derive(Debug, Deserialize)]
struct RoutingLineRequest {
    operation: String,
    sequence: i32,
    work_center: String,
    minutes_per: i64,
    inspection_required: bool,
}

#[derive(Debug, Deserialize)]
struct RoutingSaveRequest {
    request_id: String,
    product_code: String,
    steps: Vec<RoutingLineRequest>,
}

#[derive(Debug, Deserialize)]
struct MrpStockRequest {
    code: String,
    on_hand: String,
    allocated_elsewhere: String,
    supply_on_time: String,
    supply_late: String,
    safety: String,
}

#[derive(Debug, Deserialize)]
struct MrpPlanRequest {
    request_id: String,
    demand_code: String,
    demand_qty: String,
    need_date: Option<String>,
    bom: Vec<BomLineRequest>,
    stock: Vec<MrpStockRequest>,
}

#[derive(Debug, Deserialize)]
struct ExecutionSaveRequest {
    request_id: String,
    work_order_no: String,
    issued: String,
    consumed: String,
    good: String,
    rejected: String,
    received: String,
    material_cost_cent: i64,
    labor_minutes: i64,
    labor_rate_cent: i64,
}

#[derive(Debug, Deserialize)]
struct WorkOrderSaveRequest {
    request_id: String,
    work_order_no: String,
    product_code: String,
    bom_id: Option<String>,
    order_qty: String,
}

#[derive(Debug, Deserialize)]
struct WorkOrderActionRequest {
    request_id: String,
    action: String,
    version_id: Option<String>,
    warehouse_id: Option<String>,
    sku_id: Option<String>,
    qty: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MaterialMoveRequest {
    request_id: String,
    kind: String,
    qty: String,
    reason: Option<String>,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    batch_id: Option<uuid::Uuid>,
    original_issue_id: Option<uuid::Uuid>,
}

#[derive(Debug, Deserialize)]
struct WorkOrderReleaseQuery {
    work_order_id: String,
}

async fn read_work_order_release_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Query(query): Query<WorkOrderReleaseQuery>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let Ok(work_order_id) = uuid::Uuid::parse_str(query.work_order_id.trim()) else {
        return bad_request("bad work_order_id").into_response();
    };
    match SharedContractRepo::new(state.db.pool())
        .read_work_order_release(tenant_id, work_order_id)
        .await
    {
        Ok((version_id, version_no, reserved, release_status, order_status)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "version_id": version_id,
                "version_no": version_no,
                "reserved_micros": reserved,
                "release_status": release_status,
                "order_status": order_status,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn save_work_order_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WorkOrderSaveRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let order_qty = match erp_domain::quantity::parse_qty(&req.order_qty) {
        Ok(qty) => qty,
        Err(_) => return bad_request("bad order_qty").into_response(),
    };
    let bom_id = match req
        .bom_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(value) => match uuid::Uuid::parse_str(value) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad bom_id").into_response(),
        },
        None => None,
    };
    let payload = format!(
        "{}:{}:{}:{}",
        req.work_order_no.trim(),
        req.product_code.trim(),
        req.bom_id.as_deref().unwrap_or(""),
        req.order_qty.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .save_work_order(
            tenant_id,
            &request_id,
            &payload,
            req.work_order_no.trim(),
            req.product_code.trim(),
            bom_id,
            order_qty,
        )
        .await
    {
        Ok(Ok(effect)) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn transition_work_order_route(
    State(state): State<AppState>,
    Path((tenant_id, work_order_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WorkOrderActionRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let work_order_id = match uuid::Uuid::parse_str(&work_order_id) {
        Ok(id) => id,
        Err(_) => return bad_request("bad work_order_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let action = req.action.trim();
    if action == "complete" {
        if let Some(qty_text) = req
            .qty
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let qty = match erp_domain::quantity::parse_qty(qty_text) {
                Ok(qty) => qty,
                Err(_) => return bad_request("bad qty").into_response(),
            };
            let payload = format!("{work_order_id}:complete-partial:{}", qty.as_micros());
            return match SharedContractRepo::new(state.db.pool())
                .record_partial_completion(
                    tenant_id,
                    &request_id,
                    &payload,
                    work_order_id,
                    qty.as_micros(),
                )
                .await
            {
                Ok(Ok((effect, completed))) => (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "request_id": request_id,
                        "status": "SUCCEEDED",
                        "replay": matches!(effect, SharedWrite::Replay(_)),
                        "work_order_status": "RELEASED",
                        "completed_micros": completed,
                    })),
                )
                    .into_response(),
                Ok(Err(_)) => conflict("payload conflict").into_response(),
                Err(error) => {
                    (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response()
                }
            };
        }
    }
    if action == "release" {
        let version_id = match req
            .version_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(uuid::Uuid::parse_str)
        {
            Some(Ok(id)) => id,
            _ => return bad_request("bad version_id").into_response(),
        };
        let warehouse_id = match req
            .warehouse_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(uuid::Uuid::parse_str)
        {
            Some(Ok(id)) => id,
            _ => return bad_request("bad warehouse_id").into_response(),
        };
        let sku_id = match req
            .sku_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(uuid::Uuid::parse_str)
        {
            Some(Ok(id)) => id,
            _ => return bad_request("bad sku_id").into_response(),
        };
        if let Err(resp) = ensure_warehouse_access(&ctx, warehouse_id) {
            return resp.into_response();
        }
        let qty = match req.qty.as_deref().map(erp_domain::quantity::parse_qty) {
            Some(Ok(qty)) => qty,
            _ => return bad_request("bad qty").into_response(),
        };
        let payload = format!(
            "{work_order_id}:release:{version_id}:{warehouse_id}:{sku_id}:{}",
            qty.as_micros()
        );
        return match SharedContractRepo::new(state.db.pool())
            .release_work_order(
                tenant_id,
                &request_id,
                &payload,
                work_order_id,
                version_id,
                warehouse_id,
                sku_id,
                qty.as_micros(),
            )
            .await
        {
            Ok(Ok((effect, reserved))) => (
                StatusCode::OK,
                Json(serde_json::json!({
                    "request_id": request_id,
                    "status": "SUCCEEDED",
                    "replay": matches!(effect, SharedWrite::Replay(_)),
                    "work_order_status": "RELEASED",
                    "reserved_micros": reserved,
                })),
            )
                .into_response(),
            Ok(Err(_)) => conflict("payload conflict").into_response(),
            Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
        };
    }
    let payload = format!("{work_order_id}:{action}");
    match SharedContractRepo::new(state.db.pool())
        .transition_saved_work_order(tenant_id, &request_id, &payload, work_order_id, action)
        .await
    {
        Ok(Ok((effect, status))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(effect, SharedWrite::Replay(_)),
                "work_order_status": status.as_str(),
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct MaterialHistoryQuery {
    offset: i64,
    limit: i64,
}

async fn read_material_history_route(
    State(state): State<AppState>,
    Path((tenant_id, work_order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Query(query): Query<MaterialHistoryQuery>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let Ok(work_order_id) = uuid::Uuid::parse_str(work_order_id.trim()) else {
        return bad_request("bad work_order_id").into_response();
    };
    match SharedContractRepo::new(state.db.pool())
        .read_material_history(tenant_id, work_order_id, query.offset, query.limit)
        .await
    {
        Ok((total, lines)) => {
            let lines = lines
                .into_iter()
                .map(|line| {
                    serde_json::json!({
                        "move_id": line.move_id,
                        "kind": line.kind,
                        "qty_micros": line.qty_micros,
                        "warehouse_id": line.warehouse_id,
                        "sku_id": line.sku_id,
                        "batch_id": line.batch_id,
                        "original_issue_id": line.original_issue_id,
                        "reason": line.reason,
                        "issued_micros": line.issued_micros,
                        "returned_micros": line.returned_micros,
                        "consumed_micros": line.consumed_micros,
                        "shop_micros": line.shop_micros,
                    })
                })
                .collect::<Vec<_>>();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "total": total,
                    "offset": query.offset,
                    "limit": query.limit,
                    "lines": lines,
                })),
            )
                .into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OperationReportRequest {
    request_id: String,
    operation_code: String,
    good_qty: String,
    rejected_qty: String,
    rework_qty: String,
    minutes: i64,
    operator_id: String,
    session_id: String,
    crew_code: String,
    started_at: String,
    ended_at: String,
    previous_good_qty: String,
    reverses_report_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OperationReportQuery {
    operation_code: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProductionReceiptRequest {
    request_id: String,
    qty: String,
    status: String,
    report_id: String,
    batch_code: String,
    inspection_id: String,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
}

async fn save_production_receipt_route(
    State(state): State<AppState>,
    Path((tenant_id, work_order_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProductionReceiptRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let work_order_id = match uuid::Uuid::parse_str(&work_order_id) {
        Ok(id) => id,
        Err(_) => return bad_request("bad work_order_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_warehouse_access(&ctx, req.warehouse_id) {
        return resp.into_response();
    }
    let qty = match erp_domain::quantity::parse_qty(&req.qty) {
        Ok(qty) => qty,
        Err(_) => return bad_request("bad qty").into_response(),
    };
    let status = match req.status.trim() {
        "HOLD" => erp_domain::manufacturing::ReceiptStatus::Hold,
        "RELEASED" => erp_domain::manufacturing::ReceiptStatus::Released,
        _ => return bad_request("bad status").into_response(),
    };
    let Ok(report_id) = uuid::Uuid::parse_str(req.report_id.trim()) else {
        return bad_request("bad report_id").into_response();
    };
    let Ok(inspection_id) = uuid::Uuid::parse_str(req.inspection_id.trim()) else {
        return bad_request("bad inspection_id").into_response();
    };
    let payload = match serde_json::to_string(&(
        1,
        work_order_id,
        qty.as_micros(),
        status.as_str(),
        report_id,
        req.batch_code.trim(),
        inspection_id,
        req.warehouse_id,
        req.sku_id,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_production_receipt(
            tenant_id,
            &request_id,
            &payload,
            work_order_id,
            qty,
            status,
            report_id,
            req.batch_code.trim(),
            inspection_id,
            req.warehouse_id,
            req.sku_id,
        )
        .await
    {
        Ok(Ok((write, received, available))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
                "received_micros": received,
                "available_micros": available,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProductionReleaseRequest {
    request_id: String,
}

async fn release_production_receipt_route(
    State(state): State<AppState>,
    Path((tenant_id, work_order_id, receipt_id)): Path<(String, String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Extension(request_ctx): Extension<RequestContext>,
    Json(req): Json<ProductionReleaseRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let work_order_id = match uuid::Uuid::parse_str(work_order_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad work_order_id").into_response(),
    };
    let receipt_id = match uuid::Uuid::parse_str(receipt_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad receipt_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = match serde_json::to_string(&(1, work_order_id, receipt_id)) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .release_production_receipt_with_actor(
            tenant_id, &request_id, &payload, receipt_id, Some(work_order_id),
            &ctx.production_release_actor(&request_ctx),
        )
        .await
    {
        Ok(Ok((write, available))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
                "available_micros": available,
                "available_micros_exact": available.to_string(),
                "quantity_scale": 1_000_000,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => production_release_error_response(error),
    }
}

fn production_release_error_response(error: anyhow::Error) -> Response {
    use erp_repo::shared_contract::ProductionReleaseError;
    let (status, code) = match error.downcast_ref::<ProductionReleaseError>() {
        Some(ProductionReleaseError::Forbidden) => {
            (StatusCode::FORBIDDEN, "warehouse_access_denied")
        }
        Some(ProductionReleaseError::NotFound) => {
            (StatusCode::NOT_FOUND, "production_receipt_not_found")
        }
        Some(ProductionReleaseError::InspectionBlocked) => {
            (StatusCode::CONFLICT, "inspection_blocked")
        }
        Some(ProductionReleaseError::InspectionLimit) => {
            (StatusCode::CONFLICT, "inspection_budget_exceeded")
        }
        Some(ProductionReleaseError::ScopeLimit) => {
            (StatusCode::FORBIDDEN, "warehouse_scope_budget_exceeded")
        }
        Some(ProductionReleaseError::ReconciliationRequired(_)) => (
            StatusCode::CONFLICT,
            "production_release_reconciliation_required",
        ),
        None if error
            .downcast_ref::<erp_repo::inventory_precision::InventoryPrecisionError>()
            .is_some() =>
        {
            (StatusCode::CONFLICT, "inventory_reconciliation_required")
        }
        None => {
            tracing::error!(error = ?error, "production receipt release transaction failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "production_release_storage_failed",
            )
        }
    };
    (status, Json(serde_json::json!({"error":{"code":code}}))).into_response()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SemiReissueRequest {
    request_id: String,
    semi_receipt_id: String,
    qty: String,
    warehouse_id: uuid::Uuid,
    location_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    batch_code: String,
}

async fn reissue_semi_finished_route(
    State(state): State<AppState>,
    Path((tenant_id, work_order_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SemiReissueRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let work_order_id = match uuid::Uuid::parse_str(work_order_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad work_order_id").into_response(),
    };
    let semi_receipt_id = match uuid::Uuid::parse_str(req.semi_receipt_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad semi_receipt_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_warehouse_access(&ctx, req.warehouse_id) {
        return resp.into_response();
    }
    let qty = match erp_domain::quantity::parse_qty(&req.qty) {
        Ok(qty) => qty,
        Err(_) => return bad_request("bad qty").into_response(),
    };
    let payload = match serde_json::to_string(&(
        1,
        work_order_id,
        semi_receipt_id,
        qty.as_micros(),
        req.warehouse_id,
        req.location_id,
        req.sku_id,
        req.batch_code.trim(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .reissue_semi_finished(
            tenant_id,
            &request_id,
            &payload,
            work_order_id,
            semi_receipt_id,
            qty,
            req.warehouse_id,
            req.location_id,
            req.sku_id,
            req.batch_code.trim(),
        )
        .await
    {
        Ok(Ok((write, left))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
                "left_micros": left,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkOrderCostRequest {
    request_id: String,
    item_code: String,
    center_id: uuid::Uuid,
    as_of: String,
    overhead_cent: Option<i64>,
    completed_qty: String,
    rejected_qty: String,
    unfinished_qty: String,
}

async fn confirm_work_order_cost_route(
    State(state): State<AppState>,
    Path((tenant_id, work_order_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    epoch: Option<Extension<VerifiedCredentialEpoch>>,
    Json(req): Json<WorkOrderCostRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let work_order_id = match uuid::Uuid::parse_str(work_order_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad work_order_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let completed = match erp_domain::quantity::parse_qty(&req.completed_qty) {
        Ok(qty) => qty,
        Err(_) => return bad_request("bad completed_qty").into_response(),
    };
    let rejected = match erp_domain::quantity::parse_qty(&req.rejected_qty) {
        Ok(qty) => qty,
        Err(_) => return bad_request("bad rejected_qty").into_response(),
    };
    let unfinished = match erp_domain::quantity::parse_qty(&req.unfinished_qty) {
        Ok(qty) => qty,
        Err(_) => return bad_request("bad unfinished_qty").into_response(),
    };
    let payload = match serde_json::to_string(&(
        1,
        work_order_id,
        req.item_code.trim(),
        req.center_id,
        req.as_of.trim(),
        req.overhead_cent,
        completed.as_micros(),
        rejected.as_micros(),
        unfinished.as_micros(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    let (Some(session_id),Some(credential_epoch))=(ctx.session_id,epoch.and_then(|e|e.0.0)) else {return (StatusCode::UNAUTHORIZED,"current cost session required").into_response();};
    let actor=erp_repo::shared_contract::CostValueActor{authentication_tenant:ctx.tenant_id,user_id:ctx.user_id,session_id,credential_epoch};
    match SharedContractRepo::new(state.db.pool())
        .confirm_work_order_cost_as_actor(tenant_id,&request_id,&payload,erp_repo::shared_contract::WorkOrderCostInput{work_order_id,item_code:req.item_code.trim(),center_id:req.center_id,as_of:req.as_of.trim(),overhead_cent:req.overhead_cent,completed_qty:completed,rejected_qty:rejected,unfinished_qty:unfinished},actor)
        .await
    {
        Ok(Ok((write, total, unit, estimated))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
                "total_cent": total,
                "unit_cent": unit,
                "estimated": estimated,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => cost_response::error(error),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LateCostRequest {
    request_id: String,
    fee_cent: i64,
}

async fn save_late_cost_adjustment_route(
    State(state): State<AppState>,
    Path((tenant_id, work_order_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LateCostRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let work_order_id = match uuid::Uuid::parse_str(work_order_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad work_order_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = match serde_json::to_string(&(1, work_order_id, req.fee_cent)) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_late_cost_adjustment(tenant_id, &request_id, &payload, work_order_id, req.fee_cent)
        .await
    {
        Ok(Ok((write, fee))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
                "fee_cent": fee,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MissingFeeEstimateRequest {
    request_id: String,
    item_code: String,
    center_id: uuid::Uuid,
    as_of: String,
    completed_qty: String,
}

async fn save_missing_fee_estimate_route(
    State(state): State<AppState>,
    Path((tenant_id, work_order_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    epoch: Option<Extension<VerifiedCredentialEpoch>>,
    Json(req): Json<MissingFeeEstimateRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let work_order_id = match uuid::Uuid::parse_str(work_order_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad work_order_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let completed = match erp_domain::quantity::parse_qty(&req.completed_qty) {
        Ok(qty) => qty,
        Err(_) => return bad_request("bad completed_qty").into_response(),
    };
    let payload = match serde_json::to_string(&(
        1,
        work_order_id,
        req.item_code.trim(),
        req.center_id,
        req.as_of.trim(),
        completed.as_micros(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    let (Some(session_id),Some(credential_epoch))=(ctx.session_id,epoch.and_then(|e|e.0.0)) else {return (StatusCode::UNAUTHORIZED,"current cost session required").into_response();};
    let actor=erp_repo::shared_contract::CostValueActor{authentication_tenant:ctx.tenant_id,user_id:ctx.user_id,session_id,credential_epoch};
    match SharedContractRepo::new(state.db.pool())
        .save_missing_fee_estimate_as_actor(tenant_id,&request_id,&payload,erp_repo::shared_contract::WorkOrderCostInput{work_order_id,item_code:req.item_code.trim(),center_id:req.center_id,as_of:req.as_of.trim(),overhead_cent:None,completed_qty:completed,rejected_qty:erp_domain::quantity::ScaledQty::ZERO,unfinished_qty:erp_domain::quantity::ScaledQty::ZERO},actor)
        .await
    {
        Ok(Ok((write, total, unit, estimated))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
                "total_cent": total,
                "unit_cent": unit,
                "estimated": estimated,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => cost_response::error(error),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfirmedVarianceRequest {
    request_id: String,
    standard_id: uuid::Uuid,
}

async fn save_confirmed_cost_variance_route(
    State(state): State<AppState>,
    Path((tenant_id, work_order_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ConfirmedVarianceRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let work_order_id = match uuid::Uuid::parse_str(work_order_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad work_order_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = match serde_json::to_string(&(1, work_order_id, req.standard_id)) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_confirmed_cost_variance(
            tenant_id,
            &request_id,
            &payload,
            work_order_id,
            req.standard_id,
        )
        .await
    {
        Ok(Ok((write, variance_id, total, estimated))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": variance_id,
                "variance_cent": total,
                "estimated": estimated,
                "stored": matches!(write, SharedWrite::Stored(_)),
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkOrderExceptionRequest {
    request_id: String,
    kind: String,
    qty: String,
    reason: String,
    cost_cent: i64,
    source_work_order_id: Option<uuid::Uuid>,
    added_minutes: i64,
}

async fn save_work_order_exception_route(
    State(state): State<AppState>,
    Path((tenant_id, work_order_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WorkOrderExceptionRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let work_order_id = match uuid::Uuid::parse_str(work_order_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad work_order_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let qty = match erp_domain::quantity::parse_qty(&req.qty) {
        Ok(qty) => qty,
        Err(_) => return bad_request("bad qty").into_response(),
    };
    let payload = match serde_json::to_string(&(
        1,
        work_order_id,
        req.kind.trim(),
        qty.as_micros(),
        req.reason.trim(),
        req.cost_cent,
        req.source_work_order_id,
        req.added_minutes,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_work_order_exception(
            tenant_id,
            &request_id,
            &payload,
            work_order_id,
            req.kind.trim(),
            qty,
            req.reason.trim(),
            req.cost_cent,
            req.source_work_order_id,
            req.added_minutes,
        )
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExceptionCloseRequest {
    request_id: String,
}

async fn close_work_order_after_exceptions_route(
    State(state): State<AppState>,
    Path((tenant_id, work_order_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ExceptionCloseRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let work_order_id = match uuid::Uuid::parse_str(work_order_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad work_order_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = match serde_json::to_string(&(1, work_order_id, "exception-close")) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .close_work_order_after_exceptions(tenant_id, &request_id, &payload, work_order_id)
        .await
    {
        Ok(Ok((write, status))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": work_order_id,
                "work_order_status": status.as_str(),
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SubcontractOrderRequest {
    request_id: String,
    work_order_id: uuid::Uuid,
    supplier_id: uuid::Uuid,
    component_code: String,
    our_material: bool,
}

async fn save_subcontract_order_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SubcontractOrderRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = match serde_json::to_string(&(
        1,
        req.work_order_id,
        req.supplier_id,
        req.component_code.trim(),
        req.our_material,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_subcontract_order(
            tenant_id,
            &request_id,
            &payload,
            req.work_order_id,
            req.supplier_id,
            req.component_code.trim(),
            req.our_material,
        )
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SubcontractMoveRequest {
    request_id: String,
    kind: String,
    qty: String,
    fee_cent: i64,
    inspection: Option<String>,
    period_key: Option<String>,
}

async fn save_subcontract_move_route(
    State(state): State<AppState>,
    Path((tenant_id, subcontract_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SubcontractMoveRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let subcontract_id = match uuid::Uuid::parse_str(subcontract_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad subcontract_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let qty = match erp_domain::quantity::parse_qty(&req.qty) {
        Ok(qty) => qty,
        Err(_) => return bad_request("bad qty").into_response(),
    };
    let period_key = req.period_key.as_deref().unwrap_or("").trim().to_string();
    let payload = match serde_json::to_string(&(
        1,
        subcontract_id,
        req.kind.trim(),
        qty.as_micros(),
        req.fee_cent,
        req.inspection.as_deref().unwrap_or("").trim(),
        period_key.as_str(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_subcontract_move_in_period(
            tenant_id,
            &request_id,
            &payload,
            subcontract_id,
            req.kind.trim(),
            qty,
            req.fee_cent,
            req.inspection.as_deref().map(str::trim).filter(|value| !value.is_empty()),
            &period_key,
            None,
            None,
            None,
            None,
        )
        .await
    {
        Ok(Ok((write, at_supplier))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
                "at_supplier_micros": at_supplier,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SubstituteRequest {
    request_id: String,
    original_code: String,
    replacement_code: String,
    ratio: String,
    effective_from: String,
    effective_to: String,
    approver: String,
}

async fn save_substitute_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SubstituteRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let ratio = match erp_domain::quantity::parse_qty(&req.ratio) {
        Ok(ratio) => ratio,
        Err(_) => return bad_request("bad ratio").into_response(),
    };
    let payload = match serde_json::to_string(&(
        1,
        req.original_code.trim(),
        req.replacement_code.trim(),
        ratio.as_micros(),
        req.effective_from.trim(),
        req.effective_to.trim(),
        req.approver.trim(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_substitute(
            tenant_id,
            &request_id,
            &payload,
            req.original_code.trim(),
            req.replacement_code.trim(),
            ratio,
            req.effective_from.trim(),
            req.effective_to.trim(),
            req.approver.trim(),
        )
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UnitSubstituteRequest {
    request_id: String,
    original_code: String,
    replacement_code: String,
    original_unit: String,
    replacement_unit: String,
    effective_from: String,
    effective_to: String,
    approver: String,
}

async fn save_unit_substitute_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<UnitSubstituteRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = match serde_json::to_string(&(
        1,
        req.original_code.trim(),
        req.replacement_code.trim(),
        req.original_unit.trim(),
        req.replacement_unit.trim(),
        req.effective_from.trim(),
        req.effective_to.trim(),
        req.approver.trim(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_unit_substitute(
            tenant_id,
            &request_id,
            &payload,
            req.original_code.trim(),
            req.replacement_code.trim(),
            req.original_unit.trim(),
            req.replacement_unit.trim(),
            req.effective_from.trim(),
            req.effective_to.trim(),
            req.approver.trim(),
        )
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EngineeringChangeRequest {
    request_id: String,
    substitute_id: uuid::Uuid,
    need_date: String,
}

async fn apply_engineering_change_route(
    State(state): State<AppState>,
    Path((tenant_id, work_order_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<EngineeringChangeRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let work_order_id = match uuid::Uuid::parse_str(work_order_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad work_order_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload =
        match serde_json::to_string(&(1, work_order_id, req.substitute_id, req.need_date.trim())) {
            Ok(payload) => payload,
            Err(error) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
            }
        };
    match SharedContractRepo::new(state.db.pool())
        .apply_engineering_change(
            tenant_id,
            &request_id,
            &payload,
            work_order_id,
            req.substitute_id,
            req.need_date.trim(),
        )
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CoProductSplitRequest {
    request_id: String,
    total_cent: i64,
    primary_code: String,
    primary_weight: i64,
    co_code: String,
    co_weight: i64,
}

async fn save_co_product_split_route(
    State(state): State<AppState>,
    Path((tenant_id, work_order_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CoProductSplitRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let work_order_id = match uuid::Uuid::parse_str(work_order_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad work_order_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = match serde_json::to_string(&(
        1,
        work_order_id,
        req.total_cent,
        req.primary_code.trim(),
        req.primary_weight,
        req.co_code.trim(),
        req.co_weight,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_co_product_split(
            tenant_id,
            &request_id,
            &payload,
            work_order_id,
            req.total_cent,
            req.primary_code.trim(),
            req.primary_weight,
            req.co_code.trim(),
            req.co_weight,
        )
        .await
    {
        Ok(Ok((write, primary_cent, co_cent))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
                "primary_cent": primary_cent,
                "co_cent": co_cent,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DisassemblyRecoveryRequest {
    request_id: String,
    source_output: String,
    component_code: String,
    qty: String,
}

async fn save_disassembly_recovery_route(
    State(state): State<AppState>,
    Path((tenant_id, work_order_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<DisassemblyRecoveryRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let work_order_id = match uuid::Uuid::parse_str(work_order_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad work_order_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let qty = match erp_domain::quantity::parse_qty(&req.qty) {
        Ok(qty) => qty,
        Err(_) => return bad_request("bad qty").into_response(),
    };
    let payload = match serde_json::to_string(&(
        1,
        work_order_id,
        req.source_output.trim(),
        req.component_code.trim(),
        qty.as_micros(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_disassembly_recovery(
            tenant_id,
            &request_id,
            &payload,
            work_order_id,
            req.source_output.trim(),
            req.component_code.trim(),
            qty,
            None,
            None,
            None,
            None,
        )
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BackflushRequest {
    request_id: String,
    standard_qty: String,
}

async fn save_backflush_route(
    State(state): State<AppState>,
    Path((tenant_id, work_order_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<BackflushRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let work_order_id = match uuid::Uuid::parse_str(work_order_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad work_order_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let standard_qty = match erp_domain::quantity::parse_qty(&req.standard_qty) {
        Ok(qty) => qty,
        Err(_) => return bad_request("bad standard_qty").into_response(),
    };
    let payload = match serde_json::to_string(&(1, work_order_id, standard_qty.as_micros())) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_backflush(
            tenant_id,
            &request_id,
            &payload,
            work_order_id,
            standard_qty,
        )
        .await
    {
        Ok(Ok((write, posted_micros))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
                "posted_micros": posted_micros,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BoardRefreshRequest {
    request_id: String,
}

async fn enqueue_board_refresh_route(
    State(state): State<AppState>,
    Path((tenant_id, work_order_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<BoardRefreshRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let work_order_id = match uuid::Uuid::parse_str(work_order_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad work_order_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = match serde_json::to_string(&(1, work_order_id)) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .enqueue_board_refresh(tenant_id, &request_id, &payload, work_order_id)
        .await
    {
        Ok(Ok((write, task_id))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": task_id,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct BoardSnapshotQuery {
    today: String,
    due_date: Option<String>,
    minutes_per_day: Option<i64>,
}

async fn read_board_snapshot_route(
    State(state): State<AppState>,
    Path((tenant_id, work_order_id)): Path<(String, String)>,
    Query(query): Query<BoardSnapshotQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let work_order_id = match uuid::Uuid::parse_str(work_order_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad work_order_id").into_response(),
    };
    if query.today.trim().len() != 10 {
        return bad_request("bad today").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_board_snapshot(tenant_id, work_order_id)
        .await
    {
        Ok(Some((
            plan,
            issued,
            consumed,
            good,
            rejected,
            received,
            shortage,
            load_minutes,
            estimate_status,
            estimated_date,
            late,
        ))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "work_order_id": work_order_id,
                "plan_qty_micros": plan,
                "issued_micros": issued,
                "consumed_micros": consumed,
                "good_micros": good,
                "rejected_micros": rejected,
                "received_micros": received,
                "shortage_micros": shortage,
                "load_minutes": load_minutes,
                "estimate_status": estimate_status,
                "estimated_date": estimated_date,
                "late": late,
                "as_of": query.today.trim(),
                "due_date": query.due_date.as_deref().map(str::trim),
                "minutes_per_day": query.minutes_per_day,
            })),
        )
            .into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "board snapshot is missing").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InspectionTemplateLineRequest {
    item_code: String,
    required: bool,
    min_qty: String,
    max_qty: String,
    sample_count: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InspectionTemplateRequest {
    request_id: String,
    template_code: String,
    kind: String,
    lines: Vec<InspectionTemplateLineRequest>,
}

async fn save_inspection_template_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<InspectionTemplateRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.template_code.trim().is_empty() || req.kind.trim().is_empty() || req.lines.is_empty() {
        return bad_request("bad inspection template").into_response();
    }
    let mut lines = Vec::with_capacity(req.lines.len());
    for line in &req.lines {
        let min = match erp_domain::quantity::parse_qty(&line.min_qty) {
            Ok(qty) => qty.as_micros(),
            Err(_) => return bad_request("bad min_qty").into_response(),
        };
        let max = match erp_domain::quantity::parse_qty(&line.max_qty) {
            Ok(qty) => qty.as_micros(),
            Err(_) => return bad_request("bad max_qty").into_response(),
        };
        lines.push(erp_domain::manufacturing::FrozenInspectionLine {
            item_code: line.item_code.trim().to_string(),
            required: line.required,
            min_micros: min,
            max_micros: max,
            sample_count: line.sample_count,
        });
    }
    let payload = match serde_json::to_string(&(
        1,
        req.template_code.trim(),
        req.kind.trim(),
        lines
            .iter()
            .map(|line| {
                (
                    line.item_code.as_str(),
                    line.required,
                    line.min_micros,
                    line.max_micros,
                    line.sample_count,
                )
            })
            .collect::<Vec<_>>(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_inspection_template(
            tenant_id,
            &request_id,
            &payload,
            req.template_code.trim(),
            req.kind.trim(),
            &lines,
        )
        .await
    {
        Ok(Ok((write, template_id, version_no))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": template_id,
                "version_no": version_no,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InspectionFreezeRequest {
    request_id: String,
    template_id: String,
    source_kind: String,
    source_id: String,
    item_code: String,
    batch_code: String,
}

fn inspection_write_error(error: anyhow::Error) -> Response {
    if let Some(kind) = error.downcast_ref::<erp_repo::shared_contract::InspectionOutcomeError>() {
        use erp_repo::shared_contract::InspectionOutcomeError;
        let status = match kind {
            InspectionOutcomeError::Forbidden => StatusCode::FORBIDDEN,
            InspectionOutcomeError::SourceClosed | InspectionOutcomeError::HistoryRequired => StatusCode::CONFLICT,
            InspectionOutcomeError::Limit => StatusCode::UNPROCESSABLE_ENTITY,
        };
        return (status, kind.to_string()).into_response();
    }
    if error.downcast_ref::<sqlx::Error>().is_some() {
        tracing::error!(error = %error, "Inspection persistence failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "inspection persistence failed",
        )
            .into_response()
    } else {
        (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response()
    }
}

async fn freeze_inspection_template_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<InspectionFreezeRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let template_id = match uuid::Uuid::parse_str(req.template_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad template_id").into_response(),
    };
    let source_id = match uuid::Uuid::parse_str(req.source_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad source_id").into_response(),
    };
    if req.source_kind.trim().is_empty()
        || req.item_code.trim().is_empty()
        || req.batch_code.trim().is_empty()
    {
        return bad_request("bad inspection freeze").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        template_id,
        req.source_kind.trim(),
        source_id,
        req.item_code.trim(),
        req.batch_code.trim(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .freeze_inspection_template(
            tenant_id,
            &request_id,
            &payload,
            template_id,
            req.source_kind.trim(),
            source_id,
            req.item_code.trim(),
            req.batch_code.trim(),
        )
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => inspection_write_error(error),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InspectionReadingRequest {
    request_id: String,
    item_code: String,
    reading_qty: Option<String>,
    inspector: String,
    reviewer: String,
}

async fn save_frozen_inspection_reading_route(
    State(state): State<AppState>,
    Path((tenant_id, freeze_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<InspectionReadingRequest>,
) -> impl IntoResponse {
    let tenant_id = match inspection_outcomes::quality_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp.into_response(),
    };
    if let Err(response) = inspection_outcomes::require_quality(&ctx, "quality.write") {
        return response.into_response();
    }
    let freeze_id = match uuid::Uuid::parse_str(freeze_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad freeze_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let reading = match req.reading_qty.as_deref() {
        Some(text) => match erp_domain::quantity::parse_qty(text) {
            Ok(qty) => Some(qty.as_micros()),
            Err(_) => return bad_request("bad reading_qty").into_response(),
        },
        None => None,
    };
    if req.item_code.trim().is_empty() || req.reviewer.trim().is_empty() {
        return bad_request("bad inspection reading").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        freeze_id,
        req.item_code.trim(),
        reading,
        req.inspector.trim(),
        req.reviewer.trim(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_inspection_outcome(
            tenant_id,
            &request_id,
            &payload,
            erp_repo::shared_contract::InspectionReadingInput {
                freeze_id,
                item_code: req.item_code.trim(),
                reading,
                inspector: req.inspector.trim(),
                declared_reviewer: req.reviewer.trim(),
                previous_item_id: None,
                reason: "",
            },
            inspection_outcomes::actor(&ctx),
        )
        .await
    {
        Ok(Ok(result)) => inspection_outcomes::outcome_response(&request_id, result, "reading"),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => inspection_write_error(error),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QualityHoldRequest {
    request_id: String,
    work_order_id: String,
    batch_code: String,
    qty: String,
}

async fn save_quality_hold_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<QualityHoldRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let work_order_id = match uuid::Uuid::parse_str(req.work_order_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad work_order_id").into_response(),
    };
    let qty = match erp_domain::quantity::parse_qty(&req.qty) {
        Ok(qty) => qty,
        Err(_) => return bad_request("bad qty").into_response(),
    };
    if req.batch_code.trim().is_empty() {
        return bad_request("bad quality hold").into_response();
    }
    let payload =
        match serde_json::to_string(&(1, work_order_id, req.batch_code.trim(), qty.as_micros())) {
            Ok(payload) => payload,
            Err(error) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
            }
        };
    match SharedContractRepo::new(state.db.pool())
        .save_quality_hold(
            tenant_id,
            &request_id,
            &payload,
            work_order_id,
            req.batch_code.trim(),
            qty,
        )
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QualityDispositionRequest {
    request_id: String,
    #[serde(default)]
    hold_id: Option<String>,
    #[serde(default)]
    receipt_id: Option<uuid::Uuid>,
    #[serde(default)]
    parent_disposition_id: Option<uuid::Uuid>,
    #[serde(default)]
    returned_source_move_id: Option<uuid::Uuid>,
    location_id: String,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    kind: String,
    qty: String,
    reason: String,
    #[serde(default)]
    rework_work_order_id: String,
}

async fn save_cited_quality_disposition_route(
    State(state): State<AppState>,
    Path(path): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    epoch: Option<Extension<VerifiedCredentialEpoch>>,
    Json(req): Json<QualityDispositionRequest>,
) -> impl IntoResponse {
    let tenant = match TenantId::from_str(&path) {
        Ok(id) => id,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if ctx.tenant_id != tenant { return forbidden("business tenant mismatch").into_response(); }
    let request = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        _ => return bad_request("request_id required").into_response(),
    };
    receipt_dispositions::post(&state, &ctx, epoch.and_then(|e| e.0.0), tenant, &request, &req).await
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GenealogyEdgeRequest {
    request_id: String,
    source_action: String,
    input_code: String,
    output_code: String,
    qty: String,
    work_order_no: String,
    output_batch: String,
    #[serde(default)]
    customer_code: String,
}

async fn save_genealogy_edge_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<GenealogyEdgeRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let qty = match erp_domain::quantity::parse_qty(&req.qty) {
        Ok(qty) => qty,
        Err(_) => return bad_request("bad qty").into_response(),
    };
    if req.source_action.trim().is_empty()
        || req.input_code.trim().is_empty()
        || req.output_code.trim().is_empty()
        || req.work_order_no.trim().is_empty()
        || req.output_batch.trim().is_empty()
    {
        return bad_request("bad genealogy edge").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        req.source_action.trim(),
        req.input_code.trim(),
        req.output_code.trim(),
        qty.as_micros(),
        req.work_order_no.trim(),
        req.output_batch.trim(),
        req.customer_code.trim(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_genealogy_edge(
            tenant_id,
            &request_id,
            &payload,
            req.source_action.trim(),
            req.input_code.trim(),
            req.output_code.trim(),
            qty,
            req.work_order_no.trim(),
            req.output_batch.trim(),
            req.customer_code.trim(),
        )
        .await
    {
        Ok(Ok((write, _edge_id))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScopedRecallRequest {
    request_id: String,
    input_batch: String,
    rule_code: String,
    rule_version: i32,
    stock_qty: String,
    node_cap: i64,
}

async fn save_scoped_recall_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ScopedRecallRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let stock_qty = match erp_domain::quantity::parse_qty(&req.stock_qty) {
        Ok(qty) => qty,
        Err(_) => return bad_request("bad stock qty").into_response(),
    };
    if req.input_batch.trim().is_empty()
        || req.rule_code.trim().is_empty()
        || req.rule_version <= 0
        || req.node_cap <= 0
    {
        return bad_request("bad recall scope").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        req.input_batch.trim(),
        req.rule_code.trim(),
        req.rule_version,
        stock_qty.as_micros(),
        req.node_cap,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_scoped_recall(
            tenant_id,
            &request_id,
            &payload,
            req.input_batch.trim(),
            req.rule_code.trim(),
            req.rule_version,
            stock_qty,
            req.node_cap,
        )
        .await
    {
        Ok(Ok((write, _recall_id, item_count, _frozen))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
                "item_count": item_count,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecallTaskRequest {
    request_id: String,
}

async fn enqueue_recall_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<RecallTaskRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = match serde_json::to_string(&(1, "recall:open")) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .enqueue_recall(tenant_id, &request_id, &payload)
        .await
    {
        Ok(Ok((write, task_id))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": task_id,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecallCompleteRequest {
    owner: String,
    input_batch: String,
    rule_code: String,
    rule_version: i32,
    stock_qty: String,
    node_cap: i64,
}

async fn complete_recall_route(
    State(state): State<AppState>,
    Path((tenant_id, task_id)): Path<(String, uuid::Uuid)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<RecallCompleteRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let stock_qty = match erp_domain::quantity::parse_qty(&req.stock_qty) {
        Ok(qty) => qty,
        Err(_) => return bad_request("bad stock qty").into_response(),
    };
    if req.owner.trim().is_empty()
        || req.input_batch.trim().is_empty()
        || req.rule_code.trim().is_empty()
        || req.rule_version <= 0
        || req.node_cap <= 0
    {
        return bad_request("bad recall completion").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .complete_recall(
            tenant_id,
            task_id,
            req.owner.trim(),
            req.input_batch.trim(),
            req.rule_code.trim(),
            req.rule_version,
            stock_qty,
            req.node_cap,
        )
        .await
    {
        Ok(recall_id) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "task_id": task_id,
                "status": "DONE",
                "effect_id": recall_id,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TaxCostRuleRequest {
    request_id: String,
    tax_is_cost: bool,
}

async fn save_tax_cost_rule_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<TaxCostRuleRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = match serde_json::to_string(&(1, req.tax_is_cost)) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_tax_cost_rule(tenant_id, &request_id, &payload, req.tax_is_cost)
        .await
    {
        Ok(Ok((write, tax_is_cost))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": "tax-rule",
                "tax_is_cost": tax_is_cost,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpenseWeight {
    target_code: String,
    weight: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpenseAllocationRequest {
    request_id: String,
    category: String,
    partner_code: String,
    purchase_id: uuid::Uuid,
    currency: String,
    gross_cent: i64,
    deductible_tax_cent: i64,
    weights: Vec<ExpenseWeight>,
}

async fn save_cited_expense_allocation_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ExpenseAllocationRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.category.trim().is_empty()
        || req.partner_code.trim().is_empty()
        || req.currency.trim().is_empty()
        || req.weights.is_empty()
        || req
            .weights
            .iter()
            .any(|item| item.target_code.trim().is_empty() || item.weight <= 0)
    {
        return bad_request("bad expense allocation").into_response();
    }
    let weights: Vec<(String, i64)> = req
        .weights
        .iter()
        .map(|item| (item.target_code.trim().to_string(), item.weight))
        .collect();
    let weight_refs: Vec<(&str, i64)> = weights
        .iter()
        .map(|item| (item.0.as_str(), item.1))
        .collect();
    let payload = match serde_json::to_string(&(
        1,
        req.category.trim(),
        req.partner_code.trim(),
        req.purchase_id,
        req.currency.trim(),
        req.gross_cent,
        req.deductible_tax_cent,
        &weights,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_cited_expense_allocation(erp_repo::shared_contract::CitedExpenseAllocationInput { tenant_id, request_id: &request_id, payload_hash: &payload, category: req.category.trim(), partner_code: req.partner_code.trim(), purchase_id: req.purchase_id, currency: req.currency.trim(), gross_cent: req.gross_cent, deductible_tax_cent: req.deductible_tax_cent, weights: &weight_refs })
        .await
    {
        Ok(Ok((write, expense_id, cost_cent, tax_is_cost))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": expense_id,
                "cost_cent": cost_cent,
                "tax_is_cost": tax_is_cost,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpenseAdjustmentRequest {
    request_id: String,
    expense_id: uuid::Uuid,
    amount_cent: i64,
    reason: String,
}

async fn save_expense_adjustment_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ExpenseAdjustmentRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.reason.trim().is_empty() || req.amount_cent == 0 {
        return bad_request("bad expense adjustment").into_response();
    }
    let payload =
        match serde_json::to_string(&(1, req.expense_id, req.amount_cent, req.reason.trim())) {
            Ok(payload) => payload,
            Err(error) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
            }
        };
    match SharedContractRepo::new(state.db.pool())
        .save_expense_adjustment(
            tenant_id,
            &request_id,
            &payload,
            req.expense_id,
            req.amount_cent,
            req.reason.trim(),
        )
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CashEntryRequest {
    request_id: String,
    direction: String,
    partner_code: String,
    currency: String,
    account_code: String,
    amount_cent: i64,
    balance_id: Option<uuid::Uuid>,
    reverses_settlement_id: Option<uuid::Uuid>,
}

async fn save_cash_entry_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CashEntryRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if !matches!(
        req.direction.as_str(),
        "RECEIPT" | "PAYMENT" | "REFUND" | "REVERSAL"
    ) || req.partner_code.trim().is_empty()
        || req.currency.trim().is_empty()
        || req.account_code.trim().is_empty()
        || req.amount_cent == 0
    {
        return bad_request("bad cash entry").into_response();
    }
    if req.direction == "REVERSAL"
        && (req.amount_cent >= 0
            || req.reverses_settlement_id.is_none()
            || req.balance_id.is_none())
    {
        return bad_request("bad cash reversal").into_response();
    }
    if req.direction != "REVERSAL" && req.amount_cent <= 0 {
        return bad_request("bad cash entry").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        req.direction.as_str(),
        req.partner_code.trim(),
        req.currency.trim(),
        req.account_code.trim(),
        req.amount_cent,
        req.balance_id,
        req.reverses_settlement_id,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_cash_entry(erp_repo::shared_contract::CashEntryInput { tenant_id, request_id: &request_id, payload_hash: &payload, direction: req.direction.as_str(), partner_code: req.partner_code.trim(), currency: req.currency.trim(), account_code: req.account_code.trim(), amount_cent: req.amount_cent, balance_id: req.balance_id, reverses_settlement_id: req.reverses_settlement_id })
        .await
    {
        Ok(Ok((write, effect_id, open_cent))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": effect_id,
                "open_cent": open_cent,
                "bank_executed": false,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OpeningBalanceRequest {
    request_id: String,
    kind: String,
    partner_code: String,
    currency: String,
    source_id: String,
    amount_cent: i64,
    due_date: String,
}

async fn save_opening_balance_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OpeningBalanceRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let kind = match req.kind.as_str() {
        "OPENING_AR" => erp_domain::trade::PartnerBalanceKind::OpeningReceivable,
        "OPENING_AP" => erp_domain::trade::PartnerBalanceKind::OpeningPayable,
        _ => return bad_request("bad opening kind").into_response(),
    };
    if req.partner_code.trim().is_empty()
        || req.currency.trim().is_empty()
        || req.source_id.trim().is_empty()
        || req.due_date.trim().is_empty()
        || req.amount_cent <= 0
    {
        return bad_request("bad opening balance").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        req.kind.as_str(),
        req.partner_code.trim(),
        req.currency.trim(),
        req.source_id.trim(),
        req.amount_cent,
        req.due_date.trim(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_opening_balance(erp_repo::shared_contract::OpeningBalanceInput { tenant_id, request_id: &request_id, payload_hash: &payload, kind, partner_code: req.partner_code.trim(), currency: req.currency.trim(), source_id: req.source_id.trim(), amount_cent: req.amount_cent, due_date: req.due_date.trim() })
        .await
    {
        Ok(Ok((write, balance_id))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": balance_id,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentBalanceRequest {
    request_id: String,
    document_id: uuid::Uuid,
    document_kind: String,
    due_date: String,
}

async fn confirm_cited_document_balance_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<DocumentBalanceRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if !matches!(req.document_kind.as_str(), "SALES" | "PURCHASE" | "CREDIT")
        || (req.document_kind != "PURCHASE" && req.due_date.trim().is_empty())
    {
        return bad_request("bad document balance").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        req.document_id,
        req.document_kind.as_str(),
        req.due_date.trim(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .confirm_cited_document_balance(
            tenant_id,
            &request_id,
            &payload,
            req.document_id,
            req.document_kind.as_str(),
            req.due_date.trim(),
        )
        .await
    {
        Ok(Ok((write, balance_id, open_cent))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": balance_id,
                "open_cent": open_cent,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CostFactRequest {
    request_id: String,
    period_code: String,
    bucket: String,
    amount_cent: i64,
    source_ref: String,
}

async fn save_cost_fact_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CostFactRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.period_code.trim().is_empty()
        || !matches!(req.bucket.as_str(), "INVENTORY" | "WIP" | "FINISHED" | "COGS")
        || req.source_ref.trim().is_empty()
        || req.amount_cent < 0
    {
        return bad_request("bad cost fact").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        req.period_code.trim(),
        req.bucket.as_str(),
        req.amount_cent,
        req.source_ref.trim(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_cost_fact(erp_repo::shared_contract::CostFactInput { tenant_id, request_id: &request_id, payload_hash: &payload, period_code: req.period_code.trim(), bucket: req.bucket.as_str(), amount_cent: req.amount_cent, source_ref: req.source_ref.trim() })
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CostBookAmount {
    bucket: String,
    book_cent: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PostedCostReconciliationRequest {
    request_id: String,
    period_code: String,
    currency: String,
    cutoff_at: String,
    books: Vec<CostBookAmount>,
}

async fn save_posted_cost_reconciliation_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PostedCostReconciliationRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.period_code.trim().is_empty()
        || req.currency.trim().is_empty()
        || req.cutoff_at.trim().is_empty()
        || req.books.is_empty()
        || req.books.iter().any(|item| {
            !matches!(item.bucket.as_str(), "INVENTORY" | "WIP" | "FINISHED" | "COGS")
                || item.book_cent < 0
        })
    {
        return bad_request("bad cost reconciliation").into_response();
    }
    let books: Vec<(String, i64)> = req
        .books
        .iter()
        .map(|item| (item.bucket.clone(), item.book_cent))
        .collect();
    let book_refs: Vec<(&str, i64)> = books.iter().map(|item| (item.0.as_str(), item.1)).collect();
    let payload = match serde_json::to_string(&(
        1,
        req.period_code.trim(),
        req.currency.trim(),
        req.cutoff_at.trim(),
        &books,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_posted_cost_reconciliation(erp_repo::shared_contract::PostedCostReconciliationInput { tenant_id, request_id: &request_id, payload_hash: &payload, period_code: req.period_code.trim(), currency: req.currency.trim(), cutoff_at: req.cutoff_at.trim(), books: &book_refs })
        .await
    {
        Ok(Ok((write, reconciliation_id, difference_cent))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": reconciliation_id,
                "difference_cent": difference_cent,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CostPeriodCloseRequest {
    request_id: String,
    period_code: String,
}

async fn close_cost_period_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CostPeriodCloseRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.period_code.trim().is_empty() {
        return bad_request("bad period close").into_response();
    }
    let payload = match serde_json::to_string(&(1, req.period_code.trim())) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .close_cost_period(tenant_id, &request_id, &payload, req.period_code.trim())
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": req.period_code.trim(),
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PeriodAdjustmentRequest {
    request_id: String,
    period_code: String,
    document_date: String,
    amount_cent: i64,
    reason: String,
}

async fn save_period_adjustment_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PeriodAdjustmentRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.period_code.trim().is_empty()
        || req.document_date.trim().is_empty()
        || req.reason.trim().is_empty()
        || req.amount_cent == 0
    {
        return bad_request("bad period adjustment").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        req.period_code.trim(),
        req.document_date.trim(),
        req.amount_cent,
        req.reason.trim(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_period_adjustment(erp_repo::shared_contract::PeriodAdjustmentInput { tenant_id, request_id: &request_id, payload_hash: &payload, period_code: req.period_code.trim(), document_date: req.document_date.trim(), amount_cent: req.amount_cent, reason: req.reason.trim() })
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CitedInvoiceRequest {
    request_id: String,
    invoice_number: String,
    issuer_code: String,
    invoice_date: String,
    balance_id: uuid::Uuid,
    requested_official: bool,
}

async fn save_cited_invoice_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CitedInvoiceRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.invoice_number.trim().is_empty()
        || req.issuer_code.trim().is_empty()
        || req.invoice_date.trim().is_empty()
    {
        return bad_request("bad invoice record").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        req.invoice_number.trim(),
        req.issuer_code.trim(),
        req.invoice_date.trim(),
        req.balance_id,
        req.requested_official,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_cited_invoice(erp_repo::shared_contract::CitedInvoiceInput { tenant_id, request_id: &request_id, payload_hash: &payload, number: req.invoice_number.trim(), issuer: req.issuer_code.trim(), invoice_date: req.invoice_date.trim(), balance_id: req.balance_id, requested_official: req.requested_official })
        .await
    {
        Ok(Ok((write, invoice_id, official_issued))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": invoice_id,
                "official_issued": official_issued,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CitedLateInvoiceRequest {
    request_id: String,
    invoice_number: String,
    issuer_code: String,
    invoice_date: String,
    balance_id: uuid::Uuid,
    requested_official: bool,
}

async fn save_cited_late_invoice_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CitedLateInvoiceRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.invoice_number.trim().is_empty()
        || req.issuer_code.trim().is_empty()
        || req.invoice_date.trim().is_empty()
    {
        return bad_request("bad late invoice").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        req.invoice_number.trim(),
        req.issuer_code.trim(),
        req.invoice_date.trim(),
        req.balance_id,
        req.requested_official,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_cited_late_invoice(erp_repo::shared_contract::CitedInvoiceInput { tenant_id, request_id: &request_id, payload_hash: &payload, number: req.invoice_number.trim(), issuer: req.issuer_code.trim(), invoice_date: req.invoice_date.trim(), balance_id: req.balance_id, requested_official: req.requested_official })
        .await
    {
        Ok(Ok((write, invoice_id, official_issued))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": invoice_id,
                "official_issued": official_issued,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CitedInvoiceAttachmentRequest {
    request_id: String,
    invoice_id: uuid::Uuid,
    file_name: String,
    digest: String,
    byte_count: i64,
}

async fn save_cited_invoice_attachment_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CitedInvoiceAttachmentRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.file_name.trim().is_empty() || req.digest.trim().is_empty() || req.byte_count <= 0 {
        return bad_request("bad invoice attachment").into_response();
    }
    let actor = ctx.user_id.to_string();
    let payload = match serde_json::to_string(&(
        1,
        req.invoice_id,
        req.file_name.trim(),
        req.digest.trim(),
        req.byte_count,
        actor.as_str(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_cited_invoice_attachment(
            tenant_id,
            &request_id,
            &payload,
            req.invoice_id,
            req.file_name.trim(),
            req.digest.trim(),
            req.byte_count,
            &actor,
        )
        .await
    {
        Ok(Ok((write, attachment_id, version_no))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": attachment_id,
                "version_no": version_no,
                "file_stored": false,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CitedAgingRequest {
    request_id: String,
    cutoff_date: String,
    balance_ids: Vec<uuid::Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StandardCostSnapshotRequest {
    request_id: String,
    work_order_id: uuid::Uuid,
    material_qty_micros: i64,
    material_price_cent: i64,
    labor_minutes: i64,
    labor_rate_cent: i64,
    overhead_rate_cent: i64,
}

async fn save_standard_cost_snapshot_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<StandardCostSnapshotRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.material_qty_micros < 0
        || req.material_price_cent < 0
        || req.labor_minutes < 0
        || req.labor_rate_cent < 0
        || req.overhead_rate_cent < 0
    {
        return bad_request("bad standard snapshot").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        req.work_order_id,
        req.material_qty_micros,
        req.material_price_cent,
        req.labor_minutes,
        req.labor_rate_cent,
        req.overhead_rate_cent,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_standard_cost_snapshot(
            tenant_id,
            &request_id,
            &payload,
            req.work_order_id,
            req.material_qty_micros,
            req.material_price_cent,
            req.labor_minutes,
            req.labor_rate_cent,
            req.overhead_rate_cent,
        )
        .await
    {
        Ok(Ok((write, standard_id, version_no))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": standard_id,
                "version_no": version_no,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CitedCostVarianceRequest {
    request_id: String,
    standard_id: uuid::Uuid,
    material_qty_micros: i64,
    material_price_cent: i64,
    labor_minutes: i64,
    labor_rate_cent: i64,
    overhead_cent: Option<i64>,
}

async fn save_cited_cost_variance_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CitedCostVarianceRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.material_qty_micros < 0
        || req.material_price_cent < 0
        || req.labor_minutes < 0
        || req.labor_rate_cent < 0
        || req.overhead_cent.is_some_and(|value| value < 0)
    {
        return bad_request("bad cited variance").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        req.standard_id,
        req.material_qty_micros,
        req.material_price_cent,
        req.labor_minutes,
        req.labor_rate_cent,
        req.overhead_cent,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_cited_cost_variance(
            tenant_id,
            &request_id,
            &payload,
            req.standard_id,
            req.material_qty_micros,
            req.material_price_cent,
            req.labor_minutes,
            req.labor_rate_cent,
            req.overhead_cent,
        )
        .await
    {
        Ok(Ok((write, variance_id, total_variance, estimated))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": variance_id,
                "variance_cent": total_variance,
                "estimated": estimated,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OperatingReportRequest {
    request_id: String,
    cutoff_at: String,
    date_from: Option<String>,
    date_to: Option<String>,
    partner_code: Option<String>,
    warehouse_id: Option<uuid::Uuid>,
    page_count: Option<i64>,
}

async fn save_operating_report_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OperatingReportRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.cutoff_at.trim().len() != 10 {
        return bad_request("bad operating report").into_response();
    }
    let date_from = req.date_from.as_deref().map(str::trim).filter(|value| !value.is_empty());
    let date_to = req.date_to.as_deref().map(str::trim).filter(|value| !value.is_empty());
    let partner = req.partner_code.as_deref().map(str::trim).filter(|value| !value.is_empty());
    if date_from.is_some_and(|value| value.len() != 10) || date_to.is_some_and(|value| value.len() != 10) {
        return bad_request("bad operating report").into_response();
    }
    if let Some(page_count) = req.page_count {
        if SharedContractRepo::new(state.db.pool())
            .refuse_page_operating_report(page_count, page_count.saturating_add(1))
            .await
            .is_err()
        {
            return bad_request("a selected page cannot replace the authorized range").into_response();
        }
    }
    let payload = match serde_json::to_string(&(
        1,
        req.cutoff_at.trim(),
        date_from,
        date_to,
        partner,
        req.warehouse_id,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_operating_report(
            tenant_id,
            &request_id,
            &payload,
            req.cutoff_at.trim(),
            date_from,
            date_to,
            partner,
            req.warehouse_id,
        )
        .await
    {
        Ok(Ok((write, run_id, lines))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": run_id,
                "lines": lines,
                "page_is_total": false,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManufacturingReportRequest {
    request_id: String,
    cutoff_at: String,
    work_order_id: Option<uuid::Uuid>,
}

async fn save_manufacturing_report_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ManufacturingReportRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.cutoff_at.trim().len() != 10 {
        return bad_request("bad manufacturing report").into_response();
    }
    let payload = match serde_json::to_string(&(1, req.cutoff_at.trim(), req.work_order_id)) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_manufacturing_report(
            tenant_id,
            &request_id,
            &payload,
            req.cutoff_at.trim(),
            req.work_order_id,
        )
        .await
    {
        Ok(Ok((write, run_id, lines))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": run_id,
                "lines": lines,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CitedExportRequest {
    request_id: String,
    snapshot_id: uuid::Uuid,
    output_schema: String,
    #[serde(default)]
    excluded: Vec<String>,
    failed_source: Option<String>,
    extra_source: Option<String>,
}

async fn save_cited_export_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CitedExportRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.output_schema.trim().is_empty() {
        return bad_request("bad export schema").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        req.snapshot_id,
        req.output_schema.trim(),
        &req.excluded,
        req.failed_source.as_deref().map(str::trim),
        req.extra_source.as_deref().map(str::trim),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_cited_export(
            tenant_id,
            &request_id,
            &payload,
            req.snapshot_id,
            req.output_schema.trim(),
            &req.excluded,
            req.failed_source.as_deref().map(str::trim).filter(|value| !value.is_empty()),
            req.extra_source.as_deref().map(str::trim).filter(|value| !value.is_empty()),
        )
        .await
    {
        Ok(Ok((write, job_id, items))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": job_id,
                "items": items,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportDownloadRequest {
    request_id: String,
}

async fn authorize_export_download_route(
    State(state): State<AppState>,
    Path((tenant_id, job_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ExportDownloadRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let job_id = match uuid::Uuid::parse_str(&job_id) {
        Ok(id) => id,
        Err(_) => return bad_request("bad job_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let actor = ctx.user_id.to_string();
    let payload = match serde_json::to_string(&(1, job_id, actor.trim())) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .authorize_export_download(tenant_id, &request_id, &payload, job_id, actor.trim())
        .await
    {
        Ok(Ok((write, authorized, content))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "authorized": authorized,
                "content": content,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn cancel_export_read_route(
    State(state): State<AppState>,
    Path((tenant_id, job_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ExportDownloadRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let job_id = match uuid::Uuid::parse_str(&job_id) {
        Ok(id) => id,
        Err(_) => return bad_request("bad job_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = match serde_json::to_string(&(1, job_id)) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .cancel_export_read(tenant_id, &request_id, &payload, job_id)
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": job_id,
                "job_removed": false,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn mark_export_expired_route(
    State(state): State<AppState>,
    Path((tenant_id, job_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ExportDownloadRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let job_id = match uuid::Uuid::parse_str(&job_id) {
        Ok(id) => id,
        Err(_) => return bad_request("bad job_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = match serde_json::to_string(&(1, job_id)) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .mark_export_expired(tenant_id, &request_id, &payload, job_id)
        .await
    {
        Ok(Ok(removed)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "removed_files": removed,
                "job_removed": false,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedReportViewRequest {
    request_id: String,
    report_scope: String,
    name: String,
    date_from: String,
    date_to: String,
    dimension: String,
    measure: String,
    chart_type: String,
    formula: String,
}

async fn save_report_view_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SavedReportViewRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let owner = ctx.user_id.to_string();
    if owner.trim().is_empty()
        || req.report_scope.trim().is_empty()
        || req.name.trim().is_empty()
        || req.formula.trim().is_empty()
    {
        return bad_request("bad saved report view").into_response();
    }
    let view = erp_domain::trade::SavedReportView {
        owner_user_id: owner,
        report_scope: req.report_scope.trim().to_string(),
        name: req.name.trim().to_string(),
        date_from: req.date_from.trim().to_string(),
        date_to: req.date_to.trim().to_string(),
        dimension: req.dimension.trim().to_string(),
        measure: req.measure.trim().to_string(),
        chart_type: req.chart_type.trim().to_string(),
        formula: req.formula.trim().to_string(),
    };
    let payload = match serde_json::to_string(&(
        1,
        view.owner_user_id.as_str(),
        view.report_scope.as_str(),
        view.name.as_str(),
        view.date_from.as_str(),
        view.date_to.as_str(),
        view.dimension.as_str(),
        view.measure.as_str(),
        view.chart_type.as_str(),
        view.formula.as_str(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_report_view(tenant_id, &request_id, &payload, &view)
        .await
    {
        Ok(Ok((write, view_id))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": view_id,
                "owner_from_request": false,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReportFieldCatalogRequest {
    request_id: String,
    report_scope: String,
    fields: Vec<String>,
}

async fn save_report_field_catalog_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ReportFieldCatalogRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.report_scope.trim().is_empty() || req.fields.is_empty() {
        return bad_request("bad report field catalog").into_response();
    }
    let fields: Vec<String> = req.fields.iter().map(|field| field.trim().to_string()).collect();
    let payload = match serde_json::to_string(&(1, req.report_scope.trim(), &fields)) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_report_field_catalog(
            tenant_id,
            &request_id,
            &payload,
            req.report_scope.trim(),
            &fields,
        )
        .await
    {
        Ok(Ok((write, count))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "field_count": count,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReportStaleRequest {
    request_id: String,
    caller_fields: Vec<String>,
}

async fn mark_cited_report_stale_route(
    State(state): State<AppState>,
    Path((tenant_id, view_id)): Path<(String, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ReportStaleRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = match serde_json::to_string(&(1, view_id)) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .mark_cited_report_stale(
            tenant_id,
            &request_id,
            &payload,
            view_id,
            &req.caller_fields,
        )
        .await
    {
        Ok(Ok((write, stale_count))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": view_id,
                "stale_count": stale_count,
                "formula_changed": false,
                "caller_fields_used": false,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReportRestoreRequest {
    request_id: String,
    session_id: uuid::Uuid,
    caller_query: String,
}

async fn restore_cited_report_pair_route(
    State(state): State<AppState>,
    Path((tenant_id, view_id)): Path<(String, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ReportRestoreRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.caller_query.trim().is_empty() {
        return bad_request("bad report restore").into_response();
    }
    let actor = ctx.user_id.to_string();
    let payload = match serde_json::to_string(&(
        1,
        view_id,
        actor.as_str(),
        req.session_id,
        req.caller_query.trim(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .restore_cited_report_pair(
            tenant_id,
            &request_id,
            &payload,
            &actor,
            req.session_id,
            view_id,
            req.caller_query.trim(),
        )
        .await
    {
        Ok(Ok((write, chart, formula))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": view_id,
                "chart_type": chart,
                "formula": formula,
                "caller_query_used": false,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedViewTrendRequest {
    request_id: String,
    grain: String,
    caller_amount: i64,
}

async fn save_saved_view_trend_route(
    State(state): State<AppState>,
    Path((tenant_id, view_id)): Path<(String, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SavedViewTrendRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.grain.trim() != "MONTH" {
        return bad_request("bad saved view trend").into_response();
    }
    let actor = ctx.user_id.to_string();
    let payload = match serde_json::to_string(&(1, view_id, actor.as_str(), "MONTH")) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_saved_view_trend(
            tenant_id,
            &request_id,
            &payload,
            &actor,
            view_id,
            req.caller_amount,
        )
        .await
    {
        Ok(Ok((write, bucket_count))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": view_id,
                "bucket_count": bucket_count,
                "grain": "MONTH",
                "caller_amount_used": false,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OpenCommandRequest {
    request_id: String,
    object_type: String,
    object_id: String,
    version_no: i64,
}

async fn open_command_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OpenCommandRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.object_type.trim().is_empty() || req.object_id.trim().is_empty() || req.version_no < 1 {
        return bad_request("bad command").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        req.object_type.trim(),
        req.object_id.trim(),
        req.version_no,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .open_command(
            tenant_id,
            &request_id,
            &payload,
            req.object_type.trim(),
            req.object_id.trim(),
            req.version_no,
        )
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "ACCEPTED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": req.object_id.trim(),
                "executed": false,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdvanceCommandRequest {
    reported_status: String,
    error_text: String,
    recoverable: bool,
    payload_hash: String,
}

async fn advance_command_route(
    State(state): State<AppState>,
    Path((tenant_id, request_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AdvanceCommandRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = request_id.trim().to_string();
    if request_id.is_empty() || req.payload_hash.trim().is_empty() {
        return bad_request("bad command advance").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .advance_command(
            tenant_id,
            &request_id,
            req.payload_hash.trim(),
            req.reported_status.trim(),
            req.error_text.trim(),
            req.recoverable,
        )
        .await
    {
        Ok(Ok(write)) => {
            let status = match &write {
                SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => {
                    receipt.status.as_str().to_string()
                }
            };
            let display = match status.as_str() {
                "ACCEPTED" => "已受理",
                "RUNNING" => "执行中",
                "SUCCEEDED" => "已成功",
                "FAILED" => "已失败",
                "RECONCILING" => "待核对",
                _ => "待核对",
            };
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "request_id": request_id,
                    "status": status,
                    "display_status": display,
                    "replay": matches!(write, SharedWrite::Replay(_)),
                    "effect_id": effect_id(&write),
                    "executed": false,
                })),
            )
                .into_response()
        }
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => command_readback::error_response(error),
    }
}

async fn read_command_route(
    State(state): State<AppState>,
    Path((tenant_id, request_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    match SharedContractRepo::new(state.db.pool())
        .read_command(tenant_id, request_id.trim())
        .await
    {
        Ok(Some((object_id, effect_id, status, version_no, line_status, recoverable, error_text))) => {
            let display = match status.as_str() {
                "ACCEPTED" => "已受理",
                "RUNNING" => "执行中",
                "SUCCEEDED" => "已成功",
                "FAILED" => "已失败",
                "RECONCILING" => "待核对",
                _ => "待核对",
            };
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "request_id": request_id.trim(),
                    "status": status,
                    "display_status": display,
                    "object_id": object_id,
                    "effect_id": effect_id,
                    "version_no": version_no,
                    "line_status": line_status,
                    "recoverable": recoverable,
                    "error_text": error_text,
                    "executed": false,
                    "found_after_restart": false,
                })),
            )
                .into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "command was not found").into_response(),
        Err(error) => command_readback::error_response(error),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LostCommandLookupRequest {
    payload_hash: String,
    revoked_session_id: uuid::Uuid,
    current_session_id: uuid::Uuid,
    client_unknown: bool,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    qty_units: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WindowCloseRequest {
    request_id: String,
    client_unknown: bool,
}

#[derive(Debug, Deserialize)]
struct WindowCloseRead {
    close_request: String,
}

async fn read_window_close_route(
    State(state): State<AppState>,
    Path((tenant_id, command_request)): Path<(String, String)>,
    Query(query): Query<WindowCloseRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_window_close(tenant_id, command_request.trim(), query.close_request.trim())
        .await
    {
        Ok((decision, effect, effect_count)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "command_request": command_request.trim(),
                "close_request": query.close_request.trim(),
                "decision": decision,
                "effect_id": effect,
                "effect_count": effect_count,
                "executed": false,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            (StatusCode::NOT_FOUND, "window close not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn record_window_close_route(
    State(state): State<AppState>,
    Path((tenant_id, command_request)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WindowCloseRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let command_request = command_request.trim().to_string();
    if command_request.is_empty() {
        return bad_request("bad window close").into_response();
    }
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = match serde_json::to_string(&(1, command_request.as_str(), req.client_unknown)) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .record_window_close(
            tenant_id,
            &request_id,
            &payload,
            &command_request,
            req.client_unknown,
        )
        .await
    {
        Ok(Ok((write, decision, effect_id))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": effect_id,
                "decision": decision,
                "executed": false,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn lookup_lost_command_route(
    State(state): State<AppState>,
    Path((tenant_id, request_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LostCommandLookupRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = request_id.trim().to_string();
    if request_id.is_empty() || req.payload_hash.trim().is_empty() || req.qty_units < 1 {
        return bad_request("bad lost command lookup").into_response();
    }
    let actor = ctx.user_id.to_string();
    match SharedContractRepo::new(state.db.pool())
        .lookup_lost_command(
            tenant_id,
            &request_id,
            req.payload_hash.trim(),
            &actor,
            req.revoked_session_id,
            req.current_session_id,
            req.client_unknown,
            req.warehouse_id,
            req.sku_id,
            req.qty_units,
        )
        .await
    {
        Ok(Ok(write)) => {
            let (status, effect) = match &write {
                SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => {
                    (receipt.status.as_str().to_string(), receipt.effect_id.clone())
                }
            };
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "request_id": request_id,
                    "status": status,
                    "replay": matches!(write, SharedWrite::Replay(_)),
                    "effect_id": effect,
                    "client_unknown_is_failure": false,
                })),
            )
                .into_response()
        }
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => command_readback::error_response(error),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedReportShareRequest {
    request_id: String,
    recipient_user_id: uuid::Uuid,
    caller_claimed_share: bool,
}

async fn share_saved_report_view_route(
    State(state): State<AppState>,
    Path((tenant_id, view_id)): Path<(String, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SavedReportShareRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.caller_claimed_share {
        return bad_request("bad saved report share").into_response();
    }
    let owner = uuid::Uuid::from(ctx.user_id);
    let payload = match serde_json::to_string(&(
        1,
        view_id,
        owner,
        req.recipient_user_id,
        "REPORT.SHARE",
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .share_saved_report_view(
            tenant_id,
            &request_id,
            &payload,
            owner,
            view_id,
            req.recipient_user_id,
            req.caller_claimed_share,
        )
        .await
    {
        Ok(Ok((write, share_id))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": share_id,
                "grant_from_request": false,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn read_shared_report_view_route(
    State(state): State<AppState>,
    Path((tenant_id, view_id)): Path<(String, uuid::Uuid)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let recipient = uuid::Uuid::from(ctx.user_id);
    match SharedContractRepo::new(state.db.pool())
        .read_shared_report_view(tenant_id, recipient, view_id)
        .await
    {
        Ok(Some((share_id, formula, measure))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "effect_id": share_id,
                "formula": formula,
                "measure": measure,
                "grant_from_request": false,
            })),
        )
            .into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "shared report is not current").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskExceptionRequest {
    request_id: String,
    kind: String,
    object_type: String,
    object_id: String,
    command_id: String,
    query_generation: i64,
    error_text: String,
}

async fn save_task_exception_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<TaskExceptionRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.query_generation < 1 || req.command_id.trim().is_empty() {
        return bad_request("bad task exception").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        req.kind.trim(),
        req.object_type.trim(),
        req.object_id.trim(),
        req.command_id.trim(),
        req.query_generation,
        req.error_text.trim(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_task_exception(
            tenant_id,
            &request_id,
            &payload,
            req.kind.trim(),
            req.object_type.trim(),
            req.object_id.trim(),
            req.command_id.trim(),
            req.query_generation,
            req.error_text.trim(),
        )
        .await
    {
        Ok(Ok((write, exception_id))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": exception_id,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskExceptionActionRequest {
    request_id: String,
    command_id: String,
    query_generation: i64,
    action: String,
}

async fn act_on_task_exception_route(
    State(state): State<AppState>,
    Path((tenant_id, exception_id)): Path<(String, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<TaskExceptionActionRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.query_generation < 1
        || req.command_id.trim().is_empty()
        || (req.action != "RETRY" && req.action != "MANUAL")
    {
        return bad_request("bad task action").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        exception_id,
        req.command_id.trim(),
        req.query_generation,
        req.action.trim(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .act_on_task_exception(
            tenant_id,
            &request_id,
            &payload,
            exception_id,
            req.command_id.trim(),
            req.query_generation,
            req.action.trim(),
            uuid::Uuid::from(ctx.user_id),
        )
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": exception_id,
                "command_kept": true,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskCancelRequest {
    request_id: String,
    command_id: String,
}

async fn cancel_committed_task_route(
    State(state): State<AppState>,
    Path((tenant_id, exception_id)): Path<(String, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<TaskCancelRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.command_id.trim().is_empty() {
        return bad_request("bad task cancel").into_response();
    }
    let payload = match serde_json::to_string(&(1, exception_id, req.command_id.trim())) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .cancel_committed_task(
            tenant_id,
            &request_id,
            &payload,
            exception_id,
            req.command_id.trim(),
        )
        .await
    {
        Ok(Ok((write, stage))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": exception_id,
                "stage": stage,
                "cancel_success": false,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskEffectRequest {
    request_id: String,
    effect_text: String,
    success_banner: String,
}

async fn save_task_effect_route(
    State(state): State<AppState>,
    Path((tenant_id, exception_id)): Path<(String, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<TaskEffectRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.effect_text.trim().is_empty() || req.success_banner.trim() == req.effect_text.trim() {
        return bad_request("bad task effect").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        exception_id,
        req.effect_text.trim(),
        req.success_banner.trim(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_task_effect(
            tenant_id,
            &request_id,
            &payload,
            exception_id,
            req.effect_text.trim(),
            req.success_banner.trim(),
        )
        .await
    {
        Ok(Ok((write, effect_id))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": effect_id,
                "banner_replaced": false,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct CommandRestartQuery {
    restarted: bool,
}

async fn read_restarted_command_route(
    State(state): State<AppState>,
    Path((tenant_id, request_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Query(query): Query<CommandRestartQuery>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if !query.restarted || request_id.trim().is_empty() {
        return bad_request("bad restarted command lookup").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_command(tenant_id, request_id.trim())
        .await
    {
        Ok(Some((object_id, effect_id, status, version_no, line_status, recoverable, error_text))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id.trim(),
                "status": status,
                "object_id": object_id,
                "effect_id": effect_id,
                "version_no": version_no,
                "line_status": line_status,
                "recoverable": recoverable,
                "error_text": error_text,
                "executed": false,
                "found_after_restart": true,
            })),
        )
            .into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "command was not found").into_response(),
        Err(error) => command_readback::error_response(error),
    }
}

#[derive(Debug, Deserialize)]
struct TaskEffectQuery {
    object_id: String,
    command_id: String,
    query_generation: i64,
}

async fn read_task_effect_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Query(query): Query<TaskEffectQuery>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if query.object_id.trim().is_empty()
        || query.command_id.trim().is_empty()
        || query.query_generation < 1
    {
        return bad_request("bad task effect lookup").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_task_effect(
            tenant_id,
            query.object_id.trim(),
            query.command_id.trim(),
            query.query_generation,
        )
        .await
    {
        Ok((effect_id, effect_text)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "effect_id": effect_id,
                "object_id": query.object_id.trim(),
                "command_id": query.command_id.trim(),
                "query_generation": query.query_generation,
                "effect_text": effect_text,
                "found_after_disconnect": true,
                "found_after_restart": true,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeadLetterPostingRequest {
    request_id: String,
    object_type: String,
    object_id: String,
    command_id: String,
    amount_micros: i64,
}

async fn post_dead_letter_once_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<DeadLetterPostingRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.object_type.trim().is_empty()
        || req.object_id.trim().is_empty()
        || req.command_id.trim().is_empty()
        || req.amount_micros <= 0
    {
        return bad_request("bad dead letter posting").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        req.object_type.trim(),
        req.object_id.trim(),
        req.command_id.trim(),
        req.amount_micros,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .post_dead_letter_once(
            tenant_id,
            &request_id,
            &payload,
            req.object_type.trim(),
            req.object_id.trim(),
            req.command_id.trim(),
            req.amount_micros,
        )
        .await
    {
        Ok(Ok((write, posting_id))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": posting_id,
                "posted_once": true,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct DeadLetterPostingQuery {
    object_id: String,
    command_id: String,
}

async fn read_dead_letter_posting_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Query(query): Query<DeadLetterPostingQuery>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if query.object_id.trim().is_empty() || query.command_id.trim().is_empty() {
        return bad_request("bad dead letter lookup").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_dead_letter_posting(tenant_id, query.object_id.trim(), query.command_id.trim())
        .await
    {
        Ok((posting_id, amount_micros)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "effect_id": posting_id,
                "object_id": query.object_id.trim(),
                "command_id": query.command_id.trim(),
                "amount_micros": amount_micros,
                "posted_once": true,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn save_cited_aging_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CitedAgingRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.cutoff_date.trim().len() != 10 || req.balance_ids.is_empty() {
        return bad_request("bad aging snapshot").into_response();
    }
    let payload = match serde_json::to_string(&(1, req.cutoff_date.trim(), &req.balance_ids)) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_cited_aging(
            tenant_id,
            &request_id,
            &payload,
            req.cutoff_date.trim(),
            &req.balance_ids,
        )
        .await
    {
        Ok(Ok((write, snapshot_id, rows))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": snapshot_id,
                "rows": rows,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct CitedAgingQuery {
    cutoff_date: String,
}

async fn read_cited_aging_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Query(query): Query<CitedAgingQuery>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if query.cutoff_date.trim().len() != 10 {
        return bad_request("bad aging snapshot").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_cited_aging(tenant_id, query.cutoff_date.trim())
        .await
    {
        Ok(rows) => {
            let total: i64 = rows.iter().map(|row| row.2).sum();
            let items: Vec<_> = rows
                .into_iter()
                .map(|(snapshot_id, balance_id, open_cent, bucket)| {
                    serde_json::json!({
                        "snapshot_id": snapshot_id,
                        "balance_id": balance_id,
                        "open_cent": open_cent,
                        "bucket": bucket,
                    })
                })
                .collect();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "cutoff_date": query.cutoff_date.trim(),
                    "rows": items.len(),
                    "open_cent": total,
                    "items": items,
                })),
            )
                .into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct BasicMarginQuery {
    sales_cent: i64,
    expense_id: Option<uuid::Uuid>,
}

async fn read_basic_margin_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Query(query): Query<BasicMarginQuery>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if query.sales_cent < 0 {
        return bad_request("bad basic margin").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_basic_margin(tenant_id, query.sales_cent, query.expense_id)
        .await
    {
        Ok(margin) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "sales_cent": query.sales_cent,
                "known": margin.is_some(),
                "margin_cent": margin,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct RebuiltBalanceQuery {
    source_id: String,
}

async fn read_rebuilt_balances_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Query(query): Query<RebuiltBalanceQuery>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let sources: Vec<&str> = query
        .source_id
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .collect();
    if sources.is_empty() {
        return bad_request("bad balance source").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_rebuilt_balances(tenant_id, &sources)
        .await
    {
        Ok(rows) => {
            let items: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|row| {
                    serde_json::json!({
                        "source_id": row.source_id,
                        "kind": row.kind,
                        "open_cent": row.open_cent,
                        "settled_cent": row.settled_cent,
                    })
                })
                .collect();
            (StatusCode::OK, Json(serde_json::json!({ "items": items }))).into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecallResultRequest {
    request_id: String,
    item_kind: String,
    item_code: String,
    field: String,
    status: String,
    warehouse_id: uuid::Uuid,
    location_id: uuid::Uuid,
    sku_id: uuid::Uuid,
}

async fn save_cited_recall_result_route(
    State(state): State<AppState>,
    Path((tenant_id, recall_id)): Path<(String, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<RecallResultRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.item_kind.trim().is_empty() || req.item_code.trim().is_empty() {
        return bad_request("bad recall result").into_response();
    }
    if !matches!(req.field.as_str(), "freeze" | "notice")
        || !matches!(req.status.as_str(), "DONE" | "FAILED")
    {
        return bad_request("bad recall result").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        recall_id,
        req.item_kind.trim(),
        req.item_code.trim(),
        req.field.as_str(),
        req.status.as_str(),
        req.warehouse_id,
        req.location_id,
        req.sku_id,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_cited_recall_result(
            tenant_id,
            &request_id,
            &payload,
            recall_id,
            req.item_kind.trim(),
            req.item_code.trim(),
            req.field.as_str(),
            req.status.as_str(),
            req.warehouse_id,
            req.location_id,
            req.sku_id,
        )
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct SamplingTrendQuery {
    rule_code: String,
    from_period: String,
    to_period: String,
}

async fn read_sampling_trend_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<SamplingTrendQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if query.rule_code.trim().is_empty()
        || query.from_period.trim().is_empty()
        || query.to_period.trim().is_empty()
        || query.from_period.trim() > query.to_period.trim()
    {
        return bad_request("bad sampling trend").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_sampling_trend(
            tenant_id,
            query.rule_code.trim(),
            query.from_period.trim(),
            query.to_period.trim(),
        )
        .await
    {
        Ok(points) => {
            let items: Vec<serde_json::Value> = points
                .iter()
                .map(|point| {
                    serde_json::json!({
                        "period_code": point.period_code,
                        "sample_size": point.sample_size,
                        "defects": point.defects,
                        "defect_ppm": point.defect_ppm,
                        "accepted": point.accepted,
                    })
                })
                .collect();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "rule_code": query.rule_code.trim(),
                    "from_period": query.from_period.trim(),
                    "to_period": query.to_period.trim(),
                    "items": items,
                })),
            )
                .into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SamplingDecisionRequest {
    request_id: String,
    rule_code: String,
    rule_version: i32,
    period_code: String,
    defects: i64,
}

async fn save_cited_sampling_decision_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SamplingDecisionRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.rule_code.trim().is_empty()
        || req.rule_version <= 0
        || req.period_code.trim().is_empty()
        || req.defects < 0
    {
        return bad_request("bad sampling decision").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        req.rule_code.trim(),
        req.rule_version,
        req.period_code.trim(),
        req.defects,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_cited_sampling_decision(
            tenant_id,
            &request_id,
            &payload,
            req.rule_code.trim(),
            req.rule_version,
            req.period_code.trim(),
            req.defects,
        )
        .await
    {
        Ok(Ok((write, accepted, defect_ppm))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
                "accepted": accepted,
                "defect_ppm": defect_ppm,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SamplingRuleRequest {
    request_id: String,
    rule_code: String,
    rule_version: i32,
    sample_size: i64,
    accept_number: i64,
}

async fn save_sampling_rule_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SamplingRuleRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.rule_code.trim().is_empty()
        || req.rule_version <= 0
        || req.sample_size <= 0
        || req.accept_number < 0
    {
        return bad_request("bad sampling rule").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        req.rule_code.trim(),
        req.rule_version,
        req.sample_size,
        req.accept_number,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_sampling_rule(
            tenant_id,
            &request_id,
            &payload,
            req.rule_code.trim(),
            req.rule_version,
            req.sample_size,
            req.accept_number,
        )
        .await
    {
        Ok(Ok((write, version_no))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
                "version_no": version_no,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TraceExportRequest {
    request_id: String,
    direction: String,
    code: String,
    node_cap: i64,
}

async fn save_trace_export_scope_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<TraceExportRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.direction.trim() != "input" && req.direction.trim() != "output" {
        return bad_request("bad direction").into_response();
    }
    if req.code.trim().is_empty() || req.node_cap <= 0 {
        return bad_request("bad trace export").into_response();
    }
    let actor = ctx.user_id.to_string();
    let payload = match serde_json::to_string(&(
        1,
        actor.trim(),
        req.direction.trim(),
        req.code.trim(),
        req.node_cap,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_trace_export_scope(
            tenant_id,
            &request_id,
            &payload,
            actor.trim(),
            req.direction.trim(),
            req.code.trim(),
            req.node_cap,
        )
        .await
    {
        Ok(Ok((write, _snapshot_id, count))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
                "source_count": count,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct GenealogyPageQuery {
    direction: String,
    code: String,
    #[serde(default)]
    after_edge_id: Option<String>,
    #[serde(default)]
    limit: Option<i64>,
    #[serde(default)]
    node_cap: Option<i64>,
}

async fn read_genealogy_keyset_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<GenealogyPageQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if query.direction.trim() != "input" && query.direction.trim() != "output" {
        return bad_request("bad direction").into_response();
    }
    if query.code.trim().is_empty() {
        return bad_request("bad code").into_response();
    }
    let after_edge_id = match query
        .after_edge_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        Some(id) => match uuid::Uuid::parse_str(id) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad after_edge_id").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(100);
    let node_cap = query.node_cap.unwrap_or(256);
    if !(1..=256).contains(&limit) || node_cap <= 0 {
        return bad_request("bad genealogy page").into_response();
    }
    let limit = usize::try_from(limit).unwrap_or(100);
    match SharedContractRepo::new(state.db.pool())
        .read_genealogy_keyset(
            tenant_id,
            query.direction.trim(),
            query.code.trim(),
            after_edge_id,
            limit,
            node_cap,
        )
        .await
    {
        Ok(page) => {
            let items: Vec<serde_json::Value> = page
                .edges
                .iter()
                .zip(page.edge_ids.iter())
                .map(|(edge, edge_id)| {
                    serde_json::json!({
                        "edge_id": edge_id,
                        "source_action": edge.source_action,
                        "input_code": edge.input_code,
                        "output_code": edge.output_code,
                        "qty_micros": edge.qty.as_micros(),
                        "work_order_no": edge.work_order,
                        "output_batch": edge.output_batch,
                        "customer_code": edge.customer,
                        "reversed": edge.reversed,
                    })
                })
                .collect();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "direction": query.direction.trim(),
                    "code": query.code.trim(),
                    "items": items,
                    "next_edge_id": page.next_edge_id,
                    "incomplete": page.incomplete,
                    "loaded": page.loaded,
                })),
            )
                .into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QualityReleaseRequest {
    request_id: String,
}

async fn release_after_quality_route(
    State(state): State<AppState>,
    Path((tenant_id, receipt_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<QualityReleaseRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let receipt_id = match uuid::Uuid::parse_str(receipt_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad receipt_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let inspector = ctx.user_id.to_string();
    if inspector.trim().is_empty() {
        return bad_request("inspector is required").into_response();
    }
    let payload = match serde_json::to_string(&(1, receipt_id, inspector.trim())) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .release_after_quality(tenant_id, &request_id, &payload, receipt_id, inspector.trim())
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => inspection_write_error(error),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BoardClaimRequest {
    request_id: String,
    owner: String,
    lease_seconds: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DurableTaskRequest {
    request_id: String,
    event_id: String,
    effect_id: String,
}

async fn enqueue_durable_task_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<DurableTaskRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.event_id.trim().is_empty() || req.effect_id.trim().is_empty() {
        return bad_request("bad durable task").into_response();
    }
    let payload = match serde_json::to_string(&(1, req.event_id.trim(), req.effect_id.trim())) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .enqueue_durable_task(
            tenant_id,
            &request_id,
            &payload,
            req.event_id.trim(),
            req.effect_id.trim(),
        )
        .await
    {
        Ok(Ok((write, task_id))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": task_id,
                "task_status": "QUEUED",
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct DurableTaskPageQuery {
    status: String,
    #[serde(default)]
    offset: i64,
    #[serde(default)]
    limit: Option<i32>,
    #[serde(default)]
    count_exact: Option<bool>,
}

async fn read_durable_task_page_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<DurableTaskPageQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if query.status.trim().is_empty() {
        return bad_request("bad task page").into_response();
    }
    let limit = query.limit.unwrap_or(100);
    let count_exact = query.count_exact.unwrap_or(true);
    match SharedContractRepo::new(state.db.pool())
        .read_durable_task_page(tenant_id, query.status.trim(), query.offset, limit, count_exact)
        .await
    {
        Ok((rows, matched, count_kind)) => {
            let items: Vec<serde_json::Value> = rows
                .iter()
                .map(|(task_id, event_id, effect_id, status)| {
                    serde_json::json!({
                        "task_id": task_id,
                        "event_id": event_id,
                        "effect_id": effect_id,
                        "status": status,
                        "version": 1,
                    })
                })
                .collect();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "query_id": format!("{}:{}:{}", query.status.trim(), query.offset, limit),
                    "status": query.status.trim(),
                    "offset": query.offset,
                    "limit": limit,
                    "items": items,
                    "matched": matched,
                    "count_kind": count_kind,
                    "version": 1,
                })),
            )
                .into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DurableClaimRequest {
    request_id: String,
    owner: String,
    lease_seconds: i64,
}

async fn claim_durable_task_route(
    State(state): State<AppState>,
    Path((tenant_id, task_id)): Path<(String, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<DurableClaimRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.owner.trim().is_empty() || req.lease_seconds < 1 {
        return bad_request("bad durable claim").into_response();
    }
    let payload = match serde_json::to_string(&(1, task_id, req.owner.trim(), req.lease_seconds)) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .claim_durable_task(
            tenant_id,
            &request_id,
            &payload,
            task_id,
            req.owner.trim(),
            req.lease_seconds,
        )
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": task_id,
                "lease_owner": req.owner.trim(),
                "task_status": "LEASED",
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DurableHeartbeatRequest {
    request_id: String,
    owner: String,
    progress: i32,
}

async fn heartbeat_durable_task_route(
    State(state): State<AppState>,
    Path((tenant_id, task_id)): Path<(String, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<DurableHeartbeatRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.owner.trim().is_empty() || !(0..=100).contains(&req.progress) {
        return bad_request("bad durable heartbeat").into_response();
    }
    let payload = match serde_json::to_string(&(1, task_id, req.owner.trim(), req.progress)) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .heartbeat_durable_task(
            tenant_id,
            &request_id,
            &payload,
            task_id,
            req.owner.trim(),
            req.progress,
        )
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": task_id,
                "lease_owner": req.owner.trim(),
                "progress": req.progress,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DurableFailureRequest {
    request_id: String,
    owner: String,
    transient: bool,
    known: bool,
    error_text: String,
}

async fn fail_durable_task_route(
    State(state): State<AppState>,
    Path((tenant_id, task_id)): Path<(String, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<DurableFailureRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.owner.trim().is_empty() || req.error_text.trim().is_empty() {
        return bad_request("bad durable failure").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        task_id,
        req.owner.trim(),
        req.transient,
        req.known,
        req.error_text.trim(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .fail_durable_task(
            tenant_id,
            &request_id,
            &payload,
            task_id,
            req.owner.trim(),
            req.transient,
            req.known,
            req.error_text.trim(),
        )
        .await
    {
        Ok(Ok((write, next, delay))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": task_id,
                "task_status": next,
                "retry_seconds": delay,
                "cancel_success": false,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DurableCancelRequest {
    request_id: String,
    owner: String,
    posted: bool,
    work_order_id: Option<uuid::Uuid>,
    qty: Option<String>,
}

async fn cancel_durable_task_route(
    State(state): State<AppState>,
    Path((tenant_id, task_id)): Path<(String, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<DurableCancelRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.owner.trim().is_empty() {
        return bad_request("bad durable cancel").into_response();
    }
    let qty = if req.posted {
        let Some(raw) = req.qty.as_deref().map(str::trim).filter(|value| !value.is_empty()) else {
            return bad_request("posted cancel needs a quantity").into_response();
        };
        match erp_domain::quantity::parse_qty(raw) {
            Ok(qty) if qty.as_micros() > 0 => Some(qty),
            _ => return bad_request("posted cancel needs a quantity").into_response(),
        }
    } else if req.qty.as_deref().is_some_and(|value| !value.trim().is_empty())
        || req.work_order_id.is_some()
    {
        return bad_request("unposted cancel has no document").into_response();
    } else {
        None
    };
    if req.posted && req.work_order_id.is_none() {
        return bad_request("posted cancel needs a work order").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        task_id,
        req.owner.trim(),
        req.posted,
        req.work_order_id,
        qty.map(|value| value.as_micros()),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    let repo = SharedContractRepo::new(state.db.pool());
    let asked = match repo
        .request_task_cancel(tenant_id, &request_id, &payload, task_id, req.owner.trim())
        .await
    {
        Ok(Ok(write)) => write,
        Ok(Err(_)) => return conflict("payload conflict").into_response(),
        Err(error) => {
            return (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response();
        }
    };
    let stage_request = format!("{request_id}:stage");
    let (write, stage, reverse_id) = if req.posted {
        let work_order_id = req.work_order_id.expect("posted work order checked");
        let qty = qty.expect("posted quantity checked");
        match repo
            .reverse_posted_task_issue(
                tenant_id,
                &stage_request,
                &payload,
                task_id,
                req.owner.trim(),
                work_order_id,
                qty,
            )
            .await
        {
            Ok(Ok(write)) => {
                let reverse_id = match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => {
                        receipt.effect_id.clone()
                    }
                };
                (write, "REVERSE_REQUIRED".to_string(), Some(reverse_id))
            }
            Ok(Err(_)) => return conflict("payload conflict").into_response(),
            Err(error) => {
                return (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response();
            }
        }
    } else {
        match repo
            .apply_cancel_checkpoint(
                tenant_id,
                &stage_request,
                &payload,
                task_id,
                req.owner.trim(),
                false,
            )
            .await
        {
            Ok(Ok((write, stage))) => (write, stage, None),
            Ok(Err(_)) => return conflict("payload conflict").into_response(),
            Err(error) => {
                return (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response();
            }
        }
    };
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "request_id": request_id,
            "status": "SUCCEEDED",
            "replay": matches!(asked, SharedWrite::Replay(_)) && matches!(write, SharedWrite::Replay(_)),
            "effect_id": task_id,
            "task_status": stage,
            "cancel_success": false,
            "reverse_move_id": reverse_id,
        })),
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DurableRestoreRequest {
    request_id: String,
}

async fn restore_dead_task_route(
    State(state): State<AppState>,
    Path((tenant_id, task_id)): Path<(String, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<DurableRestoreRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if request_id.trim().is_empty() {
        return bad_request("bad durable restore").into_response();
    }
    let payload = match serde_json::to_string(&(1, task_id)) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .restore_dead_task(tenant_id, &request_id, &payload, task_id)
        .await
    {
        Ok(Ok((write, effect_id))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": effect_id,
                "task_id": task_id,
                "task_status": "QUEUED",
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HealthSignalRequest {
    request_id: String,
    kind: String,
    value: i64,
    threshold: i64,
    duration_seconds: i64,
    command_id: String,
    action_text: String,
}

async fn record_health_signal_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<HealthSignalRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.command_id.trim().is_empty() || req.value < 0 || req.threshold < 0 || req.duration_seconds < 0 {
        return bad_request("bad health signal").into_response();
    }
    let actor = ctx.user_id.to_string();
    let payload = match serde_json::to_string(&(
        1,
        actor.as_str(),
        req.kind.trim(),
        req.value,
        req.threshold,
        req.duration_seconds,
        req.command_id.trim(),
        req.action_text.trim(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .record_health_signal(
            tenant_id,
            &request_id,
            &payload,
            req.kind.trim(),
            req.value,
            req.threshold,
            req.duration_seconds,
            req.command_id.trim(),
            &actor,
            req.action_text.trim(),
        )
        .await
    {
        Ok(Ok((write, status))) => {
            let effect = match &write {
                SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => {
                    receipt.effect_id.clone()
                }
            };
            let signal_id = effect.split(':').nth(1).unwrap_or("").to_string();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "request_id": request_id,
                    "status": "SUCCEEDED",
                    "replay": matches!(write, SharedWrite::Replay(_)),
                    "signal_id": signal_id,
                    "signal_status": status,
                    "command_id": req.command_id.trim(),
                    "actor_user_id": actor,
                })),
            )
                .into_response()
        }
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkAdmissionRequest {
    request_id: String,
    kind: String,
    command_id: String,
    retry_after_seconds: i64,
}

async fn admit_tenant_work_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WorkAdmissionRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.command_id.trim().is_empty()
        || req.retry_after_seconds < 0
        || !matches!(req.kind.trim(), "HEAVY" | "ORDER")
    {
        return bad_request("bad work admission").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        req.kind.trim(),
        req.command_id.trim(),
        req.retry_after_seconds,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .admit_tenant_work(
            tenant_id,
            &request_id,
            &payload,
            req.kind.trim(),
            req.command_id.trim(),
            req.retry_after_seconds,
        )
        .await
    {
        Ok(Ok((write, status))) => {
            let effect = match &write {
                SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => {
                    receipt.effect_id.clone()
                }
            };
            let admission_id = effect.split(':').nth(1).unwrap_or("").to_string();
            let code = if status == "429" {
                StatusCode::TOO_MANY_REQUESTS
            } else {
                StatusCode::OK
            };
            (
                code,
                Json(serde_json::json!({
                    "request_id": request_id,
                    "status": status,
                    "replay": matches!(write, SharedWrite::Replay(_)),
                    "admission_id": admission_id,
                    "command_id": req.command_id.trim(),
                    "retry_after_seconds": if status == "429" { req.retry_after_seconds } else { 0 },
                })),
            )
                .into_response()
        }
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VersionedDocumentOpenRequest {
    request_id: String,
    body_text: String,
}

async fn open_versioned_document_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<VersionedDocumentOpenRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.body_text.trim().is_empty() {
        return bad_request("bad versioned document").into_response();
    }
    let payload = match serde_json::to_string(&(1, req.body_text.trim())) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .open_versioned_document(tenant_id, &request_id, &payload, req.body_text.trim())
        .await
    {
        Ok(Ok((write, document_id, version))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "document_id": document_id,
                "version_no": version,
                "conflict": false,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn read_conflict_input_route(
    State(state): State<AppState>,
    Path((tenant_id, document_id)): Path<(String, uuid::Uuid)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let actor = ctx.user_id.to_string();
    match SharedContractRepo::new(state.db.pool())
        .read_conflict_input(tenant_id, document_id, &actor)
        .await
    {
        Ok((version, stored_body, kept_input)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "document_id": document_id,
                "version_no": version,
                "stored_body": stored_body,
                "kept_input": kept_input,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("conflict input not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VersionedDocumentSaveRequest {
    request_id: String,
    expected_version: i64,
    body_text: String,
}

async fn save_versioned_document_route(
    State(state): State<AppState>,
    Path((tenant_id, document_id)): Path<(String, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<VersionedDocumentSaveRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.body_text.trim().is_empty() || req.expected_version < 1 {
        return bad_request("bad versioned save").into_response();
    }
    let actor = ctx.user_id.to_string();
    let payload = match serde_json::to_string(&(
        1,
        document_id,
        actor.as_str(),
        req.expected_version,
        req.body_text.trim(),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_versioned_document(
            tenant_id,
            &request_id,
            &payload,
            document_id,
            &actor,
            req.expected_version,
            req.body_text.trim(),
        )
        .await
    {
        Ok(Ok((write, version, conflicted))) => {
            let effect = match &write {
                SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => {
                    receipt.effect_id.clone()
                }
            };
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "request_id": request_id,
                    "status": "SUCCEEDED",
                    "replay": matches!(write, SharedWrite::Replay(_)),
                    "document_id": document_id,
                    "version_no": version,
                    "conflict": conflicted,
                    "effect_id": effect,
                })),
            )
                .into_response()
        }
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AccessGrantRequest {
    request_id: String,
    permission: String,
    warehouse_id: uuid::Uuid,
}

async fn grant_access_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AccessGrantRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.permission.trim().is_empty() {
        return bad_request("bad access grant").into_response();
    }
    let actor = ctx.user_id.to_string();
    let payload = match serde_json::to_string(&(1, actor.as_str(), req.permission.trim(), req.warehouse_id)) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .grant_access(
            tenant_id,
            &request_id,
            &payload,
            &actor,
            req.permission.trim(),
            req.warehouse_id,
        )
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "actor_user_id": actor,
                "permission": req.permission.trim(),
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AccessObjectRequest {
    request_id: String,
    object_kind: String,
    warehouse_id: uuid::Uuid,
}

async fn register_access_object_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AccessObjectRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if !matches!(req.object_kind.trim(), "ATTACHMENT" | "PRINT" | "TASK" | "STOCK" | "BATCH") {
        return bad_request("bad access object").into_response();
    }
    let payload = match serde_json::to_string(&(1, req.object_kind.trim(), req.warehouse_id)) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .register_access_object(
            tenant_id,
            &request_id,
            &payload,
            req.object_kind.trim(),
            req.warehouse_id,
        )
        .await
    {
        Ok(Ok((write, object_id))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "object_id": object_id,
                "object_kind": req.object_kind.trim(),
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AccessActionRequest {
    request_id: String,
    permission: String,
    object_id: uuid::Uuid,
    summary: String,
    platform_operator: bool,
    retention_days: i32,
    free_plan: bool,
}

async fn authorize_audited_action_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AccessActionRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.permission.trim().is_empty() || req.summary.trim().is_empty() {
        return bad_request("bad access action").into_response();
    }
    let actor = ctx.user_id.to_string();
    let payload = match serde_json::to_string(&(
        1,
        actor.as_str(),
        req.permission.trim(),
        req.object_id,
        req.summary.trim(),
        req.platform_operator,
        req.retention_days,
        req.free_plan,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .authorize_audited_action(
            tenant_id,
            &request_id,
            &payload,
            &actor,
            req.permission.trim(),
            req.object_id,
            req.summary.trim(),
            req.platform_operator,
            req.retention_days,
            req.free_plan,
        )
        .await
    {
        Ok(Ok((write, allowed))) => {
            let effect = match &write {
                SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => {
                    receipt.effect_id.clone()
                }
            };
            let audit_id = effect.split(':').nth(1).unwrap_or("").to_string();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "request_id": request_id,
                    "status": "SUCCEEDED",
                    "replay": matches!(write, SharedWrite::Replay(_)),
                    "allowed": allowed,
                    "audit_id": audit_id,
                    "actor_user_id": actor,
                })),
            )
                .into_response()
        }
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenScopeRequest {
    request_id: String,
    permission: String,
    warehouse_id: uuid::Uuid,
}

async fn freeze_task_scope_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<FrozenScopeRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.permission.trim().is_empty() {
        return bad_request("bad frozen scope").into_response();
    }
    let actor = ctx.user_id.to_string();
    let payload = match serde_json::to_string(&(1, actor.as_str(), req.permission.trim(), req.warehouse_id)) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .freeze_task_scope(
            tenant_id,
            &request_id,
            &payload,
            &actor,
            req.permission.trim(),
            req.warehouse_id,
        )
        .await
    {
        Ok(Ok((write, task_id))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "task_id": task_id,
                "actor_user_id": actor,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenDownloadRequest {
    request_id: String,
}

async fn authorize_frozen_download_route(
    State(state): State<AppState>,
    Path((tenant_id, task_id)): Path<(String, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<FrozenDownloadRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let actor = ctx.user_id.to_string();
    let payload = match serde_json::to_string(&(1, task_id, actor.as_str())) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .authorize_frozen_download(tenant_id, &request_id, &payload, task_id, &actor)
        .await
    {
        Ok(Ok((write, allowed))) => {
            let effect = match &write {
                SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => {
                    receipt.effect_id.clone()
                }
            };
            let audit_id = effect.split(':').nth(1).unwrap_or("").to_string();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "request_id": request_id,
                    "status": "SUCCEEDED",
                    "replay": matches!(write, SharedWrite::Replay(_)),
                    "allowed": allowed,
                    "audit_id": audit_id,
                    "actor_user_id": actor,
                })),
            )
                .into_response()
        }
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WarehouseRevokeRequest {
    request_id: String,
}

async fn revoke_warehouse_scope_route(
    State(state): State<AppState>,
    Path((tenant_id, warehouse_id)): Path<(String, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WarehouseRevokeRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let actor = ctx.user_id.to_string();
    let payload = match serde_json::to_string(&(1, actor.as_str(), warehouse_id)) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .revoke_warehouse_scope(tenant_id, &request_id, &payload, &actor, warehouse_id)
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "actor_user_id": actor,
                "warehouse_id": warehouse_id,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn claim_board_refresh_route(
    State(state): State<AppState>,
    Path((tenant_id, task_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<BoardClaimRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let task_id = match uuid::Uuid::parse_str(task_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad task_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.owner.trim().is_empty() || req.lease_seconds < 0 {
        return bad_request("bad board claim").into_response();
    }
    let payload = match serde_json::to_string(&(1, task_id, req.owner.trim(), req.lease_seconds)) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .claim_durable_task(
            tenant_id,
            &request_id,
            &payload,
            task_id,
            req.owner.trim(),
            req.lease_seconds,
        )
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": task_id,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BoardCompleteRequest {
    work_order_id: String,
    owner: String,
    today: String,
    due_date: Option<String>,
    minutes_per_day: Option<i64>,
}

async fn complete_board_refresh_route(
    State(state): State<AppState>,
    Path((tenant_id, task_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<BoardCompleteRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let task_id = match uuid::Uuid::parse_str(task_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad task_id").into_response(),
    };
    let work_order_id = match uuid::Uuid::parse_str(req.work_order_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad work_order_id").into_response(),
    };
    if req.owner.trim().is_empty() || req.today.trim().len() != 10 {
        return bad_request("bad board completion").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .complete_board_refresh(
            tenant_id,
            task_id,
            req.owner.trim(),
            work_order_id,
            req.today.trim(),
            req.due_date.as_deref().map(str::trim),
            req.minutes_per_day,
        )
        .await
    {
        Ok(snapshot_id) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "task_id": task_id,
                "work_order_id": work_order_id,
                "snapshot_id": snapshot_id,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct EngineeringImpactQuery {
    original_code: String,
}

async fn read_engineering_impact_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<EngineeringImpactQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if query.original_code.trim().is_empty() {
        return bad_request("original_code required").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .engineering_impact(tenant_id, query.original_code.trim())
        .await
    {
        Ok(rows) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "original_code": query.original_code.trim(),
                "orders": rows.iter().map(|(work_order_no, status, issued)| serde_json::json!({
                    "work_order_no": work_order_no,
                    "status": status,
                    "issued_micros": issued,
                })).collect::<Vec<_>>(),
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn read_operation_report_route(
    State(state): State<AppState>,
    Path((tenant_id, work_order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Query(query): Query<OperationReportQuery>,
    Extension(epoch): Extension<VerifiedCredentialEpoch>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let Ok(work_order_id) = uuid::Uuid::parse_str(work_order_id.trim()) else {
        return bad_request("bad work_order_id").into_response();
    };
    let actor = match work_order_catalog::actor(&ctx,epoch) { Ok(a)=>a,Err(r)=>return r };
    match SharedContractRepo::new(state.db.pool())
        .read_authorized_report_totals(tenant_id, work_order_id, query.operation_code.trim(),actor)
        .await
    {
        Ok(totals) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "operation_code": query.operation_code.trim(),
                "good_micros": totals.good.as_micros(),
                "rejected_micros": totals.rejected.as_micros(),
                "rework_micros": totals.rework.as_micros(),
                "minutes": totals.minutes,
            })),
        )
            .into_response(),
        Err(error) => operation_reports::failure(error),
    }
}

async fn save_operation_report_route(
    State(state): State<AppState>,
    Path((tenant_id, work_order_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Extension(epoch): Extension<VerifiedCredentialEpoch>,
    Json(req): Json<OperationReportRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let actor = match work_order_catalog::actor(&ctx,epoch) { Ok(a)=>a,Err(r)=>return r };
    let fields = match serde_json::to_value(&req) { Ok(v)=>v,Err(_)=>return StatusCode::INTERNAL_SERVER_ERROR.into_response() };
    let work_order_id = match uuid::Uuid::parse_str(&work_order_id) {
        Ok(id) => id,
        Err(_) => return bad_request("bad work_order_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let good = match erp_domain::quantity::parse_qty(&req.good_qty) {
        Ok(qty) => qty,
        Err(_) => return bad_request("bad good_qty").into_response(),
    };
    let rejected = match erp_domain::quantity::parse_qty(&req.rejected_qty) {
        Ok(qty) => qty,
        Err(_) => return bad_request("bad rejected_qty").into_response(),
    };
    let rework = match erp_domain::quantity::parse_qty(&req.rework_qty) {
        Ok(qty) => qty,
        Err(_) => return bad_request("bad rework_qty").into_response(),
    };
    let previous = match erp_domain::quantity::parse_qty(&req.previous_good_qty) {
        Ok(qty) => qty,
        Err(_) => return bad_request("bad previous_good_qty").into_response(),
    };
    let session_user_id = ctx.user_id.to_string();
    let payload = match serde_json::to_string(&(
        1,
        work_order_id,
        req.operation_code.trim(),
        good.as_micros(),
        rejected.as_micros(),
        rework.as_micros(),
        req.minutes,
        req.operator_id.trim(),
        session_user_id.as_str(),
        req.session_id.trim(),
        req.crew_code.trim(),
        req.started_at.trim(),
        req.ended_at.trim(),
        previous.as_micros(),
        req.reverses_report_id.as_deref().map(str::trim),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    let report = erp_domain::manufacturing::OperationReport {
        operation: req.operation_code.trim().to_string(),
        good,
        rejected,
        rework,
        minutes: req.minutes,
        operator_id: req.operator_id.trim().to_string(),
        session_user_id,
        session_id: req.session_id.trim().to_string(),
        crew_code: req.crew_code.trim().to_string(),
        started_at: req.started_at.trim().to_string(),
        ended_at: req.ended_at.trim().to_string(),
        reverses_report_id: req
            .reverses_report_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
    };
    match SharedContractRepo::new(state.db.pool())
        .save_authorized_operation_report(tenant_id,erp_repo::shared_contract::OperationReportInput{
            request_id:&request_id,payload_hash:&payload,work_order_id,report:&report,fields},actor).await
    {
        Ok(snapshot)=>(StatusCode::OK,Json(snapshot)).into_response(),
        Err(error)=>operation_reports::failure(error),
    }
}

async fn save_material_route(
    State(state): State<AppState>,
    Path((tenant_id, work_order_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    epoch: Option<Extension<VerifiedCredentialEpoch>>,
    Json(req): Json<MaterialMoveRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let work_order_id = match uuid::Uuid::parse_str(&work_order_id) {
        Ok(id) => id,
        Err(_) => return bad_request("bad work_order_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_warehouse_access(&ctx, req.warehouse_id) {
        return resp.into_response();
    }
    let (Some(session),Some(epoch))=(ctx.session_id,epoch.and_then(|e|e.0.0)) else {
        return (StatusCode::UNAUTHORIZED,"current material session required").into_response();
    };
    let actor=erp_repo::shared_contract::ReceiptValueActor {authentication_tenant:ctx.tenant_id,user_id:ctx.user_id,session_id:session,credential_epoch:epoch,warehouse_id:req.warehouse_id};
    let qty = match erp_domain::quantity::parse_qty(&req.qty) {
        Ok(qty) => qty,
        Err(_) => return bad_request("bad qty").into_response(),
    };
    let kind = match req.kind.trim() {
        "ISSUE" => erp_domain::manufacturing::MaterialMoveKind::Issue,
        "SUPPLEMENT" => erp_domain::manufacturing::MaterialMoveKind::Supplement,
        "RETURN" => erp_domain::manufacturing::MaterialMoveKind::Return,
        "CONSUME" => erp_domain::manufacturing::MaterialMoveKind::Consume,
        _ => return bad_request("bad kind").into_response(),
    };
    let reason = req
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let payload = match serde_json::to_string(&(
        1,
        work_order_id,
        req.warehouse_id,
        req.sku_id,
        req.batch_id,
        req.original_issue_id,
        kind.as_str(),
        qty.as_micros(),
        reason,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .post_material_move_as_actor(
            tenant_id,
            &request_id,
            &payload,
            erp_repo::shared_contract::MaterialMoveInput {
                work_order_id,
                warehouse_id: req.warehouse_id,
                sku_id: req.sku_id,
                batch_id: req.batch_id,
                original_issue_id: req.original_issue_id,
                movement: erp_domain::manufacturing::MaterialMove {
                    kind,
                    qty,
                    supplement_reason_required: reason.is_some(),
                },
                reason,
            },
            actor,
        )
        .await
    {
        Ok(Ok(result)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(result.effect, SharedWrite::Replay(_)),
                "effect_id":result.move_id,
                "move_id":result.move_id,
                "warehouse_id":req.warehouse_id,
                "sku_id":req.sku_id,
                "batch_id":req.batch_id,
                "original_issue_id":req.original_issue_id,
                "shop_qty_micros": result.balance.shop_qty.as_micros().to_string(),
                "consumed_micros": result.balance.consumed.as_micros().to_string(),
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.downcast_ref::<erp_repo::shared_contract::MaterialCostError>().is_some()=>cost_response::error(error),
        Err(error) if error.downcast_ref::<erp_repo::shared_contract::ReceiptValueAuthorizationError>().is_some()=> {
            let status=match error.downcast_ref::<erp_repo::shared_contract::ReceiptValueAuthorizationError>() {
                Some(erp_repo::shared_contract::ReceiptValueAuthorizationError::Unauthorized)=>StatusCode::UNAUTHORIZED,
                Some(erp_repo::shared_contract::ReceiptValueAuthorizationError::Forbidden)=>StatusCode::FORBIDDEN,
                _=>StatusCode::NOT_FOUND,
            };
            (status,error.to_string()).into_response()
        }
        Err(error) if error.downcast_ref::<sqlx::Error>().is_some()=>(StatusCode::INTERNAL_SERVER_ERROR,"material storage unavailable").into_response(),
        Err(error)
            if error
                .downcast_ref::<erp_repo::inventory_batch_effects::InventoryBatchError>()
                .is_some()
                || error
                    .downcast_ref::<erp_repo::inventory_precision::InventoryPrecisionError>()
                    .is_some()
                || error
                    .downcast_ref::<erp_repo::inventory::InventoryBatchLimit>()
                    .is_some() =>
        {
            crate::inventory_views::inventory_error_response(error).into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn save_bom(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<BomSaveRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let items = match bom_lines(&req.items) {
        Ok(items) => items,
        Err(resp) => return resp,
    };
    let payload = format!("{}:{}", req.product_code.trim(), bom_payload(&req.items));
    match SharedContractRepo::new(state.db.pool())
        .save_manufacturing_bom(
            tenant_id,
            &request_id,
            &payload,
            req.product_code.trim(),
            &items,
        )
        .await
    {
        Ok(Ok(effect)) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct BomStructureLine {
    parent: String,
    component: String,
    qty_per: String,
    unit_code: String,
    scrap_ppm: i64,
    supply: String,
    line_no: i32,
}

#[derive(Debug, Deserialize)]
struct BomStructureRequest {
    request_id: String,
    product_code: String,
    #[serde(default)]
    bom_id: Option<String>,
    #[serde(default)]
    order_qty: Option<String>,
    #[serde(default)]
    lines: Vec<BomStructureLine>,
}

async fn save_bom_structure_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<BomStructureRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let repo = SharedContractRepo::new(state.db.pool());
    if let Some(bom_id) = req
        .bom_id
        .as_deref()
        .and_then(|value| uuid::Uuid::parse_str(value).ok())
    {
        let qty = match erp_domain::quantity::parse_qty(req.order_qty.as_deref().unwrap_or("")) {
            Ok(value) => value.as_micros(),
            Err(_) => return bad_request("bad order_qty").into_response(),
        };
        return match repo.explode_bom_structure(tenant_id, &request_id, &request_id, bom_id, qty).await {
            Ok(Ok((write, id, rows))) => (
                StatusCode::OK,
                Json(serde_json::json!({"explode_id": id, "rows": rows, "replay": matches!(write, SharedWrite::Replay(_))})),
            )
                .into_response(),
            Ok(Err(_)) => conflict("payload conflict").into_response(),
            Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
        };
    }
    let mut lines = Vec::new();
    for line in &req.lines {
        let qty = match erp_domain::quantity::parse_qty(&line.qty_per) {
            Ok(value) => value,
            Err(_) => return bad_request("bad qty_per").into_response(),
        };
        lines.push(erp_domain::manufacturing::BomLine {
            parent: line.parent.clone(),
            component: line.component.clone(),
            qty_per: qty,
            unit_code: line.unit_code.clone(),
            scrap_ppm: line.scrap_ppm,
            supply: line.supply.clone(),
            line_no: line.line_no,
        });
    }
    match repo
        .save_bom_structure(tenant_id, &request_id, &request_id, req.product_code.trim(), &lines)
        .await
    {
        Ok(Ok((write, id))) => (
            StatusCode::OK,
            Json(serde_json::json!({"bom_id": id, "replay": matches!(write, SharedWrite::Replay(_))})),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct BomExplodePageQuery {
    explode_id: String,
    parent: String,
    #[serde(default)]
    offset: i64,
    #[serde(default)]
    limit: Option<i64>,
    #[serde(default)]
    yield_ppm: i64,
}

async fn read_bom_explode_page_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<BomExplodePageQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let explode_id = match uuid::Uuid::parse_str(query.explode_id.trim()) {
        Ok(value) => value,
        Err(_) => return bad_request("bad explode_id").into_response(),
    };
    let limit = query.limit.unwrap_or(20);
    let page = SharedContractRepo::new(state.db.pool())
        .read_bom_explode_page(
            tenant_id,
            explode_id,
            query.parent.trim(),
            query.offset,
            limit,
            query.yield_ppm,
        )
        .await;
    match page {
        Ok((child_count, rows)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "explode_id": explode_id,
                "parent": query.parent.trim(),
                "child_count": child_count,
                "lines": rows.iter().map(|(component, qty, line_no)| serde_json::json!({
                    "component": component,
                    "qty_micros": qty,
                    "line_no": line_no,
                })).collect::<Vec<_>>(),
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct BomVersionRequest {
    request_id: String,
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    series_code: String,
    #[serde(default)]
    bom_id: Option<String>,
    #[serde(default)]
    version_id: Option<String>,
    #[serde(default)]
    start_date: String,
    #[serde(default)]
    end_date: String,
    #[serde(default)]
    make_default: bool,
    #[serde(default)]
    need_date: String,
    #[serde(default)]
    work_order_no: String,
}

#[derive(Debug, Deserialize)]
struct WorkOrderBomQuery {
    work_order_id: String,
    component: String,
}

async fn read_work_order_bom_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<WorkOrderBomQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let work_order_id = match uuid::Uuid::parse_str(query.work_order_id.trim()) {
        Ok(value) => value,
        Err(_) => return bad_request("bad work_order_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_work_order_bom(tenant_id, work_order_id, query.component.trim())
        .await
    {
        Ok((version_no, qty_micros, status)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "work_order_id": work_order_id,
                "component": query.component.trim(),
                "version_no": version_no,
                "qty_micros": qty_micros,
                "status": status,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, format!("{error}")).into_response(),
    }
}

async fn publish_bom_version_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<BomVersionRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let repo = SharedContractRepo::new(state.db.pool());
    let version_id = req
        .version_id
        .as_deref()
        .and_then(|value| uuid::Uuid::parse_str(value).ok());
    match req.action.as_deref() {
        Some("RETIRE") => {
            let Some(version_id) = version_id else {
                return bad_request("bad version_id").into_response();
            };
            match repo.retire_bom_version(tenant_id, &request_id, &request_id, version_id).await {
                Ok(Ok((write, lines))) => (
                    StatusCode::OK,
                    Json(serde_json::json!({"version_id": version_id, "lines": lines, "replay": matches!(write, SharedWrite::Replay(_))})),
                )
                    .into_response(),
                Ok(Err(_)) => conflict("payload conflict").into_response(),
                Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
            }
        }
        Some("COPY") => {
            let Some(version_id) = version_id else {
                return bad_request("bad version_id").into_response();
            };
            match repo.copy_bom_version(tenant_id, &request_id, &request_id, version_id).await {
                Ok(Ok((write, id))) => (
                    StatusCode::OK,
                    Json(serde_json::json!({"bom_id": id, "replay": matches!(write, SharedWrite::Replay(_))})),
                )
                    .into_response(),
                Ok(Err(_)) => conflict("payload conflict").into_response(),
                Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
            }
        }
        Some("SNAPSHOT") => {
            match repo
                .snapshot_work_order_bom(tenant_id, &request_id, &request_id, &req.series_code, &req.need_date, &req.work_order_no)
                .await
            {
                Ok(Ok((write, id, version_no))) => (
                    StatusCode::OK,
                    Json(serde_json::json!({"work_order_id": id, "version_no": version_no, "replay": matches!(write, SharedWrite::Replay(_))})),
                )
                    .into_response(),
                Ok(Err(_)) => conflict("payload conflict").into_response(),
                Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
            }
        }
        _ => {
            let Some(bom_id) = req
                .bom_id
                .as_deref()
                .and_then(|value| uuid::Uuid::parse_str(value).ok())
            else {
                return bad_request("bad bom_id").into_response();
            };
            match repo
                .publish_bom_version(
                    tenant_id,
                    &request_id,
                    &request_id,
                    &req.series_code,
                    bom_id,
                    &req.start_date,
                    &req.end_date,
                    req.make_default,
                )
                .await
            {
                Ok(Ok((write, id, version_no))) => (
                    StatusCode::OK,
                    Json(serde_json::json!({"version_id": id, "version_no": version_no, "replay": matches!(write, SharedWrite::Replay(_))})),
                )
                    .into_response(),
                Ok(Err(_)) => conflict("payload conflict").into_response(),
                Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
            }
        }
    }
}

async fn save_routing_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<RoutingSaveRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let steps: Vec<RoutingStep> = req
        .steps
        .iter()
        .map(|step| RoutingStep {
            operation: step.operation.clone(),
            sequence: step.sequence,
            work_center: step.work_center.clone(),
            minutes_per: step.minutes_per,
            inspection_required: step.inspection_required,
        })
        .collect();
    let payload = format!(
        "{}:{}",
        req.product_code.trim(),
        routing_payload(&req.steps)
    );
    match SharedContractRepo::new(state.db.pool())
        .save_routing(
            tenant_id,
            &request_id,
            &payload,
            req.product_code.trim(),
            &steps,
        )
        .await
    {
        Ok(Ok(effect)) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoutingChangeRequest {
    request_id: String,
    product_code: String,
    approver: String,
    steps: Vec<RoutingLineRequest>,
}

async fn save_routing_change_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<RoutingChangeRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let steps: Vec<RoutingStep> = req
        .steps
        .iter()
        .map(|step| RoutingStep {
            operation: step.operation.clone(),
            sequence: step.sequence,
            work_center: step.work_center.clone(),
            minutes_per: step.minutes_per,
            inspection_required: step.inspection_required,
        })
        .collect();
    let payload = match serde_json::to_string(&(
        1,
        req.product_code.trim(),
        req.approver.trim(),
        routing_payload(&req.steps),
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_routing_change(
            tenant_id,
            &request_id,
            &payload,
            req.product_code.trim(),
            req.approver.trim(),
            &steps,
        )
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": match &write {
                    SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
                },
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct OperationStepRequest {
    operation: String,
    sequence: i32,
    #[serde(default)]
    predecessor: String,
    work_center: String,
    setup_minutes: i64,
    minutes_per: i64,
    inspection_required: bool,
    report_field: String,
    #[serde(default)]
    display_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct OperationRouteRequest {
    request_id: String,
    #[serde(default)]
    product_code: String,
    #[serde(default)]
    route_id: Option<String>,
    #[serde(default)]
    work_order_no: String,
    #[serde(default)]
    steps: Vec<OperationStepRequest>,
}

#[derive(Debug, Deserialize)]
struct WorkOrderRouteQuery {
    #[serde(default)]
    work_order_id: String,
    #[serde(default)]
    route_id: String,
    #[serde(default)]
    offset: i64,
    #[serde(default)]
    limit: Option<i64>,
}

async fn read_work_order_route_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<WorkOrderRouteQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let repo = SharedContractRepo::new(state.db.pool());
    if !query.route_id.trim().is_empty() {
        let route_id = match uuid::Uuid::parse_str(query.route_id.trim()) {
            Ok(value) => value,
            Err(_) => return bad_request("bad route_id").into_response(),
        };
        let limit = query.limit.unwrap_or(20);
        return match repo
            .read_operation_route_page(tenant_id, route_id, query.offset, limit)
            .await
        {
            Ok((step_count, rows)) => (
                StatusCode::OK,
                Json(serde_json::json!({
                    "route_id": route_id,
                    "step_count": step_count,
                    "steps": rows.iter().map(|(operation, sequence, display, predecessor)| serde_json::json!({
                        "operation": operation,
                        "sequence": sequence,
                        "display_order": display,
                        "predecessor": predecessor,
                    })).collect::<Vec<_>>(),
                })),
            )
                .into_response(),
            Err(error) => (StatusCode::BAD_REQUEST, format!("{error}")).into_response(),
        };
    }
    let work_order_id = match uuid::Uuid::parse_str(query.work_order_id.trim()) {
        Ok(value) => value,
        Err(_) => return bad_request("bad work_order_id").into_response(),
    };
    match repo
        .read_work_order_route(tenant_id, work_order_id)
        .await
    {
        Ok(rows) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "work_order_id": work_order_id,
                "steps": rows.iter().map(|(operation, sequence, predecessor, setup)| serde_json::json!({
                    "operation": operation,
                    "sequence": sequence,
                    "predecessor": predecessor,
                    "setup_minutes": setup,
                })).collect::<Vec<_>>(),
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, format!("{error}")).into_response(),
    }
}

async fn save_operation_route_handler(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OperationRouteRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let repo = SharedContractRepo::new(state.db.pool());
    if let Some(route_id) = req
        .route_id
        .as_deref()
        .and_then(|value| uuid::Uuid::parse_str(value).ok())
    {
        return match repo
            .snapshot_work_order_route(tenant_id, &request_id, &request_id, route_id, &req.work_order_no)
            .await
        {
            Ok(Ok((write, id, version_no))) => (
                StatusCode::OK,
                Json(serde_json::json!({"work_order_id": id, "version_no": version_no, "replay": matches!(write, SharedWrite::Replay(_))})),
            )
                .into_response(),
            Ok(Err(_)) => conflict("payload conflict").into_response(),
            Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
        };
    }
    let steps: Vec<erp_domain::manufacturing::OperationStep> = req
        .steps
        .iter()
        .map(|step| erp_domain::manufacturing::OperationStep {
            operation: step.operation.clone(),
            sequence: step.sequence,
            predecessor: step.predecessor.clone(),
            work_center: step.work_center.clone(),
            setup_minutes: step.setup_minutes,
            minutes_per: step.minutes_per,
            inspection_required: step.inspection_required,
            report_field: step.report_field.clone(),
            display_order: step
                .display_order
                .unwrap_or_else(|| step.sequence.saturating_add(1_000)),
        })
        .collect();
    match repo
        .save_operation_route(tenant_id, &request_id, &request_id, req.product_code.trim(), &steps)
        .await
    {
        Ok(Ok((write, id, version_no))) => (
            StatusCode::OK,
            Json(serde_json::json!({"route_id": id, "version_no": version_no, "replay": matches!(write, SharedWrite::Replay(_))})),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct CalendarDayRequest {
    day: String,
    rest: bool,
    shift_minutes: i64,
}

#[derive(Debug, Deserialize)]
struct WorkCenterRequest {
    request_id: String,
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    center_code: String,
    #[serde(default)]
    center_id: Option<String>,
    #[serde(default)]
    machines: i64,
    #[serde(default)]
    days: Vec<CalendarDayRequest>,
    #[serde(default)]
    setup_minutes: i64,
    #[serde(default)]
    run_minutes: i64,
    #[serde(default)]
    rate_cent: i64,
    #[serde(default)]
    start_date: String,
    #[serde(default)]
    end_date: String,
    #[serde(default)]
    as_of: String,
}

#[derive(Debug, Deserialize)]
struct WorkCenterSpanQuery {
    load_id: String,
    day: String,
}

async fn read_work_center_span_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<WorkCenterSpanQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let load_id = match uuid::Uuid::parse_str(query.load_id.trim()) {
        Ok(value) => value,
        Err(_) => return bad_request("bad load_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_work_center_span(tenant_id, load_id, query.day.trim())
        .await
    {
        Ok((rest, minutes, overload)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "load_id": load_id,
                "day": query.day.trim(),
                "rest": rest,
                "minutes": minutes,
                "overload_minutes": overload,
                "optimized": false,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, format!("{error}")).into_response(),
    }
}

async fn save_work_center_handler(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WorkCenterRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let repo = SharedContractRepo::new(state.db.pool());
    let center_id = req
        .center_id
        .as_deref()
        .and_then(|value| uuid::Uuid::parse_str(value).ok());
    match req.action.as_deref() {
        Some("LOAD") => {
            let Some(center_id) = center_id else {
                return bad_request("bad center_id").into_response();
            };
            match repo
                .record_work_center_load(tenant_id, &request_id, &request_id, center_id, req.setup_minutes, req.run_minutes)
                .await
            {
                Ok(Ok((write, id, overload))) => (
                    StatusCode::OK,
                    Json(serde_json::json!({"load_id": id, "overload_minutes": overload, "optimized": false, "replay": matches!(write, SharedWrite::Replay(_))})),
                )
                    .into_response(),
                Ok(Err(_)) => conflict("payload conflict").into_response(),
                Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
            }
        }
        Some("RATE") => {
            let Some(center_id) = center_id else {
                return bad_request("bad center_id").into_response();
            };
            match repo
                .save_work_center_rate(
                    tenant_id,
                    &request_id,
                    &request_id,
                    center_id,
                    req.rate_cent,
                    &req.start_date,
                    &req.end_date,
                    &req.as_of,
                )
                .await
            {
                Ok(Ok((write, id, version_no))) => (
                    StatusCode::OK,
                    Json(serde_json::json!({"rate_id": id, "version_no": version_no, "replay": matches!(write, SharedWrite::Replay(_))})),
                )
                    .into_response(),
                Ok(Err(_)) => conflict("payload conflict").into_response(),
                Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
            }
        }
        _ => {
            let days: Vec<erp_domain::manufacturing::CalendarDay> = req
                .days
                .iter()
                .map(|day| erp_domain::manufacturing::CalendarDay {
                    day: day.day.clone(),
                    rest: day.rest,
                    shift_minutes: day.shift_minutes,
                })
                .collect();
            match repo
                .save_work_center_calendar(tenant_id, &request_id, &request_id, req.center_code.trim(), req.machines, &days)
                .await
            {
                Ok(Ok((write, id))) => (
                    StatusCode::OK,
                    Json(serde_json::json!({"center_id": id, "optimized": false, "replay": matches!(write, SharedWrite::Replay(_))})),
                )
                    .into_response(),
                Ok(Err(_)) => conflict("payload conflict").into_response(),
                Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct BucketStockRequest {
    code: String,
    on_hand: i64,
    allocated_elsewhere: i64,
    wip: i64,
    safety: i64,
    min_lot: i64,
    lot_multiple: i64,
    supply: String,
    lead_days: i32,
}

#[derive(Debug, Deserialize)]
struct BucketSupplyRequest {
    code: String,
    qty_micros: i64,
    arrive_date: String,
    accepted: bool,
}

#[derive(Debug, Deserialize)]
struct BucketDemandRequest {
    code: String,
    qty_micros: i64,
    need_date: String,
    frozen: bool,
}

#[derive(Debug, Deserialize)]
struct BucketBomRequest {
    parent: String,
    component: String,
    qty_per_micros: i64,
}

#[derive(Debug, Deserialize)]
struct BucketMrpRequest {
    request_id: String,
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    run_id: Option<String>,
    #[serde(default)]
    item_code: Option<String>,
    #[serde(default)]
    qty_micros: i64,
    #[serde(default)]
    target_kind: Option<String>,
    #[serde(default)]
    target_line: i32,
    #[serde(default)]
    conversion_request: Option<String>,
    #[serde(default)]
    warehouse: String,
    #[serde(default)]
    bom: Vec<BucketBomRequest>,
    #[serde(default)]
    stock: Vec<BucketStockRequest>,
    #[serde(default)]
    supply: Vec<BucketSupplyRequest>,
    #[serde(default)]
    demand: Vec<BucketDemandRequest>,
    #[serde(default)]
    rest_days: Vec<String>,
    #[serde(default)]
    center_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BucketSuggestionQuery {
    run_id: String,
    item_code: String,
    #[serde(default)]
    view: Option<String>,
}

async fn read_bucket_suggestion_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Query(query): Query<BucketSuggestionQuery>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let Ok(run_id) = uuid::Uuid::parse_str(query.run_id.trim()) else {
        return bad_request("bad run_id").into_response();
    };
    if query
        .view
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case("conversion"))
    {
        return match SharedContractRepo::new(state.db.pool())
            .read_plan_conversion(tenant_id, run_id, query.item_code.trim())
            .await
        {
            Ok((converted, net, accepted, status)) => (
                StatusCode::OK,
                Json(serde_json::json!({
                    "converted_micros": converted,
                    "net_micros": net,
                    "accepted": accepted,
                    "status": status,
                })),
            )
                .into_response(),
            Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
        };
    }
    match SharedContractRepo::new(state.db.pool())
        .read_bucket_suggestion(tenant_id, run_id, query.item_code.trim())
        .await
    {
        Ok((gross, net, suggested, supply, release, accepted, warehouse)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "gross_micros": gross,
                "net_micros": net,
                "suggested_micros": suggested,
                "supply_kind": supply,
                "release_date": release,
                "accepted": accepted,
                "warehouse": warehouse,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn save_bucket_mrp_handler(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<BucketMrpRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let bom = req
        .bom
        .iter()
        .map(|row| BomItem {
            parent: row.parent.clone(),
            component: row.component.clone(),
            qty_per: erp_domain::quantity::ScaledQty::from_micros(row.qty_per_micros),
        })
        .collect::<Vec<_>>();
    let stock = req
        .stock
        .iter()
        .map(|row| erp_domain::manufacturing::BucketStock {
            code: row.code.clone(),
            warehouse: req.warehouse.clone(),
            on_hand: erp_domain::quantity::ScaledQty::from_micros(row.on_hand),
            allocated_elsewhere: erp_domain::quantity::ScaledQty::from_micros(
                row.allocated_elsewhere,
            ),
            wip: erp_domain::quantity::ScaledQty::from_micros(row.wip),
            safety: erp_domain::quantity::ScaledQty::from_micros(row.safety),
            min_lot: erp_domain::quantity::ScaledQty::from_micros(row.min_lot),
            lot_multiple: erp_domain::quantity::ScaledQty::from_micros(row.lot_multiple),
            supply: row.supply.clone(),
            lead_days: row.lead_days,
        })
        .collect::<Vec<_>>();
    let supply = req
        .supply
        .iter()
        .map(|row| erp_domain::manufacturing::BucketSupply {
            code: row.code.clone(),
            warehouse: req.warehouse.clone(),
            qty: erp_domain::quantity::ScaledQty::from_micros(row.qty_micros),
            arrive_date: row.arrive_date.clone(),
            accepted: row.accepted,
        })
        .collect::<Vec<_>>();
    let demand = req
        .demand
        .iter()
        .map(|row| erp_domain::manufacturing::BucketDemand {
            code: row.code.clone(),
            qty: erp_domain::quantity::ScaledQty::from_micros(row.qty_micros),
            need_date: row.need_date.clone(),
            warehouse: req.warehouse.clone(),
            frozen: row.frozen,
        })
        .collect::<Vec<_>>();
    let repo = SharedContractRepo::new(state.db.pool());
    if req
        .action
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case("CANCEL"))
    {
        let Some(run_id) = req
            .run_id
            .as_deref()
            .map(str::trim)
            .and_then(|value| uuid::Uuid::parse_str(value).ok())
        else {
            return bad_request("bad run_id").into_response();
        };
        return match repo
            .cancel_mrp_run(tenant_id, &request_id, &request_id, run_id)
            .await
        {
            Ok(Ok((write, status, kept))) => (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": status,
                    "kept_micros": kept,
                    "replay": matches!(write, SharedWrite::Replay(_)),
                })),
            )
                .into_response(),
            Ok(Err(_)) => conflict("payload conflict").into_response(),
            Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
        };
    }
    if req
        .action
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case("EXPIRE"))
    {
        let Some(run_id) = req
            .run_id
            .as_deref()
            .map(str::trim)
            .and_then(|value| uuid::Uuid::parse_str(value).ok())
        else {
            return bad_request("bad run_id").into_response();
        };
        return match repo
            .expire_mrp_run(tenant_id, &request_id, &request_id, run_id)
            .await
        {
            Ok(Ok((write, status))) => (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": status,
                    "replay": matches!(write, SharedWrite::Replay(_)),
                })),
            )
                .into_response(),
            Ok(Err(_)) => conflict("payload conflict").into_response(),
            Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
        };
    }
    if req
        .action
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case("LOOKUP"))
    {
        let conversion_request = req.conversion_request.as_deref().unwrap_or("").trim();
        if conversion_request.is_empty() {
            return bad_request("conversion request required").into_response();
        }
        return match repo
            .lookup_plan_conversion(tenant_id, &request_id, &request_id, conversion_request)
            .await
        {
            Ok(Ok((write, target_id, qty, line))) => (
                StatusCode::OK,
                Json(serde_json::json!({
                    "target_id": target_id,
                    "qty_micros": qty,
                    "target_line": line,
                    "replay": matches!(write, SharedWrite::Replay(_)),
                })),
            )
                .into_response(),
            Ok(Err(_)) => conflict("payload conflict").into_response(),
            Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
        };
    }
    if req
        .action
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case("CONVERT"))
    {
        let Some(run_id) = req
            .run_id
            .as_deref()
            .map(str::trim)
            .and_then(|value| uuid::Uuid::parse_str(value).ok())
        else {
            return bad_request("bad run_id").into_response();
        };
        let item_code = req.item_code.as_deref().unwrap_or("").trim();
        let target_kind = req.target_kind.as_deref().unwrap_or("").trim();
        if item_code.is_empty() || target_kind.is_empty() {
            return bad_request("conversion needs an item and a target").into_response();
        }
        return match repo
            .convert_mrp_suggestion(
                tenant_id,
                &request_id,
                &request_id,
                run_id,
                item_code,
                req.qty_micros,
                target_kind,
                req.target_line,
            )
            .await
        {
            Ok(Ok((write, target_id, converted, accepted, kit_complete))) => (
                StatusCode::OK,
                Json(serde_json::json!({
                    "target_id": target_id,
                    "converted_micros": converted,
                    "accepted": accepted,
                    "kit_complete": kit_complete,
                    "replay": matches!(write, SharedWrite::Replay(_)),
                })),
            )
                .into_response(),
            Ok(Err(_)) => conflict("payload conflict").into_response(),
            Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
        };
    }
    let center_id = match req
        .center_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(value) => match uuid::Uuid::parse_str(value) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad center_id").into_response(),
        },
        None => None,
    };
    let saved = match center_id {
        Some(center_id) => {
            repo.save_bucket_mrp_on_center(
                tenant_id,
                &request_id,
                &request_id,
                req.warehouse.trim(),
                Some(center_id),
                &bom,
                &stock,
                &supply,
                &demand,
                &req.rest_days,
            )
            .await
        }
        None => {
            repo.save_bucket_mrp(
                tenant_id,
                &request_id,
                &request_id,
                req.warehouse.trim(),
                &bom,
                &stock,
                &supply,
                &demand,
                &req.rest_days,
            )
            .await
        }
    };
    match saved
    {
        Ok(Ok((write, run_id))) => (
            StatusCode::OK,
            Json(serde_json::json!({"run_id": run_id, "replay": matches!(write, SharedWrite::Replay(_))})),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct PlanBucketRequest {
    due_date: String,
    qty_micros: i64,
    source_kind: String,
}

#[derive(Debug, Deserialize)]
struct ProductionPlanRequest {
    request_id: String,
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    source_id: Option<String>,
    #[serde(default)]
    plan_id: Option<String>,
    #[serde(default)]
    line_no: i32,
    #[serde(default)]
    item_code: String,
    #[serde(default)]
    version_no: i32,
    #[serde(default)]
    open_qty: i64,
    #[serde(default)]
    due_date: String,
    #[serde(default)]
    qty_micros: i64,
    #[serde(default)]
    buckets: Vec<PlanBucketRequest>,
    #[serde(default)]
    delivery_id: Option<String>,
}

async fn save_production_plan_handler(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProductionPlanRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let repo = SharedContractRepo::new(state.db.pool());
    let plan_id = req
        .plan_id
        .as_deref()
        .and_then(|value| uuid::Uuid::parse_str(value).ok());
    match req.action.as_deref() {
        Some("EXECUTE") => {
            let Some(plan_id) = plan_id else {
                return bad_request("bad plan_id").into_response();
            };
            match repo
                .execute_plan_bucket(tenant_id, &request_id, &request_id, plan_id, &req.due_date, req.qty_micros)
                .await
            {
                Ok(Ok((write, qty))) => (
                    StatusCode::OK,
                    Json(serde_json::json!({"executed_micros": qty, "replay": matches!(write, SharedWrite::Replay(_))})),
                )
                    .into_response(),
                Ok(Err(_)) => conflict("payload conflict").into_response(),
                Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
            }
        }
        Some("CANCEL") => {
            let Some(plan_id) = plan_id else {
                return bad_request("bad plan_id").into_response();
            };
            match repo
                .cancel_plan_bucket(tenant_id, &request_id, &request_id, plan_id, &req.due_date, req.qty_micros)
                .await
            {
                Ok(Ok((write, left))) => (
                    StatusCode::OK,
                    Json(serde_json::json!({"left_micros": left, "replay": matches!(write, SharedWrite::Replay(_))})),
                )
                    .into_response(),
                Ok(Err(_)) => conflict("payload conflict").into_response(),
                Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
            }
        }
        Some("DELIVER") => {
            let Some(plan_id) = plan_id else {
                return bad_request("bad plan_id").into_response();
            };
            let Some(delivery_id) = req
                .delivery_id
                .as_deref()
                .and_then(|value| uuid::Uuid::parse_str(value).ok())
            else {
                return bad_request("bad delivery_id").into_response();
            };
            match repo
                .apply_plan_delivery(tenant_id, &request_id, &request_id, plan_id, &req.due_date, delivery_id)
                .await
            {
                Ok(Ok((write, left))) => (
                    StatusCode::OK,
                    Json(serde_json::json!({"left_micros": left, "replay": matches!(write, SharedWrite::Replay(_))})),
                )
                    .into_response(),
                Ok(Err(_)) => conflict("payload conflict").into_response(),
                Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
            }
        }
        Some("FREEZE") => {
            let Some(plan_id) = plan_id else {
                return bad_request("bad plan_id").into_response();
            };
            match repo.freeze_production_plan(tenant_id, &request_id, &request_id, plan_id).await {
                Ok(Ok((write, status))) => (
                    StatusCode::OK,
                    Json(serde_json::json!({"status": status, "replay": matches!(write, SharedWrite::Replay(_))})),
                )
                    .into_response(),
                Ok(Err(_)) => conflict("payload conflict").into_response(),
                Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
            }
        }
        _ => {
            let Some(source_id) = req
                .source_id
                .as_deref()
                .and_then(|value| uuid::Uuid::parse_str(value).ok())
            else {
                return bad_request("bad source_id").into_response();
            };
            let buckets: Vec<erp_domain::manufacturing::PlanBucket> = req
                .buckets
                .iter()
                .map(|bucket| erp_domain::manufacturing::PlanBucket {
                    due_date: bucket.due_date.clone(),
                    qty: erp_domain::quantity::ScaledQty::from_micros(bucket.qty_micros),
                    source_kind: bucket.source_kind.clone(),
                })
                .collect();
            match repo
                .save_production_plan(
                    tenant_id,
                    &request_id,
                    &request_id,
                    source_id,
                    req.line_no,
                    req.item_code.trim(),
                    req.version_no,
                    req.open_qty,
                    &buckets,
                )
                .await
            {
                Ok(Ok((write, id))) => (
                    StatusCode::OK,
                    Json(serde_json::json!({"plan_id": id, "replay": matches!(write, SharedWrite::Replay(_))})),
                )
                    .into_response(),
                Ok(Err(_)) => conflict("payload conflict").into_response(),
                Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct MrpCalendarQuery {
    run_id: String,
    purchase_code: String,
    production_code: String,
}

async fn read_mrp_calendar_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Query(query): Query<MrpCalendarQuery>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let Ok(run_id) = uuid::Uuid::parse_str(query.run_id.trim()) else {
        return bad_request("bad run_id").into_response();
    };
    match SharedContractRepo::new(state.db.pool())
        .read_mrp_calendar(
            tenant_id,
            run_id,
            query.purchase_code.trim(),
            query.production_code.trim(),
        )
        .await
    {
        Ok((center, rest, purchase, production)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "center_code": center,
                "rest_day": rest,
                "purchase_date": purchase,
                "production_date": production,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn plan_mrp_run(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<MrpPlanRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let qty = match erp_domain::quantity::parse_qty(&req.demand_qty) {
        Ok(qty) => qty,
        Err(_) => return bad_request("bad demand_qty").into_response(),
    };
    let bom = match bom_lines(&req.bom) {
        Ok(items) => items,
        Err(resp) => return resp,
    };
    let mut stock = Vec::new();
    for row in &req.stock {
        stock.push(match stock_item(row) {
            Ok(item) => item,
            Err(resp) => return resp,
        });
    }
    let need_date = req.need_date.clone().unwrap_or_default();
    let payload = format!(
        "{}:{}:{}:{}:{}",
        req.demand_code.trim(),
        req.demand_qty.trim(),
        need_date.trim(),
        bom_payload(&req.bom),
        stock_payload(&req.stock)
    );
    let demand = [DatedDemand {
        code: req.demand_code.trim().to_string(),
        qty,
        need_date,
    }];
    match SharedContractRepo::new(state.db.pool())
        .save_dated_mrp(
            tenant_id,
            Some(&request_id),
            Some(&payload),
            &bom,
            &stock,
            &demand,
        )
        .await
    {
        Ok(Ok((run_id, suggestions))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "run_id": run_id,
                "results": suggestions.iter().map(|row| serde_json::json!({
                    "code": row.code,
                    "gross": row.gross.as_micros(),
                    "net": row.net_make_or_buy.as_micros(),
                })).collect::<Vec<_>>(),
            })),
        )
            .into_response(),
        Ok(Err(error)) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error:?}")).into_response(),
        Err(error) if error.to_string().contains("payload conflict") => {
            conflict("payload conflict").into_response()
        }
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response(),
    }
}

async fn save_execution_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ExecutionSaveRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let input = match execution_input(&req) {
        Ok(input) => input,
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}:{}:{}:{}:{}:{}",
        req.work_order_no.trim(),
        req.issued.trim(),
        req.consumed.trim(),
        req.good.trim(),
        req.rejected.trim(),
        req.received.trim(),
        req.material_cost_cent,
        req.labor_minutes,
        req.labor_rate_cent
    );
    match SharedContractRepo::new(state.db.pool())
        .save_execution(
            tenant_id,
            &request_id,
            &payload,
            req.work_order_no.trim(),
            input,
        )
        .await
    {
        Ok(Ok((effect, result))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(effect, SharedWrite::Replay(_)),
                "effect_id": effect_id(&effect),
                "wip_micros": result.wip_qty.as_micros(),
                "available_micros": result.available_finished.as_micros(),
                "cost_cent": result.cost_cent,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

pub(crate) fn require_tenant(tenant_id: &str, ctx: &AuthContext) -> Result<TenantId, Response> {
    require_quality_tenant(tenant_id, ctx, false)
}

fn require_quality_tenant(
    tenant_id: &str,
    ctx: &AuthContext,
    concession: bool,
) -> Result<TenantId, Response> {
    let tenant_id =
        TenantId::from_str(tenant_id).map_err(|_| bad_request("bad tenant_id").into_response())?;
    if let Err(resp) = ensure_tenant_access(ctx, tenant_id) {
        return Err(resp.into_response());
    }
    if concession {
        if !ctx.is_admin && !ctx.perms.iter().any(|perm| perm == "QUALITY.CONCESSION") {
            return Err(forbidden("concession permission required").into_response());
        }
    } else if let Err(resp) = ensure_permission(ctx, PermissionCode::ManufacturingWrite.as_str()) {
        return Err(resp.into_response());
    }
    Ok(tenant_id)
}

fn bom_lines(items: &[BomLineRequest]) -> Result<Vec<BomItem>, Response> {
    let mut lines = Vec::new();
    for item in items {
        let qty_per = erp_domain::quantity::parse_qty(&item.qty_per)
            .map_err(|_| bad_request("bad qty_per").into_response())?;
        lines.push(BomItem {
            parent: item.parent.trim().to_string(),
            component: item.component.trim().to_string(),
            qty_per,
        });
    }
    Ok(lines)
}

fn stock_item(row: &MrpStockRequest) -> Result<MrpItem, Response> {
    let parse = |text: &str| {
        erp_domain::quantity::parse_qty(text)
            .map_err(|_| bad_request("bad stock qty").into_response())
    };
    Ok(MrpItem {
        code: row.code.trim().to_string(),
        on_hand: parse(&row.on_hand)?,
        allocated_elsewhere: parse(&row.allocated_elsewhere)?,
        supply_on_time: parse(&row.supply_on_time)?,
        supply_late: parse(&row.supply_late)?,
        safety: parse(&row.safety)?,
    })
}

fn execution_input(req: &ExecutionSaveRequest) -> Result<Execution, Response> {
    let parse = |text: &str| {
        erp_domain::quantity::parse_qty(text)
            .map_err(|_| bad_request("bad execution qty").into_response())
    };
    Ok(Execution {
        issued: parse(&req.issued)?,
        consumed: parse(&req.consumed)?,
        good: parse(&req.good)?,
        rejected: parse(&req.rejected)?,
        received: parse(&req.received)?,
        material_cost_cent: req.material_cost_cent,
        labor_minutes: req.labor_minutes,
        labor_rate_cent: req.labor_rate_cent,
    })
}

fn bom_payload(items: &[BomLineRequest]) -> String {
    items
        .iter()
        .map(|item| {
            format!(
                "{}>{}:{}",
                item.parent.trim(),
                item.component.trim(),
                item.qty_per.trim()
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn routing_payload(steps: &[RoutingLineRequest]) -> String {
    steps
        .iter()
        .map(|step| {
            format!(
                "{}:{}:{}:{}:{}",
                step.sequence,
                step.operation.trim(),
                step.work_center.trim(),
                step.minutes_per,
                step.inspection_required
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn stock_payload(rows: &[MrpStockRequest]) -> String {
    rows.iter()
        .map(|row| {
            format!(
                "{}:{}:{}:{}:{}:{}",
                row.code.trim(),
                row.on_hand.trim(),
                row.allocated_elsewhere.trim(),
                row.supply_on_time.trim(),
                row.supply_late.trim(),
                row.safety.trim()
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn effect_id(effect: &SharedWrite) -> &str {
    match effect {
        SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => &receipt.effect_id,
    }
}

fn receipt_json(request_id: &str, effect: &SharedWrite) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "request_id": request_id,
            "status": "SUCCEEDED",
            "replay": matches!(effect, SharedWrite::Replay(_)),
            "effect_id": effect_id(effect),
        })),
    )
}
