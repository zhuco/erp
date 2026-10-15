mod after_sales;
mod dlq;
mod event_router;
mod finance;
mod inbox;
mod inventory;
mod order_review;
mod printing;
mod runtime;
mod shipment;
mod sync_jobs;
mod token_refresh;
mod wave;

use after_sales::*;
use axum::{
    Router,
    body::Body,
    http::{StatusCode, header},
    response::Response,
    routing::get,
};
use chrono::{Duration as ChronoDuration, NaiveTime, Utc};
use clap::Parser;
use dlq::*;
use erp_adapter::{
    AdapterError, PlatformAdapter, PlatformTokenPayload, ShipOrderRequest, SyncCursor,
    SyncStockRequest,
};
use erp_common::{
    feature_flags::{self, FeatureFlagKey},
    sales_settings::{self, SalesShopConfig},
    settings::Settings,
    telemetry::{METRICS_CONTENT_TYPE, init_tracing, metrics, render_metrics},
    trace::TRACE_ID_HEADER,
};
use erp_domain::{
    DLQ_REPLAY_REQUESTED_EVENT, DeadLetterReplayPayload, InboxReceivedPayload,
    InventoryChangedPayload, OrderProjection, OrderReviewedPayload, OrderUpsertedPayload,
    PROFIT_COMPUTE_REQUESTED_EVENT, ProfitComputeRequestedPayload, RefundRequestedPayload,
    ShipmentFailedPayload, ShipmentRequestedPayload, ShipmentShippedPayload,
    oms::{
        OrderReviewCondition, OrderReviewContext, ReviewAction, ReviewMode, apply_review_decision,
        derive_exception_status, derive_order_status, evaluate_conditions,
    },
};
use erp_outbox::envelope::OutboxEnvelope;
use erp_repo::{
    after_sale_cases::AfterSaleCaseRepo,
    after_sales::{AfterSalesRepo, REFUND_REQUESTED_EVENT},
    audit_logs::{AuditLogCreate, AuditLogsRepo},
    customer_blacklist::CustomerBlacklistRepo,
    db::Db,
    feature_flags::{FeatureFlag, FeatureFlagsRepo},
    finance::{ProfitSnapshotUpsert, ProfitSnapshotsRepo},
    fulfillment::FulfillmentRepo,
    inbox::InboxRepo,
    inventory::{INVENTORY_CHANGED_EVENT, InventoryRepo, InventoryReservationAction},
    mapping_rules::MappingRulesRepo,
    operation_logs::{
        OP_ACTION_OMS_REVIEW_AUTO_FLAG, OP_ACTION_OMS_REVIEW_AUTO_PASS,
        OP_ACTION_OMS_REVIEW_AUTO_REJECT, OperationLogCreate, OperationLogsRepo,
    },
    order_review_rules::OrderReviewRulesRepo,
    order_reviews::{OrderReviewUpsert, OrderReviewsRepo},
    order_tag_definitions::OrderTagDefinitionsRepo,
    orders::{OrderTagCreate, OrdersRepo},
    outbox::OutboxRepo,
    platform_tokens::PlatformTokensRepo,
    print_jobs::PrintJobsRepo,
    print_templates::PrintTemplatesRepo,
    products::{ProductCreate, ProductUpdate, ProductsRepo},
    settings::{SettingListFilters, SettingsRepo},
    shipment_push_attempts::{
        SHIPMENT_PUSH_STATUS_DEAD, SHIPMENT_PUSH_STATUS_FAILED, SHIPMENT_PUSH_STATUS_RETRYING,
        SHIPMENT_PUSH_STATUS_SUCCESS, ShipmentPushAttemptCreate, ShipmentPushAttemptsRepo,
    },
    shipments::{
        SHIPMENT_CONFIRMED_EVENT, SHIPMENT_FAILED_EVENT, SHIPMENT_REQUESTED_EVENT,
        SHIPMENT_SHIPPED_EVENT, ShipmentsRepo,
    },
    shops::ShopsRepo,
    sku_serials::SkuSerialsRepo,
    skus::{SkuCreate, SkuListFilters, SkuUpdate, SkusRepo},
    stock_sync_job_events::StockSyncJobEventsRepo,
    stock_sync_jobs::StockSyncJobsRepo,
    sync_job_runs::{SyncJobRunCreate, SyncJobRunsRepo},
    sync_jobs::SyncJobsRepo,
    tenants::TenantsRepo,
    warehouses::{WarehouseListFilters, WarehousesRepo},
    waybills::WaybillsRepo,
    wms::WmsRepo,
    worker_effects::WorkerEffectsRepo,
};
use erp_types::{OrderId, OrderReviewStatus, Platform, ShopId, TenantId};
use erp_worker::jobs::{after_sales_import_jobs, export_jobs};
use event_router::route_event;
use finance::*;
use futures::StreamExt;
use inbox::*;
use inventory::*;
use order_review::*;
use printing::*;
use runtime::*;
use sha2::{Digest, Sha256};
use shipment::*;
use sqlx::PgPool;
use std::{collections::HashMap, net::SocketAddr, str::FromStr, sync::Arc, time::Duration};
use sync_jobs::*;
use token_refresh::{handle_token_refresh_requested, refresh_tokens_once};
use tokio::sync::Semaphore;
use tracing::{error, info, warn};
use wave::*;

