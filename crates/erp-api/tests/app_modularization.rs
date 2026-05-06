use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use erp_api::{app_state::AppState, router::build_app as build_app_from_module, JwtService};
use erp_repo::db::Db;
use tower::ServiceExt;

mod support;

#[tokio::test]
async fn modular_exports_build_healthz_app() {
    let database_url = support::test_database_url();
    let db = Db::connect_lazy(&database_url).expect("lazy db should be constructible");
    let jwt = JwtService::new("test-secret".to_string());
    let app = build_app_from_module(AppState::new(db, std::sync::Arc::new(jwt)));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("app should respond");

    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("healthz body should be readable");
    match status {
        StatusCode::OK => assert_eq!(body.as_ref(), b"ok"),
        StatusCode::SERVICE_UNAVAILABLE => assert_eq!(body.as_ref(), b"db not ready"),
        status => panic!("unexpected healthz status: {status}"),
    }
}
#[test]
fn api_common_helpers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let support_rs = include_str!("../src/support.rs");
    let inventory_views_rs = include_str!("../src/inventory_views.rs");
    let shop_views_rs = include_str!("../src/shop_views.rs");

    assert!(lib_rs.contains("mod support;"));
    assert!(!lib_rs.contains("\nfn ensure_permission("));
    assert!(support_rs.contains("pub(crate) fn ensure_permission("));
    assert!(support_rs.contains("pub(crate) fn error_response("));

    assert!(lib_rs.contains("mod inventory_views;"));
    assert!(!lib_rs.contains("\nfn inventory_record_response("));
    assert!(inventory_views_rs.contains("pub(crate) fn inventory_record_response("));
    assert!(
        inventory_views_rs.contains("pub(crate) async fn ensure_default_stock_count_strategies(")
    );

    assert!(lib_rs.contains("mod shop_views;"));
    assert!(!lib_rs.contains("\nfn shop_view("));
    assert!(shop_views_rs.contains("pub(crate) fn shop_view("));
    assert!(shop_views_rs.contains("pub(crate) fn derive_shop_integration_status("));
}
#[test]
fn order_settlement_handlers_are_split_from_orders_rs() {
    let orders_rs = include_str!("../src/orders.rs");
    let settlement_rs = include_str!("../src/orders/settlement.rs");

    assert!(!orders_rs.contains("\nasync fn get_order_settlement_rule("));
    assert!(!orders_rs.contains("\nasync fn execute_order_settlement_action("));
    assert!(settlement_rs.contains("pub(super) async fn get_order_settlement_rule("));
    assert!(settlement_rs.contains("pub(super) async fn execute_order_settlement_action("));
}

#[test]
fn order_reconciliation_handlers_are_split_from_orders_rs() {
    let orders_rs = include_str!("../src/orders.rs");
    let settlement_rs = include_str!("../src/orders/settlement.rs");

    assert!(!orders_rs.contains("\nasync fn list_order_reconciliations("));
    assert!(!orders_rs.contains("\nasync fn settle_order_reconciliation("));
    assert!(settlement_rs.contains("pub(super) async fn list_order_reconciliations("));
    assert!(settlement_rs.contains("pub(super) async fn settle_order_reconciliation("));
}
#[test]
fn order_exception_handlers_are_split_from_orders_rs() {
    let orders_rs = include_str!("../src/orders.rs");
    let exception_workbench_rs = include_str!("../src/orders/exception_workbench.rs");

    assert!(!orders_rs.contains("\nasync fn handle_order_exception("));
    assert!(!orders_rs.contains("\nasync fn apply_order_exception_action("));
    assert!(exception_workbench_rs.contains("pub(super) async fn handle_order_exception("));
    assert!(exception_workbench_rs.contains("pub(super) async fn apply_order_exception_action("));
}
#[test]
fn order_split_merge_handlers_are_split_from_orders_rs() {
    let orders_rs = include_str!("../src/orders.rs");
    let split_merge_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/orders/split_merge.rs"),
    )
    .expect("split_merge module should exist");

    assert!(orders_rs.contains("mod split_merge;"));
    assert!(!orders_rs.contains("\nasync fn get_order_chain("));
    assert!(!orders_rs.contains("\nasync fn merge_orders("));
    assert!(!orders_rs.contains("\nasync fn split_order("));
    assert!(split_merge_rs.contains("pub(super) async fn get_order_chain("));
    assert!(split_merge_rs.contains("pub(super) async fn merge_orders("));
    assert!(split_merge_rs.contains("pub(super) async fn split_order("));
}
#[test]
fn order_item_recovery_handlers_are_split_from_orders_rs() {
    let orders_rs = include_str!("../src/orders.rs");
    let recovery_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/orders/recovery.rs"),
    )
    .expect("recovery module should exist");

    assert!(orders_rs.contains("mod recovery;"));
    assert!(!orders_rs.contains("\nasync fn rematch_order_items("));
    assert!(!orders_rs.contains("\nasync fn return_order_to_download("));
    assert!(recovery_rs.contains("pub(super) async fn rematch_order_items("));
    assert!(recovery_rs.contains("pub(super) async fn return_order_to_download("));
}

#[test]
fn order_annotation_handlers_are_split_from_orders_rs() {
    let orders_rs = include_str!("../src/orders.rs");
    let annotations_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/orders/annotations.rs"),
    )
    .expect("annotations module should exist");

    assert!(orders_rs.contains("mod annotations;"));
    assert!(!orders_rs.contains("\nasync fn add_order_tags("));
    assert!(!orders_rs.contains("\nasync fn apply_auto_tags("));
    assert!(!orders_rs.contains("\nasync fn add_order_note("));
    assert!(annotations_rs.contains("pub(super) async fn add_order_tags("));
    assert!(annotations_rs.contains("pub(super) async fn apply_auto_tags("));
    assert!(annotations_rs.contains("pub(super) async fn add_order_note("));
}
#[test]
fn order_tag_definition_handlers_are_split_from_orders_rs() {
    let orders_rs = include_str!("../src/orders.rs");
    let annotations_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/orders/annotations.rs"),
    )
    .expect("annotations module should exist");

    assert!(!orders_rs.contains("\npub(super) async fn list_order_tag_definitions("));
    assert!(!orders_rs.contains("\npub(super) async fn create_order_tag_definition("));
    assert!(!orders_rs.contains("\npub(super) async fn list_sku_tag_definitions("));
    assert!(!orders_rs.contains("\npub(super) async fn delete_sku_tag_definition("));
    assert!(annotations_rs.contains("pub(crate) async fn list_order_tag_definitions("));
    assert!(annotations_rs.contains("pub(crate) async fn create_order_tag_definition("));
    assert!(annotations_rs.contains("pub(crate) async fn list_sku_tag_definitions("));
    assert!(annotations_rs.contains("pub(crate) async fn delete_sku_tag_definition("));
}
#[test]
fn order_review_config_handlers_are_split_from_orders_rs() {
    let orders_rs = include_str!("../src/orders.rs");
    let review_config_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/orders/review_config.rs"),
    )
    .expect("review_config module should exist");

    assert!(orders_rs.contains("mod review_config;"));
    assert!(!orders_rs.contains("\nasync fn list_review_rules("));
    assert!(!orders_rs.contains("\nasync fn create_review_rule("));
    assert!(!orders_rs.contains("\nasync fn list_exception_reasons("));
    assert!(!orders_rs.contains("\nasync fn create_customer_blacklist("));
    assert!(review_config_rs.contains("pub(super) async fn list_review_rules("));
    assert!(review_config_rs.contains("pub(super) async fn create_review_rule("));
    assert!(review_config_rs.contains("pub(super) async fn list_exception_reasons("));
    assert!(review_config_rs.contains("pub(super) async fn create_customer_blacklist("));
}
#[test]
fn order_fulfillment_handlers_are_split_from_orders_rs() {
    let orders_rs = include_str!("../src/orders.rs");
    let fulfillment_rs = include_str!("../src/orders/fulfillment.rs");

    assert!(!orders_rs.contains("\nasync fn submit_order_shipment("));
    assert!(!orders_rs.contains("\nasync fn get_order_raw("));
    assert!(!orders_rs.contains("\nasync fn ship_order("));
    assert!(!orders_rs.contains("\nasync fn allocate_oms_order("));
    assert!(!orders_rs.contains("\nasync fn list_oms_shipments("));
    assert!(!orders_rs.contains("\nasync fn handle_oms_resend("));
    assert!(fulfillment_rs.contains("pub(super) async fn submit_order_shipment("));
    assert!(fulfillment_rs.contains("pub(super) async fn get_order_raw("));
    assert!(fulfillment_rs.contains("pub(super) async fn ship_order("));
    assert!(fulfillment_rs.contains("pub(super) async fn allocate_oms_order("));
    assert!(fulfillment_rs.contains("pub(super) async fn list_oms_shipments("));
    assert!(fulfillment_rs.contains("pub(super) async fn handle_oms_resend("));
}
#[test]
fn order_detail_handlers_are_split_from_orders_rs() {
    let orders_rs = include_str!("../src/orders.rs");
    let details_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/orders/details.rs"),
    )
    .expect("details module should exist");

    assert!(orders_rs.contains("mod details;"));
    assert!(!orders_rs.contains("\nasync fn list_orders("));
    assert!(!orders_rs.contains("\nasync fn get_order_detail("));
    assert!(!orders_rs.contains("\nasync fn update_order_receiver("));
    assert!(!orders_rs.contains("\nasync fn edit_order("));
    assert!(details_rs.contains("pub(super) async fn list_orders("));
    assert!(details_rs.contains("pub(super) async fn get_order_detail("));
    assert!(details_rs.contains("pub(super) async fn update_order_receiver("));
    assert!(details_rs.contains("pub(super) async fn edit_order("));
}

