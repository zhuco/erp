//! Public enterprise opening. Handlers only call the locked shared-contract writers.

use super::*;
use erp_repo::shared_contract::{GridQueryInput, GridSelectionInput};

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/onboarding/open", post(open_onboarding_route))
        .route("/v1/onboarding/steps", post(record_onboarding_step_route))
        .route("/v1/onboarding/complete", post(complete_onboarding_route))
        .route(
            "/v1/onboarding/progress",
            get(read_onboarding_progress_route),
        )
        .route(
            "/v1/tenants/:tenant_id/login-clients/:session_id/renewals",
            post(renew_login_client_route),
        )
        .route(
            "/v1/tenants/:tenant_id/credential-epochs",
            post(change_login_password_route),
        )
        .route(
            "/v1/tenants/:tenant_id/free-seats",
            post(open_free_seat_route),
        )
        .route(
            "/v1/tenants/:tenant_id/login-clients",
            post(open_login_client_route),
        )
        .route(
            "/v1/tenants/:tenant_id/inventory/epoch-inbounds",
            post(post_epoch_bound_inbound_route),
        )
        .route(
            "/v1/tenants/:tenant_id/public-origins",
            post(save_public_origin_route),
        )
        .route(
            "/v1/tenants/:tenant_id/browser-sessions",
            post(open_browser_session_route),
        )
        .route(
            "/v1/tenants/:tenant_id/download-links",
            post(issue_download_link_route),
        )
        .route(
            "/v1/tenants/:tenant_id/download-links/:link_id/authorizations",
            post(authorize_download_link_route),
        )
        .route(
            "/v1/tenants/:tenant_id/download-links/:link_id/revocations",
            post(revoke_download_link_route),
        )
        .route(
            "/v1/tenants/:tenant_id/browser-sessions/:session_id/revocations",
            post(revoke_browser_session_route),
        )
        .route(
            "/v1/tenants/:tenant_id/login-clients/password-recoveries",
            post(recover_login_password_route),
        )
        .route(
            "/v1/tenants/:tenant_id/owned-resources",
            post(save_owned_resource_route),
        )
        .route(
            "/v1/tenants/:tenant_id/owned-resources/:resource_id",
            get(read_owned_resource_route).post(write_owned_resource_route),
        )
        .route(
            "/v1/tenants/:tenant_id/owned-resources/:resource_id/file",
            get(download_owned_resource_route),
        )
        .route(
            "/v1/tenants/:tenant_id/company-settings",
            post(save_company_setting_route),
        )
        .route(
            "/v1/tenants/:tenant_id/company-documents",
            post(save_company_document_route),
        )
        .route(
            "/v1/tenants/:tenant_id/demo-scopes",
            post(open_demo_scope_route),
        )
        .route(
            "/v1/tenants/:tenant_id/getting-started",
            post(record_getting_started_route),
        )
        .route(
            "/v1/tenants/:tenant_id/demo-scopes/leave",
            post(leave_demo_scope_route),
        )
        .route(
            "/v1/tenants/:tenant_id/scoped-documents",
            post(save_scoped_document_route),
        )
        .route(
            "/v1/tenants/:tenant_id/login-failures",
            post(record_login_failure_route),
        )
        .route(
            "/v1/tenants/:tenant_id/request-limits",
            post(admit_request_limit_route),
        )
        .route(
            "/v1/tenants/:tenant_id/device-notices",
            post(notice_login_device_route),
        )
        .route(
            "/v1/tenants/:tenant_id/admin-factors",
            post(record_admin_factor_route),
        )
        .route(
            "/v1/tenants/:tenant_id/service-statuses",
            post(publish_service_status_route),
        )
        .route(
            "/v1/tenants/:tenant_id/support-feedback",
            post(open_support_feedback_route),
        )
        .route(
            "/v1/tenants/:tenant_id/support-feedback/:feedback_no/handling",
            post(handle_support_feedback_route),
        )
        .route(
            "/v1/tenants/:tenant_id/diagnostic-exports",
            post(export_support_diagnostic_route),
        )
        .route(
            "/v1/tenants/:tenant_id/service-terms",
            post(accept_service_term_route),
        )
        .route(
            "/v1/tenants/:tenant_id/exit-requests",
            post(open_exit_request_route),
        )
        .route(
            "/v1/tenants/:tenant_id/exit-requests/deliver",
            post(deliver_exit_request_route),
        )
        .route(
            "/v1/tenants/:tenant_id/exit-manifests",
            post(record_exit_manifest_route),
        )
        .route(
            "/v1/tenants/:tenant_id/usage-quotas",
            post(save_usage_quota_route),
        )
        .route(
            "/v1/tenants/:tenant_id/usage-meters",
            get(read_usage_meter_route).post(record_usage_meter_route),
        )
        .route(
            "/v1/tenants/:tenant_id/usage-notices",
            get(read_usage_notice_route).post(record_usage_notice_route),
        )
        .route(
            "/v1/tenants/:tenant_id/page-instances",
            get(read_page_instance_route).post(save_page_instance_route),
        )
        .route(
            "/v1/tenants/:tenant_id/page-themes",
            post(switch_page_theme_route),
        )
        .route(
            "/v1/tenants/:tenant_id/page-panels/hides",
            post(hide_page_panel_route),
        )
        .route(
            "/v1/tenants/:tenant_id/page-closes",
            post(close_page_task_route),
        )
        .route(
            "/v1/tenants/:tenant_id/grid-queries",
            get(read_grid_page_route).post(save_grid_query_route),
        )
        .route(
            "/v1/tenants/:tenant_id/grid-selections",
            post(save_grid_selection_route),
        )
        .route(
            "/v1/tenants/:tenant_id/grid-batch-gates",
            post(gate_grid_batch_route),
        )
        .route(
            "/v1/tenants/:tenant_id/search-conditions",
            get(read_search_condition_route).post(save_search_condition_route),
        )
        .route(
            "/v1/tenants/:tenant_id/named-searches",
            post(save_named_search_route),
        )
        .route(
            "/v1/tenants/:tenant_id/search-resets",
            post(reset_search_route),
        )
        .route(
            "/v1/tenants/:tenant_id/query-generations",
            post(open_query_generation_route),
        )
        .route(
            "/v1/tenants/:tenant_id/query-generations/complete",
            post(complete_query_generation_route),
        )
        .route(
            "/v1/tenants/:tenant_id/cell-edits",
            post(save_cell_edit_route),
        )
        .route(
            "/v1/tenants/:tenant_id/document-saves",
            get(read_document_save_route).post(save_document_atomic_route),
        )
        .route(
            "/v1/tenants/:tenant_id/object-batch-results",
            get(read_object_batch_route).post(save_object_batch_result_route),
        )
        .route(
            "/v1/tenants/:tenant_id/edit-imports",
            post(open_edit_import_route),
        )
        .route(
            "/v1/tenants/:tenant_id/edit-drafts",
            get(read_edit_draft_route).post(save_edit_draft_route),
        )
        .route(
            "/v1/tenants/:tenant_id/edit-drafts/restore",
            post(restore_edit_draft_route),
        )
        .route(
            "/v1/tenants/:tenant_id/edit-drafts/undo",
            post(undo_edit_draft_route),
        )
        .route(
            "/v1/tenants/:tenant_id/lost-responses",
            get(read_lost_response_route).post(lookup_lost_response_route),
        )
        .route(
            "/v1/tenants/:tenant_id/zoom-states",
            get(read_zoom_state_route).post(save_zoom_state_route),
        )
        .route(
            "/v1/tenants/:tenant_id/access-nodes",
            post(save_access_node_route),
        )
        .route(
            "/v1/tenants/:tenant_id/platform-results",
            post(record_platform_result_route),
        )
        .route(
            "/v1/tenants/:tenant_id/file-maps",
            post(save_file_map_route),
        )
        .route(
            "/v1/tenants/:tenant_id/file-cells",
            get(read_file_cell_route).post(save_file_cell_route),
        )
        .route(
            "/v1/tenants/:tenant_id/file-deliveries",
            post(save_file_delivery_route),
        )
        .route(
            "/v1/tenants/:tenant_id/device-prints",
            get(read_device_print_route).post(record_device_print_route),
        )
        .route(
            "/v1/tenants/:tenant_id/report-aggregates",
            get(read_report_aggregate_route).post(save_report_aggregate_route),
        )
        .route(
            "/v1/tenants/:tenant_id/formula-faults",
            get(read_formula_fault_route).post(save_formula_fault_route),
        )
        .route(
            "/v1/tenants/:tenant_id/report-tree-nodes",
            get(read_report_tree_node_route).post(save_report_tree_node_route),
        )
        .route(
            "/v1/tenants/:tenant_id/report-charts",
            post(save_report_chart_route),
        )
        .route(
            "/v1/tenants/:tenant_id/group-exports",
            post(save_group_export_route),
        )
        .route(
            "/v1/tenants/:tenant_id/shared-items",
            get(read_shared_item_route).post(save_shared_item_route),
        )
        .route(
            "/v1/tenants/:tenant_id/shared-item-refs",
            post(reference_shared_item_route),
        )
        .route(
            "/v1/tenants/:tenant_id/shared-item-retirements",
            post(retire_shared_item_route),
        )
        .route(
            "/v1/tenants/:tenant_id/item-identities",
            post(bind_item_identity_route),
        )
        .route(
            "/v1/tenants/:tenant_id/unit-conversions",
            get(read_unit_conversion_route).post(save_unit_conversion_route),
        )
        .route(
            "/v1/tenants/:tenant_id/unit-qtys",
            get(read_unit_qty_route).post(save_unit_qty_route),
        )
        .route(
            "/v1/tenants/:tenant_id/legacy-unit-reads",
            get(read_stored_legacy_unit_route).post(read_legacy_unit_qty_route),
        )
        .route(
            "/v1/tenants/:tenant_id/shared-partners",
            post(save_shared_partner_route),
        )
        .route(
            "/v1/tenants/:tenant_id/partner-contacts",
            post(save_partner_contact_route),
        )
        .route(
            "/v1/tenants/:tenant_id/partner-merge-previews",
            post(preview_partner_merge_route),
        )
        .route(
            "/v1/tenants/:tenant_id/partner-refs",
            post(reference_shared_partner_route),
        )
        .route(
            "/v1/tenants/:tenant_id/partner-merges",
            post(merge_shared_partners_route),
        )
        .route(
            "/v1/tenants/:tenant_id/partner-disables",
            get(read_disabled_partner_route).post(disable_partner_route),
        )
        .route(
            "/v1/tenants/:tenant_id/partner-role-balances",
            post(save_partner_role_balance_route),
        )
        .route(
            "/v1/tenants/:tenant_id/shared-locations",
            get(read_location_by_code_route).post(save_shared_location_route),
        )
        .route(
            "/v1/tenants/:tenant_id/location-refs",
            post(reference_shared_location_route),
        )
        .route(
            "/v1/tenants/:tenant_id/location-retirements",
            get(read_stocked_location_route).post(retire_shared_location_route),
        )
        .route(
            "/v1/tenants/:tenant_id/location-stock",
            post(place_location_stock_route),
        )
        .route(
            "/v1/tenants/:tenant_id/location-moves",
            post(move_location_keeping_stock_route),
        )
        .route(
            "/v1/tenants/:tenant_id/location-renames",
            post(rename_shared_location_route),
        )
        .route(
            "/v1/tenants/:tenant_id/shared-employees",
            get(read_employee_seat_count_route).post(save_shared_employee_route),
        )
        .route(
            "/v1/tenants/:tenant_id/employee-assignments",
            post(assign_shared_employee_route),
        )
        .route(
            "/v1/tenants/:tenant_id/labor-reports",
            get(read_left_employee_route).post(save_labor_report_route),
        )
        .route(
            "/v1/tenants/:tenant_id/employee-leaves",
            post(leave_shared_employee_route),
        )
        .route(
            "/v1/tenants/:tenant_id/labor-rates",
            get(read_frozen_labor_rate_route).post(save_labor_rate_route),
        )
        .route(
            "/v1/tenants/:tenant_id/import-prechecks",
            get(read_import_error_line_route).post(precheck_shared_import_route),
        )
        .route(
            "/v1/tenants/:tenant_id/import-confirms",
            post(confirm_shared_import_route),
        )
        .route(
            "/v1/tenants/:tenant_id/import-downloads",
            post(download_import_result_route),
        )
        .route(
            "/v1/tenants/:tenant_id/opening-gaps",
            get(read_opening_gap_route).post(save_opening_gap_route),
        )
        .route(
            "/v1/tenants/:tenant_id/opening-confirms",
            post(confirm_opening_route),
        )
        .route(
            "/v1/tenants/:tenant_id/opening-resumes",
            post(resume_opening_route),
        )
        .route(
            "/v1/tenants/:tenant_id/opening-rebuilds",
            post(rebuild_opening_route),
        )
        .route(
            "/v1/tenants/:tenant_id/opening-corrections",
            post(correct_opening_route),
        )
        .route(
            "/v1/tenants/:tenant_id/attachment-downloads",
            get(read_attachment_download_route).post(save_attachment_download_link_route),
        )
        .route(
            "/v1/tenants/:tenant_id/attachment-revokes",
            post(revoke_attachment_grant_route),
        )
        .route(
            "/v1/tenants/:tenant_id/shared-attachments",
            post(save_shared_attachment_route),
        )
        .route(
            "/v1/tenants/:tenant_id/attachment-cites",
            post(cite_shared_attachment_route),
        )
        .route(
            "/v1/tenants/:tenant_id/attachment-quotas",
            post(save_attachment_quota_route),
        )
        .route(
            "/v1/tenants/:tenant_id/custom-fields",
            post(save_custom_field_route),
        )
        .route(
            "/v1/tenants/:tenant_id/stock-tenths",
            get(read_tenth_remainder_route),
        )
        .route(
            "/v1/tenants/:tenant_id/stock-ledgers",
            get(read_issue_remainder_route).post(append_stock_ledger_route),
        )
        .route(
            "/v1/tenants/:tenant_id/quality-rebuilds",
            get(read_balance_check_route).post(rebuild_quality_balance_route),
        )
        .route(
            "/v1/tenants/:tenant_id/stock-reservations",
            post(reserve_stock_line_route),
        )
        .route(
            "/v1/tenants/:tenant_id/stock-releases",
            get(read_other_source_reservation_route).post(release_stock_line_route),
        )
        .route(
            "/v1/tenants/:tenant_id/stock-cancellations",
            post(cancel_stock_reservation_route),
        )
        .route(
            "/v1/tenants/:tenant_id/stock-issues",
            get(read_short_issue_route).post(issue_stock_line_route),
        )
        .route(
            "/v1/tenants/:tenant_id/stock-issue-retries",
            get(read_issue_retry_route),
        )
        .route(
            "/v1/tenants/:tenant_id/stock-consumes",
            post(consume_stock_line_route),
        )
        .route(
            "/v1/tenants/:tenant_id/stock-receipts",
            post(receive_shared_stock_route),
        )
        .route(
            "/v1/tenants/:tenant_id/transfer-issues",
            get(read_partial_transfer_route).post(issue_transfer_route),
        )
        .route(
            "/v1/tenants/:tenant_id/transfer-receives",
            get(read_repeat_receive_route).post(receive_transfer_route),
        )
        .route(
            "/v1/tenants/:tenant_id/transfer-losses",
            get(read_transfer_loss_route).post(lose_transfer_route),
        )
        .route(
            "/v1/tenants/:tenant_id/count-confirmations",
            post(confirm_count_adjustment_route),
        )
        .route(
            "/v1/tenants/:tenant_id/count-sheets",
            get(read_sale_during_count_route).post(open_count_sheet_route),
        )
        .route(
            "/v1/tenants/:tenant_id/count-sales",
            post(sell_during_count_route),
        )
        .route(
            "/v1/tenants/:tenant_id/shared-batches",
            post(save_shared_batch_route),
        )
        .route(
            "/v1/tenants/:tenant_id/shared-serials",
            post(assign_shared_serial_route),
        )
        .route(
            "/v1/tenants/:tenant_id/batch-freezes",
            post(freeze_shared_batch_route),
        )
        .route(
            "/v1/tenants/:tenant_id/batch-issues",
            get(read_batch_issue_refusal_route).post(issue_shared_batch_route),
        )
        .route(
            "/v1/tenants/:tenant_id/batch-quarter-issues",
            get(read_batch_quarter_issue_route).post(issue_batch_quarter_route),
        )
        .route(
            "/v1/tenants/:tenant_id/serial-issues",
            get(read_serial_issue_route).post(issue_shared_serial_route),
        )
        .route(
            "/v1/tenants/:tenant_id/batch-uses",
            post(record_batch_use_route),
        )
        .route(
            "/v1/tenants/:tenant_id/batch-receipts",
            post(select_batch_receipt_route),
        )
        .route(
            "/v1/tenants/:tenant_id/batch-traces",
            post(trace_frozen_batch_route),
        )
        .route(
            "/v1/tenants/:tenant_id/cost-receipts",
            get(read_zero_unit_cost_route).post(post_cost_receipt_route),
        )
        .route(
            "/v1/tenants/:tenant_id/cost-issues",
            post(post_cost_issue_route),
        )
        .route(
            "/v1/tenants/:tenant_id/cost-returns",
            get(read_three_micro_return_route).post(post_cost_return_route),
        )
        .route(
            "/v1/tenants/:tenant_id/cost-period-closes",
            post(close_cost_period_route),
        )
        .route(
            "/v1/tenants/:tenant_id/cost-period-adjustments",
            get(read_closed_period_adjustment_route).post(post_closed_period_adjustment_route),
        )
        .route(
            "/v1/tenants/:tenant_id/cost-rebuilds",
            post(rebuild_item_cost_route),
        )
        .route(
            "/v1/tenants/:tenant_id/scans",
            get(read_scan_qty_route).post(record_scan_route),
        )
        .route(
            "/v1/tenants/:tenant_id/scan-batch-faults",
            get(read_wrong_batch_route).post(locate_wrong_batch_route),
        )
        .route(
            "/v1/tenants/:tenant_id/print-events",
            post(record_print_event_route),
        )
        .route(
            "/v1/tenants/:tenant_id/offline-prints",
            get(read_offline_print_route).post(record_offline_print_route),
        )
        .route(
            "/v1/tenants/:tenant_id/paper-codes",
            post(issue_paper_code_route),
        )
        .route(
            "/v1/tenants/:tenant_id/paper-rescans",
            get(read_paper_rescan_route).post(recognize_paper_code_route),
        )
        .route(
            "/v1/tenants/:tenant_id/scan-holds",
            post(hold_repeat_scan_route),
        )
        .route(
            "/v1/tenants/:tenant_id/inventory-alerts",
            post(raise_inventory_alert_route),
        )
        .route(
            "/v1/tenants/:tenant_id/available-alerts",
            get(read_available_alert_route).post(raise_available_alert_route),
        )
        .route(
            "/v1/tenants/:tenant_id/alert-clears",
            get(read_cleared_alert_route).post(clear_alert_after_stock_route),
        )
        .route(
            "/v1/tenants/:tenant_id/expiring-alerts",
            get(read_expiring_alert_route).post(raise_expiring_alert_route),
        )
        .route(
            "/v1/tenants/:tenant_id/aging-alerts",
            get(read_aging_alert_route).post(raise_aging_alert_route),
        )
        .route(
            "/v1/tenants/:tenant_id/shortage-suggestions",
            get(read_shortage_suggestion_route).post(raise_shortage_suggestion_route),
        )
        .route(
            "/v1/tenants/:tenant_id/inventory-alert-handles",
            get(read_handled_alert_facts_route).post(handle_inventory_alert_route),
        )
        .route(
            "/v1/tenants/:tenant_id/allocation-suggestions",
            post(save_allocation_row_route),
        )
        .route(
            "/v1/tenants/:tenant_id/stock-backed-suggestions",
            get(read_stock_backed_suggestion_route).post(save_stock_backed_suggestion_route),
        )
        .route(
            "/v1/tenants/:tenant_id/dated-suggestions",
            get(read_dated_suggestion_route).post(suggest_excluding_expired_route),
        )
        .route(
            "/v1/tenants/:tenant_id/frozen-suggestions",
            get(read_frozen_suggestion_route).post(suggest_excluding_frozen_route),
        )
        .route(
            "/v1/tenants/:tenant_id/short-suggestions",
            get(read_short_suggestion_route).post(suggest_short_from_stock_route),
        )
        .route(
            "/v1/tenants/:tenant_id/stock-backed-issues",
            get(read_remaining_stock_issue_route).post(issue_if_stock_remains_route),
        )
        .route(
            "/v1/tenants/:tenant_id/stock-backed-choices",
            get(read_stock_backed_choice_route).post(choose_if_stock_covers_route),
        )
        .route(
            "/v1/tenants/:tenant_id/allocation-issues",
            post(issue_allocation_suggestion_route),
        )
        .route(
            "/v1/tenants/:tenant_id/purchase-revisions",
            post(save_purchase_revision_route),
        )
        .route(
            "/v1/tenants/:tenant_id/purchase-closes",
            post(close_purchase_when_received_route),
        )
        .route(
            "/v1/tenants/:tenant_id/purchase-remainders",
            get(read_purchase_remainder_route),
        )
        .route(
            "/v1/tenants/:tenant_id/purchase-revision-floors",
            get(read_purchase_revision_floor_route),
        )
        .route(
            "/v1/tenants/:tenant_id/purchase-revision-rollbacks",
            get(read_purchase_revision_rollback_route),
        )
        .route(
            "/v1/tenants/:tenant_id/purchase-arrival-splits",
            get(read_purchase_arrival_split_route),
        )
        .route(
            "/v1/tenants/:tenant_id/purchase-receipt-offsets",
            get(read_purchase_receipt_offset_route),
        )
        .route(
            "/v1/tenants/:tenant_id/purchase-taxes",
            post(sum_purchase_tax_route),
        )
        .route(
            "/v1/tenants/:tenant_id/sales-revisions",
            post(save_sales_revision_route),
        )
        .route(
            "/v1/tenants/:tenant_id/sales-confirms",
            post(confirm_trade_order_route),
        )
        .route(
            "/v1/tenants/:tenant_id/manual-sales",
            get(read_manual_sale_route),
        )
        .route(
            "/v1/tenants/:tenant_id/sales-revision-floors",
            get(read_sales_revision_floor_route),
        )
        .route(
            "/v1/tenants/:tenant_id/sales-revision-conflicts",
            get(read_sales_revision_conflict_route),
        )
        .route(
            "/v1/tenants/:tenant_id/sales-delivery-splits",
            get(read_sales_delivery_split_route),
        )
        .route(
            "/v1/tenants/:tenant_id/sales-delivery-repeats",
            get(read_sales_delivery_repeat_route),
        )
        .route(
            "/v1/tenants/:tenant_id/sales-return-nets",
            get(read_sales_return_net_route),
        )
        .route(
            "/v1/tenants/:tenant_id/draft-sales",
            post(save_draft_sale_route),
        )
}

#[derive(Debug, Deserialize)]
struct OnboardingOpenRequest {
    tenant_id: String,
    request_id: String,
    identity: String,
    verification: String,
    company_name: String,
    mode: String,
    timezone: String,
    currency: String,
    expires_at: String,
    opened_at: String,
}

#[derive(Debug, Deserialize)]
struct OnboardingStepRequest {
    tenant_id: String,
    request_id: String,
    verification: String,
    step: String,
    failed: bool,
}

