#[test]
fn worker_order_review_handlers_are_split_from_main_rs() {
    let main_rs = include_str!("../src/main.rs");
    let order_review_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/order_review.rs"),
    )
    .expect("order_review module should exist");

    assert!(main_rs.contains("mod order_review;"));
    assert!(!main_rs.contains("\nasync fn handle_order_upserted("));
    assert!(!main_rs.contains("\nasync fn handle_order_reviewed("));
    assert!(!main_rs.contains("\nasync fn handle_order_review("));
    assert!(!main_rs.contains("\nasync fn auto_pass_order_review("));
    assert!(!main_rs.contains("\nasync fn auto_allocate_order("));
    assert!(order_review_rs.contains("pub(super) async fn handle_order_upserted("));
    assert!(order_review_rs.contains("pub(super) async fn handle_order_reviewed("));
    assert!(order_review_rs.contains("pub(super) async fn handle_order_review("));
    assert!(order_review_rs.contains("pub(super) async fn auto_pass_order_review("));
    assert!(order_review_rs.contains("pub(super) async fn auto_allocate_order("));
}
#[test]
fn worker_shipment_handlers_are_split_from_main_rs() {
    let main_rs = include_str!("../src/main.rs");
    let shipment_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shipment.rs"),
    )
    .expect("shipment module should exist");

    assert!(main_rs.contains("mod shipment;"));
    assert!(!main_rs.contains("\nasync fn handle_shipment_requested("));
    assert!(!main_rs.contains("\nasync fn handle_shipment_shipped("));
    assert!(!main_rs.contains("\nfn shipment_push_error_summary("));
    assert!(shipment_rs.contains("pub(super) async fn handle_shipment_requested("));
    assert!(shipment_rs.contains("pub(super) async fn handle_shipment_shipped("));
    assert!(shipment_rs.contains("pub(super) fn shipment_push_error_summary("));
}
#[test]
fn worker_inventory_handlers_are_split_from_main_rs() {
    let main_rs = include_str!("../src/main.rs");
    let inventory_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/inventory.rs"),
    )
    .expect("inventory module should exist");

    assert!(main_rs.contains("mod inventory;"));
    assert!(!main_rs.contains("\nasync fn handle_inventory_changed("));
    assert!(!main_rs.contains("\nasync fn run_stock_sync_jobs_once("));
    assert!(inventory_rs.contains("pub(super) async fn handle_inventory_changed("));
    assert!(inventory_rs.contains("pub(super) async fn run_stock_sync_jobs_once("));
}
#[test]
fn worker_sync_job_handlers_are_split_from_main_rs() {
    let main_rs = include_str!("../src/main.rs");
    let sync_jobs_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/sync_jobs.rs"),
    )
    .expect("sync_jobs module should exist");

    assert!(main_rs.contains("mod sync_jobs;"));
    assert!(!main_rs.contains("\nasync fn handle_sync_job_run("));
    assert!(!main_rs.contains("\nasync fn run_sync_jobs_once("));
    assert!(!main_rs.contains("\nasync fn sync_platform_products("));
    assert!(!main_rs.contains("\nfn compute_sync_dedup_key("));
    assert!(!main_rs.contains("\nasync fn apply_sku_mappings("));
    assert!(sync_jobs_rs.contains("pub(super) async fn handle_sync_job_run("));
    assert!(sync_jobs_rs.contains("pub(super) async fn run_sync_jobs_once("));
    assert!(sync_jobs_rs.contains("pub(super) async fn sync_platform_products("));
    assert!(sync_jobs_rs.contains("pub(super) fn compute_sync_dedup_key("));
    assert!(sync_jobs_rs.contains("pub(super) async fn apply_sku_mappings("));
}
#[test]
fn worker_after_sales_handlers_are_split_from_main_rs() {
    let main_rs = include_str!("../src/main.rs");
    let after_sales_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/after_sales.rs"),
    )
    .expect("after_sales module should exist");

    assert!(main_rs.contains("mod after_sales;"));
    assert!(!main_rs.contains("\nasync fn handle_refund_requested("));
    assert!(!main_rs.contains("\nasync fn rollback_refunded_order("));
    assert!(!main_rs.contains("\nasync fn has_printed_jobs("));
    assert!(!main_rs.contains("\nasync fn cancel_print_jobs("));
    assert!(after_sales_rs.contains("pub(super) async fn handle_refund_requested("));
    assert!(after_sales_rs.contains("pub(super) async fn rollback_refunded_order("));
    assert!(after_sales_rs.contains("pub(super) async fn has_printed_jobs("));
    assert!(after_sales_rs.contains("pub(super) async fn cancel_print_jobs("));
}
#[test]
fn worker_print_handlers_are_split_from_main_rs() {
    let main_rs = include_str!("../src/main.rs");
    let printing_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/printing.rs"),
    )
    .expect("printing module should exist");

    assert!(main_rs.contains("mod printing;"));
    assert!(!main_rs.contains("\nasync fn handle_print_job_queued("));
    assert!(printing_rs.contains("pub(super) async fn handle_print_job_queued("));
}