#[test]
fn order_review_flow_handlers_are_split_from_orders_rs() {
    let orders_rs = include_str!("../src/orders.rs");
    let review_flow_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/orders/review_flow.rs"),
    )
    .expect("review_flow module should exist");

    assert!(orders_rs.contains("mod review_flow;"));
    assert!(!orders_rs.contains("\nasync fn apply_review_update("));
    assert!(!orders_rs.contains("\nasync fn review_order("));
    assert!(!orders_rs.contains("\nasync fn submit_order("));
    assert!(!orders_rs.contains("\nasync fn rollback_order_to_review("));
    assert!(!orders_rs.contains("\nasync fn export_order_review_hits("));
    assert!(review_flow_rs.contains("pub(super) async fn apply_review_update("));
    assert!(review_flow_rs.contains("pub(super) async fn review_order("));
    assert!(review_flow_rs.contains("pub(super) async fn submit_order("));
    assert!(review_flow_rs.contains("pub(super) async fn rollback_order_to_review("));
    assert!(review_flow_rs.contains("pub(super) async fn export_order_review_hits("));
}
#[test]
fn order_shared_helpers_are_split_from_orders_rs() {
    let orders_rs = include_str!("../src/orders.rs");
    let shared_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/orders/shared.rs"),
    )
    .expect("shared module should exist");

    assert!(orders_rs.contains("mod shared;"));
    assert!(!orders_rs.contains("\nfn review_record_status("));
    assert!(!orders_rs.contains("\nasync fn upsert_order_review_record("));
    assert!(!orders_rs.contains("\nstruct OrderStateLogInput"));
    assert!(!orders_rs.contains("\nfn trim_to_option("));
    assert!(shared_rs.contains("pub(super) fn review_record_status("));
    assert!(shared_rs.contains("pub(super) async fn upsert_order_review_record("));
    assert!(shared_rs.contains("pub(super) struct OrderStateLogInput"));
    assert!(shared_rs.contains("pub(super) fn trim_to_option("));
}

#[test]
fn api_repo_ports_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let repo_ports_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/repo_ports.rs"),
    )
    .expect("repo_ports module should exist");

    assert!(lib_rs.contains("mod repo_ports;"));
    assert!(!lib_rs.contains("\npub trait ProductRepo"));
    assert!(!lib_rs.contains("\npub trait InventoryRepo"));
    assert!(!lib_rs.contains("\nstruct ProductsRepoDb"));
    assert!(!lib_rs.contains("\nstruct PrintJobsRepoDb"));
    assert!(repo_ports_rs.contains("pub trait ProductRepo"));
    assert!(repo_ports_rs.contains("pub trait InventoryRepo"));
    assert!(repo_ports_rs.contains("struct ProductsRepoDb"));
    assert!(repo_ports_rs.contains("struct PrintJobsRepoDb"));
}

#[test]
fn api_auth_tokens_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let auth_tokens_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/auth_tokens.rs"),
    )
    .expect("auth_tokens module should exist");

    assert!(lib_rs.contains("mod auth_tokens;"));
    assert!(!lib_rs.contains("\npub struct Claims"));
    assert!(!lib_rs.contains("\npub struct JwtService"));
    assert!(auth_tokens_rs.contains("pub struct Claims"));
    assert!(auth_tokens_rs.contains("pub struct JwtService"));
}

#[test]
fn api_adapter_registry_is_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let adapter_registry_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/adapter_registry.rs"),
    )
    .expect("adapter_registry module should exist");

    assert!(lib_rs.contains("mod adapter_registry;"));
    assert!(!lib_rs.contains("\nstruct AdapterRegistry"));
    assert!(adapter_registry_rs.contains("pub(crate) struct AdapterRegistry"));
    assert!(adapter_registry_rs.contains("pub(crate) fn platform_adapter("));
}

#[test]
fn api_auth_context_types_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let auth_context_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/auth_context.rs"),
    )
    .expect("auth_context module should exist");

    assert!(lib_rs.contains("mod auth_context;"));
    assert!(!lib_rs.contains("\nstruct AuthContext"));
    assert!(!lib_rs.contains("\nstruct DeviceAuthContext"));
    assert!(!lib_rs.contains("\nstruct RequestContext"));
    assert!(auth_context_rs.contains("pub(crate) struct AuthContext"));
    assert!(auth_context_rs.contains("pub(crate) struct DeviceAuthContext"));
    assert!(auth_context_rs.contains("pub(crate) struct RequestContext"));
}

#[test]
fn api_auth_middlewares_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let auth_layers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/auth_layers.rs"),
    )
    .expect("auth_layers module should exist");

    assert!(lib_rs.contains("mod auth_layers;"));
    assert!(!lib_rs.contains("\nasync fn request_context_middleware("));
    assert!(!lib_rs.contains("\nfn extract_ip("));
    assert!(!lib_rs.contains("\nasync fn auth_middleware("));
    assert!(!lib_rs.contains("\nasync fn device_auth_middleware("));
    assert!(auth_layers_rs.contains("pub(crate) async fn request_context_middleware("));
    assert!(auth_layers_rs.contains("pub(crate) async fn auth_middleware("));
    assert!(auth_layers_rs.contains("pub(crate) async fn device_auth_middleware("));
}

#[test]
fn api_auth_handlers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let auth_handlers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/auth_handlers.rs"),
    )
    .expect("auth_handlers module should exist");

    assert!(lib_rs.contains("mod auth_handlers;"));
    assert!(!lib_rs.contains("\nstruct LoginRequest"));
    assert!(!lib_rs.contains("\nasync fn login("));
    assert!(!lib_rs.contains("\nasync fn get_current_user("));
    assert!(!lib_rs.contains("\nasync fn update_current_user_profile("));
    assert!(!lib_rs.contains("\nasync fn update_current_user_password("));
    assert!(auth_handlers_rs.contains("struct LoginRequest"));
    assert!(auth_handlers_rs.contains("pub(crate) async fn login("));
    assert!(auth_handlers_rs.contains("pub(crate) async fn get_current_user("));
    assert!(auth_handlers_rs.contains("pub(crate) async fn update_current_user_profile("));
    assert!(auth_handlers_rs.contains("pub(crate) async fn update_current_user_password("));
}