#[derive(Debug, Deserialize)]
struct OnboardingCompleteRequest {
    tenant_id: String,
    request_id: String,
    verification: String,
}

fn parse_tenant(raw: &str) -> Result<TenantId, Response> {
    TenantId::from_str(raw).map_err(|_| bad_request("bad tenant_id").into_response())
}

fn write_result(
    request_id: &str,
    saved: anyhow::Result<Result<SharedWrite, erp_domain::receipt::ReceiptError>>,
) -> Response {
    match saved {
        Ok(Ok(effect)) => receipt_json(request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

fn receipt_json(request_id: &str, effect: &SharedWrite) -> (StatusCode, Json<serde_json::Value>) {
    let effect_id = match effect {
        SharedWrite::Stored(receipt) | SharedWrite::Replay(receipt) => receipt.effect_id.clone(),
    };
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "request_id": request_id,
            "status": "SUCCEEDED",
            "replay": matches!(effect, SharedWrite::Replay(_)),
            "effect_id": effect_id,
        })),
    )
}

async fn open_onboarding_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<OnboardingOpenRequest>,
) -> impl IntoResponse {
    let tenant_id = match parse_tenant(&req.tenant_id) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}:{}:{}:{}:{}",
        req.identity.trim(),
        req.verification.trim(),
        req.company_name.trim(),
        req.mode.trim(),
        req.timezone.trim(),
        req.currency.trim(),
        req.expires_at.trim(),
        req.opened_at.trim()
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .open_onboarding(
                tenant_id,
                &request_id,
                &payload,
                req.identity.trim(),
                req.verification.trim(),
                req.company_name.trim(),
                req.mode.trim(),
                req.timezone.trim(),
                req.currency.trim(),
                req.expires_at.trim(),
                req.opened_at.trim(),
            )
            .await,
    )
}

async fn record_onboarding_step_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<OnboardingStepRequest>,
) -> impl IntoResponse {
    let tenant_id = match parse_tenant(&req.tenant_id) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}",
        req.verification.trim(),
        req.step.trim(),
        req.failed
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .record_onboarding_step(
                tenant_id,
                &request_id,
                &payload,
                req.verification.trim(),
                req.step.trim(),
                req.failed,
            )
            .await,
    )
}

async fn read_onboarding_progress_route(
    State(state): State<AppState>,
    Query(query): Query<OnboardingProgressQuery>,
) -> impl IntoResponse {
    let tenant_id = match parse_tenant(&query.tenant_id) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if query.request_id.trim().is_empty() {
        return bad_request("request_id required").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_onboarding_progress(tenant_id, query.request_id.trim())
        .await
    {
        Ok(Some((status, verification, steps))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": query.request_id.trim(),
                "status": status,
                "verification": verification,
                "steps": steps.iter().map(|(code, step_status)| serde_json::json!({
                    "step": code,
                    "status": step_status,
                })).collect::<Vec<_>>(),
            })),
        )
            .into_response(),
        Ok(None) => not_found("onboarding request is missing").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct OnboardingProgressQuery {
    tenant_id: String,
    request_id: String,
}

#[derive(Debug, Deserialize)]
struct SessionRenewRequest {
    tenant_id: String,
    request_id: String,
    session_id: String,
}

#[derive(Debug, Deserialize)]
struct PasswordRecoverRequest {
    tenant_id: String,
    request_id: String,
    user_id: String,
    token: String,
}

#[derive(Debug, Deserialize)]
struct OwnedResourceSaveRequest {
    request_id: String,
    kind: String,
    body: String,
}

#[derive(Debug, Deserialize)]
struct OwnedResourceWriteRequest {
    request_id: String,
    body: String,
}

fn authenticated_tenant(path_tenant: &str, ctx: &AuthContext) -> Result<TenantId, Response> {
    let path_tenant = parse_tenant(path_tenant)?;
    if ctx.tenant_id != path_tenant {
        return Err(forbidden("tenant mismatch").into_response());
    }
    Ok(ctx.tenant_id)
}

