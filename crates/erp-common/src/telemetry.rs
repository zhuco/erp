use std::fmt::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use tracing_subscriber::{fmt, EnvFilter};

pub const METRICS_CONTENT_TYPE: &str = "text/plain; version=0.0.4";

#[derive(Default)]
pub struct Metrics {
    webhook_inbox_received_total: AtomicU64,
    webhook_inbox_dedup_hit_total: AtomicU64,
    outbox_send_total: AtomicU64,
    outbox_send_failed_total: AtomicU64,
    outbox_send_retry_total: AtomicU64,
    worker_consume_success_total: AtomicU64,
    worker_consume_failed_total: AtomicU64,
    worker_dlq_total: AtomicU64,
    shipping_success_total: AtomicU64,
    shipping_failed_total: AtomicU64,
    shipping_push_success_total: AtomicU64,
    shipping_push_failed_total: AtomicU64,
    shipping_push_retry_total: AtomicU64,
    shipping_push_latency_ms_sum: AtomicU64,
    shipping_push_latency_ms_count: AtomicU64,
    inventory_adjust_total: AtomicU64,
    inventory_reserve_total: AtomicU64,
    inventory_consume_total: AtomicU64,
    inventory_return_total: AtomicU64,
    after_sales_sync_success_total: AtomicU64,
    after_sales_compensation_total: AtomicU64,
    after_sales_cycle_time_ms_sum: AtomicU64,
    after_sales_cycle_time_ms_count: AtomicU64,
    after_sales_sla_overdue_total: AtomicU64,
    after_sales_sla_evaluated_total: AtomicU64,
    print_jobs_success_total: AtomicU64,
    print_jobs_failed_total: AtomicU64,
    http_request_total: AtomicU64,
    http_request_error_total: AtomicU64,
    http_request_latency_ms_sum: AtomicU64,
    http_request_latency_ms_count: AtomicU64,
    outbox_dispatch_attempt_total: AtomicU64,
    outbox_dispatch_success_total: AtomicU64,
    outbox_dispatch_failed_total: AtomicU64,
    outbox_backlog: AtomicU64,
    worker_consume_latency_ms_sum: AtomicU64,
    worker_consume_latency_ms_count: AtomicU64,
    worker_consume_lag_ms_sum: AtomicU64,
    worker_consume_lag_ms_count: AtomicU64,
    frontend_route_diagnostic_ingested_total: AtomicU64,
    frontend_route_timeout_total: AtomicU64,
    frontend_route_error_total: AtomicU64,
    frontend_route_prefetch_ready_total: AtomicU64,
}