#[test]
fn worker_wave_handlers_are_split_from_main_rs() {
    let main_rs = include_str!("../src/main.rs");
    let wave_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/wave.rs"),
    )
    .expect("wave module should exist");

    assert!(main_rs.contains("mod wave;"));
    assert!(!main_rs.contains("\nasync fn run_wave_jobs_once("));
    assert!(!main_rs.contains("\nfn resolve_wave_cutoff_at("));
    assert!(!main_rs.contains("\nasync fn fetch_warehouse_workflow_enabled("));
    assert!(wave_rs.contains("pub(super) async fn run_wave_jobs_once("));
    assert!(wave_rs.contains("pub(super) fn resolve_wave_cutoff_at("));
    assert!(wave_rs.contains("pub(super) async fn fetch_warehouse_workflow_enabled("));
}

#[test]
fn worker_inbox_handlers_are_split_from_main_rs() {
    let main_rs = include_str!("../src/main.rs");
    let inbox_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/inbox.rs"),
    )
    .expect("inbox module should exist");

    assert!(main_rs.contains("mod inbox;"));
    assert!(!main_rs.contains("\nasync fn handle_inbox_received("));
    assert!(!main_rs.contains("\nasync fn record_inbox_dedup_hits("));
    assert!(inbox_rs.contains("pub(super) async fn handle_inbox_received("));
    assert!(inbox_rs.contains("pub(super) async fn record_inbox_dedup_hits("));
}

#[test]
fn worker_dead_letter_handlers_are_split_from_main_rs() {
    let main_rs = include_str!("../src/main.rs");
    let dlq_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/dlq.rs"),
    )
    .expect("dlq module should exist");

    assert!(main_rs.contains("mod dlq;"));
    assert!(!main_rs.contains("\nasync fn handle_dead_letter_replay("));
    assert!(dlq_rs.contains("pub(super) async fn handle_dead_letter_replay("));
}

#[test]
fn worker_finance_handlers_are_split_from_main_rs() {
    let main_rs = include_str!("../src/main.rs");
    let finance_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/finance.rs"),
    )
    .expect("finance module should exist");

    assert!(main_rs.contains("mod finance;"));
    assert!(!main_rs.contains("\nasync fn handle_profit_compute_requested("));
    assert!(finance_rs.contains("pub(super) async fn handle_profit_compute_requested("));
}

#[test]
fn worker_runtime_helpers_are_split_from_main_rs() {
    let main_rs = include_str!("../src/main.rs");
    let runtime_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime.rs"),
    )
    .expect("runtime module should exist");

    assert!(main_rs.contains("mod runtime;"));
    assert!(!main_rs.contains("\nstruct AdapterRegistry"));
    assert!(!main_rs.contains("\nasync fn is_pdd_only_mode("));
    assert!(!main_rs.contains("\nasync fn feature_flag_enabled("));
    assert!(!main_rs.contains("\nasync fn read_only_enabled("));
    assert!(!main_rs.contains("\nasync fn metrics_handler("));
    assert!(runtime_rs.contains("pub(super) struct AdapterRegistry"));
    assert!(runtime_rs.contains("pub(super) async fn is_pdd_only_mode("));
    assert!(runtime_rs.contains("pub(super) async fn feature_flag_enabled("));
    assert!(runtime_rs.contains("pub(super) async fn read_only_enabled("));
    assert!(runtime_rs.contains("pub(super) async fn metrics_handler("));
}

#[test]
fn worker_main_tests_are_split_from_main_rs() {
    let main_rs = include_str!("../src/main.rs");
    let tests_rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tests.rs"),
    )
    .expect("worker test module should exist");

    let normalized_main_rs = main_rs.replace("\r\n", "\n");
    assert!(normalized_main_rs.contains("#[cfg(test)]\nmod tests;"));
    assert!(!normalized_main_rs.contains("\nmod tests {"));
    assert!(tests_rs.contains("fn event_router_module_exports_route_event_symbol("));
    assert!(tests_rs.contains("async fn shipment_retry_consumes_once("));
    assert!(tests_rs.contains("async fn token_refresh_success_updates_token("));
}