async fn save_owned_resource_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OwnedResourceSaveRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!("{}:{}", req.kind.trim(), req.body.trim());
    match SharedContractRepo::new(state.db.pool())
        .save_owned_resource(
            tenant_id,
            tenant_id,
            &request_id,
            &payload,
            req.kind.trim(),
            req.body.trim(),
        )
        .await
    {
        Ok(Ok((effect, resource_id))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(effect, SharedWrite::Replay(_)),
                "resource_id": resource_id,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn read_owned_resource_route(
    State(state): State<AppState>,
    Path((path_tenant, resource_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let resource_id = match uuid::Uuid::parse_str(resource_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad resource_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_owned_resource(tenant_id, tenant_id, resource_id)
        .await
    {
        Ok(Some(body)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "resource_id": resource_id,
                "body": body,
            })),
        )
            .into_response(),
        Ok(None) => not_found("resource not found").into_response(),
        Err(_) => not_found("resource not found").into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct CompanySettingSaveRequest {
    request_id: String,
    company_name: String,
    numbering_rule: String,
    currency: String,
    timezone: String,
    qty_precision: i32,
    buy_enabled: bool,
    make_enabled: bool,
    sell_enabled: bool,
    default_warehouse: String,
    effective_at: String,
}

async fn save_company_setting_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CompanySettingSaveRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
        req.company_name.trim(),
        req.numbering_rule.trim(),
        req.currency.trim(),
        req.timezone.trim(),
        req.qty_precision,
        req.buy_enabled,
        req.make_enabled,
        req.sell_enabled,
        req.default_warehouse.trim(),
        req.effective_at.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .save_company_setting(
            tenant_id,
            &request_id,
            &payload,
            req.company_name.trim(),
            req.numbering_rule.trim(),
            req.currency.trim(),
            req.timezone.trim(),
            req.qty_precision,
            req.buy_enabled,
            req.make_enabled,
            req.sell_enabled,
            req.default_warehouse.trim(),
            req.effective_at.trim(),
        )
        .await
    {
        Ok(Ok((effect, version))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(effect, SharedWrite::Replay(_)),
                "version": version,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct CompanyDocumentSaveRequest {
    request_id: String,
    document_no: String,
    kind: String,
    amount_cent: i64,
    document_date: String,
    timezone: String,
}

async fn save_company_document_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CompanyDocumentSaveRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}:{}",
        req.document_no.trim(),
        req.kind.trim(),
        req.amount_cent,
        req.document_date.trim(),
        req.timezone.trim()
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .save_company_document(
                tenant_id,
                &request_id,
                &payload,
                req.document_no.trim(),
                req.kind.trim(),
                req.amount_cent,
                req.document_date.trim(),
                req.timezone.trim(),
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct DemoOpenRequest {
    request_id: String,
}

async fn open_demo_scope_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<DemoOpenRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .open_demo_scope(tenant_id, &request_id, &request_id)
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct GettingStartedRequest {
    request_id: String,
    scope: String,
    step: String,
}

async fn record_getting_started_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<GettingStartedRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!("{}:{}", req.scope.trim(), req.step.trim());
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .record_getting_started(
                tenant_id,
                &request_id,
                &payload,
                req.scope.trim(),
                req.step.trim(),
            )
            .await,
    )
}

async fn leave_demo_scope_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<DemoOpenRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .leave_demo_scope(tenant_id, &request_id, &request_id)
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct ScopedDocumentSaveRequest {
    request_id: String,
    scope: String,
    document_no: String,
    amount_cent: i64,
    warehouse_id: Option<uuid::Uuid>,
    sku_id: Option<uuid::Uuid>,
}

async fn save_scoped_document_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ScopedDocumentSaveRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}:{}",
        req.scope.trim(),
        req.document_no.trim(),
        req.amount_cent,
        req.warehouse_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
        req.sku_id.map(|id| id.to_string()).unwrap_or_default()
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .save_scoped_document_owned(
                tenant_id,
                &request_id,
                &payload,
                req.scope.trim(),
                req.document_no.trim(),
                req.amount_cent,
                req.warehouse_id,
                req.sku_id,
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct LoginFailureRequest {
    request_id: String,
    account: String,
    source: String,
    device: String,
    secret_ref: String,
    failed_count: i32,
}

async fn record_login_failure_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LoginFailureRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}:{}",
        req.account.trim(),
        req.source.trim(),
        req.device.trim(),
        req.secret_ref.trim(),
        req.failed_count
    );
    match SharedContractRepo::new(state.db.pool())
        .record_login_failure(
            tenant_id,
            &request_id,
            &payload,
            req.account.trim(),
            req.source.trim(),
            req.device.trim(),
            req.secret_ref.trim(),
            req.failed_count,
        )
        .await
    {
        Ok(Ok((effect, wait))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(effect, SharedWrite::Replay(_)),
                "backoff_seconds": wait,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct RequestLimitRequest {
    request_id: String,
    account: String,
    source: String,
    observed: i64,
    limit: i64,
}

async fn admit_request_limit_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<RequestLimitRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}",
        req.account.trim(),
        req.source.trim(),
        req.observed,
        req.limit
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .admit_request_limit(
                tenant_id,
                &request_id,
                &payload,
                req.account.trim(),
                req.source.trim(),
                req.observed,
                req.limit,
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct DeviceNoticeRequest {
    request_id: String,
    account: String,
    device: String,
    known: bool,
}

async fn notice_login_device_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<DeviceNoticeRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!("{}:{}:{}", req.account.trim(), req.device.trim(), req.known);
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .notice_login_device(
                tenant_id,
                &request_id,
                &payload,
                req.account.trim(),
                req.device.trim(),
                req.known,
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct AdminFactorRequest {
    request_id: String,
    operator_id: String,
    factor_ok: bool,
    revoked: bool,
}

async fn record_admin_factor_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AdminFactorRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}",
        req.operator_id.trim(),
        req.factor_ok,
        req.revoked
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .record_admin_factor(
                tenant_id,
                &request_id,
                &payload,
                req.operator_id.trim(),
                req.factor_ok,
                req.revoked,
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct ServiceStatusRequest {
    request_id: String,
    code: String,
}

async fn publish_service_status_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ServiceStatusRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .publish_service_status(tenant_id, &request_id, req.code.trim(), req.code.trim())
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct SupportFeedbackOpenRequest {
    request_id: String,
    feedback_no: String,
    request_ref: String,
    feature_id: String,
    document_no: String,
}

async fn open_support_feedback_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SupportFeedbackOpenRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}",
        req.feedback_no.trim(),
        req.request_ref.trim(),
        req.feature_id.trim(),
        req.document_no.trim()
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .open_support_feedback(
                tenant_id,
                &request_id,
                &payload,
                req.feedback_no.trim(),
                req.request_ref.trim(),
                req.feature_id.trim(),
                req.document_no.trim(),
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct SupportHandlingRequest {
    request_id: String,
    scope: String,
    reason: String,
    handler: String,
    result: String,
}

async fn handle_support_feedback_route(
    State(state): State<AppState>,
    Path((path_tenant, feedback_no)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SupportHandlingRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}:{}",
        feedback_no.trim(),
        req.scope.trim(),
        req.reason.trim(),
        req.handler.trim(),
        req.result.trim()
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .handle_support_feedback(
                tenant_id,
                &request_id,
                &payload,
                feedback_no.trim(),
                req.scope.trim(),
                req.reason.trim(),
                req.handler.trim(),
                req.result.trim(),
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct DiagnosticExportRequest {
    request_id: String,
    lines: i64,
    bound: i64,
}

async fn export_support_diagnostic_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<DiagnosticExportRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!("{}:{}", req.lines, req.bound);
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .export_support_diagnostic(tenant_id, &request_id, &payload, req.lines, req.bound)
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct ServiceTermRequest {
    request_id: String,
    version: String,
    purpose: String,
    retention: String,
}

async fn accept_service_term_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ServiceTermRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}",
        req.version.trim(),
        req.purpose.trim(),
        req.retention.trim()
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .accept_service_term(
                tenant_id,
                &request_id,
                &payload,
                req.version.trim(),
                req.purpose.trim(),
                req.retention.trim(),
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct ExitRequestOpen {
    request_id: String,
    applicant: String,
    confirmation: String,
    cutoff: String,
    file_range: String,
    wipe: bool,
}

async fn open_exit_request_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ExitRequestOpen>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}:{}",
        req.applicant.trim(),
        req.confirmation.trim(),
        req.cutoff.trim(),
        req.file_range.trim(),
        req.wipe
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .open_exit_request(
                tenant_id,
                &request_id,
                &payload,
                req.applicant.trim(),
                req.confirmation.trim(),
                req.cutoff.trim(),
                req.file_range.trim(),
                req.wipe,
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct ExitManifestRequest {
    request_id: String,
    item_kind: String,
    item_no: String,
    #[serde(default)]
    line_count: Option<i64>,
    #[serde(default)]
    digest: Option<String>,
}

async fn record_exit_manifest_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ExitManifestRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let owner = tenant_id.0.to_string();
    let digest = req.digest.as_deref().map(str::trim).unwrap_or("");
    let payload = format!(
        "{}:{}:{}:{}:{}",
        owner,
        req.item_kind.trim(),
        req.item_no.trim(),
        req.line_count
            .map(|lines| lines.to_string())
            .unwrap_or_default(),
        digest
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .record_exit_manifest(
                tenant_id,
                &request_id,
                &payload,
                &owner,
                req.item_kind.trim(),
                req.item_no.trim(),
                req.line_count,
                req.digest.as_deref(),
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct ExitDeliverRequest {
    request_id: String,
    wipe: bool,
}

async fn deliver_exit_request_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ExitDeliverRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!("{}", req.wipe);
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .deliver_exit_request(tenant_id, &request_id, &payload, req.wipe)
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct UsageQuotaRequest {
    request_id: String,
    kind: String,
    quota: i64,
    task_cap: i32,
    feature_code: String,
}

async fn save_usage_quota_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<UsageQuotaRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}",
        req.kind.trim(),
        req.quota,
        req.task_cap,
        req.feature_code.trim()
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .save_usage_quota(
                tenant_id,
                &request_id,
                &payload,
                req.kind.trim(),
                req.quota,
                req.task_cap,
                req.feature_code.trim(),
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct UsageMeterRequest {
    request_id: String,
    task: String,
    kind: String,
    quantity: i64,
    window: String,
}

async fn record_usage_meter_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<UsageMeterRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}",
        req.task.trim(),
        req.kind.trim(),
        req.quantity,
        req.window.trim()
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .record_usage_meter(
                tenant_id,
                &request_id,
                &payload,
                req.task.trim(),
                req.kind.trim(),
                req.quantity,
                req.window.trim(),
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct UsageMeterRead {
    kind: String,
    window: String,
}

#[derive(Debug, Deserialize)]
struct UsageNoticeRequest {
    request_id: String,
    kind: String,
    window: String,
    adding: i64,
    offered: i64,
}

async fn read_usage_meter_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<UsageMeterRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if query.kind.trim().is_empty() || query.window.trim().is_empty() {
        return bad_request("usage kind and window are required").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_usage_meter(tenant_id, query.kind.trim(), query.window.trim())
        .await
    {
        Ok((quota, used, tasks)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "kind": query.kind.trim(),
                "window": query.window.trim(),
                "quota": quota,
                "used": used,
                "tasks": tasks,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct PageInstanceRequest {
    request_id: String,
    page: String,
    instance: String,
    service: String,
    theme: String,
    left_hidden: bool,
    right_hidden: bool,
    subtable_hidden: bool,
    query: String,
    draft: String,
}

#[derive(Debug, Deserialize)]
struct PageInstanceRead {
    page: String,
    instance: String,
}

#[derive(Debug, Deserialize)]
struct PagePanelHideRequest {
    request_id: String,
    page: String,
    instance: String,
    panel: String,
}

#[derive(Debug, Deserialize)]
struct PageCloseRequest {
    request_id: String,
    page: String,
    instance: String,
    task_kind: String,
    dirty: bool,
    accepted_write: bool,
}

async fn save_page_instance_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PageInstanceRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let user_id = ctx.user_id.to_string();
    let payload = format!(
        "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
        user_id,
        req.page.trim(),
        req.instance.trim(),
        req.service.trim(),
        req.theme.trim(),
        req.left_hidden,
        req.right_hidden,
        req.subtable_hidden,
        req.query.trim(),
        req.draft.trim()
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .save_page_instance(
                tenant_id,
                &request_id,
                &payload,
                &user_id,
                req.page.trim(),
                req.instance.trim(),
                req.service.trim(),
                req.theme.trim(),
                req.left_hidden,
                req.right_hidden,
                req.subtable_hidden,
                req.query.trim(),
                req.draft.trim(),
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct PageThemeRequest {
    request_id: String,
    page: String,
    instance: String,
    theme: String,
}

async fn switch_page_theme_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PageThemeRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let user_id = ctx.user_id.to_string();
    let payload = format!(
        "{}:{}:{}:{}",
        user_id,
        req.page.trim(),
        req.instance.trim(),
        req.theme.trim()
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .switch_page_theme(
                tenant_id,
                &request_id,
                &payload,
                &user_id,
                req.page.trim(),
                req.instance.trim(),
                req.theme.trim(),
            )
            .await,
    )
}

async fn read_page_instance_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<PageInstanceRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    if query.page.trim().is_empty() || query.instance.trim().is_empty() {
        return bad_request("page and instance are required").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_page_instance(
            tenant_id,
            &ctx.user_id.to_string(),
            query.page.trim(),
            query.instance.trim(),
        )
        .await
    {
        Ok((query_text, draft, left_hidden, right_hidden, subtable_hidden, theme)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "page": query.page.trim(),
                "instance": query.instance.trim(),
                "query": query_text,
                "draft": draft,
                "left_hidden": left_hidden,
                "right_hidden": right_hidden,
                "subtable_hidden": subtable_hidden,
                "theme": theme,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("page instance not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn hide_page_panel_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PagePanelHideRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let user_id = ctx.user_id.to_string();
    let payload = format!(
        "{}:{}:{}:{}",
        user_id,
        req.page.trim(),
        req.instance.trim(),
        req.panel.trim()
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .hide_page_panel(
                tenant_id,
                &request_id,
                &payload,
                &user_id,
                req.page.trim(),
                req.instance.trim(),
                req.panel.trim(),
            )
            .await,
    )
}

async fn close_page_task_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PageCloseRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}:{}",
        req.page.trim(),
        req.instance.trim(),
        req.task_kind.trim(),
        req.dirty,
        req.accepted_write
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .close_page_task(
                tenant_id,
                &request_id,
                &payload,
                req.page.trim(),
                req.instance.trim(),
                req.task_kind.trim(),
                req.dirty,
                req.accepted_write,
            )
            .await,
    )
}

async fn read_usage_notice_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<UsageMeterRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if query.kind.trim().is_empty() {
        return bad_request("usage kind is required").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_usage_notice(tenant_id, query.kind.trim())
        .await
    {
        Ok(Some((decision, expand))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "kind": query.kind.trim(),
                "decision": decision,
                "expand": expand,
            })),
        )
            .into_response(),
        Ok(None) => not_found("usage notice not found").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn record_usage_notice_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<UsageNoticeRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}",
        req.kind.trim(),
        req.window.trim(),
        req.adding,
        req.offered
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .record_usage_notice(
                tenant_id,
                &request_id,
                &payload,
                req.kind.trim(),
                req.window.trim(),
                req.adding,
                req.offered,
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct GridQueryRequest {
    request_id: String,
    page: String,
    filter: String,
    sort_key: String,
    row_id: String,
    offset: i64,
    limit: i32,
    loaded: i64,
    matched: i64,
    exact: bool,
}

async fn save_grid_query_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<GridQueryRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}:{}:{}:{}:{}:{}",
        req.page.trim(),
        req.filter.trim(),
        req.sort_key.trim(),
        req.row_id.trim(),
        req.offset,
        req.limit,
        req.loaded,
        req.matched,
        req.exact
    );
    match SharedContractRepo::new(state.db.pool())
        .save_grid_query(
            tenant_id,
            &request_id,
            &payload,
            GridQueryInput {
                page: req.page.trim(),
                filter: req.filter.trim(),
                sort_key: req.sort_key.trim(),
                row_id: req.row_id.trim(),
                offset: req.offset,
                limit: req.limit,
                loaded: req.loaded,
                matched: req.matched,
                exact: req.exact,
            },
        )
        .await
    {
        Ok(Ok((effect, query_id))) => {
            let (status, Json(mut body)) = receipt_json(&request_id, &effect);
            if let Some(map) = body.as_object_mut() {
                map.insert("query_id".to_string(), serde_json::json!(query_id));
            }
            (status, Json(body)).into_response()
        }
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct GridSelectionRequest {
    request_id: String,
    query_id: String,
    kind: String,
    row_id: String,
    snapshot: String,
    excluded: i64,
    loaded: i64,
    matched: i64,
}

#[derive(Debug, Deserialize)]
struct GridPageRead {
    query_id: String,
}

async fn read_grid_page_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<GridPageRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let query_id = match uuid::Uuid::parse_str(query.query_id.trim()) {
        Ok(id) => id,
        Err(_) => return bad_request("bad query_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_grid_page(tenant_id, query_id)
        .await
    {
        Ok((offset, limit, loaded, matched, rows)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "query_id": query_id,
                "offset": offset,
                "limit": limit,
                "loaded": loaded,
                "matched": matched,
                "cross_page_rows": rows,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("grid query not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn save_grid_selection_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<GridSelectionRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let query_id = match uuid::Uuid::parse_str(req.query_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad query_id").into_response(),
    };
    let payload = format!(
        "{}:{}:{}:{}:{}:{}:{}",
        query_id,
        req.kind.trim(),
        req.row_id.trim(),
        req.snapshot.trim(),
        req.excluded,
        req.loaded,
        req.matched
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .save_grid_selection(
                tenant_id,
                &request_id,
                &payload,
                GridSelectionInput {
                    query_id,
                    kind: req.kind.trim(),
                    row_id: req.row_id.trim(),
                    snapshot: req.snapshot.trim(),
                    excluded: req.excluded,
                    loaded: req.loaded,
                    matched: req.matched,
                },
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct GridBatchGateRequest {
    request_id: String,
    current: String,
    requested: String,
}

async fn gate_grid_batch_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<GridBatchGateRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!("{}:{}", req.current.trim(), req.requested.trim());
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .gate_grid_batch(
                tenant_id,
                &request_id,
                &payload,
                req.current.trim(),
                req.requested.trim(),
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct SearchRead {
    page: String,
}

async fn read_search_condition_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<SearchRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_search_condition(tenant_id, query.page.trim())
        .await
    {
        Ok((decision, low, high, late_result, current_result)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "page": query.page.trim(),
                "decision": decision,
                "low": low,
                "high": high,
                "late_result": late_result,
                "current_result": current_result,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("search condition not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct SearchConditionRequest {
    request_id: String,
    user_id: String,
    page: String,
    kind: String,
    join: String,
    field: String,
    low: String,
    high: String,
    generation: i64,
}

async fn save_search_condition_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SearchConditionRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}:{}:{}:{}:{}",
        req.user_id.trim(),
        req.page.trim(),
        req.kind.trim(),
        req.join.trim(),
        req.field.trim(),
        req.low.trim(),
        req.high.trim(),
        req.generation
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .save_search_condition(
                tenant_id,
                &request_id,
                &payload,
                req.user_id.trim(),
                req.page.trim(),
                req.kind.trim(),
                req.join.trim(),
                req.field.trim(),
                req.low.trim(),
                req.high.trim(),
                req.generation,
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct NamedSearchRequest {
    request_id: String,
    user_id: String,
    page: String,
    name: String,
    field: String,
    field_known: bool,
}

async fn save_named_search_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<NamedSearchRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}:{}",
        req.user_id.trim(),
        req.page.trim(),
        req.name.trim(),
        req.field.trim(),
        req.field_known
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .save_named_search(
                tenant_id,
                &request_id,
                &payload,
                req.user_id.trim(),
                req.page.trim(),
                req.name.trim(),
                req.field.trim(),
                req.field_known,
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct SearchResetRequest {
    request_id: String,
    page: String,
    previous: String,
}

async fn reset_search_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SearchResetRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!("{}:{}", req.page.trim(), req.previous.trim());
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .reset_search(
                tenant_id,
                &request_id,
                &payload,
                req.page.trim(),
                req.previous.trim(),
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct QueryGenerationRequest {
    request_id: String,
    scope: String,
    generation: i64,
    digest: String,
}

async fn open_query_generation_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<QueryGenerationRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}",
        req.scope.trim(),
        req.generation,
        req.digest.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .open_query_generation(
            tenant_id,
            &request_id,
            &payload,
            req.scope.trim(),
            req.generation,
            req.digest.trim(),
        )
        .await
    {
        Ok(Ok((effect, query_id))) => {
            let (status, Json(mut body)) = receipt_json(&request_id, &effect);
            if let Some(object) = body.as_object_mut() {
                object.insert("query_id".to_string(), serde_json::json!(query_id));
            }
            (status, Json(body)).into_response()
        }
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct QueryCompleteRequest {
    request_id: String,
    query_id: String,
    result: String,
}

async fn complete_query_generation_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<QueryCompleteRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let query_id = match uuid::Uuid::parse_str(req.query_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad query_id").into_response(),
    };
    let payload = format!("{}:{}", query_id, req.result.trim());
    match SharedContractRepo::new(state.db.pool())
        .complete_query_generation(
            tenant_id,
            &request_id,
            &payload,
            query_id,
            req.result.trim(),
        )
        .await
    {
        Ok(Ok((effect, _applied))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct CellEditRequest {
    request_id: String,
    document_no: String,
    row_id: String,
    column_id: String,
    unit: String,
    display: String,
    micros: i64,
    read_only: bool,
    confirmed: bool,
}

async fn save_cell_edit_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CellEditRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}:{}:{}:{}:{}",
        req.document_no.trim(),
        req.row_id.trim(),
        req.column_id.trim(),
        req.unit.trim(),
        req.display.trim(),
        req.micros,
        req.read_only,
        req.confirmed
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .save_cell_edit(
                tenant_id,
                &request_id,
                &payload,
                req.document_no.trim(),
                req.row_id.trim(),
                req.column_id.trim(),
                req.unit.trim(),
                req.display.trim(),
                req.micros,
                req.read_only,
                req.confirmed,
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct DocumentSaveRead {
    document_no: String,
}

async fn read_document_save_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<DocumentSaveRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_document_save(tenant_id, query.document_no.trim())
        .await
    {
        Ok((decision, saved_lines, invalid_row)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "document_no": query.document_no.trim(),
                "decision": decision,
                "saved_lines": saved_lines,
                "invalid_row": invalid_row,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("document save not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct DocumentSaveRequest {
    request_id: String,
    document_no: String,
    line_count: i64,
    valid_count: i64,
}

async fn save_document_atomic_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<DocumentSaveRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}",
        req.document_no.trim(),
        req.line_count,
        req.valid_count
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .save_document_atomic(
                tenant_id,
                &request_id,
                &payload,
                req.document_no.trim(),
                req.line_count,
                req.valid_count,
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct ObjectBatchRead {
    saved_id: String,
    conflict_id: String,
}

async fn read_object_batch_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<ObjectBatchRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_object_batch(tenant_id, query.saved_id.trim(), query.conflict_id.trim())
        .await
    {
        Ok((saved, saved_version, conflict, conflict_version)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "saved_id": query.saved_id.trim(),
                "saved": saved,
                "saved_version": saved_version,
                "conflict_id": query.conflict_id.trim(),
                "conflict": conflict,
                "conflict_version": conflict_version,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("object batch not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct ObjectBatchResultRequest {
    request_id: String,
    #[serde(default)]
    object_id: String,
    #[serde(default)]
    version: i64,
    #[serde(default)]
    conflict: bool,
    saved_id: Option<String>,
    saved_version: Option<i64>,
    conflict_id: Option<String>,
    conflict_version: Option<i64>,
}

async fn save_object_batch_result_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ObjectBatchResultRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if let (Some(saved_id), Some(saved_version), Some(conflict_id), Some(conflict_version)) = (
        req.saved_id.as_deref(),
        req.saved_version,
        req.conflict_id.as_deref(),
        req.conflict_version,
    ) {
        let payload = format!(
            "{}:{}:{}:{}",
            saved_id.trim(),
            saved_version,
            conflict_id.trim(),
            conflict_version
        );
        return write_result(
            &request_id,
            SharedContractRepo::new(state.db.pool())
                .save_object_batch_pair(
                    tenant_id,
                    &request_id,
                    &payload,
                    saved_id.trim(),
                    saved_version,
                    conflict_id.trim(),
                    conflict_version,
                )
                .await,
        );
    }
    if req.object_id.trim().is_empty() || req.version < 1 {
        return bad_request("object batch needs one object or a located pair").into_response();
    }
    let payload = format!("{}:{}:{}", req.object_id.trim(), req.version, req.conflict);
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .save_object_batch_result(
                tenant_id,
                &request_id,
                &payload,
                req.object_id.trim(),
                req.version,
                req.conflict,
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct EditImportRequest {
    request_id: String,
    rows: i64,
    budget: i64,
}

async fn open_edit_import_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<EditImportRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!("{}:{}", req.rows, req.budget);
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .open_edit_import(tenant_id, &request_id, &payload, req.rows, req.budget)
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct EditDraftRead {
    user_id: String,
    page: String,
    object_id: String,
    #[serde(default)]
    refused_text: String,
}

async fn read_edit_draft_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<EditDraftRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let repo = SharedContractRepo::new(state.db.pool());
    if !query.refused_text.trim().is_empty() {
        return match repo
            .read_refused_qty_draft(
                tenant_id,
                query.user_id.trim(),
                query.page.trim(),
                query.object_id.trim(),
                query.refused_text.trim(),
            )
            .await
        {
            Ok((body, conversions, samples)) => (
                StatusCode::OK,
                Json(serde_json::json!({
                    "body": body,
                    "tenant_id": tenant_id.to_string(),
                    "conversions": conversions,
                    "samples": samples,
                    "written": false,
                })),
            )
                .into_response(),
            Err(error) if error.to_string().contains("was not found") => {
                not_found("edit draft not found").into_response()
            }
            Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
        };
    }
    match repo
        .read_edit_draft(
            tenant_id,
            query.user_id.trim(),
            query.page.trim(),
            query.object_id.trim(),
        )
        .await
    {
        Ok((body, owner)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "body": body,
                "tenant_id": owner,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("edit draft not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct EditDraftRequest {
    request_id: String,
    user_id: String,
    page: String,
    object_id: String,
    version: i64,
    body: String,
    service_address: String,
}

async fn save_edit_draft_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<EditDraftRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}:{}:{}",
        req.user_id.trim(),
        req.page.trim(),
        req.object_id.trim(),
        req.version,
        req.body.trim(),
        req.service_address.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .save_edit_draft(
            tenant_id,
            &request_id,
            &payload,
            req.user_id.trim(),
            req.page.trim(),
            req.object_id.trim(),
            req.version,
            req.body.trim(),
            req.service_address.trim(),
        )
        .await
    {
        Ok(Ok((effect, _draft_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct DraftRestoreRequest {
    request_id: String,
    draft_id: String,
    current_version: i64,
    service_address: String,
}

async fn restore_edit_draft_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<DraftRestoreRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let draft_id = match uuid::Uuid::parse_str(req.draft_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad draft_id").into_response(),
    };
    let payload = format!(
        "{}:{}:{}",
        draft_id,
        req.current_version,
        req.service_address.trim()
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .restore_edit_draft(
                tenant_id,
                &request_id,
                &payload,
                draft_id,
                req.current_version,
                req.service_address.trim(),
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct DraftUndoRequest {
    request_id: String,
    draft_id: String,
    action: String,
    accepted_write: bool,
}

async fn undo_edit_draft_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<DraftUndoRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let draft_id = match uuid::Uuid::parse_str(req.draft_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad draft_id").into_response(),
    };
    let payload = format!("{}:{}:{}", draft_id, req.action.trim(), req.accepted_write);
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .undo_edit_draft(
                tenant_id,
                &request_id,
                &payload,
                draft_id,
                req.action.trim(),
                req.accepted_write,
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct LostResponseRead {
    command_request: String,
}

async fn read_lost_response_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<LostResponseRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_lost_response(tenant_id, query.command_request.trim())
        .await
    {
        Ok((status, effect, effect_count)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "command_request": query.command_request.trim(),
                "status": status,
                "effect_id": effect,
                "effect_count": effect_count,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("lost response not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct LostResponseRequest {
    request_id: String,
    command_request: String,
}

async fn lookup_lost_response_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LostResponseRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = req.command_request.trim().to_string();
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .lookup_lost_response(tenant_id, &request_id, &payload, req.command_request.trim())
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct ZoomStateRequest {
    request_id: String,
    page: String,
    percent: i32,
    before_row: String,
    after_row: String,
    draft_before: String,
    draft_after: String,
}

#[derive(Debug, Deserialize)]
struct ZoomStateRead {
    page: String,
    percent: i32,
}

async fn read_zoom_state_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<ZoomStateRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_zoom_state(tenant_id, query.page.trim(), query.percent)
        .await
    {
        Ok((percent, row_id, draft)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "page": query.page.trim(),
                "percent": percent,
                "row_id": row_id,
                "draft": draft,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("zoom state not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn save_zoom_state_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ZoomStateRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}:{}:{}",
        req.page.trim(),
        req.percent,
        req.before_row.trim(),
        req.after_row.trim(),
        req.draft_before.trim(),
        req.draft_after.trim()
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .save_zoom_state(
                tenant_id,
                &request_id,
                &payload,
                req.page.trim(),
                req.percent,
                req.before_row.trim(),
                req.after_row.trim(),
                req.draft_before.trim(),
                req.draft_after.trim(),
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct AccessNodeRequest {
    request_id: String,
    control_id: String,
    role: String,
    operable: bool,
}

async fn save_access_node_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AccessNodeRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}",
        req.control_id.trim(),
        req.role.trim(),
        req.operable
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .save_access_node(
                tenant_id,
                &request_id,
                &payload,
                req.control_id.trim(),
                req.role.trim(),
                req.operable,
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct PlatformResultRequest {
    request_id: String,
    feature: String,
    supported: bool,
    source: String,
}

async fn record_platform_result_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PlatformResultRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}",
        req.feature.trim(),
        req.supported,
        req.source.trim()
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .record_platform_result(
                tenant_id,
                &request_id,
                &payload,
                req.feature.trim(),
                req.supported,
                req.source.trim(),
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct FileMapRequest {
    request_id: String,
    format: String,
    source: String,
    field: String,
    required: bool,
}

async fn save_file_map_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<FileMapRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}",
        req.format.trim(),
        req.source.trim(),
        req.field.trim(),
        req.required
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .save_file_map(
                tenant_id,
                &request_id,
                &payload,
                req.format.trim(),
                req.source.trim(),
                req.field.trim(),
                req.required,
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct FileCellRequest {
    request_id: String,
    file_no: String,
    kind: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct FileCellRead {
    file_no: String,
    kind: String,
}

async fn read_file_cell_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<FileCellRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_file_cell(tenant_id, query.file_no.trim(), query.kind.trim())
        .await
    {
        Ok((kind, value, encoding)) => {
            let mut writer = csv::WriterBuilder::new().from_writer(Vec::new());
            if writer.write_record([&kind, &value, &encoding]).is_err() {
                return (StatusCode::INTERNAL_SERVER_ERROR, "export parse failed").into_response();
            }
            let bytes = match writer.into_inner() {
                Ok(bytes) => bytes,
                Err(_) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, "export parse failed")
                        .into_response();
                }
            };
            let parsed = csv::ReaderBuilder::new()
                .has_headers(false)
                .from_reader(bytes.as_slice())
                .records()
                .next();
            let Some(Ok(record)) = parsed else {
                return (StatusCode::INTERNAL_SERVER_ERROR, "export parse failed").into_response();
            };
            if record.get(1) != Some(value.as_str()) || record.get(2) != Some(encoding.as_str()) {
                return (StatusCode::UNPROCESSABLE_ENTITY, "export parse mismatch").into_response();
            }
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "file_no": query.file_no.trim(),
                    "kind": record.get(0).unwrap_or(""),
                    "parsed_value": record.get(1).unwrap_or(""),
                    "encoding": record.get(2).unwrap_or(""),
                    "formula": false,
                })),
            )
                .into_response()
        }
        Err(error) if error.to_string().contains("was not found") => {
            not_found("file cell not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn save_file_cell_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<FileCellRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}",
        req.file_no.trim(),
        req.kind.trim(),
        req.value.trim()
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .save_file_cell(
                tenant_id,
                &request_id,
                &payload,
                req.file_no.trim(),
                req.kind.trim(),
                req.value.trim(),
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct FileDeliveryRequest {
    request_id: String,
    file_no: String,
    rows: i64,
    bound: i64,
}

async fn save_file_delivery_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<FileDeliveryRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!("{}:{}:{}", req.file_no.trim(), req.rows, req.bound);
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .save_file_delivery(
                tenant_id,
                &request_id,
                &payload,
                req.file_no.trim(),
                req.rows,
                req.bound,
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct DevicePrintRequest {
    request_id: String,
    device_id: String,
    document_no: String,
    agent: bool,
    status: String,
}

#[derive(Debug, Deserialize)]
struct DevicePrintRead {
    device_id: String,
    document_no: String,
}

async fn read_device_print_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<DevicePrintRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_device_print(tenant_id, query.device_id.trim(), query.document_no.trim())
        .await
    {
        Ok((device, document, status)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "device_id": device,
                "document_no": document,
                "status": status,
                "printed": status == "PRINTED",
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("device print not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn record_device_print_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<DevicePrintRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}",
        req.device_id.trim(),
        req.document_no.trim(),
        req.agent,
        req.status.trim()
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .record_device_print(
                tenant_id,
                &request_id,
                &payload,
                req.device_id.trim(),
                req.document_no.trim(),
                req.agent,
                req.status.trim(),
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct ReportAggregateRead {
    query_no: String,
    dimension: String,
}

#[derive(Debug, Deserialize)]
struct ReportAggregateRequest {
    request_id: String,
    query_no: String,
    scope: String,
    dimension: String,
    total: i64,
    formula: String,
}

#[derive(Debug, Deserialize)]
struct FormulaFaultRead {
    query_no: String,
    field: String,
}

#[derive(Debug, Deserialize)]
struct FormulaFaultRequest {
    request_id: String,
    query_no: String,
    field: String,
    formula: String,
}

async fn read_formula_fault_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<FormulaFaultRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_formula_fault(tenant_id, query.query_no.trim(), query.field.trim())
        .await
    {
        Ok((field, formula, decision)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "query_no": query.query_no.trim(),
                "field": field,
                "formula": formula,
                "decision": decision,
                "executed": false,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("formula fault not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn save_formula_fault_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<FormulaFaultRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}",
        req.query_no.trim(),
        req.field.trim(),
        req.formula.trim()
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .save_formula_fault(
                tenant_id,
                &request_id,
                &payload,
                req.query_no.trim(),
                req.field.trim(),
                req.formula.trim(),
            )
            .await,
    )
}

async fn read_report_aggregate_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<ReportAggregateRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_report_aggregate(tenant_id, query.query_no.trim(), query.dimension.trim())
        .await
    {
        Ok((page_total, full_total, loaded, matched)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "query_no": query.query_no.trim(),
                "dimension": query.dimension.trim(),
                "page_total": page_total,
                "full_total": full_total,
                "loaded": loaded,
                "matched": matched,
                "page_stands_for_full": false,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("report aggregate not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn save_report_aggregate_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ReportAggregateRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}:{}",
        req.query_no.trim(),
        req.scope.trim(),
        req.dimension.trim(),
        req.total,
        req.formula.trim()
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .save_report_aggregate(
                tenant_id,
                &request_id,
                &payload,
                req.query_no.trim(),
                req.scope.trim(),
                req.dimension.trim(),
                req.total,
                req.formula.trim(),
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct ReportTreeNodeRequest {
    request_id: String,
    query_no: String,
    level: i32,
    node: String,
    parent: String,
    total_before: i64,
    total_after: i64,
}

#[derive(Debug, Deserialize)]
struct ReportTreeNodeRead {
    query_no: String,
    node: String,
}

async fn read_report_tree_node_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<ReportTreeNodeRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_report_tree_node(tenant_id, query.query_no.trim(), query.node.trim())
        .await
    {
        Ok((level, node, total)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "query_no": query.query_no.trim(),
                "level": level,
                "node": node,
                "total": total,
                "folded_changed_total": false,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("tree node not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn save_report_tree_node_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ReportTreeNodeRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}:{}:{}",
        req.query_no.trim(),
        req.level,
        req.node.trim(),
        req.parent.trim(),
        req.total_before,
        req.total_after
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .save_report_tree_node(
                tenant_id,
                &request_id,
                &payload,
                req.query_no.trim(),
                req.level,
                req.node.trim(),
                req.parent.trim(),
                req.total_before,
                req.total_after,
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct ReportChartRequest {
    request_id: String,
    query_no: String,
    query_total: i64,
    chart_total: i64,
}

async fn save_report_chart_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ReportChartRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}",
        req.query_no.trim(),
        req.query_total,
        req.chart_total
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .save_report_chart(
                tenant_id,
                &request_id,
                &payload,
                req.query_no.trim(),
                req.query_total,
                req.chart_total,
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct GroupExportRequest {
    request_id: String,
    query_no: String,
    scope: String,
    exported: i64,
    sql_sum: i64,
}

async fn save_group_export_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<GroupExportRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}",
        req.query_no.trim(),
        req.scope.trim(),
        req.exported,
        req.sql_sum
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .save_group_export(
                tenant_id,
                &request_id,
                &payload,
                req.query_no.trim(),
                req.scope.trim(),
                req.exported,
                req.sql_sum,
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct SharedItemRequest {
    request_id: String,
    item_code: String,
    name: String,
    spec: String,
    category: String,
    barcode: String,
    buy: bool,
    make: bool,
    sell: bool,
}

#[derive(Debug, Deserialize)]
struct SharedItemRead {
    item_code: String,
}

async fn read_shared_item_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<SharedItemRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_shared_item(tenant_id, query.item_code.trim())
        .await
    {
        Ok((item_id, status, buy, make, sell, refs)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "item_id": item_id,
                "item_code": query.item_code.trim(),
                "status": status,
                "buy": buy,
                "make": make,
                "sell": sell,
                "refs": refs,
                "hard_deleted": false,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("shared item not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn save_shared_item_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SharedItemRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}:{}:{}:{}:{}",
        req.item_code.trim(),
        req.name.trim(),
        req.spec.trim(),
        req.category.trim(),
        req.barcode.trim(),
        req.buy,
        req.make,
        req.sell
    );
    match SharedContractRepo::new(state.db.pool())
        .save_shared_item(
            tenant_id,
            &request_id,
            &payload,
            req.item_code.trim(),
            req.name.trim(),
            req.spec.trim(),
            req.category.trim(),
            req.barcode.trim(),
            req.buy,
            req.make,
            req.sell,
        )
        .await
    {
        Ok(Ok((effect, _item_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct SharedItemRefRequest {
    request_id: String,
    item_id: String,
    document_id: String,
}

async fn reference_shared_item_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SharedItemRefRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let item_id = match uuid::Uuid::parse_str(req.item_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad item_id").into_response(),
    };
    let payload = format!("{}:{}", item_id, req.document_id.trim());
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .reference_shared_item(
                tenant_id,
                &request_id,
                &payload,
                item_id,
                req.document_id.trim(),
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct SharedItemRetireRequest {
    request_id: String,
    item_id: String,
    requested: String,
}

async fn retire_shared_item_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SharedItemRetireRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let item_id = match uuid::Uuid::parse_str(req.item_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad item_id").into_response(),
    };
    let payload = format!("{}:{}", item_id, req.requested.trim());
    match SharedContractRepo::new(state.db.pool())
        .retire_shared_item(
            tenant_id,
            &request_id,
            &payload,
            item_id,
            req.requested.trim(),
        )
        .await
    {
        Ok(Ok((effect, _decision))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct ItemIdentityRequest {
    request_id: String,
    item_id: String,
    product_code: String,
    sku_code: String,
}

async fn bind_item_identity_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ItemIdentityRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let item_id = match uuid::Uuid::parse_str(req.item_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad item_id").into_response(),
    };
    let payload = format!(
        "{}:{}:{}",
        item_id,
        req.product_code.trim(),
        req.sku_code.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .bind_item_identity(
            tenant_id,
            &request_id,
            &payload,
            item_id,
            req.product_code.trim(),
            req.sku_code.trim(),
        )
        .await
    {
        Ok(Ok((effect, _product_id, _sku_id))) => {
            receipt_json(&request_id, &effect).into_response()
        }
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct UnitConversionRequest {
    request_id: String,
    base_unit: String,
    aux_unit: String,
    ratio_text: String,
    rounding_code: String,
    aux_text: String,
    display_text: String,
}

#[derive(Debug, Deserialize)]
struct UnitConversionRead {
    base_unit: String,
    aux_unit: String,
    rounding_code: String,
}

async fn read_unit_conversion_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<UnitConversionRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_unit_conversion(
            tenant_id,
            query.base_unit.trim(),
            query.aux_unit.trim(),
            query.rounding_code.trim(),
        )
        .await
    {
        Ok((ratio, aux, base)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "base_unit": query.base_unit.trim(),
                "aux_unit": query.aux_unit.trim(),
                "rounding_code": query.rounding_code.trim(),
                "ratio_micros": ratio,
                "aux_micros": aux,
                "base_micros": base,
                "zero_ratio_stored": false,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("unit conversion not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn save_unit_conversion_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<UnitConversionRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}:{}:{}",
        req.base_unit.trim(),
        req.aux_unit.trim(),
        req.ratio_text.trim(),
        req.rounding_code.trim(),
        req.aux_text.trim(),
        req.display_text.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .save_unit_conversion(
            tenant_id,
            &request_id,
            &payload,
            req.base_unit.trim(),
            req.aux_unit.trim(),
            req.ratio_text.trim(),
            req.rounding_code.trim(),
            req.aux_text.trim(),
            req.display_text.trim(),
        )
        .await
    {
        Ok(Ok((effect, _base_micros, _ratio_micros))) => {
            receipt_json(&request_id, &effect).into_response()
        }
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct UnitQtyRequest {
    request_id: String,
    unit_code: String,
    qty_text: String,
    display_text: String,
}

#[derive(Debug, Deserialize)]
struct UnitQtyRead {
    unit_code: String,
}

async fn read_unit_qty_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<UnitQtyRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_unit_qty(tenant_id, query.unit_code.trim())
        .await
    {
        Ok((micros, text)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "unit_code": query.unit_code.trim(),
                "qty_micros": micros,
                "qty_text": text,
                "collapsed_to_zero": false,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("unit quantity not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct LegacyUnitStoredRead {
    legacy_units: i64,
}

async fn read_stored_legacy_unit_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<LegacyUnitStoredRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_stored_legacy_unit(tenant_id, query.legacy_units)
        .await
    {
        Ok((micros, text)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "legacy_units": query.legacy_units,
                "qty_micros": micros,
                "qty_text": text,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("legacy quantity not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn save_unit_qty_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<UnitQtyRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}",
        req.unit_code.trim(),
        req.qty_text.trim(),
        req.display_text.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .save_unit_qty(
            tenant_id,
            &request_id,
            &payload,
            req.unit_code.trim(),
            req.qty_text.trim(),
            req.display_text.trim(),
        )
        .await
    {
        Ok(Ok((effect, _micros))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct LegacyUnitReadRequest {
    request_id: String,
    legacy_units: i64,
    display_text: String,
}

async fn read_legacy_unit_qty_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LegacyUnitReadRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!("{}:{}", req.legacy_units, req.display_text.trim());
    match SharedContractRepo::new(state.db.pool())
        .read_legacy_unit_qty(
            tenant_id,
            &request_id,
            &payload,
            req.legacy_units,
            req.display_text.trim(),
        )
        .await
    {
        Ok(Ok((effect, _micros))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct SharedPartnerRequest {
    request_id: String,
    name: String,
    customer: bool,
    supplier: bool,
    customer_no: String,
    supplier_no: String,
    term_days: i32,
    tax_no: String,
}

async fn save_shared_partner_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SharedPartnerRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}:{}:{}:{}",
        req.name.trim(),
        req.customer,
        req.supplier,
        req.customer_no.trim(),
        req.supplier_no.trim(),
        req.term_days,
        req.tax_no.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .save_shared_partner(erp_repo::shared_contract::SharedPartnerInput { tenant_id, request_id: &request_id, payload_hash: &payload, name: req.name.trim(), customer: req.customer, supplier: req.supplier, customer_no: req.customer_no.trim(), supplier_no: req.supplier_no.trim(), term_days: req.term_days, tax_no: req.tax_no.trim() })
        .await
    {
        Ok(Ok((effect, _partner_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct PartnerContactRequest {
    request_id: String,
    partner_id: String,
    contact_name: String,
    phone: String,
    address: String,
}

async fn save_partner_contact_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PartnerContactRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let partner_id = match uuid::Uuid::parse_str(req.partner_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad partner_id").into_response(),
    };
    let payload = format!(
        "{}:{}:{}:{}",
        partner_id,
        req.contact_name.trim(),
        req.phone.trim(),
        req.address.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .save_partner_contact(erp_repo::shared_contract::PartnerContactInput { tenant_id, request_id: &request_id, payload_hash: &payload, partner_id, contact_name: req.contact_name.trim(), phone: req.phone.trim(), address: req.address.trim() })
        .await
    {
        Ok(Ok(effect)) => write_result(&request_id, Ok(Ok(effect))).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct PartnerMergePreviewRequest {
    request_id: String,
    source_id: String,
    target_id: String,
}

async fn preview_partner_merge_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PartnerMergePreviewRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let source_id = match uuid::Uuid::parse_str(req.source_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad source_id").into_response(),
    };
    let target_id = match uuid::Uuid::parse_str(req.target_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad target_id").into_response(),
    };
    let payload = format!("{source_id}:{target_id}");
    match SharedContractRepo::new(state.db.pool())
        .preview_partner_merge(tenant_id, &request_id, &payload, source_id, target_id)
        .await
    {
        Ok(Ok((effect, _count))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct PartnerRefRequest {
    request_id: String,
    partner_id: String,
    document_id: String,
    document_date: String,
}

async fn reference_shared_partner_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PartnerRefRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let partner_id = match uuid::Uuid::parse_str(req.partner_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad partner_id").into_response(),
    };
    let payload = format!(
        "{}:{}:{}",
        partner_id,
        req.document_id.trim(),
        req.document_date.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .reference_shared_partner(
            tenant_id,
            &request_id,
            &payload,
            partner_id,
            req.document_id.trim(),
            req.document_date.trim(),
        )
        .await
    {
        Ok(Ok(effect)) => write_result(&request_id, Ok(Ok(effect))).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct PartnerMergeRequest {
    request_id: String,
    source_id: String,
    target_id: String,
}

async fn merge_shared_partners_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PartnerMergeRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let source_id = match uuid::Uuid::parse_str(req.source_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad source_id").into_response(),
    };
    let target_id = match uuid::Uuid::parse_str(req.target_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad target_id").into_response(),
    };
    let payload = format!("{source_id}:{target_id}");
    match SharedContractRepo::new(state.db.pool())
        .merge_shared_partners(tenant_id, &request_id, &payload, source_id, target_id)
        .await
    {
        Ok(Ok((effect, _moved))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct PartnerRoleBalanceRequest {
    request_id: String,
    partner_id: String,
    receivable_cent: i64,
    payable_cent: i64,
}

async fn save_partner_role_balance_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PartnerRoleBalanceRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let partner_id = match uuid::Uuid::parse_str(req.partner_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad partner_id").into_response(),
    };
    let payload = format!(
        "{}:{}:{}",
        partner_id, req.receivable_cent, req.payable_cent
    );
    match SharedContractRepo::new(state.db.pool())
        .reconcile_partner_roles(
            tenant_id,
            &request_id,
            &payload,
            partner_id,
            req.receivable_cent,
            req.payable_cent,
        )
        .await
    {
        Ok(Ok((effect, _, _))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct PartnerDisableRead {
    partner_id: String,
}

#[derive(Debug, Deserialize)]
struct PartnerDisableRequest {
    request_id: String,
    partner_id: String,
    requested: String,
}

async fn read_disabled_partner_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<PartnerDisableRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let partner_id = match uuid::Uuid::parse_str(query.partner_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad partner_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_disabled_partner(tenant_id, partner_id)
        .await
    {
        Ok((status, receivable, payable)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "partner_id": partner_id,
                "status": status,
                "receivable_cent": receivable,
                "payable_cent": payable,
                "balance_removed": false,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("disabled partner not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn disable_partner_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PartnerDisableRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let partner_id = match uuid::Uuid::parse_str(req.partner_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad partner_id").into_response(),
    };
    let payload = format!("{}:{}", partner_id, req.requested.trim());
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .disable_partner_with_receivable(
                tenant_id,
                &request_id,
                &payload,
                partner_id,
                req.requested.trim(),
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct SharedLocationRequest {
    request_id: String,
    parent_id: Option<String>,
    code: String,
    purpose: String,
    is_default: bool,
}

#[derive(Debug, Deserialize)]
struct LocationCodeRead {
    code: String,
}

async fn read_location_by_code_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<LocationCodeRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_location_by_code(tenant_id, query.code.trim())
        .await
    {
        Ok((location_id, status, rows)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "location_id": location_id.to_string(),
                "code": query.code.trim(),
                "status": status,
                "rows": rows,
                "duplicated": false,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("location not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn save_shared_location_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SharedLocationRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let parent_id = match req
        .parent_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(id) if !id.is_nil() => Some(id),
            _ => return bad_request("bad parent_id").into_response(),
        },
        None => None,
    };
    let payload = format!(
        "{}:{}:{}:{}",
        parent_id.map(|id| id.to_string()).unwrap_or_default(),
        req.code.trim(),
        req.purpose.trim(),
        req.is_default
    );
    match SharedContractRepo::new(state.db.pool())
        .save_shared_location(
            tenant_id,
            &request_id,
            &payload,
            parent_id,
            req.code.trim(),
            req.purpose.trim(),
            req.is_default,
        )
        .await
    {
        Ok(Ok((effect, _location_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct LocationRefRequest {
    request_id: String,
    location_id: String,
    document_id: String,
}

async fn reference_shared_location_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LocationRefRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let location_id = match uuid::Uuid::parse_str(req.location_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad location_id").into_response(),
    };
    let payload = format!("{}:{}", location_id, req.document_id.trim());
    match SharedContractRepo::new(state.db.pool())
        .reference_shared_location(
            tenant_id,
            &request_id,
            &payload,
            location_id,
            req.document_id.trim(),
        )
        .await
    {
        Ok(Ok(effect)) => write_result(&request_id, Ok(Ok(effect))).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct LocationRetirementRead {
    location_id: String,
}

async fn read_stocked_location_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<LocationRetirementRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let location_id = match uuid::Uuid::parse_str(query.location_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad location_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_stocked_location(tenant_id, location_id)
        .await
    {
        Ok((code, status, on_hand, owner, deleted)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "location_id": location_id.to_string(),
                "code": code,
                "status": status,
                "on_hand": on_hand,
                "owner_code": owner,
                "deleted": deleted,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("stocked location not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct LocationRetirementRequest {
    request_id: String,
    location_id: String,
    requested: String,
}

async fn retire_shared_location_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LocationRetirementRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let location_id = match uuid::Uuid::parse_str(req.location_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad location_id").into_response(),
    };
    let payload = format!("{}:{}", location_id, req.requested.trim());
    match SharedContractRepo::new(state.db.pool())
        .retire_shared_location(
            tenant_id,
            &request_id,
            &payload,
            location_id,
            req.requested.trim(),
        )
        .await
    {
        Ok(Ok((effect, _decision))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct LocationRenameRequest {
    request_id: String,
    location_id: String,
    code: String,
}

async fn rename_shared_location_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LocationRenameRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let location_id = match uuid::Uuid::parse_str(req.location_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad location_id").into_response(),
    };
    let payload = format!("{}:{}", location_id, req.code.trim());
    match SharedContractRepo::new(state.db.pool())
        .rename_shared_location(
            tenant_id,
            &request_id,
            &payload,
            location_id,
            req.code.trim(),
        )
        .await
    {
        Ok(Ok(effect)) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct LocationStockRequest {
    request_id: String,
    location_id: String,
    sku_id: String,
    on_hand: i64,
    owner_code: String,
}

async fn place_location_stock_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LocationStockRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let location_id = match uuid::Uuid::parse_str(req.location_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad location_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    let payload = format!(
        "{}:{}:{}:{}",
        location_id,
        sku_id,
        req.on_hand,
        req.owner_code.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .place_location_stock(
            tenant_id,
            &request_id,
            &payload,
            location_id,
            sku_id,
            req.on_hand,
            req.owner_code.trim(),
        )
        .await
    {
        Ok(Ok(effect)) => write_result(&request_id, Ok(Ok(effect))).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct LocationMoveRequest {
    request_id: String,
    location_id: String,
    parent_id: String,
    sku_id: String,
}

async fn move_location_keeping_stock_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LocationMoveRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let location_id = match uuid::Uuid::parse_str(req.location_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad location_id").into_response(),
    };
    let parent_id = match uuid::Uuid::parse_str(req.parent_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad parent_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    let payload = format!("{location_id}:{parent_id}:{sku_id}");
    match SharedContractRepo::new(state.db.pool())
        .move_location_keeping_stock(
            tenant_id,
            &request_id,
            &payload,
            location_id,
            parent_id,
            sku_id,
        )
        .await
    {
        Ok(Ok(effect)) => write_result(&request_id, Ok(Ok(effect))).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct SharedEmployeeRequest {
    request_id: String,
    employee_code: String,
    user_id: String,
    department: String,
}

async fn read_employee_seat_count_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_employee_seat_count(tenant_id)
        .await
    {
        Ok((files, seats)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "files": files,
                "seats": seats,
                "seats_opened": false,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn save_shared_employee_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SharedEmployeeRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}",
        req.employee_code.trim(),
        req.user_id.trim(),
        req.department.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .save_shared_employee(
            tenant_id,
            &request_id,
            &payload,
            req.employee_code.trim(),
            req.user_id.trim(),
            req.department.trim(),
        )
        .await
    {
        Ok(Ok((effect, _employee_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct EmployeeAssignmentRequest {
    request_id: String,
    employee_id: String,
    crew_code: String,
    work_center: String,
    start_date: String,
    end_date: String,
}

async fn assign_shared_employee_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<EmployeeAssignmentRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let employee_id = match uuid::Uuid::parse_str(req.employee_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad employee_id").into_response(),
    };
    let payload = format!(
        "{}:{}:{}:{}:{}",
        employee_id,
        req.crew_code.trim(),
        req.work_center.trim(),
        req.start_date.trim(),
        req.end_date.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .assign_shared_employee(
            tenant_id,
            &request_id,
            &payload,
            employee_id,
            req.crew_code.trim(),
            req.work_center.trim(),
            req.start_date.trim(),
            req.end_date.trim(),
        )
        .await
    {
        Ok(Ok(effect)) => write_result(&request_id, Ok(Ok(effect))).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct LeftEmployeeRead {
    employee_id: String,
}

async fn read_left_employee_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<LeftEmployeeRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let employee_id = match uuid::Uuid::parse_str(query.employee_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad employee_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_left_employee(tenant_id, employee_id)
        .await
    {
        Ok((status, report_id, seats, reporter, certifier, reports)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "employee_id": employee_id.to_string(),
                "status": status,
                "report_id": report_id,
                "seats": seats,
                "reporter_user_id": reporter,
                "certifier_user_id": certifier,
                "reports": reports,
                "seat_opened": false,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("left employee not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct EmployeeLeaveRequest {
    request_id: String,
    employee_id: String,
}

async fn leave_shared_employee_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<EmployeeLeaveRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let employee_id = match uuid::Uuid::parse_str(req.employee_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad employee_id").into_response(),
    };
    let payload = employee_id.to_string();
    match SharedContractRepo::new(state.db.pool())
        .leave_shared_employee(tenant_id, &request_id, &payload, employee_id)
        .await
    {
        Ok(Ok(effect)) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct LaborRateRequest {
    request_id: String,
    employee_id: String,
    order_no: String,
    rate_cent: i64,
    frozen: bool,
}

#[derive(Debug, Deserialize)]
struct LaborRateRead {
    employee_id: String,
    order_no: String,
}

async fn read_frozen_labor_rate_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<LaborRateRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let employee_id = match uuid::Uuid::parse_str(query.employee_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad employee_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_frozen_labor_rate(tenant_id, employee_id, query.order_no.trim())
        .await
    {
        Ok((frozen_rate, next_rate)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "employee_id": employee_id.to_string(),
                "order_no": query.order_no.trim(),
                "frozen_rate_cent": frozen_rate,
                "next_rate_cent": next_rate,
                "frozen_rewritten": false,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("frozen labor rate not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn save_labor_rate_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LaborRateRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let employee_id = match uuid::Uuid::parse_str(req.employee_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad employee_id").into_response(),
    };
    let payload = format!(
        "{}:{}:{}:{}",
        employee_id,
        req.order_no.trim(),
        req.rate_cent,
        req.frozen
    );
    match SharedContractRepo::new(state.db.pool())
        .save_labor_rate(
            tenant_id,
            &request_id,
            &payload,
            employee_id,
            req.order_no.trim(),
            req.rate_cent,
            req.frozen,
        )
        .await
    {
        Ok(Ok(effect)) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct LaborReportRequest {
    request_id: String,
    employee_id: String,
    reporter_user_id: String,
    certifier_user_id: String,
    minutes: i64,
}

async fn save_labor_report_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LaborReportRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let employee_id = match uuid::Uuid::parse_str(req.employee_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad employee_id").into_response(),
    };
    let payload = format!(
        "{}:{}:{}:{}",
        employee_id,
        req.reporter_user_id.trim(),
        req.certifier_user_id.trim(),
        req.minutes
    );
    match SharedContractRepo::new(state.db.pool())
        .save_labor_report(
            tenant_id,
            &request_id,
            &payload,
            employee_id,
            req.reporter_user_id.trim(),
            req.certifier_user_id.trim(),
            req.minutes,
        )
        .await
    {
        Ok(Ok(effect)) => write_result(&request_id, Ok(Ok(effect))).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct ImportPrecheckRow {
    line_no: i32,
    item_code: String,
    name: String,
    unit_code: String,
}

#[derive(Debug, Deserialize)]
struct ImportPrecheckRequest {
    request_id: String,
    file_digest: String,
    column_map: String,
    rows: Vec<ImportPrecheckRow>,
}

#[derive(Debug, Deserialize)]
struct ImportErrorLineRead {
    job_id: String,
    line_no: i32,
}

async fn read_import_error_line_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<ImportErrorLineRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let job_id = match uuid::Uuid::parse_str(query.job_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad job_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_import_error_line(tenant_id, job_id, query.line_no)
        .await
    {
        Ok((status, error_text, items)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "job_id": job_id.to_string(),
                "line_no": query.line_no,
                "status": status,
                "error_text": error_text,
                "items_written": items,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("import error line not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn precheck_shared_import_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ImportPrecheckRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let owned: Vec<(i32, String, String, String)> = req
        .rows
        .iter()
        .map(|row| {
            (
                row.line_no,
                row.item_code.trim().to_string(),
                row.name.trim().to_string(),
                row.unit_code.trim().to_string(),
            )
        })
        .collect();
    let borrowed: Vec<(i32, &str, &str, &str)> = owned
        .iter()
        .map(|row| (row.0, row.1.as_str(), row.2.as_str(), row.3.as_str()))
        .collect();
    let payload = format!(
        "{}:{}:{}",
        req.file_digest.trim(),
        req.column_map.trim(),
        borrowed.len()
    );
    match SharedContractRepo::new(state.db.pool())
        .precheck_shared_import(
            tenant_id,
            &request_id,
            &payload,
            req.file_digest.trim(),
            req.column_map.trim(),
            &borrowed,
        )
        .await
    {
        Ok(Ok((effect, _job_id, _errors))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct ImportConfirmRequest {
    request_id: String,
    job_id: String,
    file_digest: String,
    duplicate_policy: String,
}

async fn confirm_shared_import_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ImportConfirmRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let job_id = match uuid::Uuid::parse_str(req.job_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad job_id").into_response(),
    };
    let payload = format!(
        "{}:{}:{}",
        job_id,
        req.file_digest.trim(),
        req.duplicate_policy.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .confirm_shared_import(
            tenant_id,
            &request_id,
            &payload,
            job_id,
            req.file_digest.trim(),
            req.duplicate_policy.trim(),
        )
        .await
    {
        Ok(Ok((effect, _imported, _errors, _skipped))) => {
            receipt_json(&request_id, &effect).into_response()
        }
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct ImportDownloadRequest {
    request_id: String,
    job_id: String,
}

async fn download_import_result_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ImportDownloadRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let job_id = match uuid::Uuid::parse_str(req.job_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad job_id").into_response(),
    };
    let payload = format!("{job_id}");
    match SharedContractRepo::new(state.db.pool())
        .download_import_result(tenant_id, &request_id, &payload, job_id)
        .await
    {
        Ok(Ok((effect, _imported, _errors, _skipped))) => {
            receipt_json(&request_id, &effect).into_response()
        }
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct OpeningGapRead {
    cutoff_date: String,
}

#[derive(Debug, Deserialize)]
struct OpeningGapRequest {
    request_id: String,
    cutoff_date: String,
    qty_gap: i64,
    cost_gap: i64,
    explanation: String,
}

async fn read_opening_gap_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<OpeningGapRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_opening_gap(tenant_id, query.cutoff_date.trim())
        .await
    {
        Ok((qty_gap, cost_gap, explanation, held)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "cutoff_date": query.cutoff_date.trim(),
                "qty_gap": qty_gap,
                "cost_gap": cost_gap,
                "explanation": explanation,
                "enabled": !held,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("opening gap not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn save_opening_gap_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OpeningGapRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}",
        req.cutoff_date.trim(),
        req.qty_gap,
        req.cost_gap,
        req.explanation.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .save_opening_gap(
            tenant_id,
            &request_id,
            &payload,
            req.cutoff_date.trim(),
            req.qty_gap,
            req.cost_gap,
            req.explanation.trim(),
        )
        .await
    {
        Ok(Ok(effect)) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct OpeningConfirmRequest {
    request_id: String,
    cutoff_date: String,
    qty_micros: i64,
    cost_cent: i64,
    partner_cent: i64,
}

async fn confirm_opening_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OpeningConfirmRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.qty_micros <= 0 || req.cost_cent < 0 || req.partner_cent == 0 {
        return bad_request("opening amounts are invalid").into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}",
        req.cutoff_date.trim(),
        req.qty_micros,
        req.cost_cent,
        req.partner_cent
    );
    match SharedContractRepo::new(state.db.pool())
        .confirm_opening(
            tenant_id,
            &request_id,
            &payload,
            req.cutoff_date.trim(),
            req.qty_micros,
            req.cost_cent,
            req.partner_cent,
        )
        .await
    {
        Ok(Ok((effect, _opening_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct OpeningResumeRequest {
    request_id: String,
    opening_id: String,
}

async fn resume_opening_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OpeningResumeRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let opening_id = match uuid::Uuid::parse_str(req.opening_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad opening_id").into_response(),
    };
    let payload = format!("{opening_id}");
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .resume_opening(tenant_id, &request_id, &payload, opening_id)
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct OpeningRebuildRequest {
    request_id: String,
    opening_id: String,
}

async fn rebuild_opening_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OpeningRebuildRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let opening_id = match uuid::Uuid::parse_str(req.opening_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad opening_id").into_response(),
    };
    let payload = format!("{opening_id}");
    match SharedContractRepo::new(state.db.pool())
        .rebuild_opening(tenant_id, &request_id, &payload, opening_id)
        .await
    {
        Ok(Ok((effect, _qty, _cost, _partner))) => {
            receipt_json(&request_id, &effect).into_response()
        }
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct OpeningCorrectionRequest {
    request_id: String,
    opening_id: String,
    qty_delta: i64,
    cost_delta: i64,
    partner_delta: i64,
}

async fn correct_opening_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OpeningCorrectionRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let opening_id = match uuid::Uuid::parse_str(req.opening_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad opening_id").into_response(),
    };
    let payload = format!(
        "{}:{}:{}:{}",
        opening_id, req.qty_delta, req.cost_delta, req.partner_delta
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .correct_opening(
                tenant_id,
                &request_id,
                &payload,
                opening_id,
                req.qty_delta,
                req.cost_delta,
                req.partner_delta,
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct SharedAttachmentRequest {
    request_id: String,
    object_id: String,
    file_name: String,
    digest: String,
    byte_count: i64,
    actor_user_id: String,
}

#[derive(Debug, Deserialize)]
struct AttachmentDownloadRead {
    attachment_id: String,
    actor_user_id: String,
    now_unix: i64,
}

#[derive(Debug, Deserialize)]
struct AttachmentDownloadRequest {
    request_id: String,
    attachment_id: String,
    actor_user_id: String,
    expires_at: i64,
    now_unix: i64,
}

async fn read_attachment_download_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<AttachmentDownloadRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let attachment_id = match uuid::Uuid::parse_str(query.attachment_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad attachment_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_attachment_download(
            tenant_id,
            attachment_id,
            query.actor_user_id.trim(),
            query.now_unix,
        )
        .await
    {
        Ok((storage_key, expires_at, revoked)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "attachment_id": attachment_id.to_string(),
                "actor_user_id": query.actor_user_id.trim(),
                "storage_key": storage_key,
                "expires_at": expires_at,
                "revoked": revoked,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("attachment download not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn save_attachment_download_link_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AttachmentDownloadRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let attachment_id = match uuid::Uuid::parse_str(req.attachment_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad attachment_id").into_response(),
    };
    let payload = format!(
        "{}:{}:{}:{}",
        attachment_id,
        req.actor_user_id.trim(),
        req.expires_at,
        req.now_unix
    );
    match SharedContractRepo::new(state.db.pool())
        .save_attachment_download_link(
            tenant_id,
            &request_id,
            &payload,
            attachment_id,
            req.actor_user_id.trim(),
            req.expires_at,
            req.now_unix,
        )
        .await
    {
        Ok(Ok(effect)) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct AttachmentRevokeRequest {
    request_id: String,
    attachment_id: String,
    actor_user_id: String,
}

async fn revoke_attachment_grant_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AttachmentRevokeRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let attachment_id = match uuid::Uuid::parse_str(req.attachment_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad attachment_id").into_response(),
    };
    let payload = format!("{}:{}", attachment_id, req.actor_user_id.trim());
    match SharedContractRepo::new(state.db.pool())
        .revoke_attachment_grant(
            tenant_id,
            &request_id,
            &payload,
            attachment_id,
            req.actor_user_id.trim(),
        )
        .await
    {
        Ok(Ok(effect)) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn save_shared_attachment_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SharedAttachmentRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.byte_count <= 0 {
        return bad_request("attachment size is invalid").into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}:{}",
        req.object_id.trim(),
        req.file_name.trim(),
        req.digest.trim(),
        req.byte_count,
        req.actor_user_id.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .save_shared_attachment(
            tenant_id,
            &request_id,
            &payload,
            req.object_id.trim(),
            req.file_name.trim(),
            req.digest.trim(),
            req.byte_count,
            req.actor_user_id.trim(),
        )
        .await
    {
        Ok(Ok((effect, _attachment_id, _version_no))) => {
            receipt_json(&request_id, &effect).into_response()
        }
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct AttachmentCiteRequest {
    request_id: String,
    object_id: String,
    attachment_id: String,
}

async fn cite_shared_attachment_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AttachmentCiteRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let attachment_id = match uuid::Uuid::parse_str(req.attachment_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad attachment_id").into_response(),
    };
    let payload = format!("{}:{attachment_id}", req.object_id.trim());
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .cite_shared_attachment(
                tenant_id,
                &request_id,
                &payload,
                req.object_id.trim(),
                attachment_id,
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct AttachmentQuotaRequest {
    request_id: String,
    quota_bytes: i64,
}

async fn save_attachment_quota_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AttachmentQuotaRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.quota_bytes <= 0 {
        return bad_request("attachment quota must be positive").into_response();
    }
    let payload = req.quota_bytes.to_string();
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .save_attachment_quota(tenant_id, &request_id, &payload, req.quota_bytes)
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct CustomFieldRequest {
    request_id: String,
    object_id: String,
    field_name: String,
    field_type: String,
    field_value: String,
}

async fn save_custom_field_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CustomFieldRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!(
        "{}:{}:{}:{}",
        req.object_id.trim(),
        req.field_name.trim(),
        req.field_type.trim(),
        req.field_value.trim()
    );
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .save_custom_field(
                tenant_id,
                &request_id,
                &payload,
                req.object_id.trim(),
                req.field_name.trim(),
                req.field_type.trim(),
                req.field_value.trim(),
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct StockLedgerRequest {
    request_id: String,
    warehouse_id: String,
    location_id: String,
    sku_id: String,
    batch_code: String,
    quality_status: String,
    bucket: String,
    delta_micros: i64,
}

#[derive(Debug, Deserialize)]
struct IssueRemainderRead {
    warehouse_id: String,
    sku_id: String,
    batch_code: String,
}

async fn read_tenth_remainder_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<IssueRemainderRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let warehouse_id = match uuid::Uuid::parse_str(query.warehouse_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad warehouse_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(query.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_tenth_remainder(tenant_id, warehouse_id, sku_id, query.batch_code.trim())
        .await
    {
        Ok((received, first, second, left)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "warehouse_id": warehouse_id.to_string(),
                "sku_id": sku_id.to_string(),
                "batch_code": query.batch_code.trim(),
                "received": received,
                "first": first,
                "second": second,
                "left": left,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            not_found("tenth remainder not found").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn read_issue_remainder_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<IssueRemainderRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let warehouse_id = match uuid::Uuid::parse_str(query.warehouse_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad warehouse_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(query.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_issue_remainder(tenant_id, warehouse_id, sku_id, query.batch_code.trim())
        .await
    {
        Ok((on_hand, issued, sellable)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "warehouse_id": warehouse_id.to_string(),
                "sku_id": sku_id.to_string(),
                "batch_code": query.batch_code.trim(),
                "on_hand": on_hand,
                "issued": issued,
                "sellable": sellable,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("wrong sellable") => {
            (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn append_stock_ledger_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<StockLedgerRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let warehouse_id = match uuid::Uuid::parse_str(req.warehouse_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad warehouse_id").into_response(),
    };
    let location_id = match uuid::Uuid::parse_str(req.location_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad location_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if req.delta_micros == 0 {
        return bad_request("stock ledger needs a delta").into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}:{}:{}:{}",
        warehouse_id,
        location_id,
        sku_id,
        req.batch_code.trim(),
        req.quality_status.trim(),
        req.bucket.trim(),
        req.delta_micros
    );
    match SharedContractRepo::new(state.db.pool())
        .append_stock_ledger(
            tenant_id,
            &request_id,
            &payload,
            warehouse_id,
            location_id,
            sku_id,
            req.batch_code.trim(),
            req.quality_status.trim(),
            req.bucket.trim(),
            req.delta_micros,
        )
        .await
    {
        Ok(Ok((effect, _ledger_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct QualityRebuildRequest {
    request_id: String,
    warehouse_id: String,
    location_id: String,
    sku_id: String,
    batch_code: String,
    quality_status: String,
}

#[derive(Debug, Deserialize)]
struct BalanceCheckRead {
    warehouse_id: String,
    sku_id: String,
    batch_code: String,
}

async fn read_balance_check_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<BalanceCheckRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let warehouse_id = match uuid::Uuid::parse_str(query.warehouse_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad warehouse_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(query.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_balance_check(tenant_id, warehouse_id, sku_id, query.batch_code.trim())
        .await
    {
        Ok((opening, ledger, stored, status)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "warehouse_id": warehouse_id,
                "sku_id": sku_id,
                "batch_code": query.batch_code.trim(),
                "opening": opening,
                "ledger": ledger,
                "stored": stored,
                "status": status,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("was not found") => {
            (StatusCode::NOT_FOUND, format!("{error}")).into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn rebuild_quality_balance_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<QualityRebuildRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let warehouse_id = match uuid::Uuid::parse_str(req.warehouse_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad warehouse_id").into_response(),
    };
    let location_id = match uuid::Uuid::parse_str(req.location_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad location_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    let payload = format!(
        "{}:{}:{}:{}:{}",
        warehouse_id,
        location_id,
        sku_id,
        req.batch_code.trim(),
        req.quality_status.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .rebuild_quality_balance(
            tenant_id,
            &request_id,
            &payload,
            warehouse_id,
            location_id,
            sku_id,
            req.batch_code.trim(),
            req.quality_status.trim(),
        )
        .await
    {
        Ok(Ok((effect, _available))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("balance does not equal opening plus valid ledger").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct StockReservationRequest {
    request_id: String,
    warehouse_id: String,
    location_id: String,
    sku_id: String,
    batch_code: String,
    quality_status: String,
    source_line: String,
    qty_micros: i64,
}

async fn reserve_stock_line_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<StockReservationRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let warehouse_id = match uuid::Uuid::parse_str(req.warehouse_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad warehouse_id").into_response(),
    };
    let location_id = match uuid::Uuid::parse_str(req.location_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad location_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if req.qty_micros <= 0 || req.source_line.trim().is_empty() {
        return bad_request("reservation needs a source line and a positive quantity")
            .into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}:{}:{}:{}",
        warehouse_id,
        location_id,
        sku_id,
        req.batch_code.trim(),
        req.quality_status.trim(),
        req.source_line.trim(),
        req.qty_micros
    );
    match SharedContractRepo::new(state.db.pool())
        .reserve_stock_line(
            tenant_id,
            &request_id,
            &payload,
            warehouse_id,
            location_id,
            sku_id,
            req.batch_code.trim(),
            req.quality_status.trim(),
            req.source_line.trim(),
            req.qty_micros,
        )
        .await
    {
        Ok(Ok((effect, _ledger_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct StockReleaseRequest {
    request_id: String,
    warehouse_id: String,
    location_id: String,
    sku_id: String,
    batch_code: String,
    quality_status: String,
    source_line: String,
    qty_micros: i64,
}

#[derive(Debug, Deserialize)]
struct OtherSourceRead {
    source_x: String,
    source_y: String,
}

async fn read_other_source_reservation_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<OtherSourceRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_other_source_reservation(tenant_id, query.source_x.trim(), query.source_y.trim())
        .await
    {
        Ok((released_x, open_y, on_hand)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "source_x": query.source_x.trim(),
                "source_y": query.source_y.trim(),
                "released_x": released_x,
                "open_y": open_y,
                "on_hand": on_hand,
            })),
        )
            .into_response(),
        Err(error) if error.to_string().contains("reduced the other") => {
            (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn release_stock_line_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<StockReleaseRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let warehouse_id = match uuid::Uuid::parse_str(req.warehouse_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad warehouse_id").into_response(),
    };
    let location_id = match uuid::Uuid::parse_str(req.location_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad location_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if req.qty_micros <= 0 || req.source_line.trim().is_empty() {
        return bad_request("release needs a source line and a positive quantity").into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}:{}:{}:{}",
        warehouse_id,
        location_id,
        sku_id,
        req.batch_code.trim(),
        req.quality_status.trim(),
        req.source_line.trim(),
        req.qty_micros
    );
    match SharedContractRepo::new(state.db.pool())
        .release_stock_line(
            tenant_id,
            &request_id,
            &payload,
            warehouse_id,
            location_id,
            sku_id,
            req.batch_code.trim(),
            req.quality_status.trim(),
            req.source_line.trim(),
            req.qty_micros,
        )
        .await
    {
        Ok(Ok((effect, _ledger_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct StockCancelRequest {
    request_id: String,
    warehouse_id: String,
    location_id: String,
    sku_id: String,
    batch_code: String,
    quality_status: String,
    source_line: String,
}

async fn cancel_stock_reservation_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<StockCancelRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let warehouse_id = match uuid::Uuid::parse_str(req.warehouse_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad warehouse_id").into_response(),
    };
    let location_id = match uuid::Uuid::parse_str(req.location_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad location_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if req.source_line.trim().is_empty() {
        return bad_request("cancel needs a source line").into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}:{}:{}",
        warehouse_id,
        location_id,
        sku_id,
        req.batch_code.trim(),
        req.quality_status.trim(),
        req.source_line.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .cancel_stock_reservation(
            tenant_id,
            &request_id,
            &payload,
            warehouse_id,
            location_id,
            sku_id,
            req.batch_code.trim(),
            req.quality_status.trim(),
            req.source_line.trim(),
        )
        .await
    {
        Ok(Ok((effect, _released))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct StockIssueRequest {
    request_id: String,
    warehouse_id: String,
    location_id: String,
    sku_id: String,
    batch_code: String,
    quality_status: String,
    source_line: String,
    qty_micros: i64,
}

async fn issue_stock_line_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<StockIssueRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let warehouse_id = match uuid::Uuid::parse_str(req.warehouse_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad warehouse_id").into_response(),
    };
    let location_id = match uuid::Uuid::parse_str(req.location_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad location_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if req.qty_micros <= 0 || req.source_line.trim().is_empty() {
        return bad_request("issue needs a source line and a positive quantity").into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}:{}:{}:{}",
        warehouse_id,
        location_id,
        sku_id,
        req.batch_code.trim(),
        req.quality_status.trim(),
        req.source_line.trim(),
        req.qty_micros
    );
    match SharedContractRepo::new(state.db.pool())
        .issue_stock_line(
            tenant_id,
            &request_id,
            &payload,
            warehouse_id,
            location_id,
            sku_id,
            req.batch_code.trim(),
            req.quality_status.trim(),
            req.source_line.trim(),
            req.qty_micros,
        )
        .await
    {
        Ok(Ok((effect, _ledger_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("issue exceeds available quantity").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct ShortIssueRead {
    source_line: String,
}

async fn read_short_issue_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<ShortIssueRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_short_issue(tenant_id, query.source_line.trim())
        .await
    {
        Ok((source_status, stock_rows, receipts, on_hand, requested, available)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "source_line": query.source_line.trim(),
                "source_status": source_status,
                "stock_rows": stock_rows,
                "receipts": receipts,
                "on_hand": on_hand,
                "requested": requested,
                "available": available,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct IssueRetryRead {
    request_id: String,
    source_line: String,
    first_ledger: String,
}

async fn read_issue_retry_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<IssueRetryRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    if query.request_id.trim().is_empty()
        || query.source_line.trim().is_empty()
        || query.first_ledger.trim().is_empty()
    {
        return bad_request("issue retry needs a request, a source line, and the first ledger")
            .into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_issue_retry(
            tenant_id,
            query.request_id.trim(),
            query.source_line.trim(),
            query.first_ledger.trim(),
        )
        .await
    {
        Ok((ledger_id, ledger_rows, receipt_rows, issued_micros)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": query.request_id.trim(),
                "source_line": query.source_line.trim(),
                "first_ledger": query.first_ledger.trim(),
                "ledger_id": ledger_id,
                "ledger_rows": ledger_rows,
                "receipt_rows": receipt_rows,
                "issued_micros": issued_micros,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct StockConsumeRequest {
    request_id: String,
    warehouse_id: String,
    location_id: String,
    sku_id: String,
    batch_code: String,
    quality_status: String,
    source_line: String,
    qty_micros: i64,
}

async fn consume_stock_line_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<StockConsumeRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let warehouse_id = match uuid::Uuid::parse_str(req.warehouse_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad warehouse_id").into_response(),
    };
    let location_id = match uuid::Uuid::parse_str(req.location_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad location_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if req.qty_micros <= 0 || req.source_line.trim().is_empty() {
        return bad_request("consume needs a source line and a positive quantity").into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}:{}:{}:{}",
        warehouse_id,
        location_id,
        sku_id,
        req.batch_code.trim(),
        req.quality_status.trim(),
        req.source_line.trim(),
        req.qty_micros
    );
    match SharedContractRepo::new(state.db.pool())
        .consume_stock_line(
            tenant_id,
            &request_id,
            &payload,
            warehouse_id,
            location_id,
            sku_id,
            req.batch_code.trim(),
            req.quality_status.trim(),
            req.source_line.trim(),
            req.qty_micros,
        )
        .await
    {
        Ok(Ok((effect, _ledger_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct StockReceiptRequest {
    request_id: String,
    warehouse_id: String,
    location_id: String,
    sku_id: String,
    batch_code: String,
    quality_status: String,
    source_line: String,
    qty_micros: i64,
}

async fn receive_shared_stock_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<StockReceiptRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let warehouse_id = match uuid::Uuid::parse_str(req.warehouse_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad warehouse_id").into_response(),
    };
    let location_id = match uuid::Uuid::parse_str(req.location_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad location_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if req.qty_micros <= 0 || req.source_line.trim().is_empty() {
        return bad_request("receipt needs a source line and a positive quantity").into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}:{}:{}:{}",
        warehouse_id,
        location_id,
        sku_id,
        req.batch_code.trim(),
        req.quality_status.trim(),
        req.source_line.trim(),
        req.qty_micros
    );
    match SharedContractRepo::new(state.db.pool())
        .receive_shared_stock(
            tenant_id,
            &request_id,
            &payload,
            warehouse_id,
            location_id,
            sku_id,
            req.batch_code.trim(),
            req.quality_status.trim(),
            req.source_line.trim(),
            req.qty_micros,
        )
        .await
    {
        Ok(Ok((effect, _ledger_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct PartialTransferRead {
    transfer_id: String,
}

async fn read_partial_transfer_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<PartialTransferRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let transfer_id = match uuid::Uuid::parse_str(query.transfer_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad transfer_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_partial_transfer(tenant_id, transfer_id)
        .await
    {
        Ok((issued, received, in_transit)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "transfer_id": transfer_id,
                "issued": issued,
                "received": received,
                "in_transit": in_transit,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct TransferIssueRequest {
    request_id: String,
    from_warehouse_id: String,
    to_warehouse_id: String,
    location_id: String,
    sku_id: String,
    batch_code: String,
    qty_micros: i64,
}

async fn issue_transfer_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<TransferIssueRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let from_warehouse = match uuid::Uuid::parse_str(req.from_warehouse_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad from_warehouse_id").into_response(),
    };
    let to_warehouse = match uuid::Uuid::parse_str(req.to_warehouse_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad to_warehouse_id").into_response(),
    };
    let location_id = match uuid::Uuid::parse_str(req.location_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad location_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if req.qty_micros <= 0 || from_warehouse == to_warehouse || req.batch_code.trim().is_empty() {
        return bad_request(
            "transfer issue needs two warehouses, a batch, and a positive quantity",
        )
        .into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}:{}:{}",
        from_warehouse,
        to_warehouse,
        location_id,
        sku_id,
        req.batch_code.trim(),
        req.qty_micros
    );
    match SharedContractRepo::new(state.db.pool())
        .issue_transfer(
            tenant_id,
            &request_id,
            &payload,
            from_warehouse,
            to_warehouse,
            location_id,
            sku_id,
            req.batch_code.trim(),
            req.qty_micros,
        )
        .await
    {
        Ok(Ok((effect, _transfer_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct TransferReceiveRequest {
    request_id: String,
    transfer_id: String,
    to_warehouse_id: String,
    location_id: String,
    sku_id: String,
    batch_code: String,
    qty_micros: i64,
}

async fn receive_transfer_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<TransferReceiveRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let transfer_id = match uuid::Uuid::parse_str(req.transfer_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad transfer_id").into_response(),
    };
    let to_warehouse = match uuid::Uuid::parse_str(req.to_warehouse_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad to_warehouse_id").into_response(),
    };
    let location_id = match uuid::Uuid::parse_str(req.location_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad location_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if req.qty_micros <= 0 || req.batch_code.trim().is_empty() {
        return bad_request("transfer receive needs a batch and a positive quantity")
            .into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}:{}:{}",
        transfer_id,
        to_warehouse,
        location_id,
        sku_id,
        req.batch_code.trim(),
        req.qty_micros
    );
    match SharedContractRepo::new(state.db.pool())
        .receive_transfer(
            tenant_id,
            &request_id,
            &payload,
            transfer_id,
            to_warehouse,
            location_id,
            sku_id,
            req.batch_code.trim(),
            req.qty_micros,
        )
        .await
    {
        Ok(Ok(effect)) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("transfer receive exceeds open in-transit quantity").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn read_repeat_receive_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<PartialTransferRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let transfer_id = match uuid::Uuid::parse_str(query.transfer_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad transfer_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_repeat_receive(tenant_id, transfer_id)
        .await
    {
        Ok((received, refused, in_transit, issued)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "transfer_id": transfer_id,
                "issued": issued,
                "received": received,
                "refused": refused,
                "in_transit": in_transit,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct TransferLossRequest {
    request_id: String,
    transfer_id: String,
    to_warehouse_id: String,
    location_id: String,
    sku_id: String,
    batch_code: String,
    qty_micros: i64,
}

async fn read_transfer_loss_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<PartialTransferRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let transfer_id = match uuid::Uuid::parse_str(query.transfer_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad transfer_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_transfer_loss(tenant_id, transfer_id)
        .await
    {
        Ok((issued, received, loss, in_transit)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "transfer_id": transfer_id,
                "issued": issued,
                "received": received,
                "loss": loss,
                "in_transit": in_transit,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn lose_transfer_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<TransferLossRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let transfer_id = match uuid::Uuid::parse_str(req.transfer_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad transfer_id").into_response(),
    };
    let to_warehouse = match uuid::Uuid::parse_str(req.to_warehouse_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad to_warehouse_id").into_response(),
    };
    let location_id = match uuid::Uuid::parse_str(req.location_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad location_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if req.qty_micros <= 0 || req.batch_code.trim().is_empty() {
        return bad_request("transfer loss needs a batch and a positive quantity").into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}:{}:{}",
        transfer_id,
        to_warehouse,
        location_id,
        sku_id,
        req.batch_code.trim(),
        req.qty_micros
    );
    match SharedContractRepo::new(state.db.pool())
        .lose_transfer(
            tenant_id,
            &request_id,
            &payload,
            transfer_id,
            to_warehouse,
            location_id,
            sku_id,
            req.batch_code.trim(),
            req.qty_micros,
        )
        .await
    {
        Ok(Ok(effect)) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct CountConfirmRequest {
    request_id: String,
    warehouse_id: String,
    location_id: String,
    sku_id: String,
    batch_code: String,
    cutoff_at: String,
    counted_qty: i64,
}

async fn confirm_count_adjustment_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CountConfirmRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let warehouse_id = match uuid::Uuid::parse_str(req.warehouse_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad warehouse_id").into_response(),
    };
    let location_id = match uuid::Uuid::parse_str(req.location_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad location_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if req.cutoff_at.trim().is_empty() || req.batch_code.trim().is_empty() || req.counted_qty < 0 {
        return bad_request("count confirmation needs a cutoff, a batch, and a counted quantity")
            .into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}:{}:{}",
        warehouse_id,
        location_id,
        sku_id,
        req.batch_code.trim(),
        req.cutoff_at.trim(),
        req.counted_qty
    );
    match SharedContractRepo::new(state.db.pool())
        .confirm_count_adjustment(
            tenant_id,
            &request_id,
            &payload,
            warehouse_id,
            location_id,
            sku_id,
            req.batch_code.trim(),
            req.cutoff_at.trim(),
            req.counted_qty,
        )
        .await
    {
        Ok(Ok((effect, _delta))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct CountSheetRequest {
    request_id: String,
    warehouse_id: String,
    location_id: String,
    sku_id: String,
    batch_code: String,
    cutoff_at: String,
}

#[derive(Debug, Deserialize)]
struct CountSheetRead {
    count_id: String,
}

async fn read_sale_during_count_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<CountSheetRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let count_id = match uuid::Uuid::parse_str(query.count_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad count_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_sale_during_count(tenant_id, count_id)
        .await
    {
        Ok((counted, sale_rows, sold)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "count_id": count_id,
                "counted": counted,
                "sale_rows": sale_rows,
                "sold": sold,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn open_count_sheet_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CountSheetRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let warehouse_id = match uuid::Uuid::parse_str(req.warehouse_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad warehouse_id").into_response(),
    };
    let location_id = match uuid::Uuid::parse_str(req.location_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad location_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if req.cutoff_at.trim().is_empty() || req.batch_code.trim().is_empty() {
        return bad_request("count sheet needs a cutoff and a batch").into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}:{}",
        warehouse_id,
        location_id,
        sku_id,
        req.batch_code.trim(),
        req.cutoff_at.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .open_count_sheet(
            tenant_id,
            &request_id,
            &payload,
            warehouse_id,
            location_id,
            sku_id,
            req.batch_code.trim(),
            req.cutoff_at.trim(),
        )
        .await
    {
        Ok(Ok((effect, _count_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct CountSaleRequest {
    request_id: String,
    count_id: String,
    warehouse_id: String,
    location_id: String,
    sku_id: String,
    batch_code: String,
    qty_micros: i64,
}

async fn sell_during_count_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CountSaleRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let count_id = match uuid::Uuid::parse_str(req.count_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad count_id").into_response(),
    };
    let warehouse_id = match uuid::Uuid::parse_str(req.warehouse_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad warehouse_id").into_response(),
    };
    let location_id = match uuid::Uuid::parse_str(req.location_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad location_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if req.qty_micros <= 0 || req.batch_code.trim().is_empty() {
        return bad_request("sale during count needs a batch and a positive quantity")
            .into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}:{}:{}",
        count_id,
        warehouse_id,
        location_id,
        sku_id,
        req.batch_code.trim(),
        req.qty_micros
    );
    match SharedContractRepo::new(state.db.pool())
        .sell_during_count(
            tenant_id,
            &request_id,
            &payload,
            count_id,
            warehouse_id,
            location_id,
            sku_id,
            req.batch_code.trim(),
            req.qty_micros,
        )
        .await
    {
        Ok(Ok(effect)) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct SharedBatchRequest {
    request_id: String,
    item_code: String,
    batch_code: String,
    produced_on: String,
    expires_on: String,
    serialized: bool,
    #[serde(default)]
    quality_status: Option<String>,
}

async fn save_shared_batch_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SharedBatchRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.item_code.trim().is_empty() || req.batch_code.trim().is_empty() {
        return bad_request("batch needs an item and a batch code").into_response();
    }
    let quality = req.quality_status.as_deref().unwrap_or("GOOD").trim();
    let payload = format!(
        "{}:{}:{}:{}:{}:{}",
        req.item_code.trim(),
        req.batch_code.trim(),
        req.produced_on.trim(),
        req.expires_on.trim(),
        req.serialized,
        quality
    );
    match SharedContractRepo::new(state.db.pool())
        .save_shared_batch(
            tenant_id,
            &request_id,
            &payload,
            req.item_code.trim(),
            req.batch_code.trim(),
            req.produced_on.trim(),
            req.expires_on.trim(),
            req.serialized,
            req.quality_status.as_deref().unwrap_or("GOOD"),
        )
        .await
    {
        Ok(Ok((effect, _batch_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct SharedSerialRequest {
    request_id: String,
    item_code: String,
    batch_code: String,
    serial_no: String,
}

async fn assign_shared_serial_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SharedSerialRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.item_code.trim().is_empty()
        || req.batch_code.trim().is_empty()
        || req.serial_no.trim().is_empty()
    {
        return bad_request("serial needs an item, a batch, and a serial number").into_response();
    }
    let payload = format!(
        "{}:{}:{}",
        req.item_code.trim(),
        req.batch_code.trim(),
        req.serial_no.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .assign_shared_serial(
            tenant_id,
            &request_id,
            &payload,
            req.item_code.trim(),
            req.batch_code.trim(),
            req.serial_no.trim(),
        )
        .await
    {
        Ok(Ok((effect, _serial_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct BatchFreezeRequest {
    request_id: String,
    item_code: String,
    batch_code: String,
}

async fn freeze_shared_batch_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<BatchFreezeRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.item_code.trim().is_empty() || req.batch_code.trim().is_empty() {
        return bad_request("freeze needs an item and a batch").into_response();
    }
    let payload = format!("{}:{}", req.item_code.trim(), req.batch_code.trim());
    match SharedContractRepo::new(state.db.pool())
        .freeze_shared_batch(
            tenant_id,
            &request_id,
            &payload,
            req.item_code.trim(),
            req.batch_code.trim(),
        )
        .await
    {
        Ok(Ok(effect)) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct BatchIssueRequest {
    request_id: String,
    item_code: String,
    batch_code: String,
    as_of: String,
    qty_micros: i64,
}

async fn issue_shared_batch_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<BatchIssueRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.item_code.trim().is_empty() || req.batch_code.trim().is_empty() || req.qty_micros <= 0 {
        return bad_request("batch issue needs an item, a batch, and a positive quantity")
            .into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}",
        req.item_code.trim(),
        req.batch_code.trim(),
        req.as_of.trim(),
        req.qty_micros
    );
    match SharedContractRepo::new(state.db.pool())
        .issue_shared_batch(
            tenant_id,
            &request_id,
            &payload,
            req.item_code.trim(),
            req.batch_code.trim(),
            req.as_of.trim(),
            req.qty_micros,
        )
        .await
    {
        Ok(Ok(effect)) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("expired or held batch cannot be issued").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct BatchQuarterIssueRequest {
    request_id: String,
    warehouse_id: String,
    location_id: String,
    sku_id: String,
    item_code: String,
    batch_code: String,
    as_of: String,
}

async fn issue_batch_quarter_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<BatchQuarterIssueRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let warehouse_id = match uuid::Uuid::parse_str(req.warehouse_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad warehouse_id").into_response(),
    };
    let location_id = match uuid::Uuid::parse_str(req.location_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad location_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if req.item_code.trim().is_empty() || req.batch_code.trim().is_empty() {
        return bad_request("batch quarter issue needs an item and a batch").into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}:{}:{}:250000",
        warehouse_id,
        location_id,
        sku_id,
        req.item_code.trim(),
        req.batch_code.trim(),
        req.as_of.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .issue_batch_quarter(
            tenant_id,
            &request_id,
            &payload,
            warehouse_id,
            location_id,
            sku_id,
            req.item_code.trim(),
            req.batch_code.trim(),
            req.as_of.trim(),
        )
        .await
    {
        Ok(Ok((effect, _ledger_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("expired or held batch cannot be issued").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct BatchQuarterRead {
    request_id: String,
    batch_code: String,
}

async fn read_batch_quarter_issue_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<BatchQuarterRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_batch_quarter_issue(tenant_id, query.request_id.trim(), query.batch_code.trim())
        .await
    {
        Ok((ledger_id, issued_micros, rows)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": query.request_id.trim(),
                "batch_code": query.batch_code.trim(),
                "ledger_id": ledger_id,
                "issued_micros": issued_micros,
                "rows": rows,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct SerialIssueRequest {
    request_id: String,
    warehouse_id: String,
    location_id: String,
    sku_id: String,
    item_code: String,
    batch_code: String,
    serial_no: String,
    as_of: String,
}

async fn issue_shared_serial_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SerialIssueRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let warehouse_id = match uuid::Uuid::parse_str(req.warehouse_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad warehouse_id").into_response(),
    };
    let location_id = match uuid::Uuid::parse_str(req.location_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad location_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if req.item_code.trim().is_empty()
        || req.batch_code.trim().is_empty()
        || req.serial_no.trim().is_empty()
    {
        return bad_request("serial issue needs an item, a batch, and a serial").into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}:{}:{}:{}",
        warehouse_id,
        location_id,
        sku_id,
        req.item_code.trim(),
        req.batch_code.trim(),
        req.serial_no.trim(),
        req.as_of.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .issue_shared_serial(
            tenant_id,
            &request_id,
            &payload,
            warehouse_id,
            location_id,
            sku_id,
            req.item_code.trim(),
            req.batch_code.trim(),
            req.serial_no.trim(),
            req.as_of.trim(),
        )
        .await
    {
        Ok(Ok((effect, _ledger_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("serial is already issued").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct SerialIssueRead {
    item_code: String,
    serial_no: String,
}

async fn read_serial_issue_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<SerialIssueRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_serial_issue(tenant_id, query.item_code.trim(), query.serial_no.trim())
        .await
    {
        Ok((serial_id, successes, issue_rows, status)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "item_code": query.item_code.trim(),
                "serial_no": query.serial_no.trim(),
                "serial_id": serial_id,
                "successes": successes,
                "issue_rows": issue_rows,
                "status": status,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct BatchIssueRefusalRead {
    item_code: String,
    batch_code: String,
    as_of: String,
}

async fn read_batch_issue_refusal_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<BatchIssueRefusalRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    if query.as_of.trim().is_empty() {
        return bad_request("batch issue refusal needs the issue date").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_batch_issue_refusal(
            tenant_id,
            query.item_code.trim(),
            query.batch_code.trim(),
            query.as_of.trim(),
        )
        .await
    {
        Ok((reason, quality_status, frozen, expires_on, receipts, stock_rows)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "item_code": query.item_code.trim(),
                "batch_code": query.batch_code.trim(),
                "as_of": query.as_of.trim(),
                "expires_on": expires_on,
                "reason": reason,
                "quality_status": quality_status,
                "frozen": frozen,
                "receipts": receipts,
                "stock_rows": stock_rows,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct BatchUseRequest {
    request_id: String,
    input_batch: String,
    output_batch: String,
    qty_micros: i64,
    work_order: String,
}

async fn record_batch_use_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<BatchUseRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.input_batch.trim().is_empty()
        || req.output_batch.trim().is_empty()
        || req.work_order.trim().is_empty()
        || req.qty_micros <= 0
    {
        return bad_request("batch use needs actual input, output, work order, and quantity")
            .into_response();
    }
    if req.work_order.trim() == "PLANNED_BOM" {
        return bad_request("planned BOM is not actual consumption").into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}",
        req.input_batch.trim(),
        req.output_batch.trim(),
        req.qty_micros,
        req.work_order.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .record_batch_use(
            tenant_id,
            &request_id,
            &payload,
            req.input_batch.trim(),
            req.output_batch.trim(),
            req.qty_micros,
            req.work_order.trim(),
        )
        .await
    {
        Ok(Ok((effect, _use_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct BatchReceiptRequest {
    request_id: String,
    item_code: String,
    batch_code: String,
    as_of: String,
    qty_micros: i64,
}

async fn select_batch_receipt_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<BatchReceiptRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.item_code.trim().is_empty()
        || req.batch_code.trim().is_empty()
        || req.as_of.trim().is_empty()
        || req.qty_micros <= 0
    {
        return bad_request(
            "batch receipt needs an item, a batch, a date, and a positive quantity",
        )
        .into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}",
        req.item_code.trim(),
        req.batch_code.trim(),
        req.as_of.trim(),
        req.qty_micros
    );
    match SharedContractRepo::new(state.db.pool())
        .select_batch_receipt(
            tenant_id,
            &request_id,
            &payload,
            req.item_code.trim(),
            req.batch_code.trim(),
            req.as_of.trim(),
            req.qty_micros,
        )
        .await
    {
        Ok(Ok((effect, _selection_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct BatchTraceRequest {
    request_id: String,
    item_code: String,
    batch_code: String,
}

async fn trace_frozen_batch_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<BatchTraceRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.item_code.trim().is_empty() || req.batch_code.trim().is_empty() {
        return bad_request("frozen trace needs an item and a batch").into_response();
    }
    let payload = format!("{}:{}", req.item_code.trim(), req.batch_code.trim());
    match SharedContractRepo::new(state.db.pool())
        .trace_frozen_batch(
            tenant_id,
            &request_id,
            &payload,
            req.item_code.trim(),
            req.batch_code.trim(),
        )
        .await
    {
        Ok(Ok(effect)) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct CostReceiptRequest {
    request_id: String,
    item_code: String,
    qty_micros: i64,
    amount_cent: i64,
    period_code: String,
}

async fn post_cost_receipt_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CostReceiptRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.item_code.trim().is_empty() || req.period_code.trim().is_empty() || req.qty_micros <= 0 {
        return bad_request("cost receipt needs an item, a period, and a positive quantity")
            .into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}",
        req.item_code.trim(),
        req.qty_micros,
        req.amount_cent,
        req.period_code.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .post_cost_receipt(erp_repo::shared_contract::CostReceiptInput { tenant_id, request_id: &request_id, payload_hash: &payload, item_code: req.item_code.trim(), qty_micros: req.qty_micros, amount_cent: req.amount_cent, period_code: req.period_code.trim() })
        .await
    {
        Ok(Ok((effect, _amount, _estimated))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct ZeroUnitCostRead {
    item_code: String,
}

async fn read_zero_unit_cost_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<ZeroUnitCostRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_zero_unit_cost(tenant_id, query.item_code.trim())
        .await
    {
        Ok((qty_micros, value_cent, unit_cent, estimated)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "item_code": query.item_code.trim(),
                "qty_micros": qty_micros,
                "value_cent": value_cent,
                "unit_cent": unit_cent,
                "estimated": estimated,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct CostIssueRequest {
    request_id: String,
    item_code: String,
    qty_micros: i64,
    period_code: String,
}

async fn post_cost_issue_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CostIssueRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.item_code.trim().is_empty() || req.period_code.trim().is_empty() || req.qty_micros <= 0 {
        return bad_request("cost issue needs an item, a period, and a positive quantity")
            .into_response();
    }
    let payload = format!(
        "{}:{}:{}",
        req.item_code.trim(),
        req.qty_micros,
        req.period_code.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .post_cost_issue(erp_repo::shared_contract::CostIssueInput { tenant_id, request_id: &request_id, payload_hash: &payload, item_code: req.item_code.trim(), qty_micros: req.qty_micros, period_code: req.period_code.trim() })
        .await
    {
        Ok(Ok((effect, _amount, _estimated))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct CostReturnRequest {
    request_id: String,
    item_code: String,
    issue_request: String,
    qty_micros: i64,
    period_code: String,
}

async fn post_cost_return_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CostReturnRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.item_code.trim().is_empty()
        || req.issue_request.trim().is_empty()
        || req.period_code.trim().is_empty()
        || req.qty_micros <= 0
    {
        return bad_request(
            "cost return needs an item, the original issue, a period, and a positive quantity",
        )
        .into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}",
        req.item_code.trim(),
        req.issue_request.trim(),
        req.qty_micros,
        req.period_code.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .post_cost_return(erp_repo::shared_contract::CostReturnCommand { tenant_id, request_id: &request_id, payload_hash: &payload, item_code: req.item_code.trim(), issue_request: req.issue_request.trim(), return_qty: req.qty_micros, period_code: req.period_code.trim() })
        .await
    {
        Ok(Ok((effect, _amount, _estimated))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct ThreeMicroReturnRead {
    item_code: String,
    issue_request: String,
}

async fn read_three_micro_return_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<ThreeMicroReturnRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_three_micro_return(
            tenant_id,
            query.item_code.trim(),
            query.issue_request.trim(),
        )
        .await
    {
        Ok((splits, original_cent, returned_cent)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "item_code": query.item_code.trim(),
                "issue_request": query.issue_request.trim(),
                "first_cent": splits[0],
                "second_cent": splits[1],
                "third_cent": splits[2],
                "original_cent": original_cent,
                "returned_cent": returned_cent,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct CostPeriodCloseRequest {
    request_id: String,
    period_code: String,
}

async fn close_cost_period_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CostPeriodCloseRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.period_code.trim().is_empty() {
        return bad_request("cost period is required").into_response();
    }
    let payload = req.period_code.trim().to_string();
    match SharedContractRepo::new(state.db.pool())
        .close_item_cost_period(tenant_id, &request_id, &payload, req.period_code.trim())
        .await
    {
        Ok(Ok(effect)) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct ClosedPeriodAdjustmentRequest {
    request_id: String,
    item_code: String,
    amount_cent: i64,
    closed_period: String,
    next_period: String,
}

async fn post_closed_period_adjustment_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ClosedPeriodAdjustmentRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.item_code.trim().is_empty()
        || req.closed_period.trim().is_empty()
        || req.next_period.trim().is_empty()
        || req.amount_cent == 0
    {
        return bad_request(
            "closed period adjustment needs an item, two periods, and a nonzero amount",
        )
        .into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}",
        req.item_code.trim(),
        req.amount_cent,
        req.closed_period.trim(),
        req.next_period.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .post_closed_period_adjustment(erp_repo::shared_contract::ClosedCostAdjustmentInput { tenant_id, request_id: &request_id, payload_hash: &payload, item_code: req.item_code.trim(), amount_cent: req.amount_cent, closed_period: req.closed_period.trim(), next_period: req.next_period.trim() })
        .await
    {
        Ok(Ok((effect, _entry_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("closed period cannot be adjusted in place").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct ClosedPeriodAdjustmentRead {
    item_code: String,
    closed_period: String,
    next_period: String,
}

async fn read_closed_period_adjustment_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<ClosedPeriodAdjustmentRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_closed_period_adjustment(
            tenant_id,
            query.item_code.trim(),
            query.closed_period.trim(),
            query.next_period.trim(),
        )
        .await
    {
        Ok((original_cent, adjustment_cent, period_code)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "item_code": query.item_code.trim(),
                "closed_period": query.closed_period.trim(),
                "original_cent": original_cent,
                "adjustment_cent": adjustment_cent,
                "period_code": period_code,
                "original_rewritten": false,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct CostRebuildRequest {
    request_id: String,
    item_code: String,
}

async fn rebuild_item_cost_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CostRebuildRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.item_code.trim().is_empty() {
        return bad_request("cost rebuild needs an item").into_response();
    }
    let payload = req.item_code.trim().to_string();
    match SharedContractRepo::new(state.db.pool())
        .rebuild_item_cost(tenant_id, &request_id, &payload, req.item_code.trim())
        .await
    {
        Ok(Ok(effect)) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct WrongBatchRequest {
    request_id: String,
    scanned_batch: String,
    expected_batch: String,
}

async fn locate_wrong_batch_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WrongBatchRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.scanned_batch.trim().is_empty() || req.expected_batch.trim().is_empty() {
        return bad_request("wrong batch needs a scanned batch and an expected batch")
            .into_response();
    }
    let payload = format!("{}:{}", req.scanned_batch.trim(), req.expected_batch.trim());
    match SharedContractRepo::new(state.db.pool())
        .locate_wrong_batch(
            tenant_id,
            &request_id,
            &payload,
            req.scanned_batch.trim(),
            req.expected_batch.trim(),
        )
        .await
    {
        Ok(Ok((effect, _scan_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("wrong batch was not located").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct WrongBatchRead {
    scanned_batch: String,
    expected_batch: String,
}

async fn read_wrong_batch_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<WrongBatchRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_wrong_batch(
            tenant_id,
            query.scanned_batch.trim(),
            query.expected_batch.trim(),
        )
        .await
    {
        Ok((scan_id, status, expected_batch)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "scan_id": scan_id,
                "scanned_batch": query.scanned_batch.trim(),
                "expected_batch": expected_batch,
                "status": status,
                "document_rows": 0,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct ScanQtyRead {
    code: String,
    kind: String,
}

async fn read_scan_qty_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<ScanQtyRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_scan_qty(tenant_id, query.code.trim(), query.kind.trim())
        .await
    {
        Ok((scan_id, qty_micros, verified_rows, document_rows)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "code": query.code.trim(),
                "kind": query.kind.trim(),
                "scan_id": scan_id,
                "qty_micros": qty_micros,
                "verified_rows": verified_rows,
                "document_rows": document_rows,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct ScanRequest {
    request_id: String,
    code: String,
    kind: String,
    actor_user_id: String,
}

async fn record_scan_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ScanRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.code.trim().is_empty()
        || req.kind.trim().is_empty()
        || req.actor_user_id.trim().is_empty()
    {
        return bad_request("scan needs a code, a kind, and a business user").into_response();
    }
    let payload = format!(
        "{}:{}:{}",
        req.code.trim(),
        req.kind.trim(),
        req.actor_user_id.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .record_scan(
            tenant_id,
            &request_id,
            &payload,
            req.code.trim(),
            req.kind.trim(),
            req.actor_user_id.trim(),
        )
        .await
    {
        Ok(Ok((effect, _scan_id, _status))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct PrintEventRequest {
    request_id: String,
    job_ref: String,
    state: String,
    copies: i64,
    reason: String,
    actor_kind: String,
    actor_user_id: String,
}

async fn record_print_event_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PrintEventRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.job_ref.trim().is_empty()
        || req.state.trim().is_empty()
        || req.actor_user_id.trim().is_empty()
    {
        return bad_request("print event needs a job, a state, and an actor").into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}:{}:{}",
        req.job_ref.trim(),
        req.state.trim(),
        req.copies,
        req.reason.trim(),
        req.actor_kind.trim(),
        req.actor_user_id.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .record_print_event(
            tenant_id,
            &request_id,
            &payload,
            req.job_ref.trim(),
            req.state.trim(),
            req.copies,
            req.reason.trim(),
            req.actor_kind.trim(),
            req.actor_user_id.trim(),
        )
        .await
    {
        Ok(Ok((effect, _event_id, _not_seat))) => {
            receipt_json(&request_id, &effect).into_response()
        }
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct OfflinePrintRequest {
    request_id: String,
    job_ref: String,
    actor_user_id: String,
    complete: bool,
}

#[derive(Debug, Deserialize)]
struct OfflinePrintRead {
    job_ref: String,
}

async fn read_offline_print_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<OfflinePrintRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_offline_print(tenant_id, query.job_ref.trim())
        .await
    {
        Ok((event_id, status, failed_rows, confirmed_rows)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "job_ref": query.job_ref.trim(),
                "event_id": event_id,
                "status": status,
                "failed_rows": failed_rows,
                "confirmed_rows": confirmed_rows,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn record_offline_print_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OfflinePrintRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.job_ref.trim().is_empty() || req.actor_user_id.trim().is_empty() {
        return bad_request("offline print needs a job and a business user").into_response();
    }
    if req.complete {
        return match SharedContractRepo::new(state.db.pool())
            .refuse_offline_completion(tenant_id, &request_id, req.job_ref.trim())
            .await
        {
            Ok(()) => conflict("offline device cannot complete paper").into_response(),
            Err(error) if error.to_string().contains("CONFLICT:") => {
                conflict("offline device cannot complete paper").into_response()
            }
            Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
        };
    }
    let payload = format!(
        "{}:OFFLINE:{}",
        req.job_ref.trim(),
        req.actor_user_id.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .record_offline_print(
            tenant_id,
            &request_id,
            &payload,
            req.job_ref.trim(),
            req.actor_user_id.trim(),
        )
        .await
    {
        Ok(Ok((effect, _event_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("offline print was already stored").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct PaperCodeRequest {
    request_id: String,
    paper_code: String,
    item_code: String,
    actor_user_id: String,
}

#[derive(Debug, Deserialize)]
struct PaperRescanRequest {
    request_id: String,
    paper_code: String,
    actor_user_id: String,
}

#[derive(Debug, Deserialize)]
struct PaperRescanRead {
    paper_code: String,
}

async fn issue_paper_code_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PaperCodeRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.paper_code.trim().is_empty()
        || req.item_code.trim().is_empty()
        || req.actor_user_id.trim().is_empty()
    {
        return bad_request("paper code needs a code, an item, and a business user")
            .into_response();
    }
    let payload = format!(
        "{}:{}:{}",
        req.paper_code.trim(),
        req.item_code.trim(),
        req.actor_user_id.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .issue_paper_code(
            tenant_id,
            &request_id,
            &payload,
            req.paper_code.trim(),
            req.item_code.trim(),
            req.actor_user_id.trim(),
        )
        .await
    {
        Ok(Ok((effect, _code_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("paper code was already stored").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn read_paper_rescan_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<PaperRescanRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_paper_rescan(tenant_id, query.paper_code.trim())
        .await
    {
        Ok((scan_id, item_code, status, document_rows)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "paper_code": query.paper_code.trim(),
                "scan_id": scan_id,
                "item_code": item_code,
                "status": status,
                "document_rows": document_rows,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn recognize_paper_code_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PaperRescanRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.paper_code.trim().is_empty() || req.actor_user_id.trim().is_empty() {
        return bad_request("paper rescan needs a code and a business user").into_response();
    }
    let payload = format!("{}:{}", req.paper_code.trim(), req.actor_user_id.trim());
    match SharedContractRepo::new(state.db.pool())
        .recognize_paper_code(
            tenant_id,
            &request_id,
            &payload,
            req.paper_code.trim(),
            req.actor_user_id.trim(),
        )
        .await
    {
        Ok(Ok((effect, _scan_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("paper code was not recognized").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct ScanHoldRequest {
    request_id: String,
    code: String,
    kind: String,
}

async fn hold_repeat_scan_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ScanHoldRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.code.trim().is_empty() || req.kind.trim().is_empty() {
        return bad_request("repeat scan needs a code and a kind").into_response();
    }
    let payload = format!("{}:{}", req.code.trim(), req.kind.trim());
    match SharedContractRepo::new(state.db.pool())
        .hold_repeat_scan(
            tenant_id,
            &request_id,
            &payload,
            req.code.trim(),
            req.kind.trim(),
        )
        .await
    {
        Ok(Ok(effect)) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct AvailableAlertRequest {
    request_id: String,
    sku_id: String,
    object_code: String,
    cutoff_at: String,
    threshold: i64,
}

#[derive(Debug, Deserialize)]
struct AvailableAlertRead {
    sku_id: String,
    object_code: String,
}

async fn read_available_alert_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<AvailableAlertRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let sku_id = match uuid::Uuid::parse_str(query.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_available_alert(tenant_id, sku_id, query.object_code.trim())
        .await
    {
        Ok((alert_id, observed, on_hand, unavailable, stock_rows)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "sku_id": sku_id.to_string(),
                "object_code": query.object_code.trim(),
                "alert_id": alert_id,
                "observed": observed,
                "on_hand": on_hand,
                "unavailable": unavailable,
                "stock_rows": stock_rows,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn raise_available_alert_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AvailableAlertRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if req.object_code.trim().is_empty() || req.cutoff_at.trim().is_empty() {
        return bad_request("available alert needs an object and a cutoff").into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}",
        sku_id,
        req.object_code.trim(),
        req.cutoff_at.trim(),
        req.threshold
    );
    match SharedContractRepo::new(state.db.pool())
        .raise_available_alert(
            tenant_id,
            &request_id,
            &payload,
            sku_id,
            req.object_code.trim(),
            req.cutoff_at.trim(),
            req.threshold,
        )
        .await
    {
        Ok(Ok((effect, _alert_id, _observed))) => {
            receipt_json(&request_id, &effect).into_response()
        }
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("available alert was already open").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct AlertClearRequest {
    request_id: String,
    alert_id: String,
    sku_id: String,
}

#[derive(Debug, Deserialize)]
struct AlertClearRead {
    alert_id: String,
    sku_id: String,
}

async fn read_cleared_alert_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<AlertClearRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let alert_id = match uuid::Uuid::parse_str(query.alert_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad alert_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(query.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_cleared_alert(tenant_id, alert_id, sku_id)
        .await
    {
        Ok((status, observed, on_hand, purchase_rows)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "alert_id": alert_id.to_string(),
                "sku_id": sku_id.to_string(),
                "status": status,
                "observed": observed,
                "on_hand": on_hand,
                "purchase_rows": purchase_rows,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn clear_alert_after_stock_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AlertClearRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let alert_id = match uuid::Uuid::parse_str(req.alert_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad alert_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    let payload = format!("{alert_id}:{sku_id}");
    match SharedContractRepo::new(state.db.pool())
        .clear_alert_after_stock(tenant_id, &request_id, &payload, alert_id, sku_id)
        .await
    {
        Ok(Ok((effect, _observed))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("stock did not clear the alert").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct ShortageSuggestionRequest {
    request_id: String,
    item_code: String,
    sku_id: String,
}

#[derive(Debug, Deserialize)]
struct ShortageSuggestionRead {
    item_code: String,
    sku_id: String,
}

async fn read_shortage_suggestion_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<ShortageSuggestionRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let sku_id = match uuid::Uuid::parse_str(query.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if query.item_code.trim().is_empty() {
        return bad_request("shortage suggestion needs an item").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_shortage_suggestion(tenant_id, query.item_code.trim(), sku_id)
        .await
    {
        Ok((status, demand, on_hand, shortage, paid, work_rows)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "item_code": query.item_code.trim(),
                "sku_id": sku_id.to_string(),
                "status": status,
                "demand": demand,
                "on_hand": on_hand,
                "shortage": shortage,
                "paid": paid,
                "work_rows": work_rows,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn raise_shortage_suggestion_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ShortageSuggestionRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if req.item_code.trim().is_empty() {
        return bad_request("shortage suggestion needs an item").into_response();
    }
    let payload = format!("{}:{}", req.item_code.trim(), sku_id);
    match SharedContractRepo::new(state.db.pool())
        .raise_shortage_suggestion(
            tenant_id,
            &request_id,
            &payload,
            req.item_code.trim(),
            sku_id,
        )
        .await
    {
        Ok(Ok((effect, _suggestion_id, _shortage))) => {
            receipt_json(&request_id, &effect).into_response()
        }
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("stored demand did not open a suggestion").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct AgingAlertRequest {
    request_id: String,
    sku_id: String,
    cutoff: String,
}

#[derive(Debug, Deserialize)]
struct AgingAlertRead {
    alert_id: String,
    sku_id: String,
}

async fn read_aging_alert_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<AgingAlertRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let alert_id = match uuid::Uuid::parse_str(query.alert_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad alert_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(query.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_aging_alert(tenant_id, alert_id, sku_id)
        .await
    {
        Ok((status, inbound_on, cutoff, on_hand, purchase_rows, work_rows)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "alert_id": alert_id.to_string(),
                "sku_id": sku_id.to_string(),
                "status": status,
                "inbound_on": inbound_on,
                "cutoff": cutoff,
                "on_hand": on_hand,
                "purchase_rows": purchase_rows,
                "work_rows": work_rows,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn raise_aging_alert_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AgingAlertRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if req.cutoff.trim().is_empty() {
        return bad_request("aging alert needs a cutoff").into_response();
    }
    let payload = format!("{sku_id}:{}", req.cutoff.trim());
    match SharedContractRepo::new(state.db.pool())
        .raise_aging_alert(tenant_id, &request_id, &payload, sku_id, req.cutoff.trim())
        .await
    {
        Ok(Ok((effect, _alert_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("stored inbound date did not open an alert").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct ExpiringAlertRequest {
    request_id: String,
    item_code: String,
    batch_code: String,
    cutoff: String,
}

#[derive(Debug, Deserialize)]
struct ExpiringAlertRead {
    alert_id: String,
    item_code: String,
    batch_code: String,
}

async fn read_expiring_alert_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<ExpiringAlertRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let alert_id = match uuid::Uuid::parse_str(query.alert_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad alert_id").into_response(),
    };
    if query.item_code.trim().is_empty() || query.batch_code.trim().is_empty() {
        return bad_request("expiring alert needs an item and a batch").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_expiring_alert(
            tenant_id,
            alert_id,
            query.item_code.trim(),
            query.batch_code.trim(),
        )
        .await
    {
        Ok((status, expires_on, cutoff, produced_on, purchase_rows, work_rows)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "alert_id": alert_id.to_string(),
                "item_code": query.item_code.trim(),
                "batch_code": query.batch_code.trim(),
                "status": status,
                "expires_on": expires_on,
                "cutoff": cutoff,
                "produced_on": produced_on,
                "purchase_rows": purchase_rows,
                "work_rows": work_rows,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn raise_expiring_alert_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ExpiringAlertRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.item_code.trim().is_empty()
        || req.batch_code.trim().is_empty()
        || req.cutoff.trim().is_empty()
    {
        return bad_request("expiring alert needs an item, a batch, and a cutoff").into_response();
    }
    let payload = format!(
        "{}:{}:{}",
        req.item_code.trim(),
        req.batch_code.trim(),
        req.cutoff.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .raise_expiring_alert(
            tenant_id,
            &request_id,
            &payload,
            req.item_code.trim(),
            req.batch_code.trim(),
            req.cutoff.trim(),
        )
        .await
    {
        Ok(Ok((effect, _alert_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("stored expiry did not open an alert").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct InventoryAlertRequest {
    request_id: String,
    rule_code: String,
    object_code: String,
    cutoff_at: String,
    reason: String,
    observed: i64,
    threshold: i64,
}

async fn raise_inventory_alert_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<InventoryAlertRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.rule_code.trim().is_empty()
        || req.object_code.trim().is_empty()
        || req.cutoff_at.trim().is_empty()
    {
        return bad_request("inventory alert needs a rule, an object, and a cutoff")
            .into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}:{}:{}",
        req.rule_code.trim(),
        req.object_code.trim(),
        req.cutoff_at.trim(),
        req.reason.trim(),
        req.observed,
        req.threshold
    );
    match SharedContractRepo::new(state.db.pool())
        .raise_inventory_alert(
            tenant_id,
            &request_id,
            &payload,
            req.rule_code.trim(),
            req.object_code.trim(),
            req.cutoff_at.trim(),
            req.reason.trim(),
            req.observed,
            req.threshold,
        )
        .await
    {
        Ok(Ok((effect, _alert_id, _notified))) => {
            receipt_json(&request_id, &effect).into_response()
        }
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct InventoryAlertHandleRequest {
    request_id: String,
    alert_id: uuid::Uuid,
    decision: String,
}

#[derive(Debug, Deserialize)]
struct HandledAlertRead {
    alert_id: String,
    sku_id: String,
}

async fn read_handled_alert_facts_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<HandledAlertRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let alert_id = match uuid::Uuid::parse_str(query.alert_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad alert_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(query.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_handled_alert_facts(tenant_id, alert_id, sku_id)
        .await
    {
        Ok((status, stock_rows, purchase_rows, work_rows)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "alert_id": alert_id.to_string(),
                "sku_id": sku_id.to_string(),
                "status": status,
                "stock_rows": stock_rows,
                "purchase_rows": purchase_rows,
                "work_rows": work_rows,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn handle_inventory_alert_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<InventoryAlertHandleRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.decision.trim().is_empty() {
        return bad_request("alert handling needs a decision").into_response();
    }
    let payload = format!("{}:{}", req.alert_id, req.decision.trim());
    match SharedContractRepo::new(state.db.pool())
        .handle_inventory_alert(
            tenant_id,
            &request_id,
            &payload,
            req.alert_id,
            req.decision.trim(),
        )
        .await
    {
        Ok(Ok(effect)) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct AllocationSuggestionRequest {
    request_id: String,
    rule_code: String,
    request_ref: String,
    batch_code: String,
    location_code: String,
    qty_micros: i64,
    stock_version: i64,
}

async fn save_allocation_row_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AllocationSuggestionRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.request_ref.trim().is_empty()
        || req.batch_code.trim().is_empty()
        || req.location_code.trim().is_empty()
        || req.qty_micros <= 0
    {
        return bad_request(
            "allocation row needs a request, a batch, a location, and a positive quantity",
        )
        .into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}:{}:{}",
        req.rule_code.trim(),
        req.request_ref.trim(),
        req.batch_code.trim(),
        req.location_code.trim(),
        req.qty_micros,
        req.stock_version
    );
    match SharedContractRepo::new(state.db.pool())
        .save_allocation_row(
            tenant_id,
            &request_id,
            &payload,
            req.rule_code.trim(),
            req.request_ref.trim(),
            req.batch_code.trim(),
            req.location_code.trim(),
            req.qty_micros,
            req.stock_version,
        )
        .await
    {
        Ok(Ok((effect, _suggestion_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct ShortSuggestionRequest {
    request_id: String,
    item_code: String,
    sku_id: String,
    location_code: String,
    request_ref: String,
}

#[derive(Debug, Deserialize)]
struct ShortSuggestionRead {
    item_code: String,
    request_ref: String,
    sku_id: String,
}

async fn read_short_suggestion_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<ShortSuggestionRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let sku_id = match uuid::Uuid::parse_str(query.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if query.item_code.trim().is_empty() || query.request_ref.trim().is_empty() {
        return bad_request("short suggestion needs an item and a request").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_short_suggestion(
            tenant_id,
            query.item_code.trim(),
            query.request_ref.trim(),
            sku_id,
        )
        .await
    {
        Ok((status, batch_code, chosen_qty, excluded, issue_rows)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_ref": query.request_ref.trim(),
                "sku_id": sku_id.to_string(),
                "status": status,
                "batch_code": batch_code,
                "chosen_qty": chosen_qty,
                "excluded": excluded,
                "issue_rows": issue_rows,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn suggest_short_from_stock_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ShortSuggestionRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if req.item_code.trim().is_empty()
        || req.location_code.trim().is_empty()
        || req.request_ref.trim().is_empty()
    {
        return bad_request("short suggestion needs an item, a location, and a request")
            .into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}",
        req.item_code.trim(),
        sku_id,
        req.location_code.trim(),
        req.request_ref.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .suggest_short_from_stock(
            tenant_id,
            &request_id,
            &payload,
            req.item_code.trim(),
            sku_id,
            req.location_code.trim(),
            req.request_ref.trim(),
        )
        .await
    {
        Ok(Ok((effect, _suggestion_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("stored stock does not name a shortfall").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct FrozenSuggestionRequest {
    request_id: String,
    item_code: String,
    sku_id: String,
    location_code: String,
    request_ref: String,
}

#[derive(Debug, Deserialize)]
struct FrozenSuggestionRead {
    item_code: String,
    request_ref: String,
    sku_id: String,
}

async fn read_frozen_suggestion_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<FrozenSuggestionRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let sku_id = match uuid::Uuid::parse_str(query.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if query.item_code.trim().is_empty() || query.request_ref.trim().is_empty() {
        return bad_request("frozen suggestion needs an item and a request").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_frozen_suggestion(
            tenant_id,
            query.item_code.trim(),
            query.request_ref.trim(),
            sku_id,
        )
        .await
    {
        Ok((status, chosen, excluded, issue_rows)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_ref": query.request_ref.trim(),
                "sku_id": sku_id.to_string(),
                "status": status,
                "chosen": chosen,
                "excluded": excluded,
                "issue_rows": issue_rows,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn suggest_excluding_frozen_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<FrozenSuggestionRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if req.item_code.trim().is_empty()
        || req.location_code.trim().is_empty()
        || req.request_ref.trim().is_empty()
    {
        return bad_request("frozen suggestion needs an item, a location, and a request")
            .into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}",
        req.item_code.trim(),
        sku_id,
        req.location_code.trim(),
        req.request_ref.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .suggest_excluding_frozen(
            tenant_id,
            &request_id,
            &payload,
            req.item_code.trim(),
            sku_id,
            req.location_code.trim(),
            req.request_ref.trim(),
        )
        .await
    {
        Ok(Ok((effect, _suggestion_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("stored freeze flag did not exclude a batch").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct DatedSuggestionRequest {
    request_id: String,
    item_code: String,
    sku_id: String,
    location_code: String,
    request_ref: String,
    cutoff: String,
}

#[derive(Debug, Deserialize)]
struct DatedSuggestionRead {
    item_code: String,
    request_ref: String,
    sku_id: String,
    cutoff: String,
}

async fn read_dated_suggestion_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<DatedSuggestionRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let sku_id = match uuid::Uuid::parse_str(query.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if query.item_code.trim().is_empty()
        || query.request_ref.trim().is_empty()
        || query.cutoff.trim().len() != 10
    {
        return bad_request("dated suggestion needs an item, a request, and a cutoff")
            .into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_dated_suggestion(
            tenant_id,
            query.item_code.trim(),
            query.request_ref.trim(),
            sku_id,
            query.cutoff.trim(),
        )
        .await
    {
        Ok((status, chosen, excluded, issue_rows)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_ref": query.request_ref.trim(),
                "sku_id": sku_id.to_string(),
                "status": status,
                "chosen": chosen,
                "excluded": excluded,
                "issue_rows": issue_rows,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn suggest_excluding_expired_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<DatedSuggestionRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if req.item_code.trim().is_empty()
        || req.location_code.trim().is_empty()
        || req.request_ref.trim().is_empty()
        || req.cutoff.trim().len() != 10
    {
        return bad_request("dated suggestion needs an item, a location, a request, and a cutoff")
            .into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}:{}",
        req.item_code.trim(),
        sku_id,
        req.location_code.trim(),
        req.request_ref.trim(),
        req.cutoff.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .suggest_excluding_expired(
            tenant_id,
            &request_id,
            &payload,
            req.item_code.trim(),
            sku_id,
            req.location_code.trim(),
            req.request_ref.trim(),
            req.cutoff.trim(),
        )
        .await
    {
        Ok(Ok((effect, _ids))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("stored dates did not exclude an expired batch").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct StockBackedSuggestionRequest {
    request_id: String,
    sku_id: String,
    batch_code: String,
    location_code: String,
    request_ref: String,
}

#[derive(Debug, Deserialize)]
struct StockBackedSuggestionRead {
    suggestion_id: String,
    sku_id: String,
}

async fn read_stock_backed_suggestion_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<StockBackedSuggestionRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let suggestion_id = match uuid::Uuid::parse_str(query.suggestion_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad suggestion_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(query.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_stock_backed_suggestion(tenant_id, suggestion_id, sku_id)
        .await
    {
        Ok((status, suggested, on_hand, issue_rows, batch_code)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "suggestion_id": suggestion_id.to_string(),
                "sku_id": sku_id.to_string(),
                "status": status,
                "suggested": suggested,
                "on_hand": on_hand,
                "issue_rows": issue_rows,
                "batch_code": batch_code,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn save_stock_backed_suggestion_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<StockBackedSuggestionRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if req.batch_code.trim().is_empty()
        || req.location_code.trim().is_empty()
        || req.request_ref.trim().is_empty()
    {
        return bad_request("stock-backed suggestion needs a batch, a location, and a request")
            .into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}",
        sku_id,
        req.batch_code.trim(),
        req.location_code.trim(),
        req.request_ref.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .save_stock_backed_suggestion(
            tenant_id,
            &request_id,
            &payload,
            sku_id,
            req.batch_code.trim(),
            req.location_code.trim(),
            req.request_ref.trim(),
        )
        .await
    {
        Ok(Ok((effect, _suggestion_id, _qty))) => {
            receipt_json(&request_id, &effect).into_response()
        }
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("stored stock did not keep a suggestion").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct RemainingStockIssueRequest {
    request_id: String,
    suggestion_id: String,
    sku_id: String,
}

#[derive(Debug, Deserialize)]
struct RemainingStockIssueRead {
    suggestion_id: String,
    sku_id: String,
}

async fn read_remaining_stock_issue_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<RemainingStockIssueRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let suggestion_id = match uuid::Uuid::parse_str(query.suggestion_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad suggestion_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(query.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_remaining_stock_issue(tenant_id, suggestion_id, sku_id)
        .await
    {
        Ok((status, suggested, on_hand, issued)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "suggestion_id": suggestion_id.to_string(),
                "sku_id": sku_id.to_string(),
                "status": status,
                "suggested": suggested,
                "on_hand": on_hand,
                "issued": issued,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn issue_if_stock_remains_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<RemainingStockIssueRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let suggestion_id = match uuid::Uuid::parse_str(req.suggestion_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad suggestion_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    let payload = format!("{suggestion_id}:{sku_id}");
    match SharedContractRepo::new(state.db.pool())
        .issue_if_stock_remains(tenant_id, &request_id, &payload, suggestion_id, sku_id)
        .await
    {
        Ok(Ok((effect, _issue_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("stored stock no longer covers the suggestion").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct StockBackedChoiceRequest {
    request_id: String,
    suggestion_id: String,
    sku_id: String,
    chosen_batch: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct StockBackedChoiceRead {
    suggestion_id: String,
    sku_id: String,
}

async fn read_stock_backed_choice_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<StockBackedChoiceRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let suggestion_id = match uuid::Uuid::parse_str(query.suggestion_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad suggestion_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(query.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_stock_backed_choice(tenant_id, suggestion_id, sku_id)
        .await
    {
        Ok((status, suggested_batch, chosen_batch, good, held, issue_rows)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "suggestion_id": suggestion_id.to_string(),
                "sku_id": sku_id.to_string(),
                "status": status,
                "suggested_batch": suggested_batch,
                "chosen_batch": chosen_batch,
                "good": good,
                "held": held,
                "issue_rows": issue_rows,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn choose_if_stock_covers_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<StockBackedChoiceRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let suggestion_id = match uuid::Uuid::parse_str(req.suggestion_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad suggestion_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad sku_id").into_response(),
    };
    if req.chosen_batch.trim().is_empty() || req.reason.trim().is_empty() {
        return bad_request("stock-backed choice needs a batch and a reason").into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}",
        suggestion_id,
        sku_id,
        req.chosen_batch.trim(),
        req.reason.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .choose_if_stock_covers(
            tenant_id,
            &request_id,
            &payload,
            suggestion_id,
            sku_id,
            req.chosen_batch.trim(),
            req.reason.trim(),
        )
        .await
    {
        Ok(Ok((effect, _choice_id, _qty))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("stored stock does not cover the choice").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct AllocationIssueRequest {
    request_id: String,
    suggestion_id: uuid::Uuid,
    current_version: i64,
    available: i64,
}

async fn issue_allocation_suggestion_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<AllocationIssueRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.available <= 0 {
        return bad_request("allocation issue needs a positive available quantity").into_response();
    }
    let payload = format!(
        "{}:{}:{}",
        req.suggestion_id, req.current_version, req.available
    );
    match SharedContractRepo::new(state.db.pool())
        .issue_allocation_suggestion(
            tenant_id,
            &request_id,
            &payload,
            req.suggestion_id,
            req.current_version,
            req.available,
        )
        .await
    {
        Ok(Ok((effect, _issue_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct PurchaseRevisionLineRequest {
    line_no: i32,
    item_code: String,
    qty_micros: i64,
    price_cent: i64,
    tax_cent: i64,
    due_date: String,
}

#[derive(Debug, Deserialize)]
struct PurchaseRevisionRequest {
    request_id: String,
    po_id: Option<uuid::Uuid>,
    supplier_code: String,
    warehouse_code: String,
    action: String,
    lines: Vec<PurchaseRevisionLineRequest>,
}

async fn save_purchase_revision_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PurchaseRevisionRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.supplier_code.trim().is_empty()
        || req.warehouse_code.trim().is_empty()
        || req.lines.is_empty()
    {
        return bad_request(
            "purchase revision needs a supplier, a warehouse, and at least one line",
        )
        .into_response();
    }
    let lines: Vec<erp_domain::trade::PurchaseLine> = req
        .lines
        .iter()
        .map(|line| erp_domain::trade::PurchaseLine {
            line_no: line.line_no,
            item_code: line.item_code.trim().to_string(),
            qty_micros: line.qty_micros,
            price_cent: line.price_cent,
            tax_cent: line.tax_cent,
            due_date: line.due_date.trim().to_string(),
        })
        .collect();
    let payload = format!(
        "{}:{}:{}:{}:{}",
        req.po_id.map(|id| id.to_string()).unwrap_or_default(),
        req.supplier_code.trim(),
        req.warehouse_code.trim(),
        req.action.trim(),
        lines
            .iter()
            .map(|line| format!(
                "{}:{}:{}:{}:{}:{}",
                line.line_no,
                line.item_code,
                line.qty_micros,
                line.price_cent,
                line.tax_cent,
                line.due_date
            ))
            .collect::<Vec<_>>()
            .join(",")
    );
    match SharedContractRepo::new(state.db.pool())
        .save_purchase_revision(erp_repo::shared_contract::TradePurchaseRevisionInput { tenant_id, request_id: &request_id, payload_hash: &payload, po_id: req.po_id, supplier_code: req.supplier_code.trim(), warehouse_code: req.warehouse_code.trim(), action: req.action.trim(), lines: &lines })
        .await
    {
        Ok(Ok((effect, _revision_id, _version))) => {
            receipt_json(&request_id, &effect).into_response()
        }
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("purchase revision conflicts with stored receipts").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct PurchaseReceiptOffsetRead {
    quality_id: String,
}

async fn read_purchase_receipt_offset_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<PurchaseReceiptOffsetRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let quality_id = match uuid::Uuid::parse_str(query.quality_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad quality_id").into_response(),
    };
    let session_id = match ctx.session_id {
        Some(id) if !id.is_nil() => id,
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                "receipt offset needs an active session",
            )
                .into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .read_purchase_receipt_offset(
            tenant_id,
            quality_id,
            erp_repo::shared_contract::ReceiptOffsetActor {
                user_id: ctx.user_id,
                session_id,
            },
        )
        .await
    {
        Ok(snapshot) => (StatusCode::OK, Json(snapshot)).into_response(),
        Err(erp_repo::shared_contract::ReceiptOffsetError::InvalidIdentity) => {
            bad_request("bad receipt offset identity").into_response()
        }
        Err(erp_repo::shared_contract::ReceiptOffsetError::NotFound) => {
            not_found("receipt offset unavailable").into_response()
        }
        Err(erp_repo::shared_contract::ReceiptOffsetError::ReconciliationRequired) => {
            conflict("receipt_offset_reconciliation_required").into_response()
        }
        Err(erp_repo::shared_contract::ReceiptOffsetError::Storage(_)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "receipt offset storage unavailable",
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct PurchaseArrivalSplitRead {
    po_id: String,
    line_no: i32,
}

async fn read_purchase_arrival_split_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<PurchaseArrivalSplitRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let po_id = match uuid::Uuid::parse_str(query.po_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad po_id").into_response(),
    };
    if query.line_no <= 0 {
        return bad_request("purchase arrival split needs a line").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_purchase_arrival_split(tenant_id, po_id, query.line_no)
        .await
    {
        Ok((status, ordered, first, second, over_rows)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "po_id": po_id.to_string(),
                "line_no": query.line_no,
                "status": status,
                "ordered": ordered,
                "first": first,
                "second": second,
                "over_rows": over_rows,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct PurchaseRevisionFloorRead {
    po_id: String,
    line_no: i32,
    attempted: i64,
}

async fn read_purchase_revision_floor_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<PurchaseRevisionFloorRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let po_id = match uuid::Uuid::parse_str(query.po_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad po_id").into_response(),
    };
    if query.line_no <= 0 || query.attempted <= 0 {
        return bad_request("purchase revision floor needs a line and an attempt").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_purchase_revision_floor(tenant_id, po_id, query.line_no, query.attempted)
        .await
    {
        Ok((status, revision_id, version_no, ordered, received, refused_rows)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "po_id": po_id.to_string(),
                "line_no": query.line_no,
                "status": status,
                "revision_id": revision_id.to_string(),
                "version_no": version_no,
                "ordered": ordered,
                "received": received,
                "attempted": query.attempted,
                "refused_rows": refused_rows,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct PurchaseRevisionRollbackRead {
    po_id: String,
    refused_line_count: i64,
}

async fn read_purchase_revision_rollback_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<PurchaseRevisionRollbackRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let po_id = match uuid::Uuid::parse_str(query.po_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad po_id").into_response(),
    };
    if query.refused_line_count <= 0 {
        return bad_request("purchase revision rollback needs a refused line count")
            .into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_purchase_revision_rollback(tenant_id, po_id, query.refused_line_count)
        .await
    {
        Ok((status, revision_id, open_lines, later_versions)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "po_id": po_id.to_string(),
                "status": status,
                "revision_id": revision_id.to_string(),
                "open_lines": open_lines,
                "later_versions": later_versions,
                "refused_line_count": query.refused_line_count,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct PurchaseRemainderRead {
    po_id: String,
    line_no: i32,
}

async fn read_purchase_remainder_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<PurchaseRemainderRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let po_id = match uuid::Uuid::parse_str(query.po_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad po_id").into_response(),
    };
    if query.line_no <= 0 {
        return bad_request("purchase remainder needs a line").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_purchase_remainder(tenant_id, po_id, query.line_no)
        .await
    {
        Ok((status, ordered, received, remaining, closed_rows)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "po_id": po_id.to_string(),
                "line_no": query.line_no,
                "status": status,
                "ordered": ordered,
                "received": received,
                "remaining": remaining,
                "closed_rows": closed_rows,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct PurchaseCloseRequest {
    request_id: String,
    po_id: uuid::Uuid,
}

async fn close_purchase_when_received_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PurchaseCloseRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.po_id.is_nil() {
        return bad_request("purchase close needs an order").into_response();
    }
    let payload = req.po_id.to_string();
    match SharedContractRepo::new(state.db.pool())
        .close_purchase_when_received(tenant_id, &request_id, &payload, req.po_id)
        .await
    {
        Ok(Ok(effect)) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("purchase close refused while a remainder is open").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct PurchaseTaxRequest {
    request_id: String,
    revision_id: uuid::Uuid,
}

async fn sum_purchase_tax_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PurchaseTaxRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.revision_id.is_nil() {
        return bad_request("purchase tax needs a revision").into_response();
    }
    let payload = req.revision_id.to_string();
    match SharedContractRepo::new(state.db.pool())
        .sum_purchase_tax(tenant_id, &request_id, &payload, req.revision_id)
        .await
    {
        Ok(Ok((effect, _tax))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct SalesRevisionLineRequest {
    line_no: i32,
    item_code: String,
    qty_micros: i64,
    price_cent: i64,
    discount_cent: i64,
    tax_cent: i64,
    due_date: String,
    address: String,
    source: String,
}

#[derive(Debug, Deserialize)]
struct SalesRevisionRequest {
    request_id: String,
    so_id: Option<uuid::Uuid>,
    customer_code: String,
    expected_version: Option<i32>,
    action: String,
    lines: Vec<SalesRevisionLineRequest>,
}

async fn save_sales_revision_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SalesRevisionRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.customer_code.trim().is_empty() || req.lines.is_empty() {
        return bad_request("sales revision needs a customer and at least one line")
            .into_response();
    }
    let lines: Vec<erp_domain::trade::SalesLine> = req
        .lines
        .iter()
        .map(|line| erp_domain::trade::SalesLine {
            line_no: line.line_no,
            item_code: line.item_code.trim().to_string(),
            qty_micros: line.qty_micros,
            price_cent: line.price_cent,
            discount_cent: line.discount_cent,
            tax_cent: line.tax_cent,
            due_date: line.due_date.trim().to_string(),
            address: line.address.trim().to_string(),
            source: line.source.trim().to_string(),
        })
        .collect();
    let payload = format!(
        "{}:{}:{}:{}",
        req.so_id.map(|id| id.to_string()).unwrap_or_default(),
        req.customer_code.trim(),
        req.action.trim(),
        lines
            .iter()
            .map(|line| format!(
                "{}:{}:{}:{}:{}:{}:{}:{}:{}",
                line.line_no,
                line.item_code,
                line.qty_micros,
                line.price_cent,
                line.discount_cent,
                line.tax_cent,
                line.due_date,
                line.address,
                line.source
            ))
            .collect::<Vec<_>>()
            .join(",")
    );
    match SharedContractRepo::new(state.db.pool())
        .save_sales_revision(erp_repo::shared_contract::TradeSalesRevisionInput { tenant_id, request_id: &request_id, payload_hash: &payload, so_id: req.so_id, customer_code: req.customer_code.trim(), expected_version: req.expected_version, action: req.action.trim(), lines: &lines })
        .await
    {
        Ok(Ok((effect, _revision_id, _version))) => {
            receipt_json(&request_id, &effect).into_response()
        }
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) if error.to_string().contains("CONFLICT:") => {
            conflict("sales revision conflicts with stored deliveries").into_response()
        }
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct SalesRevisionFloorRead {
    so_id: String,
    line_no: i32,
    attempted: i64,
}

async fn read_sales_revision_floor_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<SalesRevisionFloorRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let so_id = match uuid::Uuid::parse_str(query.so_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad so_id").into_response(),
    };
    if query.line_no <= 0 || query.attempted <= 0 {
        return bad_request("sales revision floor needs a line and an attempt").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_sales_revision_floor(tenant_id, so_id, query.line_no, query.attempted)
        .await
    {
        Ok((status, revision_id, version_no, ordered, delivered, later_versions)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "so_id": so_id.to_string(),
                "line_no": query.line_no,
                "status": status,
                "revision_id": revision_id.to_string(),
                "version_no": version_no,
                "ordered": ordered,
                "delivered": delivered,
                "attempted": query.attempted,
                "later_versions": later_versions,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct SalesReturnNetRead {
    delivery_id: String,
}

async fn read_sales_return_net_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<SalesReturnNetRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let delivery_id = match uuid::Uuid::parse_str(query.delivery_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad delivery_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_sales_return_net(tenant_id, delivery_id)
        .await
    {
        Ok((status, delivered, returned, net, credit)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "delivery_id": delivery_id.to_string(),
                "status": status,
                "delivered": delivered,
                "returned": returned,
                "net": net,
                "credit_cent": credit,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct SalesDeliveryRepeatRead {
    delivery_id: String,
}

async fn read_sales_delivery_repeat_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<SalesDeliveryRepeatRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let delivery_id = match uuid::Uuid::parse_str(query.delivery_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad delivery_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_sales_delivery_repeat(tenant_id, delivery_id)
        .await
    {
        Ok((status, qty, on_hand, reserved, deduction_rows)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "delivery_id": delivery_id.to_string(),
                "status": status,
                "qty": qty,
                "on_hand": on_hand,
                "reserved": reserved,
                "deduction_rows": deduction_rows,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct SalesDeliverySplitRead {
    so_id: String,
    line_no: i32,
}

async fn read_sales_delivery_split_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<SalesDeliverySplitRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let so_id = match uuid::Uuid::parse_str(query.so_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad so_id").into_response(),
    };
    if query.line_no <= 0 {
        return bad_request("sales delivery split needs a line").into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_sales_delivery_split(tenant_id, so_id, query.line_no)
        .await
    {
        Ok((status, ordered, first, second, stock_rows)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "so_id": so_id.to_string(),
                "line_no": query.line_no,
                "status": status,
                "ordered": ordered,
                "first": first,
                "second": second,
                "stock_rows": stock_rows,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct SalesRevisionConflictRead {
    so_id: String,
    user_id: String,
    expected_version: i32,
}

async fn read_sales_revision_conflict_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<SalesRevisionConflictRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let so_id = match uuid::Uuid::parse_str(query.so_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad so_id").into_response(),
    };
    if query.user_id.trim().is_empty() || query.expected_version <= 0 {
        return bad_request("sales revision conflict needs a user and a stale version")
            .into_response();
    }
    match SharedContractRepo::new(state.db.pool())
        .read_sales_revision_conflict(
            tenant_id,
            so_id,
            query.user_id.trim(),
            query.expected_version,
        )
        .await
    {
        Ok((status, revision_id, stored_version, later_versions, draft_id, body)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "so_id": so_id.to_string(),
                "status": status,
                "revision_id": revision_id.to_string(),
                "stored_version": stored_version,
                "expected_version": query.expected_version,
                "later_versions": later_versions,
                "draft_id": draft_id.to_string(),
                "draft_body": body,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct ManualSaleRead {
    so_id: String,
}

async fn read_manual_sale_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    Query(query): Query<ManualSaleRead>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let so_id = match uuid::Uuid::parse_str(query.so_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad so_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_manual_sale(tenant_id, so_id)
        .await
    {
        Ok((status, shop_rows, platform_rows, confirm_rows, demand_rows, delivered)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "so_id": so_id.to_string(),
                "status": status,
                "shop_rows": shop_rows,
                "platform_rows": platform_rows,
                "confirm_rows": confirm_rows,
                "demand_rows": demand_rows,
                "delivered": delivered,
            })),
        )
            .into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct SalesConfirmRequest {
    request_id: String,
    so_id: uuid::Uuid,
    line_no: i32,
}

async fn confirm_trade_order_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SalesConfirmRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.so_id.is_nil() || req.line_no <= 0 {
        return bad_request("sales confirm needs an order and a line").into_response();
    }
    let payload = format!("{}:{}", req.so_id, req.line_no);
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .confirm_trade_order(tenant_id, &request_id, &payload, req.so_id, req.line_no)
            .await,
    )
}

#[derive(Debug, Deserialize)]
struct DraftSaleRequest {
    request_id: String,
    customer_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    amount_cent: i64,
    currency: String,
}

async fn save_draft_sale_route(
    State(state): State<AppState>,
    Path(path_tenant): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<DraftSaleRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.customer_id.is_nil()
        || req.sku_id.is_nil()
        || req.amount_cent <= 0
        || req.currency.trim().is_empty()
    {
        return bad_request(
            "draft sale needs a customer, a sku, a positive amount, and a currency",
        )
        .into_response();
    }
    let payload = format!(
        "{}:{}:{}:{}",
        req.customer_id,
        req.sku_id,
        req.amount_cent,
        req.currency.trim()
    );
    match SharedContractRepo::new(state.db.pool())
        .save_draft_sale(
            tenant_id,
            &request_id,
            &payload,
            req.customer_id,
            req.sku_id,
            req.amount_cent,
            req.currency.trim(),
        )
        .await
    {
        Ok(Ok((effect, _order_id))) => receipt_json(&request_id, &effect).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn download_owned_resource_route(
    State(state): State<AppState>,
    Path((path_tenant, resource_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let resource_id = match uuid::Uuid::parse_str(resource_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad resource_id").into_response(),
    };
    match SharedContractRepo::new(state.db.pool())
        .read_owned_resource(tenant_id, tenant_id, resource_id)
        .await
    {
        Ok(Some(body)) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            body,
        )
            .into_response(),
        Ok(None) => not_found("resource not found").into_response(),
        Err(_) => not_found("resource not found").into_response(),
    }
}

async fn write_owned_resource_route(
    State(state): State<AppState>,
    Path((path_tenant, resource_id)): Path<(String, String)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OwnedResourceWriteRequest>,
) -> impl IntoResponse {
    let tenant_id = match authenticated_tenant(&path_tenant, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let resource_id = match uuid::Uuid::parse_str(resource_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad resource_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!("{}:{}", resource_id, req.body.trim());
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .write_owned_resource(
                tenant_id,
                tenant_id,
                &request_id,
                &payload,
                resource_id,
                req.body.trim(),
            )
            .await,
    )
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PublicOriginRequest {
    request_id: String,
    origin: String,
}

async fn save_public_origin_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PublicOriginRequest>,
) -> impl IntoResponse {
    let tenant_id = match crate::manufacturing::require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.origin.trim().is_empty() {
        return bad_request("bad public origin").into_response();
    }
    let payload = match serde_json::to_string(&(1, req.origin.trim())) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .save_public_origin(tenant_id, &request_id, &payload, req.origin.trim())
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "origin": req.origin.trim(),
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DownloadLinkRequest {
    request_id: String,
    object_id: String,
}

async fn issue_download_link_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<DownloadLinkRequest>,
) -> impl IntoResponse {
    let tenant_id = match export_link_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.object_id.trim().is_empty() {
        return bad_request("bad download link").into_response();
    }
    let actor = ctx.user_id.to_string();
    let payload = match serde_json::to_string(&(1, actor.as_str(), req.object_id.trim())) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .issue_download_link(
            tenant_id,
            &request_id,
            &payload,
            &actor,
            req.object_id.trim(),
        )
        .await
    {
        Ok(Ok((write, link_id))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "link_id": link_id,
                "object_id": req.object_id.trim(),
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
struct DownloadAuthorizationRequest {
    request_id: String,
    object_id: String,
}

async fn authorize_download_link_route(
    State(state): State<AppState>,
    Path((tenant_id, link_id)): Path<(String, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<DownloadAuthorizationRequest>,
) -> impl IntoResponse {
    let tenant_id = match export_link_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.object_id.trim().is_empty() {
        return bad_request("bad download authorization").into_response();
    }
    let actor = ctx.user_id.to_string();
    let payload = match serde_json::to_string(&(1, link_id, actor.as_str(), req.object_id.trim())) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .authorize_download_object(
            tenant_id,
            &request_id,
            &payload,
            link_id,
            &actor,
            req.object_id.trim(),
        )
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "link_id": link_id,
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
struct DownloadRevocationRequest {
    request_id: String,
}

async fn revoke_download_link_route(
    State(state): State<AppState>,
    Path((tenant_id, link_id)): Path<(String, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<DownloadRevocationRequest>,
) -> impl IntoResponse {
    let tenant_id = match export_link_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = match serde_json::to_string(&(1, link_id, "REVOKED")) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .revoke_download_link(tenant_id, &request_id, &payload, link_id)
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "link_id": link_id,
                "link_status": "REVOKED",
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

fn export_link_tenant(tenant_id: &str, ctx: &AuthContext) -> Result<TenantId, Response> {
    let tenant_id =
        TenantId::from_str(tenant_id).map_err(|_| bad_request("bad tenant_id").into_response())?;
    if let Err(resp) = ensure_tenant_access(ctx, tenant_id) {
        return Err(resp.into_response());
    }
    if let Err(resp) = ensure_permission(ctx, "exports.read") {
        return Err(resp.into_response());
    }
    Ok(tenant_id)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BrowserSessionRequest {
    request_id: String,
    mode: String,
}

async fn open_browser_session_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<BrowserSessionRequest>,
) -> impl IntoResponse {
    let tenant_id = match crate::manufacturing::require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.mode.trim() != "BEARER" {
        return bad_request("bad browser session").into_response();
    }
    let actor = ctx.user_id.to_string();
    let payload = match serde_json::to_string(&(1, actor.as_str(), "BEARER")) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .open_browser_session(
            tenant_id,
            &request_id,
            &payload,
            &actor,
            "BEARER",
            false,
            false,
            "",
            "",
        )
        .await
    {
        Ok(Ok((write, session_id, _epoch))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": session_id,
                "session_status": "ACTIVE",
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn revoke_browser_session_route(
    State(state): State<AppState>,
    Path((tenant_id, session_id)): Path<(String, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<BrowserSessionRequest>,
) -> impl IntoResponse {
    let tenant_id = match crate::manufacturing::require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = match serde_json::to_string(&(1, session_id, "REVOKED")) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .revoke_browser_session(tenant_id, &request_id, &payload, session_id)
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "effect_id": session_id,
                "session_status": "REVOKED",
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FreeSeatRequest {
    request_id: String,
    user_id: String,
    client_kind: String,
}

async fn open_free_seat_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<FreeSeatRequest>,
) -> impl IntoResponse {
    let tenant_id = match crate::manufacturing::require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.user_id.trim().is_empty() || req.client_kind.trim().is_empty() {
        return bad_request("bad free seat").into_response();
    }
    let payload = match serde_json::to_string(&(1, req.user_id.trim(), req.client_kind.trim())) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .open_free_login(
            tenant_id,
            &request_id,
            &payload,
            req.user_id.trim(),
            req.client_kind.trim(),
        )
        .await
    {
        Ok(Ok(write)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "user_id": req.user_id.trim(),
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginClientRequest {
    request_id: String,
    user_id: String,
    client_kind: String,
}

async fn open_login_client_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LoginClientRequest>,
) -> impl IntoResponse {
    let tenant_id = match crate::manufacturing::require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.user_id.trim().is_empty() || req.client_kind.trim().is_empty() {
        return bad_request("bad login client").into_response();
    }
    let payload = match serde_json::to_string(&(1, req.user_id.trim(), req.client_kind.trim())) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .open_login_client(
            tenant_id,
            &request_id,
            &payload,
            req.user_id.trim(),
            req.client_kind.trim(),
        )
        .await
    {
        Ok(Ok((write, session_id))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "session_id": session_id,
                "user_id": req.user_id.trim(),
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PasswordChangeRequest {
    request_id: String,
}

async fn change_login_password_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PasswordChangeRequest>,
) -> impl IntoResponse {
    let tenant_id = match crate::manufacturing::require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let user_id = ctx.user_id.to_string();
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = match serde_json::to_string(&(1, user_id.as_str())) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .change_login_password(tenant_id, &request_id, &payload, &user_id)
        .await
    {
        Ok(Ok((write, epoch))) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": request_id,
                "status": "SUCCEEDED",
                "replay": matches!(write, SharedWrite::Replay(_)),
                "epoch": epoch,
                "user_id": user_id,
            })),
        )
            .into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EpochInboundRequest {
    request_id: String,
    session_id: uuid::Uuid,
    token_epoch: i32,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    quantity: i64,
}

async fn post_epoch_bound_inbound_route(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<EpochInboundRequest>,
) -> impl IntoResponse {
    let tenant_id = match crate::manufacturing::require_tenant(&tenant_id, &ctx) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.quantity <= 0 {
        return bad_request("quantity must be positive").into_response();
    }
    let payload = match serde_json::to_string(&(
        1,
        req.session_id,
        req.token_epoch,
        req.warehouse_id,
        req.sku_id,
        req.quantity,
    )) {
        Ok(payload) => payload,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match SharedContractRepo::new(state.db.pool())
        .post_epoch_bound_inbound(
            tenant_id,
            &request_id,
            &payload,
            req.session_id,
            req.token_epoch,
            req.warehouse_id,
            req.sku_id,
            req.quantity,
        )
        .await
    {
        Ok(Ok(write)) => {
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
                    "effect_id": effect,
                })),
            )
                .into_response()
        }
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => (StatusCode::UNPROCESSABLE_ENTITY, format!("{error}")).into_response(),
    }
}

async fn renew_login_client_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SessionRenewRequest>,
) -> impl IntoResponse {
    let tenant_id = match parse_tenant(&req.tenant_id) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let session_id = match uuid::Uuid::parse_str(req.session_id.trim()) {
        Ok(id) if !id.is_nil() => id,
        _ => return bad_request("bad session_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = session_id.to_string();
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .renew_login_client(tenant_id, &request_id, &payload, session_id)
            .await,
    )
}

async fn recover_login_password_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<PasswordRecoverRequest>,
) -> impl IntoResponse {
    let tenant_id = match parse_tenant(&req.tenant_id) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = format!("{}:{}", req.user_id.trim(), req.token.trim());
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .recover_login_password(
                tenant_id,
                &request_id,
                &payload,
                req.user_id.trim(),
                req.token.trim(),
            )
            .await,
    )
}

async fn complete_onboarding_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<OnboardingCompleteRequest>,
) -> impl IntoResponse {
    let tenant_id = match parse_tenant(&req.tenant_id) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    let payload = req.verification.trim().to_string();
    write_result(
        &request_id,
        SharedContractRepo::new(state.db.pool())
            .complete_onboarding(tenant_id, &request_id, &payload, req.verification.trim())
            .await,
    )
}