#[test]
fn api_settings_payload_helpers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let settings_payloads_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/settings_payloads.rs"),
    )
    .expect("settings_payloads module should exist");

    assert!(lib_rs.contains("mod settings_payloads;"));
    assert!(!lib_rs.contains("\nfn default_wms_labor_tasks("));
    assert!(!lib_rs.contains("\nfn parse_wms_labor_tasks_value("));
    assert!(!lib_rs.contains("\nasync fn fetch_tenant_setting_value("));
    assert!(!lib_rs.contains("\nasync fn load_wms_inbound_settings("));
    assert!(!lib_rs.contains("\nasync fn load_system_workflow_catalog("));
    assert!(!lib_rs.contains("\nfn parse_bool_setting("));
    assert!(settings_payloads_rs.contains("pub(crate) fn default_wms_labor_tasks("));
    assert!(settings_payloads_rs.contains("pub(crate) async fn fetch_tenant_setting_value("));
    assert!(settings_payloads_rs.contains("pub(crate) async fn load_wms_inbound_settings("));
    assert!(settings_payloads_rs.contains("pub(crate) async fn load_system_workflow_catalog("));
    assert!(settings_payloads_rs.contains("pub(crate) fn parse_bool_setting("));
}

#[test]
fn api_procurement_views_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let procurement_views_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/procurement_views.rs"),
    )
    .expect("procurement_views module should exist");

    assert!(lib_rs.contains("mod procurement_views;"));
    assert!(!lib_rs.contains("\nfn purchase_order_summary_response("));
    assert!(!lib_rs.contains("\nfn procurement_plan_response("));
    assert!(!lib_rs.contains("\nfn supplier_performance_response("));
    assert!(!lib_rs.contains("\nfn normalize_purchase_order_status("));
    assert!(procurement_views_rs.contains("pub(crate) fn purchase_order_summary_response("));
    assert!(procurement_views_rs.contains("pub(crate) fn procurement_plan_response("));
    assert!(procurement_views_rs.contains("pub(crate) fn supplier_performance_response("));
    assert!(procurement_views_rs.contains("pub(crate) fn normalize_purchase_order_status("));
}

#[test]
fn api_export_job_views_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let export_job_views_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/export_job_views.rs"),
    )
    .expect("export_job_views module should exist");

    assert!(lib_rs.contains("mod export_job_views;"));
    assert!(!lib_rs.contains("\nfn export_job_response("));
    assert!(!lib_rs.contains("\nfn normalize_duplicate_strategy("));
    assert!(!lib_rs.contains("\nfn build_sku_import_job_response("));
    assert!(export_job_views_rs.contains("pub(crate) fn export_job_response("));
    assert!(export_job_views_rs.contains("pub(crate) fn normalize_duplicate_strategy("));
    assert!(export_job_views_rs.contains("pub(crate) fn build_sku_import_job_response("));
}

#[test]
fn api_stats_helpers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let stats_helpers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/stats_helpers.rs"),
    )
    .expect("stats_helpers module should exist");

    assert!(lib_rs.contains("mod stats_helpers;"));
    assert!(!lib_rs.contains("\nstruct StatsMetricBase"));
    assert!(!lib_rs.contains("\nfn parse_stats_metrics_csv("));
    assert!(!lib_rs.contains("\nasync fn compute_stats_metrics("));
    assert!(!lib_rs.contains("\nasync fn compute_stats_breakdown("));
    assert!(!lib_rs.contains("\nfn parse_report_range("));
    assert!(stats_helpers_rs.contains("pub(crate) struct StatsMetricBase"));
    assert!(stats_helpers_rs.contains("pub(crate) fn parse_stats_metrics_csv("));
    assert!(stats_helpers_rs.contains("pub(crate) async fn compute_stats_metrics("));
    assert!(stats_helpers_rs.contains("pub(crate) async fn compute_stats_breakdown("));
    assert!(stats_helpers_rs.contains("pub(crate) fn parse_report_range("));
}

#[test]
fn api_file_path_helpers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let file_paths_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/file_paths.rs"),
    )
    .expect("file_paths module should exist");

    assert!(lib_rs.contains("mod file_paths;"));
    assert!(!lib_rs.contains("\nfn sku_import_base_dir("));
    assert!(!lib_rs.contains("\nfn sku_import_file_path("));
    assert!(!lib_rs.contains("\nfn sku_import_template_path("));
    assert!(!lib_rs.contains("\nfn export_job_file_path("));
    assert!(!lib_rs.contains("\nfn after_sales_import_file_path("));
    assert!(file_paths_rs.contains("pub(crate) fn sku_import_base_dir("));
    assert!(file_paths_rs.contains("pub(crate) fn sku_import_file_path("));
    assert!(file_paths_rs.contains("pub(crate) fn sku_import_template_path("));
    assert!(file_paths_rs.contains("pub(crate) fn export_job_file_path("));
    assert!(file_paths_rs.contains("pub(crate) fn after_sales_import_file_path("));
}

#[test]
fn api_inventory_error_mapper_is_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let inventory_views_rs = include_str!("../src/inventory_views.rs");

    assert!(!lib_rs.contains("\nfn inventory_error_response("));
    assert!(inventory_views_rs.contains("pub(crate) fn inventory_error_response("));
    assert!(inventory_views_rs.contains("InventoryError::InsufficientAvailable"));
    assert!(inventory_views_rs.contains("inventory_insufficient"));
}

#[test]
fn api_shipment_serial_helpers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let shipment_serials_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shipment_serials.rs"),
    )
    .expect("shipment_serials module should exist");

    assert!(lib_rs.contains("mod shipment_serials;"));
    assert!(!lib_rs.contains("\nasync fn bind_serials_for_shipment("));
    assert!(!lib_rs.contains("\nasync fn resolve_serial_order_item_id("));
    assert!(!lib_rs.contains("\nfn order_item_uuid("));
    assert!(!lib_rs.contains("\nfn component_order_item_id("));
    assert!(shipment_serials_rs.contains("pub(super) async fn bind_serials_for_shipment("));
    assert!(shipment_serials_rs.contains("pub(super) async fn resolve_serial_order_item_id("));
    assert!(shipment_serials_rs.contains("pub(super) fn order_item_uuid("));
    assert!(shipment_serials_rs.contains("pub(super) fn component_order_item_id("));
}

#[test]
fn api_sales_order_helpers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let sales_order_helpers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/sales_order_helpers.rs"),
    )
    .expect("sales_order_helpers module should exist");

    assert!(lib_rs.contains("mod sales_order_helpers;"));
    assert!(!lib_rs.contains("\nfn derive_delivery_status("));
    assert!(!lib_rs.contains("\nfn format_sales_order_status("));
    assert!(!lib_rs.contains("\nfn append_status_flags("));
    assert!(!lib_rs.contains("\nasync fn record_sales_order_state_logs("));
    assert!(sales_order_helpers_rs.contains("pub(crate) fn derive_delivery_status("));
    assert!(sales_order_helpers_rs.contains("pub(crate) fn format_sales_order_status("));
    assert!(sales_order_helpers_rs.contains("pub(crate) fn append_status_flags("));
    assert!(sales_order_helpers_rs.contains("pub(crate) async fn record_sales_order_state_logs("));
}

#[test]
fn api_sales_config_helpers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let sales_config_helpers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/sales_config_helpers.rs"),
    )
    .expect("sales_config_helpers module should exist");

    assert!(lib_rs.contains("mod sales_config_helpers;"));
    assert!(!lib_rs.contains("\nasync fn save_sales_shop_config("));
    assert!(!lib_rs.contains("\nasync fn load_sales_config_for_shop("));
    assert!(!lib_rs.contains("\nasync fn load_sales_configs_for_shops("));
    assert!(sales_config_helpers_rs.contains("pub(crate) async fn save_sales_shop_config("));
    assert!(sales_config_helpers_rs.contains("pub(crate) async fn load_sales_config_for_shop("));
    assert!(sales_config_helpers_rs.contains("pub(crate) async fn load_sales_configs_for_shops("));
}

#[test]
fn api_audit_helpers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let audit_helpers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/audit_helpers.rs"),
    )
    .expect("audit_helpers module should exist");

    assert!(lib_rs.contains("mod audit_helpers;"));
    assert!(!lib_rs.contains("\nasync fn record_audit_log("));
    assert!(!lib_rs.contains("\nfn token_audit_payload("));
    assert!(audit_helpers_rs.contains("pub(crate) async fn record_audit_log("));
    assert!(audit_helpers_rs.contains("pub(crate) fn token_audit_payload("));
}