#[derive(Parser, Debug)]
struct Args {}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
struct SyncJobReplayPayload {
    tenant_id: TenantId,
    job_id: uuid::Uuid,
    shop_id: ShopId,
    job_type: String,
}

#[derive(Debug, serde::Deserialize)]
struct WarehouseWorkflowConfigLite {
    enable_wave: Option<bool>,
}

const SHIPMENT_PUSH_TRIGGER_AUTO: &str = "AUTO";
const SHIPMENT_PUSH_TRIGGER_RETRY: &str = "RETRY";
const SHIPMENT_PUSH_TRIGGER_REPLAY: &str = "REPLAY";
const SYNC_JOB_FORCE_RUN_MARKER: &str = "FORCE_RUN";
const ORDER_REVIEW_RECORD_PENDING: &str = "PENDING";
const ORDER_REVIEW_RECORD_APPROVED: &str = "APPROVED";
const ORDER_REVIEW_RECORD_REJECTED: &str = "REJECTED";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing("erp-worker");
    let _args = Args::parse();
    let settings = Settings::load()?;

    let db = Db::connect(&settings.database_url).await?;
    let adapters = std::sync::Arc::new(AdapterRegistry::new());

    let refresh_interval = Duration::from_secs(settings.token_refresh_interval_secs);
    let refresh_threshold = ChronoDuration::seconds(settings.token_refresh_threshold_secs as i64);
    let refresh_batch_limit = settings.token_refresh_batch_limit;
    let sync_interval = Duration::from_secs(settings.sync_job_interval_secs);
    let sync_lookback = ChronoDuration::minutes(settings.sync_job_lookback_minutes);
    let sync_batch_limit = settings.sync_job_batch_limit;
    let stock_sync_interval = Duration::from_secs(settings.stock_sync_job_interval_secs);
    let stock_sync_batch_limit = settings.stock_sync_job_batch_limit;
    let stock_sync_debounce = ChronoDuration::milliseconds(settings.stock_sync_debounce_ms);
    let wave_run_interval = Duration::from_secs(settings.wave_run_interval_secs);
    let retry_policy = dlq::RetryPolicy {
        max_attempts: settings.worker_retry_max_attempts,
        base_delay_ms: settings.worker_retry_base_delay_ms,
        max_delay_ms: settings.worker_retry_max_delay_ms,
    };

    {
        let db = db.clone();
        let adapters = adapters.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(refresh_interval);
            loop {
                ticker.tick().await;
                if let Err(err) = refresh_tokens_once(
                    &db,
                    adapters.as_ref(),
                    refresh_threshold,
                    refresh_batch_limit,
                )
                .await
                {
                    warn!(error=%err, "token refresh loop failed");
                }
            }
        });
    }

    {
        let db = db.clone();
        let adapters = adapters.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(sync_interval);
            loop {
                ticker.tick().await;
                if let Err(err) =
                    run_sync_jobs_once(&db, adapters.as_ref(), sync_lookback, sync_batch_limit)
                        .await
                {
                    warn!(error=%err, "sync job loop failed");
                }
            }
        });
    }

    {
        let db = db.clone();
        let adapters = adapters.clone();
        let retry_policy = retry_policy.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(stock_sync_interval);
            loop {
                ticker.tick().await;
                if let Err(err) = run_stock_sync_jobs_once(
                    &db,
                    adapters.as_ref(),
                    stock_sync_batch_limit,
                    &retry_policy,
                )
                .await
                {
                    warn!(error=%err, "stock sync job loop failed");
                }
            }
        });
    }

    {
        let db = db.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(wave_run_interval);
            loop {
                ticker.tick().await;
                if let Err(err) = run_wave_jobs_once(&db).await {
                    warn!(error=%err, "wave auto run loop failed");
                }
            }
        });
    }

    let nats = async_nats::connect(settings.nats_url.clone()).await?;
    let mut sub = nats.subscribe(settings.worker_subject.clone()).await?;
    let retry_policy = retry_policy.clone();
    let metrics_port: u16 = std::env::var("WORKER_METRICS_PORT")
        .unwrap_or_else(|_| "9090".to_string())
        .parse()?;
    if metrics_port == 0 {
        warn!("worker metrics disabled via WORKER_METRICS_PORT=0");
    } else {
        let metrics_addr = SocketAddr::from(([0, 0, 0, 0], metrics_port));
        let health_db = db.clone();
        let metrics_app = Router::new()
            .route("/metrics", get(metrics_handler))
            .route(
                "/healthz",
                get(move || healthz_with_db(health_db.clone())),
            );
        match tokio::net::TcpListener::bind(metrics_addr).await {
            Ok(metrics_listener) => {
                tokio::spawn(async move {
                    info!(%metrics_addr, "erp-worker metrics listening");
                    if let Err(err) = axum::serve(metrics_listener, metrics_app).await {
                        error!(error=%err, "metrics server failed");
                    }
                });
            }
            Err(err) => {
                warn!(error=%err, %metrics_addr, "metrics server bind failed, skipping");
            }
        }
    }

    {
        let db = db.clone();
        let config = export_jobs::ExportJobConfig::from_env()?;
        tokio::spawn(async move {
            if let Err(err) = export_jobs::run_loop(&db, config).await {
                error!(error=%err, "export job loop stopped");
            }
        });
    }
    {
        let db = db.clone();
        let config = after_sales_import_jobs::AfterSalesImportJobConfig::from_env()?;
        tokio::spawn(async move {
            if let Err(err) = after_sales_import_jobs::run_loop(&db, config).await {
                error!(error=%err, "after-sales import job loop stopped");
            }
        });
    }

    let semaphore = Arc::new(Semaphore::new(settings.worker_concurrency));
    info!(subject=%settings.worker_subject, concurrency=settings.worker_concurrency, "worker subscribed");

    while let Some(msg) = sub.next().await {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let db = db.clone();
        let adapters = adapters.clone();
        let nats = nats.clone();
        let retry_policy = retry_policy.clone();
        tokio::spawn(async move {
            let _permit = permit;
            let started = std::time::Instant::now();
            let attempt = dlq::attempt_from_headers(msg.headers.as_ref());
            let replay_dead_letter_id = msg
                .headers
                .as_ref()
                .and_then(|headers| headers.get(dlq::DLQ_REPLAY_ID_HEADER))
                .and_then(|value| uuid::Uuid::parse_str(value.as_str()).ok());
            let trigger = if replay_dead_letter_id.is_some()
                || msg
                    .headers
                    .as_ref()
                    .and_then(|headers| headers.get(dlq::DLQ_REPLAY_HEADER))
                    .is_some()
            {
                SHIPMENT_PUSH_TRIGGER_REPLAY
            } else if attempt > 1 {
                SHIPMENT_PUSH_TRIGGER_RETRY
            } else {
                SHIPMENT_PUSH_TRIGGER_AUTO
            };
            let env: Result<OutboxEnvelope, _> = serde_json::from_slice(&msg.payload);
            match env {
                Ok(mut env) => {
                    let effects_repo = WorkerEffectsRepo::new(db.pool());
                    let header_trace_id = msg
                        .headers
                        .as_ref()
                        .and_then(|headers| headers.get(TRACE_ID_HEADER))
                        .map(|value| value.as_str().to_string());
                    if env.trace_id.is_none() {
                        env.trace_id = header_trace_id;
                    }
                    let consume_lag_ms = Utc::now()
                        .signed_duration_since(env.produced_at)
                        .num_milliseconds()
                        .max(0) as u64;
                    metrics().inc_outbox_send_total();
                    if attempt > 1 {
                        metrics().inc_outbox_send_retry();
                    }
                    if (env.event_type == SHIPMENT_SHIPPED_EVENT
                        || env.event_type == SHIPMENT_CONFIRMED_EVENT)
                        && attempt > 1
                    {
                        metrics().inc_shipping_push_retry();
                    }
                    let claimed = match effects_repo
                        .claim_processing(env.tenant_id, env.outbox_id, &env.event_type)
                        .await
                    {
                        Ok(value) => value,
                        Err(err) => {
                            metrics().inc_worker_consume_failed();
                            metrics().inc_outbox_send_failed();
                            let err = err;
                            let outcome = dlq::handle_processing_failure(
                                db.pool(),
                                &env,
                                &err,
                                attempt,
                                &retry_policy,
                            )
                            .await;
                            match outcome {
                                Ok(dlq::FailureOutcome::Retry {
                                    delay,
                                    next_attempt,
                                    classification,
                                }) => {
                                    warn!(
                                        tenant_id = %env.tenant_id,
                                        outbox_id = %env.outbox_id,
                                        event_type = %env.event_type,
                                        attempt = attempt,
                                        next_attempt = next_attempt,
                                        delay_ms = delay.as_millis(),
                                        classification = %classification,
                                        error = %err,
                                        "worker effect claim failed; scheduling retry"
                                    );
                                    let subject = msg.subject.clone();
                                    let payload = msg.payload.clone();
                                    let headers =
                                        dlq::retry_headers(next_attempt, env.trace_id.as_deref());
                                    let nats = nats.clone();
                                    tokio::spawn(async move {
                                        tokio::time::sleep(delay).await;
                                        if let Err(err) = nats
                                            .publish_with_headers(subject, headers, payload)
                                            .await
                                        {
                                            error!(error=%err, "worker retry publish failed");
                                        }
                                    });
                                }
                                Ok(dlq::FailureOutcome::DeadLettered {
                                    dead_letter_id,
                                    classification,
                                }) => {
                                    metrics().inc_worker_dlq();
                                    error!(
                                        tenant_id = %env.tenant_id,
                                        outbox_id = %env.outbox_id,
                                        event_type = %env.event_type,
                                        attempt = attempt,
                                        classification = %classification,
                                        dead_letter_id = %dead_letter_id,
                                        error = %err,
                                        "worker effect claim moved to DLQ"
                                    );
                                }
                                Err(err) => {
                                    error!(
                                        tenant_id = %env.tenant_id,
                                        outbox_id = %env.outbox_id,
                                        event_type = %env.event_type,
                                        error = %err,
                                        "worker failure handling errored"
                                    );
                                }
                            }
                            metrics().add_worker_consume_latency_ms(
                                started.elapsed().as_millis() as u64,
                            );
                            metrics().add_worker_consume_lag_ms(consume_lag_ms);
                            return;
                        }
                    };

                    if !claimed {
                        metrics().inc_worker_consume_success();
                        metrics()
                            .add_worker_consume_latency_ms(started.elapsed().as_millis() as u64);
                        metrics().add_worker_consume_lag_ms(consume_lag_ms);
                        info!(
                            tenant_id = %env.tenant_id,
                            outbox_id = %env.outbox_id,
                            event_type = %env.event_type,
                            "worker event already processed; skipping"
                        );
                        return;
                    }

                    if let Err(e) = route_event(
                        &db,
                        adapters.as_ref(),
                        &nats,
                        env.clone(),
                        stock_sync_debounce,
                        attempt,
                        trigger,
                        replay_dead_letter_id,
                    )
                    .await
                    {
                        let _ = effects_repo
                            .mark_failed(env.tenant_id, env.outbox_id, &e.to_string())
                            .await;
                        metrics().inc_worker_consume_failed();
                        metrics().inc_outbox_send_failed();
                        let outcome = dlq::handle_processing_failure(
                            db.pool(),
                            &env,
                            &e,
                            attempt,
                            &retry_policy,
                        )
                        .await;
                        if env.event_type == SHIPMENT_SHIPPED_EVENT
                            || env.event_type == SHIPMENT_CONFIRMED_EVENT
                        {
                            let attempts_repo = ShipmentPushAttemptsRepo::new(db.pool());
                            let attempt_value = attempt.min(i32::MAX as u32) as i32;
                            match outcome.as_ref() {
                                Ok(dlq::FailureOutcome::Retry { delay, .. }) => {
                                    let next_retry_at = Utc::now()
                                        + ChronoDuration::from_std(*delay)
                                            .unwrap_or_else(|_| ChronoDuration::milliseconds(0));
                                    if let Err(err) = attempts_repo
                                        .update_attempt_outcome(
                                            env.tenant_id,
                                            env.outbox_id,
                                            attempt_value,
                                            SHIPMENT_PUSH_STATUS_RETRYING,
                                            Some(next_retry_at),
                                            replay_dead_letter_id,
                                        )
                                        .await
                                    {
                                        warn!(
                                            error = %err,
                                            "shipment push attempt status update failed"
                                        );
                                    }
                                }
                                Ok(dlq::FailureOutcome::DeadLettered {
                                    dead_letter_id, ..
                                }) => {
                                    if let Err(err) = attempts_repo
                                        .update_attempt_outcome(
                                            env.tenant_id,
                                            env.outbox_id,
                                            attempt_value,
                                            SHIPMENT_PUSH_STATUS_DEAD,
                                            None,
                                            Some(*dead_letter_id),
                                        )
                                        .await
                                    {
                                        warn!(
                                            error = %err,
                                            "shipment push attempt status update failed"
                                        );
                                    }
                                }
                                _ => {}
                            }
                        }
                        match outcome {
                            Ok(dlq::FailureOutcome::Retry {
                                delay,
                                next_attempt,
                                classification,
                            }) => {
                                warn!(
                                    tenant_id = %env.tenant_id,
                                    outbox_id = %env.outbox_id,
                                    event_type = %env.event_type,
                                    attempt = attempt,
                                    next_attempt = next_attempt,
                                    delay_ms = delay.as_millis(),
                                    classification = %classification,
                                    error = %e,
                                    "worker transient failure; scheduling retry"
                                );
                                let subject = msg.subject.clone();
                                let payload = msg.payload.clone();
                                let headers =
                                    dlq::retry_headers(next_attempt, env.trace_id.as_deref());
                                let nats = nats.clone();
                                tokio::spawn(async move {
                                    tokio::time::sleep(delay).await;
                                    if let Err(err) =
                                        nats.publish_with_headers(subject, headers, payload).await
                                    {
                                        error!(error=%err, "worker retry publish failed");
                                    }
                                });
                            }
                            Ok(dlq::FailureOutcome::DeadLettered {
                                dead_letter_id,
                                classification,
                            }) => {
                                metrics().inc_worker_dlq();
                                error!(
                                    tenant_id = %env.tenant_id,
                                    outbox_id = %env.outbox_id,
                                    event_type = %env.event_type,
                                    attempt = attempt,
                                    classification = %classification,
                                    dead_letter_id = %dead_letter_id,
                                    error = %e,
                                    "worker failure moved to DLQ"
                                );
                            }
                            Err(err) => {
                                error!(
                                    tenant_id = %env.tenant_id,
                                    outbox_id = %env.outbox_id,
                                    event_type = %env.event_type,
                                    error = %err,
                                    "worker failure handling errored"
                                );
                            }
                        }
                    } else {
                        if let Err(err) = effects_repo
                            .mark_succeeded(env.tenant_id, env.outbox_id)
                            .await
                        {
                            error!(
                                tenant_id = %env.tenant_id,
                                outbox_id = %env.outbox_id,
                                event_type = %env.event_type,
                                error = %err,
                                "worker effect mark succeeded failed"
                            );
                        }
                        metrics().inc_worker_consume_success();
                    }
                    metrics().add_worker_consume_latency_ms(started.elapsed().as_millis() as u64);
                    metrics().add_worker_consume_lag_ms(consume_lag_ms);
                }
                Err(e) => {
                    metrics().inc_worker_consume_failed();
                    metrics().inc_worker_dlq();
                    metrics().inc_outbox_send_failed();
                    warn!(error=%e, "bad envelope json");
                    metrics().add_worker_consume_latency_ms(started.elapsed().as_millis() as u64);
                    metrics().add_worker_consume_lag_ms(0);
                }
            }
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests;