impl Metrics {
    pub fn inc_webhook_inbox_received(&self) {
        self.webhook_inbox_received_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_webhook_inbox_dedup_hits(&self, hits: u64) {
        if hits > 0 {
            self.webhook_inbox_dedup_hit_total
                .fetch_add(hits, Ordering::Relaxed);
        }
    }

    pub fn inc_outbox_send_total(&self) {
        self.outbox_send_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_outbox_send_failed(&self) {
        self.outbox_send_failed_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_outbox_send_retry(&self) {
        self.outbox_send_retry_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_worker_consume_success(&self) {
        self.worker_consume_success_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_worker_consume_failed(&self) {
        self.worker_consume_failed_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_worker_dlq(&self) {
        self.worker_dlq_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_shipping_success(&self) {
        self.shipping_success_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_shipping_failed(&self) {
        self.shipping_failed_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_shipping_push_success(&self) {
        self.shipping_push_success_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_shipping_push_failed(&self) {
        self.shipping_push_failed_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_shipping_push_retry(&self) {
        self.shipping_push_retry_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_shipping_push_latency_ms(&self, latency_ms: u64) {
        self.shipping_push_latency_ms_sum
            .fetch_add(latency_ms, Ordering::Relaxed);
        self.shipping_push_latency_ms_count
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_inventory_adjust(&self) {
        self.inventory_adjust_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_inventory_reserve(&self) {
        self.inventory_reserve_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_inventory_consume(&self) {
        self.inventory_consume_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_inventory_return(&self) {
        self.inventory_return_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_after_sales_sync_success(&self) {
        self.after_sales_sync_success_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_after_sales_compensation(&self) {
        self.after_sales_compensation_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_after_sales_cycle_time_ms(&self, duration_ms: u64) {
        self.after_sales_cycle_time_ms_sum
            .fetch_add(duration_ms, Ordering::Relaxed);
        self.after_sales_cycle_time_ms_count
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_after_sales_sla_overdue(&self) {
        self.after_sales_sla_overdue_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_after_sales_sla_evaluated(&self, count: u64) {
        if count > 0 {
            self.after_sales_sla_evaluated_total
                .fetch_add(count, Ordering::Relaxed);
        }
    }

    pub fn inc_print_jobs_success(&self) {
        self.print_jobs_success_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_print_jobs_failed(&self) {
        self.print_jobs_failed_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_http_request_total(&self) {
        self.http_request_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_http_request_error(&self) {
        self.http_request_error_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_http_request_latency_ms(&self, latency_ms: u64) {
        self.http_request_latency_ms_sum
            .fetch_add(latency_ms, Ordering::Relaxed);
        self.http_request_latency_ms_count
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_outbox_dispatch_attempt(&self) {
        self.outbox_dispatch_attempt_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_outbox_dispatch_success(&self) {
        self.outbox_dispatch_success_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_outbox_dispatch_failed(&self) {
        self.outbox_dispatch_failed_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn set_outbox_backlog(&self, backlog: u64) {
        self.outbox_backlog.store(backlog, Ordering::Relaxed);
    }

    pub fn add_worker_consume_latency_ms(&self, latency_ms: u64) {
        self.worker_consume_latency_ms_sum
            .fetch_add(latency_ms, Ordering::Relaxed);
        self.worker_consume_latency_ms_count
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_worker_consume_lag_ms(&self, lag_ms: u64) {
        self.worker_consume_lag_ms_sum
            .fetch_add(lag_ms, Ordering::Relaxed);
        self.worker_consume_lag_ms_count
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_frontend_route_diagnostic_ingested(&self) {
        self.frontend_route_diagnostic_ingested_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_frontend_route_timeout(&self) {
        self.frontend_route_timeout_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_frontend_route_error(&self) {
        self.frontend_route_error_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_frontend_route_prefetch_ready(&self) {
        self.frontend_route_prefetch_ready_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        append_counter(
            &mut out,
            "erp_webhook_inbox_received_total",
            "Count of inbox webhook events processed",
            self.webhook_inbox_received_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_webhook_inbox_dedup_hit_total",
            "Duplicate webhook hits observed during inbox processing",
            self.webhook_inbox_dedup_hit_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_outbox_send_total",
            "Outbox envelopes delivered to workers",
            self.outbox_send_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_outbox_send_failed_total",
            "Outbox envelopes that failed in workers",
            self.outbox_send_failed_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_outbox_send_retry_total",
            "Outbox envelopes delivered with retries",
            self.outbox_send_retry_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_worker_consume_success_total",
            "Worker messages processed successfully",
            self.worker_consume_success_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_worker_consume_failed_total",
            "Worker messages that failed processing",
            self.worker_consume_failed_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_worker_dlq_total",
            "Worker messages treated as dead-letter",
            self.worker_dlq_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_shipping_success_total",
            "Shipments completed successfully",
            self.shipping_success_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_shipping_failed_total",
            "Shipments that failed",
            self.shipping_failed_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_shipping_push_success_total",
            "Shipment push successes",
            self.shipping_push_success_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_shipping_push_failed_total",
            "Shipment push failures",
            self.shipping_push_failed_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_shipping_push_retry_total",
            "Shipment push retries",
            self.shipping_push_retry_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_shipping_push_latency_ms_sum",
            "Sum of shipment push latency in ms",
            self.shipping_push_latency_ms_sum.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_shipping_push_latency_ms_count",
            "Count of shipment push latency samples",
            self.shipping_push_latency_ms_count.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_inventory_adjust_total",
            "Inventory adjustments applied",
            self.inventory_adjust_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_inventory_reserve_total",
            "Inventory reservations created",
            self.inventory_reserve_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_inventory_consume_total",
            "Inventory reservations consumed",
            self.inventory_consume_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_inventory_return_total",
            "Inventory reservations released",
            self.inventory_return_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_after_sales_sync_success_total",
            "After-sales events synced and closed successfully",
            self.after_sales_sync_success_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_after_sales_compensation_total",
            "After-sales compensation actions executed",
            self.after_sales_compensation_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_after_sales_cycle_time_ms_sum",
            "Sum of after-sales processing cycle time in ms",
            self.after_sales_cycle_time_ms_sum.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_after_sales_cycle_time_ms_count",
            "Count of after-sales processing cycle samples",
            self.after_sales_cycle_time_ms_count.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_after_sales_sla_overdue_total",
            "After-sales records identified as SLA overdue",
            self.after_sales_sla_overdue_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_after_sales_sla_evaluated_total",
            "After-sales records evaluated for SLA status",
            self.after_sales_sla_evaluated_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_print_jobs_success_total",
            "Print jobs marked SUCCESS",
            self.print_jobs_success_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_print_jobs_failed_total",
            "Print jobs marked FAILED",
            self.print_jobs_failed_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_http_request_total",
            "Total HTTP requests",
            self.http_request_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_http_request_error_total",
            "Total HTTP requests with 5xx status",
            self.http_request_error_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_http_request_latency_ms_sum",
            "Sum of HTTP request latency in ms",
            self.http_request_latency_ms_sum.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_http_request_latency_ms_count",
            "Count of HTTP request latency samples",
            self.http_request_latency_ms_count.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_outbox_dispatch_attempt_total",
            "Outbox dispatch attempts",
            self.outbox_dispatch_attempt_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_outbox_dispatch_success_total",
            "Outbox dispatch successes",
            self.outbox_dispatch_success_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_outbox_dispatch_failed_total",
            "Outbox dispatch failures",
            self.outbox_dispatch_failed_total.load(Ordering::Relaxed),
        );
        append_gauge(
            &mut out,
            "erp_outbox_backlog",
            "Outbox backlog (pending/failed rows fetched per batch)",
            self.outbox_backlog.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_worker_consume_latency_ms_sum",
            "Sum of worker consume latency in ms",
            self.worker_consume_latency_ms_sum.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_worker_consume_latency_ms_count",
            "Count of worker consume latency samples",
            self.worker_consume_latency_ms_count.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_worker_consume_lag_ms_sum",
            "Sum of worker consume lag in ms",
            self.worker_consume_lag_ms_sum.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_worker_consume_lag_ms_count",
            "Count of worker consume lag samples",
            self.worker_consume_lag_ms_count.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_frontend_route_diagnostic_ingested_total",
            "Frontend route diagnostic events ingested from the web client",
            self.frontend_route_diagnostic_ingested_total
                .load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_frontend_route_timeout_total",
            "Frontend route diagnostic events marked as timeout",
            self.frontend_route_timeout_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_frontend_route_error_total",
            "Frontend route diagnostic events marked as error",
            self.frontend_route_error_total.load(Ordering::Relaxed),
        );
        append_counter(
            &mut out,
            "erp_frontend_route_prefetch_ready_total",
            "Frontend route diagnostic events that hit prefetch successfully",
            self.frontend_route_prefetch_ready_total
                .load(Ordering::Relaxed),
        );
        out
    }
}

static METRICS: OnceLock<Metrics> = OnceLock::new();

pub fn metrics() -> &'static Metrics {
    METRICS.get_or_init(Metrics::default)
}

pub fn render_metrics() -> String {
    metrics().render()
}

fn append_counter(out: &mut String, name: &str, help: &str, value: u64) {
    let _ = writeln!(out, "# HELP {name} {help}");
    let _ = writeln!(out, "# TYPE {name} counter");
    let _ = writeln!(out, "{name} {value}");
}

fn append_gauge(out: &mut String, name: &str, help: &str, value: u64) {
    let _ = writeln!(out, "# HELP {name} {help}");
    let _ = writeln!(out, "# TYPE {name} gauge");
    let _ = writeln!(out, "{name} {value}");
}

pub fn init_tracing(service_name: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_level(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .init();

    tracing::info!(service = service_name, "tracing initialized");
}