#[test]
fn api_shop_runtime_helpers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let shop_runtime_helpers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shop_runtime_helpers.rs"),
    )
    .expect("shop_runtime_helpers module should exist");

    assert!(lib_rs.contains("mod shop_runtime_helpers;"));
    assert!(!lib_rs.contains("\nfn shop_token_status("));
    assert!(!lib_rs.contains("\nasync fn load_shop_credentials_for_shops("));
    assert!(!lib_rs.contains("\nasync fn load_shop_webhook_health_for_shops("));
    assert!(shop_runtime_helpers_rs.contains("pub(crate) fn shop_token_status("));
    assert!(
        shop_runtime_helpers_rs.contains("pub(crate) async fn load_shop_credentials_for_shops(")
    );
    assert!(
        shop_runtime_helpers_rs.contains("pub(crate) async fn load_shop_webhook_health_for_shops(")
    );
}

#[test]
fn api_order_sku_mapping_helpers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let order_sku_mapping_helpers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/order_sku_mapping_helpers.rs"),
    )
    .expect("order_sku_mapping_helpers module should exist");

    assert!(lib_rs.contains("mod order_sku_mapping_helpers;"));
    assert!(!lib_rs.contains("\nasync fn apply_order_item_sku_mappings("));
    assert!(!lib_rs.contains("\nasync fn resolve_order_item_sku_mapping("));
    assert!(
        order_sku_mapping_helpers_rs.contains("pub(crate) async fn apply_order_item_sku_mappings(")
    );
    assert!(order_sku_mapping_helpers_rs
        .contains("pub(crate) async fn resolve_order_item_sku_mapping("));
}

#[test]
fn api_wms_inbound_view_helpers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let wms_inbound_views_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/wms_inbound_views.rs"),
    )
    .expect("wms_inbound_views module should exist");

    assert!(lib_rs.contains("mod wms_inbound_views;"));
    assert!(!lib_rs.contains("\nfn inbound_doc_summary_response("));
    assert!(!lib_rs.contains("\nfn inbound_doc_detail_response("));
    assert!(!lib_rs.contains("\nfn inbound_reject_response("));
    assert!(wms_inbound_views_rs.contains("pub(crate) fn inbound_doc_summary_response("));
    assert!(wms_inbound_views_rs.contains("pub(crate) fn inbound_doc_detail_response("));
    assert!(wms_inbound_views_rs.contains("pub(crate) fn inbound_reject_response("));
}

#[test]
fn api_wms_normalizers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let wms_normalizers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/wms_normalizers.rs"),
    )
    .expect("wms_normalizers module should exist");

    assert!(lib_rs.contains("mod wms_normalizers;"));
    assert!(!lib_rs.contains("\nfn normalize_inbound_doc_type("));
    assert!(!lib_rs.contains("\nfn normalize_inbound_status("));
    assert!(!lib_rs.contains("\nfn normalize_warehouse_area_type("));
    assert!(!lib_rs.contains("\nfn normalize_stock_count_strategy("));
    assert!(!lib_rs.contains("\nfn normalize_stock_count_type("));
    assert!(wms_normalizers_rs.contains("pub(crate) fn normalize_inbound_doc_type("));
    assert!(wms_normalizers_rs.contains("pub(crate) fn normalize_inbound_status("));
    assert!(wms_normalizers_rs.contains("pub(crate) fn normalize_warehouse_area_type("));
    assert!(wms_normalizers_rs.contains("pub(crate) fn normalize_stock_count_strategy("));
    assert!(wms_normalizers_rs.contains("pub(crate) fn normalize_stock_count_type("));
}

#[test]
fn api_wms_config_helpers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let wms_config_helpers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/wms_config_helpers.rs"),
    )
    .expect("wms_config_helpers module should exist");

    assert!(lib_rs.contains("mod wms_config_helpers;"));
    assert!(!lib_rs.contains("\nconst WMS_WORKFLOW_CONFIG_KEY:"));
    assert!(!lib_rs.contains("\nconst SYSTEM_WORKFLOW_CATALOG_KEY:"));
    assert!(!lib_rs.contains("\nfn default_warehouse_workflow_config("));
    assert!(!lib_rs.contains("\nfn parse_workflow_config_value("));
    assert!(wms_config_helpers_rs.contains("pub(crate) const WMS_WORKFLOW_CONFIG_KEY:"));
    assert!(wms_config_helpers_rs.contains("pub(crate) const SYSTEM_WORKFLOW_CATALOG_KEY:"));
    assert!(wms_config_helpers_rs.contains("pub(crate) fn default_warehouse_workflow_config("));
    assert!(wms_config_helpers_rs.contains("pub(crate) fn parse_workflow_config_value("));
}

#[test]
fn api_shop_product_download_helpers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let shop_product_download_helpers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/shop_product_download_helpers.rs"),
    )
    .expect("shop_product_download_helpers module should exist");

    assert!(lib_rs.contains("mod shop_product_download_helpers;"));
    assert!(!lib_rs.contains("\nfn build_shop_product_download_items_from_adapter("));
    assert!(!lib_rs.contains("\nfn extract_spec_text_from_platform_spec("));
    assert!(shop_product_download_helpers_rs
        .contains("pub(crate) fn build_shop_product_download_items_from_adapter("));
    assert!(shop_product_download_helpers_rs
        .contains("pub(crate) fn extract_spec_text_from_platform_spec("));
}

#[test]
fn api_sku_import_file_helpers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let sku_import_file_helpers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/sku_import_file_helpers.rs"),
    )
    .expect("sku_import_file_helpers module should exist");

    assert!(lib_rs.contains("mod sku_import_file_helpers;"));
    assert!(!lib_rs.contains("\nfn resolve_import_file_type("));
    assert!(!lib_rs.contains("\nfn decode_base64_payload("));
    assert!(!lib_rs.contains("\nfn write_sku_import_file("));
    assert!(sku_import_file_helpers_rs.contains("pub(crate) fn resolve_import_file_type("));
    assert!(sku_import_file_helpers_rs.contains("pub(crate) fn decode_base64_payload("));
    assert!(sku_import_file_helpers_rs.contains("pub(crate) fn write_sku_import_file("));
}

#[test]
fn api_sku_import_parser_helpers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let sku_import_parser_helpers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/sku_import_parser_helpers.rs"),
    )
    .expect("sku_import_parser_helpers module should exist");

    assert!(lib_rs.contains("mod sku_import_parser_helpers;"));
    assert!(!lib_rs.contains("\nfn parse_sku_import_rows("));
    assert!(!lib_rs.contains("\nfn parse_sku_import_csv("));
    assert!(!lib_rs.contains("\nfn parse_sku_import_excel("));
    assert!(!lib_rs.contains("\nfn build_sku_import_header_index("));
    assert!(!lib_rs.contains("\nfn parse_optional_i64_field("));
    assert!(sku_import_parser_helpers_rs.contains("pub(crate) fn parse_sku_import_rows("));
    assert!(sku_import_parser_helpers_rs.contains("pub(crate) fn parse_sku_import_csv("));
    assert!(sku_import_parser_helpers_rs.contains("pub(crate) fn parse_sku_import_excel("));
    assert!(sku_import_parser_helpers_rs.contains("pub(crate) fn build_sku_import_header_index("));
    assert!(sku_import_parser_helpers_rs.contains("pub(crate) fn parse_optional_i64_field("));
}

#[test]
fn api_sku_import_lookup_helpers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let sku_import_lookup_helpers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/sku_import_lookup_helpers.rs"),
    )
    .expect("sku_import_lookup_helpers module should exist");

    assert!(lib_rs.contains("mod sku_import_lookup_helpers;"));
    assert!(!lib_rs.contains("\nasync fn fetch_existing_sku_codes("));
    assert!(!lib_rs.contains("\nasync fn fetch_existing_product_ids("));
    assert!(!lib_rs.contains("\nasync fn fetch_product_categories_by_id("));
    assert!(!lib_rs.contains("\nasync fn fetch_existing_product_categories("));
    assert!(!lib_rs.contains("\nasync fn fetch_existing_supplier_ids("));
    assert!(sku_import_lookup_helpers_rs.contains("pub(crate) async fn fetch_existing_sku_codes("));
    assert!(
        sku_import_lookup_helpers_rs.contains("pub(crate) async fn fetch_existing_product_ids(")
    );
    assert!(sku_import_lookup_helpers_rs
        .contains("pub(crate) async fn fetch_product_categories_by_id("));
    assert!(sku_import_lookup_helpers_rs
        .contains("pub(crate) async fn fetch_existing_product_categories("));
    assert!(
        sku_import_lookup_helpers_rs.contains("pub(crate) async fn fetch_existing_supplier_ids(")
    );
}

#[test]
fn api_common_parse_helpers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let common_parse_helpers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/common_parse_helpers.rs"),
    )
    .expect("common_parse_helpers module should exist");

    assert!(lib_rs.contains("mod common_parse_helpers;"));
    assert!(!lib_rs.contains("\nfn parse_role_ids("));
    assert!(!lib_rs.contains("\nfn parse_shop_ids("));
    assert!(!lib_rs.contains("\nfn parse_uuid_list("));
    assert!(!lib_rs.contains("\nfn parse_platform_optional("));
    assert!(!lib_rs.contains("\nfn normalize_optional_string("));
    assert!(!lib_rs.contains("\nfn parse_datetime_optional("));
    assert!(!lib_rs.contains("\nfn dedup_uuid_list("));
    assert!(common_parse_helpers_rs.contains("pub(crate) fn parse_role_ids("));
    assert!(common_parse_helpers_rs.contains("pub(crate) fn parse_shop_ids("));
    assert!(common_parse_helpers_rs.contains("pub(crate) fn parse_uuid_list("));
    assert!(common_parse_helpers_rs.contains("pub(crate) fn parse_platform_optional("));
    assert!(common_parse_helpers_rs.contains("pub(crate) fn normalize_optional_string("));
    assert!(common_parse_helpers_rs.contains("pub(crate) fn parse_datetime_optional("));
    assert!(common_parse_helpers_rs.contains("pub(crate) fn dedup_uuid_list("));
}

#[test]
fn api_sku_auto_match_helpers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let sku_auto_match_helpers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/sku_auto_match_helpers.rs"),
    )
    .expect("sku_auto_match_helpers module should exist");

    assert!(lib_rs.contains("mod sku_auto_match_helpers;"));
    assert!(!lib_rs.contains("\nstruct AutoMatchSummary"));
    assert!(!lib_rs.contains("\nstruct MatchedSku"));
    assert!(!lib_rs.contains("\nasync fn auto_match_sku_views("));
    assert!(!lib_rs.contains("\nasync fn match_code("));
    assert!(!lib_rs.contains("\nasync fn find_sku_by_barcode("));
    assert!(!lib_rs.contains("\nasync fn load_shop_product_skus("));
    assert!(!lib_rs.contains("\nasync fn resolve_unique_sku_code("));
    assert!(sku_auto_match_helpers_rs.contains("pub(crate) struct AutoMatchSummary"));
    assert!(sku_auto_match_helpers_rs.contains("pub(crate) struct MatchedSku"));
    assert!(sku_auto_match_helpers_rs.contains("pub(crate) async fn auto_match_sku_views("));
    assert!(sku_auto_match_helpers_rs.contains("pub(crate) async fn match_code("));
    assert!(sku_auto_match_helpers_rs.contains("pub(crate) async fn find_sku_by_barcode("));
    assert!(sku_auto_match_helpers_rs.contains("pub(crate) async fn load_shop_product_skus("));
    assert!(sku_auto_match_helpers_rs.contains("pub(crate) async fn resolve_unique_sku_code("));
}

#[test]
fn api_admin_shortcut_handlers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let admin_shortcut_handlers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/admin_shortcut_handlers.rs"),
    )
    .expect("admin_shortcut_handlers module should exist");

    assert!(lib_rs.contains("mod admin_shortcut_handlers;"));
    assert!(!lib_rs.contains("\nasync fn admin_list_permission_catalog("));
    assert!(!lib_rs.contains("\nasync fn admin_list_roles("));
    assert!(!lib_rs.contains("\nasync fn admin_create_role("));
    assert!(!lib_rs.contains("\nasync fn admin_list_users("));
    assert!(!lib_rs.contains("\nasync fn admin_replay_dead_letter("));
    assert!(
        admin_shortcut_handlers_rs.contains("pub(crate) async fn admin_list_permission_catalog(")
    );
    assert!(admin_shortcut_handlers_rs.contains("pub(crate) async fn admin_list_roles("));
    assert!(admin_shortcut_handlers_rs.contains("pub(crate) async fn admin_create_role("));
    assert!(admin_shortcut_handlers_rs.contains("pub(crate) async fn admin_list_users("));
    assert!(admin_shortcut_handlers_rs.contains("pub(crate) async fn admin_replay_dead_letter("));
}

#[test]
fn api_system_observability_handlers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let system_observability_handlers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/system_observability_handlers.rs"),
    )
    .expect("system_observability_handlers module should exist");

    assert!(lib_rs.contains("mod system_observability_handlers;"));
    assert!(!lib_rs.contains("\nasync fn list_audit_logs("));
    assert!(!lib_rs.contains("\nasync fn list_shipment_push_attempts("));
    assert!(!lib_rs.contains("\nasync fn list_dead_letters("));
    assert!(!lib_rs.contains("\nasync fn replay_dead_letter("));
    assert!(!lib_rs.contains("\nasync fn list_outbox_events("));
    assert!(!lib_rs.contains("\nasync fn requeue_outbox_event("));
    assert!(system_observability_handlers_rs.contains("pub(crate) async fn list_audit_logs("));
    assert!(system_observability_handlers_rs
        .contains("pub(crate) async fn list_shipment_push_attempts("));
    assert!(system_observability_handlers_rs.contains("pub(crate) async fn list_dead_letters("));
    assert!(system_observability_handlers_rs.contains("pub(crate) async fn replay_dead_letter("));
    assert!(system_observability_handlers_rs.contains("pub(crate) async fn list_outbox_events("));
    assert!(system_observability_handlers_rs.contains("pub(crate) async fn requeue_outbox_event("));
}

#[test]
fn api_system_catalog_handlers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let system_catalog_handlers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/system_catalog_handlers.rs"),
    )
    .expect("system_catalog_handlers module should exist");

    assert!(lib_rs.contains("mod system_catalog_handlers;"));
    assert!(!lib_rs.contains("\nasync fn get_system_workflow_catalog("));
    assert!(!lib_rs.contains("\nasync fn update_system_workflow_catalog("));
    assert!(!lib_rs.contains("\nasync fn get_system_custom_fields_catalog("));
    assert!(!lib_rs.contains("\nasync fn update_system_custom_fields_catalog("));
    assert!(!lib_rs.contains("\nasync fn get_system_integrations_catalog("));
    assert!(!lib_rs.contains("\nasync fn update_system_integrations_catalog("));
    assert!(!lib_rs.contains("\nasync fn get_system_audit_reports_catalog("));
    assert!(!lib_rs.contains("\nasync fn update_system_audit_reports_catalog("));
    assert!(system_catalog_handlers_rs.contains("pub(crate) async fn get_system_workflow_catalog("));
    assert!(
        system_catalog_handlers_rs.contains("pub(crate) async fn update_system_workflow_catalog(")
    );
    assert!(system_catalog_handlers_rs
        .contains("pub(crate) async fn get_system_custom_fields_catalog("));
    assert!(system_catalog_handlers_rs
        .contains("pub(crate) async fn update_system_custom_fields_catalog("));
    assert!(
        system_catalog_handlers_rs.contains("pub(crate) async fn get_system_integrations_catalog(")
    );
    assert!(system_catalog_handlers_rs
        .contains("pub(crate) async fn update_system_integrations_catalog("));
    assert!(system_catalog_handlers_rs
        .contains("pub(crate) async fn get_system_audit_reports_catalog("));
    assert!(system_catalog_handlers_rs
        .contains("pub(crate) async fn update_system_audit_reports_catalog("));
}

#[test]
fn api_system_settings_handlers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let system_settings_handlers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/system_settings_handlers.rs"),
    )
    .expect("system_settings_handlers module should exist");

    assert!(lib_rs.contains("mod system_settings_handlers;"));
    assert!(!lib_rs.contains("\nasync fn list_settings("));
    assert!(!lib_rs.contains("\nasync fn upsert_setting("));
    assert!(system_settings_handlers_rs.contains("pub(crate) async fn list_settings("));
    assert!(system_settings_handlers_rs.contains("pub(crate) async fn upsert_setting("));
}

#[test]
fn api_rbac_role_handlers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let rbac_role_handlers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/rbac_role_handlers.rs"),
    )
    .expect("rbac_role_handlers module should exist");

    assert!(lib_rs.contains("mod rbac_role_handlers;"));
    assert!(!lib_rs.contains("\nasync fn list_roles("));
    assert!(!lib_rs.contains("\nasync fn create_role("));
    assert!(!lib_rs.contains("\nasync fn update_role("));
    assert!(!lib_rs.contains("\nasync fn list_role_permissions("));
    assert!(!lib_rs.contains("\nasync fn update_role_permissions("));
    assert!(rbac_role_handlers_rs.contains("pub(crate) async fn list_roles("));
    assert!(rbac_role_handlers_rs.contains("pub(crate) async fn create_role("));
    assert!(rbac_role_handlers_rs.contains("pub(crate) async fn update_role("));
    assert!(rbac_role_handlers_rs.contains("pub(crate) async fn list_role_permissions("));
    assert!(rbac_role_handlers_rs.contains("pub(crate) async fn update_role_permissions("));
}

#[test]
fn api_rbac_user_handlers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let rbac_user_handlers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/rbac_user_handlers.rs"),
    )
    .expect("rbac_user_handlers module should exist");

    assert!(lib_rs.contains("mod rbac_user_handlers;"));
    assert!(!lib_rs.contains("\nstruct UserListQuery"));
    assert!(!lib_rs.contains("\nasync fn list_users("));
    assert!(!lib_rs.contains("\nasync fn create_user("));
    assert!(!lib_rs.contains("\nasync fn update_user("));
    assert!(!lib_rs.contains("\nasync fn get_user_roles("));
    assert!(!lib_rs.contains("\nasync fn update_user_roles("));
    assert!(!lib_rs.contains("\nasync fn get_user_data_scopes("));
    assert!(!lib_rs.contains("\nasync fn update_user_data_scopes("));
    assert!(rbac_user_handlers_rs.contains("pub(crate) struct UserListQuery"));
    assert!(rbac_user_handlers_rs.contains("pub(crate) async fn list_users("));
    assert!(rbac_user_handlers_rs.contains("pub(crate) async fn create_user("));
    assert!(rbac_user_handlers_rs.contains("pub(crate) async fn update_user("));
    assert!(rbac_user_handlers_rs.contains("pub(crate) async fn get_user_roles("));
    assert!(rbac_user_handlers_rs.contains("pub(crate) async fn update_user_roles("));
    assert!(rbac_user_handlers_rs.contains("pub(crate) async fn get_user_data_scopes("));
    assert!(rbac_user_handlers_rs.contains("pub(crate) async fn update_user_data_scopes("));
}

#[test]
fn api_permission_catalog_helpers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let permission_catalog_helpers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/permission_catalog_helpers.rs"),
    )
    .expect("permission_catalog_helpers module should exist");

    assert!(lib_rs.contains("mod permission_catalog_helpers;"));
    assert!(!lib_rs.contains("\nfn permission_catalog("));
    assert!(!lib_rs.contains("\nfn describe_permission("));
    assert!(!lib_rs.contains("\nfn permission_module_label("));
    assert!(!lib_rs.contains("\nfn permission_resource_label("));
    assert!(!lib_rs.contains("\nfn permission_action_name("));
    assert!(!lib_rs.contains("\nfn permission_action_description("));
    assert!(permission_catalog_helpers_rs.contains("pub(crate) fn permission_catalog("));
    assert!(permission_catalog_helpers_rs.contains("pub(crate) fn describe_permission("));
    assert!(permission_catalog_helpers_rs.contains("pub(crate) fn permission_module_label("));
    assert!(permission_catalog_helpers_rs.contains("pub(crate) fn permission_resource_label("));
    assert!(permission_catalog_helpers_rs.contains("pub(crate) fn permission_action_name("));
    assert!(permission_catalog_helpers_rs.contains("pub(crate) fn permission_action_description("));
}

#[test]
fn api_permission_catalog_handlers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let permission_catalog_handlers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/permission_catalog_handlers.rs"),
    )
    .expect("permission_catalog_handlers module should exist");

    assert!(lib_rs.contains("mod permission_catalog_handlers;"));
    assert!(!lib_rs.contains("\nasync fn list_permission_catalog("));
    assert!(permission_catalog_handlers_rs.contains("pub(crate) async fn list_permission_catalog("));
}

#[test]
fn api_permission_normalization_helpers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let permission_normalization_helpers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/permission_normalization_helpers.rs"),
    )
    .expect("permission_normalization_helpers module should exist");

    assert!(lib_rs.contains("mod permission_normalization_helpers;"));
    assert!(!lib_rs.contains("\nfn normalize_permission_code("));
    assert!(!lib_rs.contains("\nfn normalize_permissions("));
    assert!(
        permission_normalization_helpers_rs.contains("pub(crate) fn normalize_permission_code(")
    );
    assert!(permission_normalization_helpers_rs.contains("pub(crate) fn normalize_permissions("));
}

#[test]
fn api_wms_operation_handlers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let wms_operation_handlers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/wms_operation_handlers.rs"),
    )
    .expect("wms_operation_handlers module should exist");

    assert!(lib_rs.contains("mod wms_operation_handlers;"));
    assert!(wms_operation_handlers_rs.contains("mod outbound_core;"));
    assert!(wms_operation_handlers_rs.contains("pub(crate) use fulfillment::*;"));
    assert!(wms_operation_handlers_rs.contains("pub(crate) use views::*;"));
    assert!(!lib_rs.contains("\nasync fn list_wms_outbounds("));
    assert!(!lib_rs.contains("\nasync fn create_wms_outbound("));
    assert!(!lib_rs.contains("\nasync fn run_wms_wave("));
    assert!(!lib_rs.contains("\nasync fn confirm_wms_pick_task("));
    assert!(!lib_rs.contains("\nasync fn ship_wms_outbound("));
    assert!(!lib_rs.contains("\nfn wms_outbound_doc_response("));
    let wms_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/wms_operation_handlers");
    let outbound_core_rs = std::fs::read_to_string(wms_dir.join("outbound_core.rs"))
        .expect("wms outbound_core module should exist");
    let wave_rs =
        std::fs::read_to_string(wms_dir.join("wave.rs")).expect("wms wave module should exist");
    let fulfillment_rs = std::fs::read_to_string(wms_dir.join("fulfillment.rs"))
        .expect("wms fulfillment module should exist");
    let views_rs =
        std::fs::read_to_string(wms_dir.join("views.rs")).expect("wms views module should exist");

    assert!(outbound_core_rs.contains("pub(crate) async fn list_wms_outbounds("));
    assert!(outbound_core_rs.contains("pub(crate) async fn create_wms_outbound("));
    assert!(wave_rs.contains("pub(crate) async fn run_wms_wave("));
    assert!(wave_rs.contains("pub(crate) async fn confirm_wms_pick_task("));
    assert!(fulfillment_rs.contains("pub(crate) async fn ship_wms_outbound("));
    assert!(views_rs.contains("pub(crate) fn wms_outbound_doc_response("));
}

#[test]
fn api_product_handlers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let product_handlers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/product_handlers.rs"),
    )
    .expect("product_handlers module should exist");

    assert!(lib_rs.contains("mod product_handlers;"));
    assert!(!lib_rs.contains("\nasync fn list_products("));
    assert!(!lib_rs.contains("\nasync fn create_product("));
    assert!(!lib_rs.contains("\nasync fn list_product_categories("));
    assert!(!lib_rs.contains("\nasync fn list_product_channel_availability("));
    assert!(!lib_rs.contains("\nasync fn delete_product_lifecycle("));
    assert!(product_handlers_rs.contains("pub(crate) async fn list_products("));
    assert!(product_handlers_rs.contains("pub(crate) async fn create_product("));
    assert!(product_handlers_rs.contains("pub(crate) async fn list_product_categories("));
    assert!(product_handlers_rs.contains("pub(crate) async fn list_product_channel_availability("));
    assert!(product_handlers_rs.contains("pub(crate) async fn delete_product_lifecycle("));
}

#[test]
fn api_sku_handlers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let sku_handlers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/sku_handlers.rs"),
    )
    .expect("sku_handlers module should exist");

    assert!(lib_rs.contains("mod sku_handlers;"));
    assert!(!lib_rs.contains("\nasync fn list_skus("));
    assert!(!lib_rs.contains("\nasync fn create_sku("));
    assert!(!lib_rs.contains("\nasync fn precheck_sku_import("));
    assert!(!lib_rs.contains("\nasync fn save_kit_bom("));
    assert!(!lib_rs.contains("\nasync fn print_sku_serials("));
    assert!(sku_handlers_rs.contains("pub(crate) async fn list_skus("));
    assert!(sku_handlers_rs.contains("pub(crate) async fn create_sku("));
    assert!(sku_handlers_rs.contains("pub(crate) async fn precheck_sku_import("));
    assert!(sku_handlers_rs.contains("pub(crate) async fn save_kit_bom("));
    assert!(sku_handlers_rs.contains("pub(crate) async fn print_sku_serials("));
}

#[test]
fn api_mapping_handlers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let mapping_handlers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/mapping_handlers.rs"),
    )
    .expect("mapping_handlers module should exist");

    assert!(lib_rs.contains("mod mapping_handlers;"));
    assert!(!lib_rs.contains("\nasync fn list_channel_sku_mappings("));
    assert!(!lib_rs.contains("\nasync fn list_mappings("));
    assert!(!lib_rs.contains("\nasync fn download_shop_products("));
    assert!(!lib_rs.contains("\nasync fn manual_match_shop_products("));
    assert!(!lib_rs.contains("\nasync fn update_mapping_rule("));
    assert!(mapping_handlers_rs.contains("pub(crate) async fn list_channel_sku_mappings("));
    assert!(mapping_handlers_rs.contains("pub(crate) async fn list_mappings("));
    assert!(mapping_handlers_rs.contains("pub(crate) async fn download_shop_products("));
    assert!(mapping_handlers_rs.contains("pub(crate) async fn manual_match_shop_products("));
    assert!(mapping_handlers_rs.contains("pub(crate) async fn update_mapping_rule("));
}

#[test]
fn api_logistics_handlers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let logistics_handlers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/logistics_handlers.rs"),
    )
    .expect("logistics_handlers module should exist");

    assert!(lib_rs.contains("mod logistics_handlers;"));
    assert!(!lib_rs.contains("\nasync fn callback_shipment("));
    assert!(!lib_rs.contains("\nasync fn list_logistics_match_rules("));
    assert!(!lib_rs.contains("\nasync fn request_shipment("));
    assert!(!lib_rs.contains("\nasync fn import_waybills("));
    assert!(logistics_handlers_rs.contains("pub(crate) async fn callback_shipment("));
    assert!(logistics_handlers_rs.contains("pub(crate) async fn list_logistics_match_rules("));
    assert!(logistics_handlers_rs.contains("pub(crate) async fn request_shipment("));
    assert!(logistics_handlers_rs.contains("pub(crate) async fn import_waybills("));
}

#[test]
fn api_finance_handlers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let finance_handlers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/finance_handlers.rs"),
    )
    .expect("finance_handlers module should exist");

    assert!(lib_rs.contains("mod finance_handlers;"));
    assert!(!lib_rs.contains("\nasync fn import_statements("));
    assert!(!lib_rs.contains("\nasync fn run_reconcile("));
    assert!(!lib_rs.contains("\nasync fn list_ap_docs("));
    assert!(!lib_rs.contains("\nasync fn get_profit_snapshot_detail("));
    assert!(finance_handlers_rs.contains("pub(crate) async fn import_statements("));
    assert!(finance_handlers_rs.contains("pub(crate) async fn run_reconcile("));
    assert!(finance_handlers_rs.contains("pub(crate) async fn list_ap_docs("));
    assert!(finance_handlers_rs.contains("pub(crate) async fn get_profit_snapshot_detail("));
}

#[test]
fn api_after_sales_handlers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let after_sales_handlers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/after_sales_handlers.rs"),
    )
    .expect("after_sales_handlers module should exist");

    assert!(lib_rs.contains("mod after_sales_handlers;"));
    assert!(!lib_rs.contains("\nasync fn list_after_sale_cases("));
    assert!(!lib_rs.contains("\nasync fn create_after_sales("));
    assert!(!lib_rs.contains("\nasync fn create_after_sales_exchange_shipment("));
    assert!(!lib_rs.contains("\nfn after_sales_view("));
    assert!(after_sales_handlers_rs.contains("pub(crate) async fn list_after_sale_cases("));
    assert!(after_sales_handlers_rs.contains("pub(crate) async fn create_after_sales("));
    assert!(after_sales_handlers_rs
        .contains("pub(crate) async fn create_after_sales_exchange_shipment("));
    assert!(after_sales_handlers_rs.contains("pub(crate) fn after_sales_view("));
}

#[test]
fn api_warehouse_master_handlers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let warehouse_master_handlers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/warehouse_master_handlers.rs"),
    )
    .expect("warehouse_master_handlers module should exist");

    assert!(lib_rs.contains("mod warehouse_master_handlers;"));
    assert!(!lib_rs.contains("\nasync fn list_warehouses("));
    assert!(!lib_rs.contains("\nasync fn list_warehouse_locations("));
    assert!(!lib_rs.contains("\nasync fn list_sender_addresses("));
    assert!(!lib_rs.contains("\nasync fn list_channels("));
    assert!(warehouse_master_handlers_rs.contains("pub(crate) async fn list_warehouses("));
    assert!(warehouse_master_handlers_rs.contains("pub(crate) async fn list_warehouse_locations("));
    assert!(warehouse_master_handlers_rs.contains("pub(crate) async fn list_sender_addresses("));
    assert!(warehouse_master_handlers_rs.contains("pub(crate) async fn list_channels("));
}

#[test]
fn api_shop_sync_handlers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let shop_sync_handlers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shop_sync_handlers.rs"),
    )
    .expect("shop_sync_handlers module should exist");

    assert!(lib_rs.contains("mod shop_sync_handlers;"));
    assert!(!lib_rs.contains("\nasync fn list_shops("));
    assert!(!lib_rs.contains("\nasync fn shop_auth_callback("));
    assert!(!lib_rs.contains("\nasync fn download_platform_orders_by_order_nos("));
    assert!(!lib_rs.contains("\nasync fn run_platform_after_sales_sync("));
    assert!(shop_sync_handlers_rs.contains("pub(crate) async fn list_shops("));
    assert!(shop_sync_handlers_rs.contains("pub(crate) async fn shop_auth_callback("));
    assert!(shop_sync_handlers_rs
        .contains("pub(crate) async fn download_platform_orders_by_order_nos("));
    assert!(shop_sync_handlers_rs.contains("pub(crate) async fn run_platform_after_sales_sync("));
}

#[test]
fn api_crm_handlers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let crm_handlers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/crm_handlers.rs"),
    )
    .expect("crm_handlers module should exist");

    assert!(lib_rs.contains("mod crm_handlers;"));
    assert!(!lib_rs.contains("\nasync fn list_suppliers("));
    assert!(!lib_rs.contains("\nasync fn list_customers("));
    assert!(!lib_rs.contains("\nasync fn list_crm_service_tickets("));
    assert!(!lib_rs.contains("\nasync fn list_crm_points_ledgers("));
    assert!(crm_handlers_rs.contains("pub(crate) async fn list_suppliers("));
    assert!(crm_handlers_rs.contains("pub(crate) async fn list_customers("));
    assert!(crm_handlers_rs.contains("pub(crate) async fn list_crm_service_tickets("));
    assert!(crm_handlers_rs.contains("pub(crate) async fn list_crm_points_ledgers("));
}

#[test]
fn api_procurement_handlers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let procurement_handlers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/procurement_handlers.rs"),
    )
    .expect("procurement_handlers module should exist");

    assert!(lib_rs.contains("mod procurement_handlers;"));
    assert!(!lib_rs.contains("\nasync fn list_purchase_orders("));
    assert!(!lib_rs.contains("\nasync fn receive_purchase_order("));
    assert!(!lib_rs.contains("\nasync fn generate_procurement_plan("));
    assert!(!lib_rs.contains("\nasync fn list_supplier_performance("));
    assert!(procurement_handlers_rs.contains("pub(crate) async fn list_purchase_orders("));
    assert!(procurement_handlers_rs.contains("pub(crate) async fn receive_purchase_order("));
    assert!(procurement_handlers_rs.contains("pub(crate) async fn generate_procurement_plan("));
    assert!(procurement_handlers_rs.contains("pub(crate) async fn list_supplier_performance("));
}

#[test]
fn api_inventory_print_report_handlers_are_split_from_lib_rs() {
    let lib_rs = include_str!("../src/lib.rs");
    let inventory_print_report_handlers_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/inventory_print_report_handlers.rs"),
    )
    .expect("inventory_print_report_handlers module should exist");

    assert!(lib_rs.contains("mod inventory_print_report_handlers;"));
    assert!(inventory_print_report_handlers_rs.contains("mod inventory_core;"));
    assert!(inventory_print_report_handlers_rs.contains("pub(crate) use printing::*;"));
    assert!(inventory_print_report_handlers_rs.contains("pub(crate) use reports_exports::*;"));
    assert!(!lib_rs.contains("\nasync fn list_inventory("));
    assert!(!lib_rs.contains("\nasync fn create_stock_count("));
    assert!(!lib_rs.contains("\nasync fn list_inbounds("));
    assert!(!lib_rs.contains("\nasync fn create_print_job("));
    assert!(!lib_rs.contains("\nasync fn get_stats_report("));
    assert!(!lib_rs.contains("\nasync fn download_export_job("));
    let inventory_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/inventory_print_report_handlers");
    let inventory_core_rs = std::fs::read_to_string(inventory_dir.join("inventory_core.rs"))
        .expect("inventory_core module should exist");
    let stock_count_rs = std::fs::read_to_string(inventory_dir.join("stock_count.rs"))
        .expect("stock_count module should exist");
    let wms_inbound_rs = std::fs::read_to_string(inventory_dir.join("wms_inbound.rs"))
        .expect("wms_inbound module should exist");
    let printing_rs = std::fs::read_to_string(inventory_dir.join("printing.rs"))
        .expect("printing module should exist");
    let reports_exports_rs = std::fs::read_to_string(inventory_dir.join("reports_exports.rs"))
        .expect("reports_exports module should exist");

    assert!(inventory_core_rs.contains("pub(crate) async fn list_inventory("));
    assert!(stock_count_rs.contains("pub(crate) async fn create_stock_count("));
    assert!(wms_inbound_rs.contains("pub(crate) async fn list_inbounds("));
    assert!(printing_rs.contains("pub(crate) async fn create_print_job("));
    assert!(reports_exports_rs.contains("pub(crate) async fn get_stats_report("));
    assert!(reports_exports_rs.contains("pub(crate) async fn download_export_job("));
}

#[test]
fn api_non_root_modules_stay_below_industrial_line_limit() {
    fn collect_rs_files(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("api src dir should be readable") {
            let entry = entry.expect("api src entry should be readable");
            let path = entry.path();
            if path.is_dir() {
                collect_rs_files(&path, files);
            } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }

    let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs_files(&src_dir, &mut files);

    let oversized = files
        .into_iter()
        .filter(|path| path.file_name().and_then(|value| value.to_str()) != Some("lib.rs"))
        .filter_map(|path| {
            let content =
                std::fs::read_to_string(&path).expect("api source file should be readable");
            let line_count = content.lines().count();
            (line_count > 5_000).then_some((path, line_count))
        })
        .collect::<Vec<_>>();

    assert!(
        oversized.is_empty(),
        "non-root API modules must stay under 5000 lines: {oversized:?}"
    );
}

#[test]
fn api_inventory_print_report_handlers_are_split_by_bounded_context() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let aggregate_rs = std::fs::read_to_string(root.join("inventory_print_report_handlers.rs"))
        .expect("inventory_print_report_handlers aggregate should exist");

    for module_name in [
        "inventory_core",
        "stock_count",
        "stock_movement",
        "wms_inbound",
        "printing",
        "reports_exports",
    ] {
        assert!(
            aggregate_rs.contains(&format!("mod {module_name};")),
            "inventory aggregate should declare {module_name}"
        );
        assert!(
            aggregate_rs.contains(&format!("pub(crate) use {module_name}::*;")),
            "inventory aggregate should re-export {module_name}"
        );
        std::fs::read_to_string(
            root.join("inventory_print_report_handlers")
                .join(format!("{module_name}.rs")),
        )
        .unwrap_or_else(|_| panic!("inventory child module {module_name} should exist"));
    }

    assert!(!aggregate_rs.contains("\npub(crate) async fn list_inventory("));
    assert!(!aggregate_rs.contains("\npub(crate) async fn create_stock_count("));
    assert!(!aggregate_rs.contains("\npub(crate) async fn list_inbounds("));
    assert!(!aggregate_rs.contains("\npub(crate) async fn create_print_job("));
    assert!(!aggregate_rs.contains("\npub(crate) async fn download_export_job("));

    let inventory_core_rs = std::fs::read_to_string(
        root.join("inventory_print_report_handlers")
            .join("inventory_core.rs"),
    )
    .expect("inventory_core module should exist");
    let stock_count_rs = std::fs::read_to_string(
        root.join("inventory_print_report_handlers")
            .join("stock_count.rs"),
    )
    .expect("stock_count module should exist");
    let wms_inbound_rs = std::fs::read_to_string(
        root.join("inventory_print_report_handlers")
            .join("wms_inbound.rs"),
    )
    .expect("wms_inbound module should exist");
    let reports_exports_rs = std::fs::read_to_string(
        root.join("inventory_print_report_handlers")
            .join("reports_exports.rs"),
    )
    .expect("reports_exports module should exist");

    assert!(inventory_core_rs.contains("pub(crate) async fn list_inventory("));
    assert!(stock_count_rs.contains("pub(crate) async fn create_stock_count("));
    assert!(wms_inbound_rs.contains("pub(crate) async fn list_inbounds("));
    assert!(reports_exports_rs.contains("pub(crate) async fn download_export_job("));
}

#[test]
fn api_wms_operation_handlers_are_split_by_bounded_context() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let aggregate_rs = std::fs::read_to_string(root.join("wms_operation_handlers.rs"))
        .expect("wms_operation_handlers aggregate should exist");

    for module_name in ["views", "outbound_core", "wave", "workflow", "fulfillment"] {
        assert!(
            aggregate_rs.contains(&format!("mod {module_name};")),
            "wms aggregate should declare {module_name}"
        );
        assert!(
            aggregate_rs.contains(&format!("pub(crate) use {module_name}::*;")),
            "wms aggregate should re-export {module_name}"
        );
        std::fs::read_to_string(
            root.join("wms_operation_handlers")
                .join(format!("{module_name}.rs")),
        )
        .unwrap_or_else(|_| panic!("wms child module {module_name} should exist"));
    }

    assert!(!aggregate_rs.contains("\npub(crate) async fn list_wms_outbounds("));
    assert!(!aggregate_rs.contains("\npub(crate) async fn run_wms_wave("));
    assert!(!aggregate_rs.contains("\npub(crate) async fn list_wms_labor_tasks("));
    assert!(!aggregate_rs.contains("\npub(crate) async fn ship_wms_outbound("));
    assert!(!aggregate_rs.contains("\npub(crate) fn wms_outbound_doc_response("));

    let outbound_core_rs =
        std::fs::read_to_string(root.join("wms_operation_handlers").join("outbound_core.rs"))
            .expect("outbound_core module should exist");
    let wave_rs = std::fs::read_to_string(root.join("wms_operation_handlers").join("wave.rs"))
        .expect("wave module should exist");
    let fulfillment_rs =
        std::fs::read_to_string(root.join("wms_operation_handlers").join("fulfillment.rs"))
            .expect("fulfillment module should exist");
    let views_rs = std::fs::read_to_string(root.join("wms_operation_handlers").join("views.rs"))
        .expect("views module should exist");

    assert!(outbound_core_rs.contains("pub(crate) async fn list_wms_outbounds("));
    assert!(wave_rs.contains("pub(crate) async fn run_wms_wave("));
    assert!(fulfillment_rs.contains("pub(crate) async fn ship_wms_outbound("));
    assert!(views_rs.contains("pub(crate) fn wms_outbound_doc_response("));
}
