mod dlq;
mod event_router;
mod token_refresh;

use axum::{
    body::Body,
    http::{header, StatusCode},
    response::Response,
    routing::get,
    Router,
};
use chrono::{Duration as ChronoDuration, NaiveTime, Utc};
use clap::Parser;
use erp_adapter::{
    AdapterError, PlatformAdapter, PlatformTokenPayload, ShipOrderRequest, SyncCursor,
    SyncStockRequest,
};
use erp_common::{
    feature_flags::{self, FeatureFlagKey},
    sales_settings::{self, SalesShopConfig},
    settings::Settings,
    telemetry::{init_tracing, metrics, render_metrics, METRICS_CONTENT_TYPE},
    trace::TRACE_ID_HEADER,
};
use erp_domain::{
    oms::{
        apply_review_decision, derive_exception_status, derive_order_status, evaluate_conditions,
        OrderReviewCondition, OrderReviewContext, ReviewAction, ReviewMode,
    },
    DeadLetterReplayPayload, InboxReceivedPayload, InventoryChangedPayload, OrderProjection,
    OrderReviewedPayload, OrderUpsertedPayload, ProfitComputeRequestedPayload,
    RefundRequestedPayload, ShipmentFailedPayload, ShipmentRequestedPayload,
    ShipmentShippedPayload, DLQ_REPLAY_REQUESTED_EVENT, PROFIT_COMPUTE_REQUESTED_EVENT,
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
    inventory::{InventoryRepo, InventoryReservationAction, INVENTORY_CHANGED_EVENT},
    mapping_rules::MappingRulesRepo,
    operation_logs::{
        OperationLogCreate, OperationLogsRepo, OP_ACTION_OMS_REVIEW_AUTO_FLAG,
        OP_ACTION_OMS_REVIEW_AUTO_PASS, OP_ACTION_OMS_REVIEW_AUTO_REJECT,
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
        ShipmentPushAttemptCreate, ShipmentPushAttemptsRepo, SHIPMENT_PUSH_STATUS_DEAD,
        SHIPMENT_PUSH_STATUS_FAILED, SHIPMENT_PUSH_STATUS_RETRYING, SHIPMENT_PUSH_STATUS_SUCCESS,
    },
    shipments::{
        ShipmentsRepo, SHIPMENT_CONFIRMED_EVENT, SHIPMENT_FAILED_EVENT, SHIPMENT_REQUESTED_EVENT,
        SHIPMENT_SHIPPED_EVENT,
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
use token_refresh::{handle_token_refresh_requested, refresh_tokens_once};
use futures::StreamExt;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::{collections::HashMap, net::SocketAddr, str::FromStr, sync::Arc, time::Duration};
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

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

struct AdapterRegistry {
    taobao: erp_adapter_taobao::Adapter,
    jd: erp_adapter_jd::Adapter,
    pdd: erp_adapter_pdd::Adapter,
    douyin: erp_adapter_douyin::Adapter,
}

impl AdapterRegistry {
    fn new() -> Self {
        Self {
            taobao: erp_adapter_taobao::Adapter::new(),
            jd: erp_adapter_jd::Adapter::new(),
            pdd: erp_adapter_pdd::Adapter::new(),
            douyin: erp_adapter_douyin::Adapter::new(),
        }
    }

    fn platform_adapter(&self, platform: Platform) -> &dyn PlatformAdapter {
        match platform {
            Platform::Taobao => &self.taobao,
            Platform::Jd => &self.jd,
            Platform::Pdd => &self.pdd,
            Platform::Douyin => &self.douyin,
        }
    }
}

async fn is_pdd_only_mode(db: &Db, tenant_id: TenantId) -> anyhow::Result<bool> {
    if let Some(enabled) = feature_flags::pdd_only_mode_env() {
        return Ok(enabled);
    }
    let tenants_repo = TenantsRepo::new(db.pool());
    tenants_repo.is_pdd_only_mode(tenant_id).await
}

async fn feature_flag_enabled(
    db: &Db,
    tenant_id: TenantId,
    shop_id: Option<ShopId>,
    key: FeatureFlagKey,
) -> anyhow::Result<bool> {
    let repo = FeatureFlagsRepo::new(db.pool());
    let flag = FeatureFlag::from_common_key(key);
    repo.is_enabled(tenant_id, shop_id, flag).await
}

async fn read_only_enabled(db: &Db, tenant_id: TenantId) -> anyhow::Result<bool> {
    let repo = FeatureFlagsRepo::new(db.pool());
    repo.is_read_only(tenant_id, None).await
}

async fn handle_inbox_received(
    db: &Db,
    adapters: &AdapterRegistry,
    payload: InboxReceivedPayload,
) -> anyhow::Result<()> {
    if read_only_enabled(db, payload.tenant_id).await? {
        warn!(
            tenant_id = %payload.tenant_id,
            inbox_event_id = %payload.inbox_event_id,
            "read-only mode active; skipping inbox event"
        );
        return Ok(());
    }
    metrics().inc_webhook_inbox_received();
    let inbox_repo = InboxRepo::new(db.pool());
    let shops_repo = ShopsRepo::new(db.pool());
    let orders_repo = OrdersRepo::new(db.pool());
    let outbox_repo = OutboxRepo::new(db.pool());
    let mapping_rules_repo = MappingRulesRepo::new(db.pool());

    // 1) Load event raw
    let evt = inbox_repo
        .get_event_with_raw(payload.tenant_id, payload.inbox_event_id)
        .await?;
    record_inbox_dedup_hits(db, payload.tenant_id, payload.inbox_event_id).await;
    let shop = shops_repo.get_shop_auth(evt.shop_id).await?;

    if is_pdd_only_mode(db, evt.tenant_id).await? && evt.platform != Platform::Pdd {
        warn!(
            tenant_id = %evt.tenant_id,
            shop_id = %evt.shop_id,
            platform = %evt.platform,
            "pdd-only mode blocked webhook event"
        );
        inbox_repo
            .mark_failed(
                evt.tenant_id,
                payload.inbox_event_id,
                "pdd-only mode: platform not allowed",
            )
            .await?;
        return Ok(());
    }

    // 2) Verify + parse
    let adapter = adapters.platform_adapter(evt.platform);
    if let Err(err) = adapter
        .verify_signature(&shop, &evt.headers, &evt.body)
        .await
    {
        warn!(
            tenant_id = %evt.tenant_id,
            shop_id = %evt.shop_id,
            platform = %evt.platform,
            error = %err,
            "webhook signature verification failed"
        );
        inbox_repo
            .mark_verify_failed(
                evt.tenant_id,
                payload.inbox_event_id,
                &format!("verify failed: {err}"),
            )
            .await?;
        return Ok(());
    }

    let orders = match adapter
        .parse_webhook_orders(&shop, &evt.headers, &evt.body)
        .await
    {
        Ok(orders) => orders,
        Err(err) => {
            warn!(
                tenant_id = %evt.tenant_id,
                shop_id = %evt.shop_id,
                platform = %evt.platform,
                error = %err,
                "webhook parse failed"
            );
            inbox_repo
                .mark_failed(
                    evt.tenant_id,
                    payload.inbox_event_id,
                    &format!("parse failed: {err}"),
                )
                .await?;
            return Ok(());
        }
    };
    let mapping_version = mapping_rules_repo
        .latest_enabled_version(evt.tenant_id, evt.platform, "order")
        .await?;

    // 3) Apply projections
    for mut order in orders {
        annotate_mapping_version(&mut order, mapping_version);
        apply_sku_mappings(db.pool(), &outbox_repo, &mut order).await?;
        let order_id = orders_repo.upsert_order_projection(&order).await?;
        // Emit event for downstream (e.g., notify UI, trigger fulfillment)
        let out_payload = serde_json::to_value(OrderUpsertedPayload {
            tenant_id: order.tenant_id,
            platform: order.platform,
            shop_id: order.shop_id,
            platform_order_id: order.platform_order_id.clone(),
            order_id,
        })?;
        outbox_repo
            .enqueue(order.tenant_id, "ORDER_IMPORTED", &out_payload)
            .await?;
    }

    inbox_repo
        .mark_processed(
            payload.tenant_id,
            payload.inbox_event_id,
            chrono::Utc::now(),
        )
        .await?;

    Ok(())
}

async fn handle_dead_letter_replay(
    db: &Db,
    nats: &async_nats::Client,
    payload: DeadLetterReplayPayload,
) -> anyhow::Result<()> {
    dlq::replay_dead_letter(
        db.pool(),
        nats,
        uuid::Uuid::from(payload.tenant_id),
        payload.dead_letter_id,
    )
    .await?;
    Ok(())
}

async fn handle_order_upserted(db: &Db, payload: OrderUpsertedPayload) -> anyhow::Result<()> {
    if read_only_enabled(db, payload.tenant_id).await? {
        warn!(
            tenant_id = %payload.tenant_id,
            order_id = %payload.order_id,
            "read-only mode active; skipping order review and after-sale sync"
        );
        return Ok(());
    }

    let sales_config =
        match load_sales_config_for_shop(db.pool(), payload.tenant_id, payload.shop_id).await {
            Ok(v) => v,
            Err(err) => {
                warn!(
                    tenant_id = %payload.tenant_id,
                    shop_id = %payload.shop_id,
                    error = %err,
                    "failed to load sales config; fallback to defaults"
                );
                SalesShopConfig::default()
            }
        };
    if sales_config.auto_to_review {
        handle_order_review(db, &payload).await?;
    } else {
        auto_pass_order_review(db, &payload).await?;
    }
    apply_default_order_tags(db, &payload).await?;
    if is_pdd_only_mode(db, payload.tenant_id).await? && payload.platform != Platform::Pdd {
        warn!(
            tenant_id = %payload.tenant_id,
            platform = %payload.platform,
            order_id = %payload.order_id,
            "pdd-only mode blocked after-sale sync"
        );
        return Ok(());
    }

    let aftersales_enabled = feature_flag_enabled(
        db,
        payload.tenant_id,
        Some(payload.shop_id),
        FeatureFlagKey::AftersalesWrite,
    )
    .await?;
    if !aftersales_enabled {
        warn!(
            tenant_id = %payload.tenant_id,
            shop_id = %payload.shop_id,
            order_id = %payload.order_id,
            "feature disabled: aftersales write"
        );
        return Ok(());
    }

    let cases_repo = AfterSaleCaseRepo::new(db.pool());
    cases_repo
        .sync_cases_for_order(
            payload.tenant_id,
            payload.platform,
            payload.shop_id,
            payload.order_id,
        )
        .await?;

    auto_allocate_order(db, payload.tenant_id, payload.shop_id, payload.order_id).await?;
    Ok(())
}

async fn handle_order_reviewed(db: &Db, payload: OrderReviewedPayload) -> anyhow::Result<()> {
    if read_only_enabled(db, payload.tenant_id).await? {
        warn!(
            tenant_id = %payload.tenant_id,
            order_id = %payload.order_id,
            "read-only mode active; skipping order allocation"
        );
        return Ok(());
    }
    let reviewed = OrderReviewStatus::from_str(&payload.review_status)
        .map(|status| status.is_approved())
        .unwrap_or(false);
    if !reviewed {
        return Ok(());
    }
    auto_allocate_order(db, payload.tenant_id, payload.shop_id, payload.order_id).await
}

async fn apply_default_order_tags(db: &Db, payload: &OrderUpsertedPayload) -> anyhow::Result<()> {
    let tag_enabled =
        feature_flag_enabled(db, payload.tenant_id, None, FeatureFlagKey::TagSystem).await?;
    if !tag_enabled {
        return Ok(());
    }

    let tags_repo = OrderTagDefinitionsRepo::new(db.pool());
    let definitions = tags_repo.list_auto_apply_tags(payload.tenant_id).await?;
    if definitions.is_empty() {
        return Ok(());
    }

    let orders_repo = OrdersRepo::new(db.pool());
    let Some(detail) = orders_repo
        .get_order_detail(payload.tenant_id, payload.order_id)
        .await?
    else {
        return Ok(());
    };

    let mut context = build_review_context(&detail);
    context.insert_field(
        "shop_id",
        serde_json::Value::String(payload.shop_id.to_string()),
    );
    context.insert_field(
        "platform_order_id",
        serde_json::Value::String(detail.order.platform_order_id.clone()),
    );
    if let Some(address) = detail.order.receiver_address.as_ref() {
        if let Some(province) = extract_province(address) {
            context.insert_field("address_province", serde_json::Value::String(province));
        }
        if address.trim().is_empty() {
            context.insert_field("address_invalid", serde_json::Value::Bool(true));
        }
    } else {
        context.insert_field("address_invalid", serde_json::Value::Bool(true));
    }

    let existing_tags: std::collections::HashSet<String> =
        detail.tags.iter().map(|tag| tag.tag.clone()).collect();
    let mut matched_tags: Vec<String> = Vec::new();

    for definition in definitions {
        if existing_tags.contains(&definition.name) {
            continue;
        }
        let conditions: Vec<OrderReviewCondition> =
            match serde_json::from_value(definition.conditions_json.clone()) {
                Ok(value) => value,
                Err(err) => {
                    warn!(
                        tenant_id = %payload.tenant_id,
                        tag = %definition.name,
                        error = %err,
                        "invalid auto-apply conditions"
                    );
                    continue;
                }
            };
        let matched = if conditions.is_empty() {
            true
        } else {
            evaluate_conditions(&context, &conditions)
        };
        if matched {
            matched_tags.push(definition.name);
        }
    }

    if matched_tags.is_empty() {
        return Ok(());
    }

    let created_tags = matched_tags
        .iter()
        .map(|tag| OrderTagCreate {
            tag: tag.to_string(),
            created_by: None,
        })
        .collect::<Vec<_>>();
    let _ = orders_repo
        .add_order_tags(payload.tenant_id, payload.order_id, created_tags)
        .await?;

    let audit_repo = AuditLogsRepo::new(db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id: payload.tenant_id,
            user_id: None,
            action: "OMS.ORDER_TAG_AUTO".to_string(),
            entity_type: "ORDER".to_string(),
            entity_id: Some(payload.order_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({ "tags": matched_tags, "source": "AUTO" })),
            ip: None,
            ua: None,
        })
        .await;

    Ok(())
}

async fn handle_sync_job_run(db: &Db, payload: SyncJobReplayPayload) -> anyhow::Result<()> {
    let jobs_repo = SyncJobsRepo::new(db.pool());
    let Some(job) = jobs_repo.get_job(payload.tenant_id, payload.job_id).await? else {
        warn!(
            tenant_id = %payload.tenant_id,
            job_id = %payload.job_id,
            "sync job replay ignored: job missing"
        );
        return Ok(());
    };

    let config = load_sales_config_for_shop(db.pool(), payload.tenant_id, job.shop_id).await?;
    let disable_reason = match job.job_type.as_str() {
        "order_sync" if !config.auto_download_orders => Some("sales auto download disabled"),
        "refund_sync" if !config.enable_after_sales_download => {
            Some("after-sales download disabled")
        }
        "shipping_sync" if !config.enable_logistics_upload => Some("logistics upload disabled"),
        _ => None,
    };
    if let Some(reason) = disable_reason {
        let _ = jobs_repo
            .update_job_status(
                payload.tenant_id,
                job.shop_id,
                &job.job_type,
                "DISABLED",
                Some(reason.to_string()),
            )
            .await?;
        return Ok(());
    }

    let _ = jobs_repo
        .update_job_result(
            payload.tenant_id,
            payload.job_id,
            "PENDING",
            job.cursor.clone(),
            job.watermark,
            Some(SYNC_JOB_FORCE_RUN_MARKER.to_string()),
        )
        .await?;
    Ok(())
}

async fn handle_profit_compute_requested(
    db: &Db,
    payload: ProfitComputeRequestedPayload,
) -> anyhow::Result<()> {
    if read_only_enabled(db, payload.tenant_id).await? {
        warn!(
            tenant_id = %payload.tenant_id,
            order_id = %payload.order_id,
            "read-only mode active; skipping profit compute"
        );
        return Ok(());
    }

    let profit_repo = ProfitSnapshotsRepo::new(db.pool());
    let (revenue_cent, cost_cent, platform_fee_cent, shipping_fee_cent) = profit_repo
        .compute_profit_inputs(payload.tenant_id, payload.order_id)
        .await?;
    let gross_profit_cent = revenue_cent - cost_cent - platform_fee_cent - shipping_fee_cent;
    let computed_at = Utc::now();
    let snapshot = profit_repo
        .upsert_snapshot(ProfitSnapshotUpsert {
            tenant_id: payload.tenant_id,
            order_id: payload.order_id,
            revenue_cent,
            cost_cent,
            platform_fee_cent,
            shipping_fee_cent,
            gross_profit_cent,
            computed_at,
        })
        .await?;

    let audit_repo = AuditLogsRepo::new(db.pool());
    audit_repo
        .insert(AuditLogCreate {
            tenant_id: payload.tenant_id,
            user_id: None,
            action: "FINANCE.PROFIT_SNAPSHOT_UPSERT".to_string(),
            entity_type: "profit_snapshot".to_string(),
            entity_id: Some(payload.order_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::to_value(&snapshot)?),
            ip: None,
            ua: None,
        })
        .await?;

    Ok(())
}

async fn handle_order_review(db: &Db, payload: &OrderUpsertedPayload) -> anyhow::Result<()> {
    let orders_repo = OrdersRepo::new(db.pool());
    let rules_repo = OrderReviewRulesRepo::new(db.pool());
    let blacklist_repo = CustomerBlacklistRepo::new(db.pool());

    let Some(state) = orders_repo
        .get_order_review_state(payload.tenant_id, payload.order_id)
        .await?
    else {
        return Ok(());
    };
    if !state.review_status.is_pending() {
        return Ok(());
    }

    let Some(detail) = orders_repo
        .get_order_detail(payload.tenant_id, payload.order_id)
        .await?
    else {
        return Ok(());
    };

    let mut context = build_review_context(&detail);
    if let Some(address) = detail.order.receiver_address.as_ref() {
        if let Some(province) = extract_province(address) {
            context.insert_field("address_province", serde_json::Value::String(province));
        }
        if address.trim().is_empty() {
            context.insert_field("address_invalid", serde_json::Value::Bool(true));
        }
    } else {
        context.insert_field("address_invalid", serde_json::Value::Bool(true));
    }

    let rules = rules_repo.list_enabled_rules(payload.tenant_id).await?;
    let mut hits: Vec<(uuid::Uuid, bool, String)> = Vec::new();
    let mut decision: Option<ReviewAction> = None;
    let mut extra_flags: Vec<String> = Vec::new();

    for rule in rules {
        let conditions: Vec<OrderReviewCondition> =
            serde_json::from_value(rule.conditions_json.clone()).unwrap_or_default();
        let matched = if conditions.is_empty() {
            false
        } else {
            evaluate_conditions(&context, &conditions)
        };
        let message = if matched {
            if rule.reason.is_empty() {
                "matched rule".to_string()
            } else {
                rule.reason.clone()
            }
        } else {
            String::new()
        };
        hits.push((rule.rule_id, matched, message.clone()));

        if matched {
            let action = match rule.action.to_uppercase().as_str() {
                "REJECT" => ReviewAction::Reject,
                "FLAG" => ReviewAction::Flag,
                _ => ReviewAction::Pass,
            };
            if !rule.reason.trim().is_empty() {
                extra_flags.push(rule.reason.clone());
            }
            match action {
                ReviewAction::Reject => {
                    decision = Some(ReviewAction::Reject);
                    break;
                }
                ReviewAction::Pass => {
                    decision = Some(ReviewAction::Pass);
                    break;
                }
                ReviewAction::Flag => {
                    decision = Some(ReviewAction::Flag);
                }
            }
        }
    }

    if has_mapping_missing(&detail) {
        extra_flags.push("MAPPING_MISSING".to_string());
        if !matches!(decision, Some(ReviewAction::Reject)) {
            decision = Some(ReviewAction::Flag);
        }
    }

    if has_invalid_address(&detail) {
        extra_flags.push("ADDRESS_INVALID".to_string());
        if !matches!(decision, Some(ReviewAction::Reject)) {
            decision = Some(ReviewAction::Flag);
        }
    }

    if let Some(phone) = detail.order.receiver_phone.as_ref() {
        if blacklist_repo
            .find_entry(payload.tenant_id, "phone", phone)
            .await?
            .is_some()
        {
            extra_flags.push("BLACKLISTED_CUSTOMER".to_string());
            if !matches!(decision, Some(ReviewAction::Reject)) {
                decision = Some(ReviewAction::Flag);
            }
        }
    }

    if is_duplicate_order(db.pool(), payload.tenant_id, &detail).await? {
        extra_flags.push("DUPLICATE_ORDER".to_string());
        if !matches!(decision, Some(ReviewAction::Reject)) {
            decision = Some(ReviewAction::Flag);
        }
    }

    let action = decision.unwrap_or(ReviewAction::Pass);
    let outcome = apply_review_decision(
        state.oms_status,
        action,
        &state.exception_flags,
        None,
        ReviewMode::Auto,
    )
    .map_err(|err| anyhow::anyhow!(err))?;
    let mut exception_flags = outcome.exception_flags;
    for flag in extra_flags {
        if !exception_flags.iter().any(|existing| existing == &flag) {
            exception_flags.push(flag);
        }
    }

    let exception_status = derive_exception_status(&exception_flags);
    let order_status = derive_order_status(
        outcome.oms_status,
        outcome.review_status,
        state.fulfillment_status,
        exception_status,
    );
    let updated = orders_repo
        .update_order_review_state(
            payload.tenant_id,
            payload.order_id,
            erp_repo::orders::OrderReviewUpdate {
                oms_status: outcome.oms_status,
                order_status,
                review_status: outcome.review_status,
                fulfillment_status: state.fulfillment_status,
                exception_status,
                exception_flags: exception_flags.clone(),
                expected_version: Some(state.version),
            },
        )
        .await?;

    if let Some(updated) = updated {
        let review_repo = OrderReviewsRepo::new(db.pool());
        let record_status = review_record_status(updated.review_status).to_string();
        let reviewed_at = if record_status == ORDER_REVIEW_RECORD_PENDING {
            None
        } else {
            Some(Utc::now())
        };
        let _ = review_repo
            .upsert_review(OrderReviewUpsert {
                tenant_id: payload.tenant_id,
                order_id: payload.order_id,
                status: record_status,
                reviewed_by: None,
                reviewed_at,
                notes: String::new(),
            })
            .await;

        orders_repo
            .upsert_review_hits(payload.tenant_id, payload.order_id, hits)
            .await?;
        let _ = orders_repo
            .upsert_exception_tickets(
                payload.tenant_id,
                payload.order_id,
                &updated.exception_flags,
            )
            .await;

        let audit_repo = AuditLogsRepo::new(db.pool());
        let action_label = match action {
            ReviewAction::Pass => "OMS.REVIEW_AUTO_PASS",
            ReviewAction::Reject => "OMS.REVIEW_AUTO_REJECT",
            ReviewAction::Flag => "OMS.REVIEW_AUTO_FLAG",
        };
        let _ = audit_repo
            .insert(AuditLogCreate {
                tenant_id: payload.tenant_id,
                user_id: None,
                action: action_label.to_string(),
                entity_type: "ORDER".to_string(),
                entity_id: Some(payload.order_id.to_string()),
                before_json: Some(serde_json::json!({
                    "oms_status": state.oms_status,
                    "review_status": state.review_status,
                    "exception_flags": state.exception_flags,
                })),
                after_json: Some(serde_json::json!({
                    "oms_status": updated.oms_status,
                    "review_status": updated.review_status,
                    "exception_flags": updated.exception_flags,
                })),
                ip: None,
                ua: None,
            })
            .await;

        let operation_action = match action {
            ReviewAction::Pass => OP_ACTION_OMS_REVIEW_AUTO_PASS,
            ReviewAction::Reject => OP_ACTION_OMS_REVIEW_AUTO_REJECT,
            ReviewAction::Flag => OP_ACTION_OMS_REVIEW_AUTO_FLAG,
        };
        let logs_repo = OperationLogsRepo::new(db.pool());
        let _ = logs_repo
            .insert(OperationLogCreate {
                tenant_id: payload.tenant_id,
                actor_type: "SYSTEM".to_string(),
                actor_user_id: None,
                action: operation_action.to_string(),
                ref_type: Some("ORDER".to_string()),
                ref_id: Some(payload.order_id.to_string()),
                metadata: Some(serde_json::json!({
                    "review_status": updated.review_status.to_string(),
                    "exception_flags": updated.exception_flags,
                })),
            })
            .await;

        let outbox_repo = OutboxRepo::new(db.pool());
        let out_payload = serde_json::json!({
            "tenant_id": payload.tenant_id,
            "order_id": payload.order_id,
            "review_status": outcome.review_status.to_string(),
            "oms_status": outcome.oms_status.to_string(),
            "exception_flags": exception_flags,
        });
        outbox_repo
            .enqueue(payload.tenant_id, "ORDER_REVIEWED", &out_payload)
            .await?;
    }

    Ok(())
}

async fn auto_pass_order_review(db: &Db, payload: &OrderUpsertedPayload) -> anyhow::Result<()> {
    let orders_repo = OrdersRepo::new(db.pool());
    let Some(state) = orders_repo
        .get_order_review_state(payload.tenant_id, payload.order_id)
        .await?
    else {
        return Ok(());
    };
    if !state.review_status.is_pending() {
        return Ok(());
    }

    let outcome = apply_review_decision(
        state.oms_status,
        ReviewAction::Pass,
        &state.exception_flags,
        None,
        ReviewMode::Auto,
    )
    .map_err(|err| anyhow::anyhow!(err))?;
    let exception_status = derive_exception_status(&outcome.exception_flags);
    let order_status = derive_order_status(
        outcome.oms_status,
        outcome.review_status,
        state.fulfillment_status,
        exception_status,
    );
    let updated = orders_repo
        .update_order_review_state(
            payload.tenant_id,
            payload.order_id,
            erp_repo::orders::OrderReviewUpdate {
                oms_status: outcome.oms_status,
                order_status,
                review_status: outcome.review_status,
                fulfillment_status: state.fulfillment_status,
                exception_status,
                exception_flags: outcome.exception_flags.clone(),
                expected_version: Some(state.version),
            },
        )
        .await?;

    if let Some(updated) = updated {
        let review_repo = OrderReviewsRepo::new(db.pool());
        let record_status = review_record_status(updated.review_status).to_string();
        let reviewed_at = if record_status == ORDER_REVIEW_RECORD_PENDING {
            None
        } else {
            Some(Utc::now())
        };
        let _ = review_repo
            .upsert_review(OrderReviewUpsert {
                tenant_id: payload.tenant_id,
                order_id: payload.order_id,
                status: record_status,
                reviewed_by: None,
                reviewed_at,
                notes: String::new(),
            })
            .await;

        let _ = orders_repo
            .upsert_exception_tickets(
                payload.tenant_id,
                payload.order_id,
                &updated.exception_flags,
            )
            .await;
        let audit_repo = AuditLogsRepo::new(db.pool());
        let _ = audit_repo
            .insert(AuditLogCreate {
                tenant_id: payload.tenant_id,
                user_id: None,
                action: "OMS.REVIEW_AUTO_PASS".to_string(),
                entity_type: "ORDER".to_string(),
                entity_id: Some(payload.order_id.to_string()),
                before_json: Some(serde_json::json!({
                    "oms_status": state.oms_status,
                    "review_status": state.review_status,
                    "exception_flags": state.exception_flags,
                })),
                after_json: Some(serde_json::json!({
                    "oms_status": updated.oms_status,
                    "review_status": updated.review_status,
                    "exception_flags": updated.exception_flags,
                })),
                ip: None,
                ua: None,
            })
            .await;

        let logs_repo = OperationLogsRepo::new(db.pool());
        let _ = logs_repo
            .insert(OperationLogCreate {
                tenant_id: payload.tenant_id,
                actor_type: "SYSTEM".to_string(),
                actor_user_id: None,
                action: OP_ACTION_OMS_REVIEW_AUTO_PASS.to_string(),
                ref_type: Some("ORDER".to_string()),
                ref_id: Some(payload.order_id.to_string()),
                metadata: Some(serde_json::json!({
                    "review_status": updated.review_status.to_string(),
                    "exception_flags": updated.exception_flags,
                })),
            })
            .await;

        let outbox_repo = OutboxRepo::new(db.pool());
        let out_payload = serde_json::json!({
            "tenant_id": payload.tenant_id,
            "order_id": payload.order_id,
            "shop_id": payload.shop_id,
            "platform": payload.platform,
            "review_status": outcome.review_status.to_string(),
            "oms_status": outcome.oms_status.to_string(),
            "exception_flags": outcome.exception_flags,
        });
        outbox_repo
            .enqueue(payload.tenant_id, "ORDER_REVIEWED", &out_payload)
            .await?;
    }

    Ok(())
}

fn build_review_context(detail: &erp_repo::orders::OmsOrderDetail) -> OrderReviewContext {
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
        "oms_status",
        serde_json::Value::String(detail.order.oms_status.to_string()),
    );
    context.insert_field(
        "review_status",
        serde_json::Value::String(detail.order.review_status.to_string()),
    );
    context.insert_field(
        "platform",
        serde_json::Value::String(detail.order.platform.to_string()),
    );
    context
}

fn extract_province(address: &str) -> Option<String> {
    const PROVINCES: [&str; 34] = [
        "北京",
        "天津",
        "河北",
        "山西",
        "内蒙古",
        "辽宁",
        "吉林",
        "黑龙江",
        "上海",
        "江苏",
        "浙江",
        "安徽",
        "福建",
        "江西",
        "山东",
        "河南",
        "湖北",
        "湖南",
        "广东",
        "广西",
        "海南",
        "重庆",
        "四川",
        "贵州",
        "云南",
        "西藏",
        "陕西",
        "甘肃",
        "青海",
        "宁夏",
        "新疆",
        "香港",
        "澳门",
        "台湾",
    ];
    let trimmed = address.trim();
    if trimmed.is_empty() {
        return None;
    }
    for province in PROVINCES {
        if trimmed.contains(province) {
            return Some(province.to_string());
        }
    }
    None
}

fn has_mapping_missing(detail: &erp_repo::orders::OmsOrderDetail) -> bool {
    detail.items.iter().any(|item| item.mapping_missing)
}

fn has_invalid_address(detail: &erp_repo::orders::OmsOrderDetail) -> bool {
    detail
        .order
        .receiver_address
        .as_deref()
        .map(|value| value.trim().len() < 6)
        .unwrap_or(true)
}

async fn is_duplicate_order(
    pool: &PgPool,
    tenant_id: TenantId,
    detail: &erp_repo::orders::OmsOrderDetail,
) -> anyhow::Result<bool> {
    let Some(phone) = detail.order.receiver_phone.as_ref() else {
        return Ok(false);
    };
    let count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from orders
        where tenant_id = $1
          and receiver_phone = $2
          and total_amount_cent = $3
          and order_id <> $4
          and order_created_at >= $5
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(phone)
    .bind(detail.order.total_amount_cent)
    .bind(uuid::Uuid::from(detail.order.id))
    .bind(detail.order.order_created_at - ChronoDuration::days(7))
    .fetch_one(pool)
    .await?;

    Ok(count > 0)
}

async fn auto_allocate_order(
    db: &Db,
    tenant_id: TenantId,
    shop_id: ShopId,
    order_id: OrderId,
) -> anyhow::Result<()> {
    let inventory_enabled =
        feature_flag_enabled(db, tenant_id, Some(shop_id), FeatureFlagKey::InventoryWrite).await?;
    if !inventory_enabled {
        warn!(
            tenant_id = %tenant_id,
            shop_id = %shop_id,
            order_id = %order_id,
            "feature disabled: inventory write"
        );
        return Ok(());
    }

    let orders_repo = OrdersRepo::new(db.pool());
    if let Some(order) = orders_repo
        .get_order_shipping_info(tenant_id, order_id)
        .await?
    {
        if order.status != erp_types::OrderStatus::PendingShipment
            && order.status != erp_types::OrderStatus::Allocated
        {
            return Ok(());
        }
    }

    let fulfillment_repo = FulfillmentRepo::new(db.pool());
    let result = fulfillment_repo
        .allocate_order(tenant_id, order_id, None)
        .await?;
    if !result.shortages.is_empty() {
        warn!(
            tenant_id = %tenant_id,
            order_id = %order_id,
            shortage_count = result.shortages.len(),
            "allocation shortages detected"
        );
    }
    Ok(())
}

async fn handle_shipment_requested(
    db: &Db,
    adapters: &AdapterRegistry,
    payload: ShipmentRequestedPayload,
) -> anyhow::Result<()> {
    if read_only_enabled(db, payload.tenant_id).await? {
        warn!(
            tenant_id = %payload.tenant_id,
            shipment_id = %payload.shipment_id,
            "read-only mode active; skipping shipment request"
        );
        return Ok(());
    }
    let shipments_repo = ShipmentsRepo::new(db.pool());
    let orders_repo = OrdersRepo::new(db.pool());
    let shops_repo = ShopsRepo::new(db.pool());
    let inventory_repo = InventoryRepo::new(db.pool());
    let serials_repo = SkuSerialsRepo::new(db.pool());
    let waybills_repo = WaybillsRepo::new(db.pool());
    let outbox_repo = OutboxRepo::new(db.pool());

    let Some(shipment) = shipments_repo
        .get_by_id(payload.tenant_id, payload.shipment_id)
        .await?
    else {
        return Err(anyhow::anyhow!("shipment not found"));
    };

    if shipment.status == "SUCCESS" {
        return Ok(());
    }

    let shipping_enabled = feature_flag_enabled(
        db,
        payload.tenant_id,
        Some(payload.shop_id),
        FeatureFlagKey::ShippingWrite,
    )
    .await?;
    if !shipping_enabled {
        warn!(
            tenant_id = %payload.tenant_id,
            shop_id = %payload.shop_id,
            shipment_id = %payload.shipment_id,
            "feature disabled: shipping write"
        );
        return Ok(());
    }

    if is_pdd_only_mode(db, payload.tenant_id).await? && payload.platform != Platform::Pdd {
        warn!(
            tenant_id = %payload.tenant_id,
            platform = %payload.platform,
            shipment_id = %payload.shipment_id,
            "pdd-only mode blocked shipment request"
        );
        shipments_repo
            .mark_failed(payload.tenant_id, payload.shipment_id)
            .await?;
        let failure = ShipmentFailedPayload {
            tenant_id: payload.tenant_id,
            shipment_id: payload.shipment_id,
            order_id: payload.order_id,
            shop_id: payload.shop_id,
            platform: payload.platform,
            platform_order_id: payload.platform_order_id,
            warehouse_id: payload.warehouse_id,
            carrier_code: payload.carrier_code,
            tracking_no: payload.tracking_no,
            last_error: "pdd-only mode: platform not allowed".to_string(),
        };
        let fail_payload = serde_json::to_value(failure)?;
        outbox_repo
            .enqueue(payload.tenant_id, SHIPMENT_FAILED_EVENT, &fail_payload)
            .await?;
        metrics().inc_shipping_failed();
        return Ok(());
    }

    let shop = shops_repo.get_shop_auth(payload.shop_id).await?;
    let adapter = adapters.platform_adapter(payload.platform);
    let request = ShipOrderRequest {
        platform_order_id: payload.platform_order_id.clone(),
        warehouse_id: payload.warehouse_id,
        carrier_code: payload.carrier_code.clone(),
        tracking_no: payload.tracking_no.clone(),
    };

    let ship_result = adapter.ship_order(&shop, request).await;
    let shipped_at = chrono::Utc::now();

    if let Err(err) = ship_result {
        if !matches!(err, erp_adapter::AdapterError::Unsupported) {
            shipments_repo
                .mark_failed(payload.tenant_id, payload.shipment_id)
                .await?;
            let failure = ShipmentFailedPayload {
                tenant_id: payload.tenant_id,
                shipment_id: payload.shipment_id,
                order_id: payload.order_id,
                shop_id: payload.shop_id,
                platform: payload.platform,
                platform_order_id: payload.platform_order_id,
                warehouse_id: payload.warehouse_id,
                carrier_code: payload.carrier_code,
                tracking_no: payload.tracking_no,
                last_error: err.to_string(),
            };
            let fail_payload = serde_json::to_value(failure)?;
            outbox_repo
                .enqueue(payload.tenant_id, SHIPMENT_FAILED_EVENT, &fail_payload)
                .await?;
            metrics().inc_shipping_failed();
            return Err(anyhow::anyhow!(err));
        }
    }

    shipments_repo
        .mark_success(payload.tenant_id, payload.shipment_id, shipped_at)
        .await?;
    orders_repo
        .mark_order_shipped(payload.tenant_id, payload.order_id, shipped_at)
        .await?;
    if let Err(err) = waybills_repo
        .mark_used(payload.tenant_id, payload.shipment_id)
        .await
    {
        warn!(
            tenant_id = %payload.tenant_id,
            shipment_id = %payload.shipment_id,
            error = %err,
            "failed to mark waybill used"
        );
    }

    let inventory_enabled = feature_flag_enabled(
        db,
        payload.tenant_id,
        Some(payload.shop_id),
        FeatureFlagKey::InventoryWrite,
    )
    .await?;
    if !inventory_enabled {
        warn!(
            tenant_id = %payload.tenant_id,
            shop_id = %payload.shop_id,
            order_id = %payload.order_id,
            "feature disabled: inventory write"
        );
        return Ok(());
    }

    let items = orders_repo
        .list_order_items(payload.tenant_id, payload.order_id)
        .await?;
    for item in items {
        let Some(sku_raw) = item.sku_id else {
            warn!(
                tenant_id = %payload.tenant_id,
                order_id = %payload.order_id,
                line_id = %item.line_id,
                "skip inventory consume: missing sku_id"
            );
            continue;
        };
        let sku_id = match uuid::Uuid::parse_str(&sku_raw) {
            Ok(v) => v,
            Err(_) => {
                warn!(
                    tenant_id = %payload.tenant_id,
                    order_id = %payload.order_id,
                    line_id = %item.line_id,
                    sku_id = %sku_raw,
                    "skip inventory consume: invalid sku_id"
                );
                continue;
            }
        };
        let order_item_id = order_item_uuid(payload.order_id, &item.line_id);
        inventory_repo
            .consume_reservation(InventoryReservationAction {
                tenant_id: payload.tenant_id,
                warehouse_id: payload.warehouse_id,
                sku_id,
                order_item_id,
                qty: item.qty,
            })
            .await?;
        metrics().inc_inventory_consume();
    }

    serials_repo
        .mark_shipment_serials_shipped(payload.tenant_id, payload.shipment_id)
        .await?;

    metrics().inc_shipping_success();
    Ok(())
}

async fn handle_inventory_changed(
    db: &Db,
    payload: InventoryChangedPayload,
    debounce: ChronoDuration,
) -> anyhow::Result<()> {
    if read_only_enabled(db, payload.tenant_id).await? {
        warn!(
            tenant_id = %payload.tenant_id,
            sku_id = %payload.sku_id,
            "read-only mode active; skipping inventory sync"
        );
        return Ok(());
    }

    #[derive(sqlx::FromRow)]
    struct MappingRow {
        shop_id: uuid::Uuid,
    }

    let mappings: Vec<MappingRow> = sqlx::query_as(
        r#"
        select m.shop_id
        from channel_sku_mappings m
        join shops s on s.shop_id = m.shop_id
        where m.tenant_id = $1
          and m.sku_id = $2
          and m.status = 'ACTIVE'
          and s.status = 'ACTIVE'
        "#,
    )
    .bind(uuid::Uuid::from(payload.tenant_id))
    .bind(payload.sku_id)
    .fetch_all(db.pool())
    .await?;

    if mappings.is_empty() {
        return Ok(());
    }

    let sync_repo = StockSyncJobsRepo::new(db.pool());
    let next_retry_at = Utc::now() + debounce;
    let mut config_cache: HashMap<ShopId, SalesShopConfig> = HashMap::new();

    for mapping in mappings {
        let shop_id = ShopId(mapping.shop_id);
        let config = if let Some(cfg) = config_cache.get(&shop_id) {
            cfg.clone()
        } else {
            let cfg = match load_sales_config_for_shop(db.pool(), payload.tenant_id, shop_id).await
            {
                Ok(v) => v,
                Err(err) => {
                    warn!(
                        tenant_id = %payload.tenant_id,
                        shop_id = %shop_id,
                        error = %err,
                        "failed to load sales config for stock sync; fallback to defaults"
                    );
                    SalesShopConfig::default()
                }
            };
            config_cache.insert(shop_id, cfg.clone());
            cfg
        };
        if !config.enable_stock_sync {
            warn!(
                tenant_id = %payload.tenant_id,
                shop_id = %shop_id,
                "stock sync disabled by sales config"
            );
            continue;
        }
        let sync_enabled = feature_flag_enabled(
            db,
            payload.tenant_id,
            Some(shop_id),
            FeatureFlagKey::InventorySync,
        )
        .await?;
        if !sync_enabled {
            warn!(
                tenant_id = %payload.tenant_id,
                shop_id = %shop_id,
                "feature disabled: inventory sync"
            );
            continue;
        }
        let _job = sync_repo
            .upsert_pending_job(erp_repo::stock_sync_jobs::StockSyncJobUpsert {
                tenant_id: payload.tenant_id,
                shop_id,
                sku_id: payload.sku_id,
                warehouse_id: payload.warehouse_id,
                target_qty: payload.available,
                next_retry_at,
            })
            .await?;
    }

    Ok(())
}

async fn handle_print_job_queued(db: &Db, env: OutboxEnvelope) -> anyhow::Result<()> {
    let job_id = env
        .payload
        .get("job_id")
        .and_then(|value| value.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing job_id"))?;
    let job_id = uuid::Uuid::parse_str(job_id)?;

    let jobs_repo = PrintJobsRepo::new(db.pool());
    let templates_repo = PrintTemplatesRepo::new(db.pool());
    let audit_repo = AuditLogsRepo::new(db.pool());

    let Some(job) = jobs_repo.get_job(env.tenant_id, job_id).await? else {
        return Ok(());
    };

    if job.status != "PENDING" {
        return Ok(());
    }
    if job.device_fingerprint.is_some() {
        return Ok(());
    }

    let attempts = job.attempts.saturating_add(1);
    let render_result: anyhow::Result<(String, String)> = async {
        let template_id = job
            .template_id
            .ok_or_else(|| anyhow::anyhow!("template_id missing"))?;
        let template = templates_repo
            .get_template(env.tenant_id, template_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("template not found"))?;

        let template_body = match template.content {
            serde_json::Value::String(value) => value,
            other => other.to_string(),
        };
        let payload_json =
            serde_json::to_string_pretty(&job.payload).unwrap_or_else(|_| job.payload.to_string());
        let body = template_body.replace("{{payload}}", &payload_json);
        let format = template.content_format.trim().to_uppercase();
        let rendered = match format.as_str() {
            "HTML" => {
                let lower = body.to_lowercase();
                if lower.contains("<html") || lower.contains("<!doctype") {
                    body.clone()
                } else {
                    format!(
                        "<!doctype html><html><head><meta charset=\"utf-8\"/></head><body>{body}</body></html>"
                    )
                }
            }
            "ZPL" | "TEXT" => body,
            _ => {
                return Err(anyhow::anyhow!(
                    "unsupported template format: {}",
                    template.content_format
                ))
            }
        };
        let output_url = format!("/v1/tenants/{}/print-jobs/{}/output", env.tenant_id, job_id);
        Ok((rendered, output_url))
    }
    .await;

    match render_result {
        Ok((html, output_url)) => {
            let updated = jobs_repo
                .update_job(
                    env.tenant_id,
                    job_id,
                    erp_repo::print_jobs::PrintJobUpdate {
                        status: Some("DONE".to_string()),
                        payload: None,
                        output_url: Some(output_url),
                        output_content: Some(html),
                        attempts: Some(attempts),
                        last_error: Some(String::new()),
                        actor_user_id: None,
                    },
                )
                .await?
                .ok_or_else(|| anyhow::anyhow!("print job not found"))?;

            let after_json = serde_json::to_value(&updated).unwrap_or(serde_json::json!({}));
            audit_repo
                .insert(AuditLogCreate {
                    tenant_id: env.tenant_id,
                    user_id: None,
                    action: "PRINT_JOB.RENDER".to_string(),
                    entity_type: "print_job".to_string(),
                    entity_id: Some(job_id.to_string()),
                    before_json: None,
                    after_json: Some(after_json),
                    ip: None,
                    ua: None,
                })
                .await?;

            metrics().inc_print_jobs_success();
        }
        Err(err) => {
            let message = err.to_string();
            let _ = jobs_repo
                .update_job(
                    env.tenant_id,
                    job_id,
                    erp_repo::print_jobs::PrintJobUpdate {
                        status: Some("FAILED".to_string()),
                        payload: None,
                        output_url: None,
                        output_content: None,
                        attempts: Some(attempts),
                        last_error: Some(message.clone()),
                        actor_user_id: None,
                    },
                )
                .await;

            let _ = audit_repo
                .insert(AuditLogCreate {
                    tenant_id: env.tenant_id,
                    user_id: None,
                    action: "PRINT_JOB.FAIL".to_string(),
                    entity_type: "print_job".to_string(),
                    entity_id: Some(job_id.to_string()),
                    before_json: None,
                    after_json: Some(serde_json::json!({"error": message})),
                    ip: None,
                    ua: None,
                })
                .await;
            metrics().inc_print_jobs_failed();
        }
    }

    Ok(())
}

async fn handle_shipment_shipped(
    db: &Db,
    adapters: &AdapterRegistry,
    payload: ShipmentShippedPayload,
    outbox_id: uuid::Uuid,
    attempt: u32,
    trigger: &str,
    replay_dead_letter_id: Option<uuid::Uuid>,
) -> anyhow::Result<()> {
    if read_only_enabled(db, payload.tenant_id).await? {
        warn!(
            tenant_id = %payload.tenant_id,
            shipment_id = %payload.shipment_id,
            "read-only mode active; skipping shipment push"
        );
        return Ok(());
    }

    let shipping_enabled = feature_flag_enabled(
        db,
        payload.tenant_id,
        Some(payload.shop_id),
        FeatureFlagKey::ShippingWrite,
    )
    .await?;
    if !shipping_enabled {
        warn!(
            tenant_id = %payload.tenant_id,
            shop_id = %payload.shop_id,
            shipment_id = %payload.shipment_id,
            "feature disabled: shipping write"
        );
        return Ok(());
    }

    if is_pdd_only_mode(db, payload.tenant_id).await? && payload.platform != Platform::Pdd {
        warn!(
            tenant_id = %payload.tenant_id,
            platform = %payload.platform,
            shipment_id = %payload.shipment_id,
            "pdd-only mode blocked shipment push"
        );
        return Ok(());
    }

    let shops_repo = ShopsRepo::new(db.pool());
    let shop = shops_repo.get_shop_auth(payload.shop_id).await?;
    let adapter = adapters.platform_adapter(payload.platform);
    #[derive(Clone)]
    struct PackageRequest {
        package_no: Option<String>,
        carrier_code: Option<String>,
        tracking_no: Option<String>,
    }

    let shipments_repo = ShipmentsRepo::new(db.pool());
    let packages = shipments_repo
        .list_packages(payload.tenant_id, payload.shipment_id)
        .await
        .unwrap_or_default();

    let mut package_requests = Vec::new();
    if packages.is_empty() {
        package_requests.push(PackageRequest {
            package_no: None,
            carrier_code: payload.carrier_code.clone(),
            tracking_no: payload.tracking_no.clone(),
        });
    } else {
        for pkg in packages {
            package_requests.push(PackageRequest {
                package_no: Some(pkg.package_no),
                carrier_code: pkg.carrier_code.clone().or(payload.carrier_code.clone()),
                tracking_no: pkg.tracking_no.clone().or(payload.tracking_no.clone()),
            });
        }
    }

    let attempt_value = attempt.min(i32::MAX as u32) as i32;
    let attempts_repo = ShipmentPushAttemptsRepo::new(db.pool());
    let mut last_error: Option<AdapterError> = None;

    if package_requests.iter().all(|req| req.tracking_no.is_none()) {
        let error_message = "tracking_no required".to_string();
        let request_json = serde_json::json!({
            "platform_order_id": payload.platform_order_id.clone(),
            "warehouse_id": payload.warehouse_id,
            "carrier_code": payload.carrier_code.clone(),
            "tracking_no": payload.tracking_no.clone(),
            "packages": package_requests
                .iter()
                .map(|pkg| serde_json::json!({
                    "package_no": pkg.package_no.clone(),
                    "carrier_code": pkg.carrier_code.clone(),
                    "tracking_no": pkg.tracking_no.clone(),
                }))
                .collect::<Vec<_>>(),
        });
        let response_json = serde_json::json!({});
        if let Err(err) = attempts_repo
            .insert_attempt(ShipmentPushAttemptCreate {
                tenant_id: payload.tenant_id,
                outbox_id,
                shipment_id: payload.shipment_id,
                shop_id: uuid::Uuid::from(payload.shop_id),
                platform: payload.platform.to_string(),
                platform_order_id: payload.platform_order_id.clone(),
                carrier_code: payload.carrier_code.clone(),
                tracking_no: payload.tracking_no.clone(),
                shipped_at: payload.shipped_at,
                status: SHIPMENT_PUSH_STATUS_FAILED.to_string(),
                attempt: attempt_value,
                duration_ms: 0,
                error_code: "MISSING_TRACKING".to_string(),
                error_message: error_message.clone(),
                request_json,
                response_json,
                next_retry_at: None,
                dead_letter_id: replay_dead_letter_id,
                trigger: trigger.to_string(),
            })
            .await
        {
            warn!(error = %err, "shipment push attempt log insert failed");
        }
        shipments_repo
            .update_push_status(
                payload.tenant_id,
                payload.shipment_id,
                "SHIP_FAILED",
                &error_message,
            )
            .await?;
        return Err(anyhow::anyhow!(error_message));
    }

    shipments_repo
        .update_push_status(payload.tenant_id, payload.shipment_id, "SHIPPING", "")
        .await?;

    for package in package_requests {
        let request = ShipOrderRequest {
            platform_order_id: payload.platform_order_id.clone(),
            warehouse_id: payload.warehouse_id,
            carrier_code: package.carrier_code.clone(),
            tracking_no: package.tracking_no.clone(),
        };

        let request_json = serde_json::json!({
            "platform_order_id": payload.platform_order_id.clone(),
            "warehouse_id": payload.warehouse_id,
            "carrier_code": package.carrier_code.clone(),
            "tracking_no": package.tracking_no.clone(),
            "package_no": package.package_no.clone(),
        });
        let started = std::time::Instant::now();
        let ship_result = adapter.ship_order(&shop, request).await;
        let duration_ms = started.elapsed().as_millis().min(i32::MAX as u128) as i32;
        metrics().add_shipping_push_latency_ms(duration_ms as u64);

        match ship_result {
            Ok(response) => {
                metrics().inc_shipping_push_success();
                let response_json = serde_json::json!({
                    "tracking_no": response.tracking_no,
                });
                if let Err(err) = attempts_repo
                    .insert_attempt(ShipmentPushAttemptCreate {
                        tenant_id: payload.tenant_id,
                        outbox_id,
                        shipment_id: payload.shipment_id,
                        shop_id: uuid::Uuid::from(payload.shop_id),
                        platform: payload.platform.to_string(),
                        platform_order_id: payload.platform_order_id.clone(),
                        carrier_code: package.carrier_code,
                        tracking_no: package.tracking_no,
                        shipped_at: payload.shipped_at,
                        status: SHIPMENT_PUSH_STATUS_SUCCESS.to_string(),
                        attempt: attempt_value,
                        duration_ms,
                        error_code: String::new(),
                        error_message: String::new(),
                        request_json,
                        response_json,
                        next_retry_at: None,
                        dead_letter_id: replay_dead_letter_id,
                        trigger: trigger.to_string(),
                    })
                    .await
                {
                    warn!(error = %err, "shipment push attempt log insert failed");
                }
            }
            Err(err) => {
                metrics().inc_shipping_push_failed();
                let (error_code, response_json) = shipment_push_error_summary(&err);
                let error_message = err.to_string();
                if let Err(err) = attempts_repo
                    .insert_attempt(ShipmentPushAttemptCreate {
                        tenant_id: payload.tenant_id,
                        outbox_id,
                        shipment_id: payload.shipment_id,
                        shop_id: uuid::Uuid::from(payload.shop_id),
                        platform: payload.platform.to_string(),
                        platform_order_id: payload.platform_order_id.clone(),
                        carrier_code: package.carrier_code,
                        tracking_no: package.tracking_no,
                        shipped_at: payload.shipped_at,
                        status: SHIPMENT_PUSH_STATUS_FAILED.to_string(),
                        attempt: attempt_value,
                        duration_ms,
                        error_code,
                        error_message: error_message.clone(),
                        request_json,
                        response_json,
                        next_retry_at: None,
                        dead_letter_id: replay_dead_letter_id,
                        trigger: trigger.to_string(),
                    })
                    .await
                {
                    warn!(error = %err, "shipment push attempt log insert failed");
                }
                last_error = Some(err);
            }
        }
    }

    match last_error {
        Some(err) => {
            shipments_repo
                .update_push_status(
                    payload.tenant_id,
                    payload.shipment_id,
                    "SHIP_FAILED",
                    &err.to_string(),
                )
                .await?;
            Err(anyhow::anyhow!(err))
        }
        None => {
            shipments_repo
                .update_push_status(payload.tenant_id, payload.shipment_id, "SHIPPED", "")
                .await?;
            Ok(())
        }
    }
}

async fn handle_refund_requested(db: &Db, payload: RefundRequestedPayload) -> anyhow::Result<()> {
    if read_only_enabled(db, payload.tenant_id).await? {
        warn!(
            tenant_id = %payload.tenant_id,
            refund_doc_id = %payload.refund_doc_id,
            "read-only mode active; skipping refund request"
        );
        return Ok(());
    }

    let repo = AfterSalesRepo::new(db.pool());
    let refund_doc = repo
        .mark_refund_done(payload.tenant_id, payload.refund_doc_id)
        .await?;
    metrics().inc_after_sales_sync_success();

    if let Ok(Some(after_sales)) = repo
        .get_after_sales(payload.tenant_id, refund_doc.after_sales_id)
        .await
    {
        let cycle_ms = Utc::now()
            .signed_duration_since(after_sales.created_at)
            .num_milliseconds();
        if cycle_ms > 0 {
            metrics().add_after_sales_cycle_time_ms(cycle_ms as u64);
        }
        if let Err(err) =
            rollback_refunded_order(db, payload.tenant_id, after_sales.order_id, refund_doc.id)
                .await
        {
            warn!(
                tenant_id = %payload.tenant_id,
                order_id = %after_sales.order_id,
                error = %err,
                "refund rollback failed"
            );
        }
    }

    let audit_repo = AuditLogsRepo::new(db.pool());
    let after_json = serde_json::json!({
        "refund_doc_id": refund_doc.id,
        "after_sales_id": refund_doc.after_sales_id,
        "amount_cent": refund_doc.amount_cent,
        "status": refund_doc.status.to_string(),
        "channel": refund_doc.channel.to_string(),
    });
    if let Err(err) = audit_repo
        .insert(AuditLogCreate {
            tenant_id: payload.tenant_id,
            user_id: None,
            action: "AFTERSALES.REFUND_DONE".to_string(),
            entity_type: "refund_doc".to_string(),
            entity_id: Some(refund_doc.id.to_string()),
            before_json: None,
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %err, "audit log insert failed for refund");
    }

    Ok(())
}

async fn rollback_refunded_order(
    db: &Db,
    tenant_id: TenantId,
    order_id: OrderId,
    refund_doc_id: uuid::Uuid,
) -> anyhow::Result<()> {
    let shipment_rows: Vec<(uuid::Uuid, String)> = sqlx::query_as(
        r#"
        select shipment_id, status
        from shipments
        where tenant_id = $1 and order_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .fetch_all(db.pool())
    .await?;
    let shipment_ids: Vec<uuid::Uuid> = shipment_rows
        .iter()
        .map(|(shipment_id, _)| *shipment_id)
        .collect();
    let has_shipped_shipment = shipment_rows.iter().any(|(_, status)| {
        status == "SHIPPED" || status == "SUCCESS" || status == "PARTIALLY_SHIPPED"
    });
    if has_shipped_shipment {
        return Ok(());
    }

    #[derive(sqlx::FromRow)]
    struct OutboundRow {
        outbound_doc_id: uuid::Uuid,
        status: String,
    }

    let outbound_rows: Vec<OutboundRow> = sqlx::query_as(
        r#"
        select d.outbound_doc_id, d.status
        from outbound_docs d
        join shipments s
          on s.tenant_id = d.tenant_id
         and s.shipment_id = d.shipment_id
        where d.tenant_id = $1 and s.order_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .fetch_all(db.pool())
    .await?;

    if outbound_rows.iter().any(|row| row.status == "SHIPPED") {
        return Ok(());
    }
    let outbound_doc_ids: Vec<uuid::Uuid> = outbound_rows
        .iter()
        .map(|row| row.outbound_doc_id)
        .collect();

    if has_printed_jobs(
        db.pool(),
        tenant_id,
        order_id,
        &shipment_ids,
        &outbound_doc_ids,
    )
    .await?
    {
        return Ok(());
    }

    let fulfillment_repo = FulfillmentRepo::new(db.pool());
    let _ = fulfillment_repo
        .release_order_reservations(tenant_id, order_id)
        .await;

    let wms_repo = WmsRepo::new(db.pool());
    let cancelled_docs = wms_repo
        .cancel_outbounds_for_order(tenant_id, order_id)
        .await
        .unwrap_or_default();

    if !shipment_ids.is_empty() {
        let _ = sqlx::query(
            r#"
            delete from shipment_packages
            where tenant_id = $1 and shipment_id = any($2)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(&shipment_ids)
        .execute(db.pool())
        .await;
    }

    cancel_print_jobs(
        db.pool(),
        tenant_id,
        order_id,
        &shipment_ids,
        &outbound_doc_ids,
    )
    .await?;

    let orders_repo = OrdersRepo::new(db.pool());
    if let Ok(Some(state)) = orders_repo
        .get_order_review_state(tenant_id, order_id)
        .await
    {
        let _ = orders_repo
            .update_order_review_state(
                tenant_id,
                order_id,
                erp_repo::orders::OrderReviewUpdate {
                    oms_status: erp_types::OmsOrderStatus::Cancelled,
                    order_status: erp_types::OrderMainStatus::Cancelled,
                    review_status: state.review_status,
                    fulfillment_status: erp_types::OrderFulfillmentStatus::NotAllocated,
                    exception_status: erp_types::OrderExceptionStatus::Closed,
                    exception_flags: state.exception_flags.clone(),
                    expected_version: Some(state.version),
                },
            )
            .await;
    }

    let reason = format!("refund rollback: {refund_doc_id}");
    let _ = orders_repo
        .mark_order_refunded(tenant_id, order_id, Some(reason))
        .await;

    let audit_repo = AuditLogsRepo::new(db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: None,
            action: "ORDER_REFUNDED_AUTO".to_string(),
            entity_type: "order".to_string(),
            entity_id: Some(order_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({
                "refund_doc_id": refund_doc_id,
                "shipment_ids": shipment_ids,
                "outbound_doc_ids": outbound_doc_ids,
                "cancelled_outbound_docs": cancelled_docs,
            })),
            ip: None,
            ua: None,
        })
        .await;
    metrics().inc_after_sales_compensation();

    Ok(())
}

async fn has_printed_jobs(
    pool: &PgPool,
    tenant_id: TenantId,
    order_id: OrderId,
    shipment_ids: &[uuid::Uuid],
    outbound_doc_ids: &[uuid::Uuid],
) -> anyhow::Result<bool> {
    let order_done: bool = sqlx::query_scalar(
        r#"
        select exists(
            select 1
            from print_jobs
            where tenant_id = $1
              and ref_type = 'ORDER'
              and ref_id = $2
              and status = 'DONE'
        )
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .fetch_one(pool)
    .await?;
    if order_done {
        return Ok(true);
    }

    if !shipment_ids.is_empty() {
        let shipped_done: bool = sqlx::query_scalar(
            r#"
            select exists(
                select 1
                from print_jobs
                where tenant_id = $1
                  and ref_type = 'SHIPMENT'
                  and ref_id = any($2)
                  and status = 'DONE'
            )
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_ids)
        .fetch_one(pool)
        .await?;
        if shipped_done {
            return Ok(true);
        }
    }

    if !outbound_doc_ids.is_empty() {
        let outbound_done: bool = sqlx::query_scalar(
            r#"
            select exists(
                select 1
                from print_jobs
                where tenant_id = $1
                  and ref_type = 'OUTBOUND_DOC'
                  and ref_id = any($2)
                  and status = 'DONE'
            )
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbound_doc_ids)
        .fetch_one(pool)
        .await?;
        if outbound_done {
            return Ok(true);
        }
    }

    Ok(false)
}

async fn cancel_print_jobs(
    pool: &PgPool,
    tenant_id: TenantId,
    order_id: OrderId,
    shipment_ids: &[uuid::Uuid],
    outbound_doc_ids: &[uuid::Uuid],
) -> anyhow::Result<()> {
    let reason = format!("order refunded: {}", order_id);

    sqlx::query(
        r#"
        update print_jobs
        set status = 'CANCELLED',
            dispatch_status = 'FAILED',
            last_error = $4,
            updated_at = now()
        where tenant_id = $1
          and ref_type = $2
          and ref_id = $3
          and status in ('QUEUED','PENDING','RUNNING')
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind("ORDER")
    .bind(uuid::Uuid::from(order_id))
    .bind(&reason)
    .execute(pool)
    .await?;

    if !shipment_ids.is_empty() {
        sqlx::query(
            r#"
            update print_jobs
            set status = 'CANCELLED',
                dispatch_status = 'FAILED',
                last_error = $4,
                updated_at = now()
            where tenant_id = $1
              and ref_type = $2
              and ref_id = any($3)
              and status in ('QUEUED','PENDING','RUNNING')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind("SHIPMENT")
        .bind(shipment_ids)
        .bind(&reason)
        .execute(pool)
        .await?;
    }

    if !outbound_doc_ids.is_empty() {
        sqlx::query(
            r#"
            update print_jobs
            set status = 'CANCELLED',
                dispatch_status = 'FAILED',
                last_error = $4,
                updated_at = now()
            where tenant_id = $1
              and ref_type = $2
              and ref_id = any($3)
              and status in ('QUEUED','PENDING','RUNNING')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind("OUTBOUND_DOC")
        .bind(outbound_doc_ids)
        .bind(&reason)
        .execute(pool)
        .await?;
    }

    Ok(())
}

fn shipment_push_error_summary(err: &AdapterError) -> (String, serde_json::Value) {
    match err {
        AdapterError::Auth(message) => (
            "AUTH".to_string(),
            serde_json::json!({ "message": message }),
        ),
        AdapterError::RateLimited { retry_after_ms } => (
            "RATE_LIMITED".to_string(),
            serde_json::json!({ "retry_after_ms": retry_after_ms }),
        ),
        AdapterError::Temporary(message) => (
            "TEMPORARY".to_string(),
            serde_json::json!({ "message": message }),
        ),
        AdapterError::BadRequest(message) => (
            "BAD_REQUEST".to_string(),
            serde_json::json!({ "message": message }),
        ),
        AdapterError::Unsupported => ("UNSUPPORTED".to_string(), serde_json::json!({})),
        AdapterError::Unexpected(message) => (
            "UNEXPECTED".to_string(),
            serde_json::json!({ "message": message }),
        ),
    }
}

fn order_item_uuid(order_id: erp_types::OrderId, line_id: &str) -> uuid::Uuid {
    uuid::Uuid::parse_str(line_id)
        .unwrap_or_else(|_| uuid::Uuid::new_v5(&uuid::Uuid::from(order_id), line_id.as_bytes()))
}

async fn apply_sku_mappings(
    pool: &PgPool,
    outbox_repo: &OutboxRepo<'_>,
    order: &mut OrderProjection,
) -> anyhow::Result<()> {
    for item in &mut order.items {
        let Some(platform_sku_id) = item.sku_id.clone() else {
            continue;
        };

        let mapped_sku_id = resolve_sku_mapping(
            pool,
            order.tenant_id,
            order.shop_id,
            order.platform,
            &platform_sku_id,
        )
        .await?;

        match mapped_sku_id {
            Some(mapped) => item.sku_id = Some(mapped),
            None => {
                item.sku_id = None;
                let payload = serde_json::json!({
                    "tenant_id": order.tenant_id,
                    "shop_id": order.shop_id,
                    "platform": order.platform,
                    "platform_sku_id": platform_sku_id,
                    "platform_order_id": order.platform_order_id,
                });
                outbox_repo
                    .enqueue(order.tenant_id, "MAPPING_MISSING", &payload)
                    .await?;
                warn!(
                    tenant_id = %order.tenant_id,
                    shop_id = %order.shop_id,
                    platform = %order.platform,
                    platform_sku_id = %platform_sku_id,
                    platform_order_id = %order.platform_order_id,
                    "sku mapping missing"
                );
            }
        }
    }
    Ok(())
}

async fn resolve_sku_mapping(
    pool: &PgPool,
    tenant_id: TenantId,
    shop_id: ShopId,
    platform: Platform,
    platform_sku_id: &str,
) -> anyhow::Result<Option<String>> {
    let mapped: Option<uuid::Uuid> = sqlx::query_scalar(
        r#"
        select sku_id
        from channel_sku_mappings
        where tenant_id = $1
          and shop_id = $2
          and platform = $3
          and platform_sku_id = $4
          and status = 'ACTIVE'
        limit 1
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(shop_id))
    .bind(platform.to_string())
    .bind(platform_sku_id)
    .fetch_optional(pool)
    .await?;

    Ok(mapped.map(|id| id.to_string()))
}

async fn metrics_handler() -> Response {
    let body = render_metrics();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, METRICS_CONTENT_TYPE)
        .body(Body::from(body))
        .unwrap()
}

async fn record_inbox_dedup_hits(db: &Db, tenant_id: TenantId, inbox_event_id: uuid::Uuid) {
    let seen_count = sqlx::query_scalar::<_, i32>(
        r#"
        select d.seen_count
        from inbox_events e
        join inbox_dedup d
          on d.tenant_id = e.tenant_id
         and d.dedup_key = e.dedup_key
        where e.tenant_id = $1 and e.inbox_event_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(inbox_event_id)
    .fetch_optional(db.pool())
    .await;

    if let Ok(Some(count)) = seen_count {
        let hits = count.saturating_sub(1) as u64;
        metrics().add_webhook_inbox_dedup_hits(hits);
    }
}

fn sync_event_type(job_type: &str) -> String {
    job_type
        .split('_')
        .map(|part| part.to_uppercase())
        .collect::<Vec<_>>()
        .join("_")
}

async fn load_sales_config_for_shop(
    pool: &PgPool,
    tenant_id: TenantId,
    shop_id: ShopId,
) -> anyhow::Result<SalesShopConfig> {
    #[derive(sqlx::FromRow)]
    struct Row {
        key: String,
        value_json: serde_json::Value,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        select key, value_json
        from settings
        where tenant_id = $1
          and scope_type = 'SHOP'
          and scope_id = $2
          and key = any($3)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(shop_id))
    .bind(
        sales_settings::SALES_CONFIG_KEYS
            .iter()
            .map(|key| key.to_string())
            .collect::<Vec<_>>(),
    )
    .fetch_all(pool)
    .await?;

    let mut config = SalesShopConfig::default();
    for row in rows {
        config.apply_setting(&row.key, &row.value_json);
    }
    Ok(config)
}

fn compute_sync_dedup_key(
    platform: &Platform,
    platform_order_id: &str,
    event_type: &str,
    version: &str,
    refund_fingerprint: Option<&str>,
) -> String {
    let base = match refund_fingerprint {
        Some(fingerprint) if !fingerprint.trim().is_empty() => {
            format!("{platform}:{platform_order_id}:{event_type}:{version}:{fingerprint}")
        }
        _ => format!("{platform}:{platform_order_id}:{event_type}:{version}"),
    };
    let mut hasher = Sha256::new();
    hasher.update(base.as_bytes());
    hex::encode(hasher.finalize())
}

fn refund_sync_dedup_fingerprint(order: &OrderProjection) -> Option<String> {
    let mut tokens: Vec<String> = Vec::new();
    for item in &order.items {
        let case_id = item
            .after_sale_id
            .as_ref()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string())
            .unwrap_or_else(|| format!("LINE:{}", item.line_id));
        let token = format!(
            "{}:{}:{}:{}",
            case_id,
            item.refund_state,
            item.after_sale_state,
            item.refund_amount_cent.unwrap_or(0)
        );
        if token.ends_with(":NONE:NONE:0") && case_id.starts_with("LINE:") {
            continue;
        }
        tokens.push(token);
    }
    if tokens.is_empty() {
        return None;
    }
    tokens.sort();
    tokens.dedup();
    let joined = tokens.join("|");
    let mut hasher = Sha256::new();
    hasher.update(joined.as_bytes());
    Some(hex::encode(hasher.finalize()))
}

fn annotate_mapping_version(
    order: &mut OrderProjection,
    mapping_version: Option<i32>,
) -> Option<i32> {
    let version = mapping_version?;
    let map = match order.raw.as_object_mut() {
        Some(map) => map,
        None => {
            order.raw = serde_json::Value::Object(serde_json::Map::new());
            order.raw.as_object_mut().expect("mapping raw object")
        }
    };
    map.insert(
        "applied_mapping_version".to_string(),
        serde_json::Value::Number(version.into()),
    );
    Some(version)
}

fn normalize_text(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn derive_platform_sku_code(sku: &erp_adapter::PlatformSku) -> String {
    let candidates = [
        sku.sku_code.as_ref(),
        sku.outer_sku_id.as_ref(),
        Some(&sku.platform_sku_id),
    ];
    for raw in candidates.into_iter().flatten() {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    sku.platform_sku_id.clone()
}

async fn sync_platform_products(
    tenant_id: TenantId,
    platform: Platform,
    products: Vec<erp_adapter::PlatformProduct>,
    products_repo: &ProductsRepo<'_>,
    skus_repo: &SkusRepo<'_>,
) -> anyhow::Result<()> {
    for product in products {
        if product.skus.is_empty() {
            continue;
        }
        let mut product_id: Option<uuid::Uuid> = None;
        let mut product_updated = false;
        for sku in product.skus {
            let sku_code = derive_platform_sku_code(&sku);
            let existing = skus_repo
                .list_skus(
                    tenant_id,
                    SkuListFilters {
                        product_id: None,
                        sku_code: Some(sku_code.clone()),
                        barcode: None,
                        keyword: None,
                        is_active: None,
                        cursor: None,
                        limit: 1,
                    },
                )
                .await?;

            if let Some(existing_sku) = existing.items.first() {
                if product_id.is_none() {
                    product_id = Some(existing_sku.product_id);
                }
                if !product_updated {
                    let _ = products_repo
                        .update_product(
                            tenant_id,
                            existing_sku.product_id,
                            ProductUpdate {
                                product_code: None,
                                name: Some(product.name.clone()),
                                alias: None,
                                brand: normalize_text(product.brand.clone()),
                                category: normalize_text(product.category.clone()),
                                base_unit: None,
                                second_unit: None,
                                default_supplier_id: None,
                                default_warehouse_id: None,
                                primary_barcode: None,
                                product_tags: None,
                                remark: None,
                                warning_stock: None,
                                batch_enabled: None,
                                shelf_life_days: None,
                                forbid_sale_days: None,
                                serial_enabled: None,
                                serial_push_platform: None,
                                attrs: Some(product.attrs.clone()),
                                status: None,
                                allow_negative_stock: None,
                            },
                        )
                        .await?;
                    product_updated = true;
                }
                let _ = skus_repo
                    .update_sku(
                        tenant_id,
                        existing_sku.sku_id,
                        SkuUpdate {
                            sku_code: None,
                            barcode: normalize_text(sku.barcode.clone()),
                            name: normalize_text(sku.name.clone()),
                            note: None,
                            spec: sku.spec.clone(),
                            tags: if sku.tags.is_empty() {
                                None
                            } else {
                                Some(sku.tags.clone())
                            },
                            image_urls: if sku.image_urls.is_empty() {
                                None
                            } else {
                                Some(sku.image_urls.clone())
                            },
                            weight_gram: sku.weight_gram,
                            length_mm: sku.length_mm,
                            width_mm: sku.width_mm,
                            height_mm: sku.height_mm,
                            volume_cm3: sku.volume_cm3,
                            sku_kind: None,
                            cost_cent: None,
                            is_active: Some(true),
                            status: None,
                            batch_enabled: None,
                            serial_enabled: None,
                        },
                    )
                    .await?;
                continue;
            }

            if product_id.is_none() {
                let created = products_repo
                    .create_product(ProductCreate {
                        tenant_id,
                        product_code: None,
                        name: product.name.clone(),
                        alias: None,
                        brand: normalize_text(product.brand.clone()),
                        category: normalize_text(product.category.clone()),
                        base_unit: None,
                        second_unit: None,
                        default_supplier_id: None,
                        default_warehouse_id: None,
                        primary_barcode: None,
                        product_tags: Vec::new(),
                        remark: None,
                        warning_stock: serde_json::json!({}),
                        batch_enabled: false,
                        shelf_life_days: None,
                        forbid_sale_days: None,
                        serial_enabled: false,
                        serial_push_platform: false,
                        attrs: product.attrs.clone(),
                        allow_negative_stock: false,
                    })
                    .await?;
                product_id = Some(created.product_id);
                info!(
                    tenant_id = %tenant_id,
                    platform = %platform,
                    product_id = %created.product_id,
                    "platform product created"
                );
            }

            let _ = skus_repo
                .create_sku(SkuCreate {
                    tenant_id,
                    product_id: product_id.expect("product_id"),
                    sku_code,
                    barcode: normalize_text(sku.barcode.clone()),
                    name: normalize_text(sku.name.clone()),
                    note: None,
                    spec: sku.spec.unwrap_or_else(|| serde_json::json!({})),
                    tags: sku.tags.clone(),
                    image_urls: sku.image_urls.clone(),
                    weight_gram: sku.weight_gram,
                    length_mm: sku.length_mm,
                    width_mm: sku.width_mm,
                    height_mm: sku.height_mm,
                    volume_cm3: sku.volume_cm3,
                    sku_kind: "NORMAL".to_string(),
                    cost_cent: None,
                    is_active: true,
                    status: None,
                    batch_enabled: false,
                    serial_enabled: false,
                })
                .await?;
        }
    }
    Ok(())
}

async fn run_sync_jobs_once(
    db: &Db,
    adapters: &AdapterRegistry,
    lookback: ChronoDuration,
    batch_limit: i64,
) -> anyhow::Result<()> {
    let jobs_repo = SyncJobsRepo::new(db.pool());
    let shops_repo = ShopsRepo::new(db.pool());
    let inbox_repo = InboxRepo::new(db.pool());
    let outbox_repo = OutboxRepo::new(db.pool());
    let mapping_rules_repo = MappingRulesRepo::new(db.pool());
    let products_repo = ProductsRepo::new(db.pool());
    let skus_repo = SkusRepo::new(db.pool());
    let runs_repo = SyncJobRunsRepo::new(db.pool());
    let mut config_cache: HashMap<ShopId, SalesShopConfig> = HashMap::new();

    let jobs = jobs_repo.list_jobs_for_run(batch_limit).await?;
    for job in jobs {
        let supported = matches!(
            job.job_type.as_str(),
            "order_sync" | "refund_sync" | "shipping_sync" | "product_sync"
        );
        if !supported {
            let _ = jobs_repo
                .update_job_result(
                    job.tenant_id,
                    job.job_id,
                    "FAILED",
                    job.cursor.clone(),
                    job.watermark,
                    Some("unsupported job_type".to_string()),
                )
                .await?;
            continue;
        }

        let config = if let Some(cfg) = config_cache.get(&job.shop_id) {
            cfg.clone()
        } else {
            let cfg = load_sales_config_for_shop(db.pool(), job.tenant_id, job.shop_id).await?;
            config_cache.insert(job.shop_id, cfg.clone());
            cfg
        };
        let disable_reason = match job.job_type.as_str() {
            "order_sync" if !config.auto_download_orders => Some("sales auto download disabled"),
            "refund_sync" if !config.enable_after_sales_download => {
                Some("after-sales download disabled")
            }
            "shipping_sync" if !config.enable_logistics_upload => Some("logistics upload disabled"),
            _ => None,
        };
        if let Some(reason) = disable_reason {
            let _ = jobs_repo
                .update_job_status(
                    job.tenant_id,
                    job.shop_id,
                    &job.job_type,
                    "DISABLED",
                    Some(reason.to_string()),
                )
                .await?;
            continue;
        }

        let force_run = job.last_error.trim() == SYNC_JOB_FORCE_RUN_MARKER;
        if !force_run && job.job_type == "order_sync" && config.download_interval_minutes > 0 {
            let interval = ChronoDuration::minutes(config.download_interval_minutes.max(1));
            if job.updated_at + interval > Utc::now() {
                continue;
            }
        }

        let sync_flag = match job.job_type.as_str() {
            "order_sync" => Some(FeatureFlagKey::OrderSync),
            "refund_sync" => Some(FeatureFlagKey::AfterSalesSync),
            "shipping_sync" => Some(FeatureFlagKey::ShippingWrite),
            "product_sync" => None,
            _ => Some(FeatureFlagKey::OrderSync),
        };
        if let Some(sync_flag) = sync_flag {
            let sync_enabled =
                feature_flag_enabled(db, job.tenant_id, Some(job.shop_id), sync_flag).await?;
            if !sync_enabled {
                warn!(
                    tenant_id = %job.tenant_id,
                    shop_id = %job.shop_id,
                    job_type = %job.job_type,
                    "feature disabled: sync job"
                );
                continue;
            }
        }
        if job.job_type == "order_sync" && config.auto_merge_non_shipping_time {
            let merge_enabled = feature_flag_enabled(
                db,
                job.tenant_id,
                Some(job.shop_id),
                FeatureFlagKey::OrderMerge,
            )
            .await?;
            if !merge_enabled {
                warn!(
                    tenant_id = %job.tenant_id,
                    shop_id = %job.shop_id,
                    job_type = %job.job_type,
                    "feature disabled: order merge"
                );
            }
        }

        let Some(job) = jobs_repo
            .try_mark_running(job.tenant_id, job.job_id)
            .await?
        else {
            continue;
        };
        let shop = match shops_repo.get_shop_auth(job.shop_id).await {
            Ok(shop) => shop,
            Err(err) => {
                let _ = jobs_repo
                    .update_job_result(
                        job.tenant_id,
                        job.job_id,
                        "FAILED",
                        job.cursor.clone(),
                        job.watermark,
                        Some(err.to_string()),
                    )
                    .await?;
                continue;
            }
        };
        let adapter = adapters.platform_adapter(shop.platform);
        let cursor = SyncCursor {
            cursor: job.cursor.clone(),
            last_time: job.watermark.or_else(|| Some(Utc::now() - lookback)),
        };

        if job.job_type == "product_sync" {
            let started_at = Utc::now();
            let result = adapter.pull_products(&shop, cursor).await;
            let finished_at = Utc::now();
            let duration_ms = (finished_at - started_at).num_milliseconds().max(0);
            match result {
                Ok(res) => {
                    let item_count = res.products.len() as i32;
                    if let Err(err) = sync_platform_products(
                        job.tenant_id,
                        shop.platform,
                        res.products,
                        &products_repo,
                        &skus_repo,
                    )
                    .await
                    {
                        let _ = jobs_repo
                            .update_job_result(
                                job.tenant_id,
                                job.job_id,
                                "FAILED",
                                job.cursor.clone(),
                                job.watermark,
                                Some(err.to_string()),
                            )
                            .await?;
                        let _ = runs_repo
                            .insert_run(SyncJobRunCreate {
                                tenant_id: job.tenant_id,
                                job_id: job.job_id,
                                shop_id: job.shop_id,
                                job_type: job.job_type.clone(),
                                status: "FAILED".to_string(),
                                item_count: 0,
                                duration_ms,
                                error: err.to_string(),
                                started_at,
                                finished_at,
                            })
                            .await;

                        let failure = anyhow::anyhow!(err);
                        let classification = dlq::classify_error(&failure);
                        let env = OutboxEnvelope {
                            tenant_id: job.tenant_id,
                            outbox_id: job.job_id,
                            event_type: "SYNC_JOB_RUN".to_string(),
                            payload: serde_json::json!({
                                "tenant_id": job.tenant_id,
                                "job_id": job.job_id,
                                "shop_id": job.shop_id,
                                "job_type": job.job_type,
                            }),
                            produced_at: Utc::now(),
                            trace_id: None,
                        };
                        let _ = dlq::insert_dead_letter(
                            db.pool(),
                            &env,
                            &failure.to_string(),
                            1,
                            classification,
                            dlq::NEXT_ACTION_MANUAL_REPLAY,
                        )
                        .await?;
                        continue;
                    }

                    let cursor = res.next_cursor.or(job.cursor.clone());
                    let last_time = res.latest_time.or(job.watermark);
                    let _ = jobs_repo
                        .update_job_result(
                            job.tenant_id,
                            job.job_id,
                            "SUCCESS",
                            cursor,
                            last_time,
                            None,
                        )
                        .await?;
                    let _ = runs_repo
                        .insert_run(SyncJobRunCreate {
                            tenant_id: job.tenant_id,
                            job_id: job.job_id,
                            shop_id: job.shop_id,
                            job_type: job.job_type.clone(),
                            status: "SUCCESS".to_string(),
                            item_count,
                            duration_ms,
                            error: String::new(),
                            started_at,
                            finished_at,
                        })
                        .await;
                }
                Err(err) => {
                    let _ = jobs_repo
                        .update_job_result(
                            job.tenant_id,
                            job.job_id,
                            "FAILED",
                            job.cursor.clone(),
                            job.watermark,
                            Some(err.to_string()),
                        )
                        .await?;
                    let _ = runs_repo
                        .insert_run(SyncJobRunCreate {
                            tenant_id: job.tenant_id,
                            job_id: job.job_id,
                            shop_id: job.shop_id,
                            job_type: job.job_type.clone(),
                            status: "FAILED".to_string(),
                            item_count: 0,
                            duration_ms,
                            error: err.to_string(),
                            started_at,
                            finished_at,
                        })
                        .await;

                    let failure = anyhow::anyhow!(err);
                    let classification = dlq::classify_error(&failure);
                    let env = OutboxEnvelope {
                        tenant_id: job.tenant_id,
                        outbox_id: job.job_id,
                        event_type: "SYNC_JOB_RUN".to_string(),
                        payload: serde_json::json!({
                            "tenant_id": job.tenant_id,
                            "job_id": job.job_id,
                            "shop_id": job.shop_id,
                            "job_type": job.job_type,
                        }),
                        produced_at: Utc::now(),
                        trace_id: None,
                    };
                    let _ = dlq::insert_dead_letter(
                        db.pool(),
                        &env,
                        &failure.to_string(),
                        1,
                        classification,
                        dlq::NEXT_ACTION_MANUAL_REPLAY,
                    )
                    .await?;
                }
            }
            continue;
        }

        let mapping_version = mapping_rules_repo
            .latest_enabled_version(job.tenant_id, shop.platform, "order")
            .await?;
        let started_at = Utc::now();
        let result = if job.job_type == "refund_sync" {
            adapter
                .pull_refunds(&shop, cursor)
                .await
                .map(|res| erp_adapter::PullOrdersResult {
                    orders: res.orders,
                    next_cursor: res.next_cursor,
                    latest_time: res.latest_time,
                })
        } else {
            adapter.pull_orders(&shop, cursor).await
        };
        let finished_at = Utc::now();
        let duration_ms = (finished_at - started_at).num_milliseconds().max(0);
        match result {
            Ok(res) => {
                let event_type = sync_event_type(&job.job_type);
                let mut latest_time = job.watermark;
                let orders = res.orders;
                let item_count = orders.len() as i32;
                for mut order in orders {
                    annotate_mapping_version(&mut order, mapping_version);
                    let version_time = order.platform_updated_at.unwrap_or(order.order_created_at);
                    let version = version_time.timestamp().to_string();
                    let refund_fingerprint = if job.job_type == "refund_sync" {
                        refund_sync_dedup_fingerprint(&order)
                    } else {
                        None
                    };
                    let dedup_key = compute_sync_dedup_key(
                        &order.platform,
                        &order.platform_order_id,
                        &event_type,
                        &version,
                        refund_fingerprint.as_deref(),
                    );
                    let headers = serde_json::json!({
                        "source": "sync_job",
                        "job_id": job.job_id,
                        "job_type": job.job_type,
                        "platform_order_id": order.platform_order_id,
                        "version": version,
                    });
                    let body = serde_json::to_vec(&order.raw)?;
                    let received_at = Utc::now();
                    let is_first = inbox_repo
                        .claim_dedup_key(order.tenant_id, &dedup_key)
                        .await?;
                    let raw_id = inbox_repo
                        .insert_raw_payload(
                            order.tenant_id,
                            order.platform,
                            order.shop_id,
                            &headers,
                            &body,
                            received_at,
                        )
                        .await?;
                    let status = if is_first { "RECEIVED" } else { "DUPLICATE" };
                    let inbox_event_id = inbox_repo
                        .insert_inbox_event(
                            order.tenant_id,
                            order.platform,
                            order.shop_id,
                            &dedup_key,
                            raw_id,
                            received_at,
                            status,
                            "PASSED",
                            None,
                        )
                        .await?;

                    if is_first {
                        let payload = serde_json::to_value(InboxReceivedPayload {
                            tenant_id: order.tenant_id,
                            platform: order.platform,
                            shop_id: order.shop_id,
                            inbox_event_id,
                        })?;
                        outbox_repo
                            .enqueue(order.tenant_id, "INBOX_EVENT_RECEIVED", &payload)
                            .await?;
                    }

                    if latest_time.map(|t| version_time > t).unwrap_or(true) {
                        latest_time = Some(version_time);
                    }
                }

                let cursor = res.next_cursor.or(job.cursor.clone());
                let last_time = res.latest_time.or(latest_time);
                let _ = jobs_repo
                    .update_job_result(
                        job.tenant_id,
                        job.job_id,
                        "SUCCESS",
                        cursor,
                        last_time,
                        None,
                    )
                    .await?;
                let _ = runs_repo
                    .insert_run(SyncJobRunCreate {
                        tenant_id: job.tenant_id,
                        job_id: job.job_id,
                        shop_id: job.shop_id,
                        job_type: job.job_type.clone(),
                        status: "SUCCESS".to_string(),
                        item_count,
                        duration_ms,
                        error: String::new(),
                        started_at,
                        finished_at,
                    })
                    .await;
            }
            Err(err) => {
                let _ = jobs_repo
                    .update_job_result(
                        job.tenant_id,
                        job.job_id,
                        "FAILED",
                        job.cursor.clone(),
                        job.watermark,
                        Some(err.to_string()),
                    )
                    .await?;
                let _ = runs_repo
                    .insert_run(SyncJobRunCreate {
                        tenant_id: job.tenant_id,
                        job_id: job.job_id,
                        shop_id: job.shop_id,
                        job_type: job.job_type.clone(),
                        status: "FAILED".to_string(),
                        item_count: 0,
                        duration_ms,
                        error: err.to_string(),
                        started_at,
                        finished_at,
                    })
                    .await;

                let failure = anyhow::anyhow!(err);
                let classification = dlq::classify_error(&failure);
                let env = OutboxEnvelope {
                    tenant_id: job.tenant_id,
                    outbox_id: job.job_id,
                    event_type: "SYNC_JOB_RUN".to_string(),
                    payload: serde_json::json!({
                        "tenant_id": job.tenant_id,
                        "job_id": job.job_id,
                        "shop_id": job.shop_id,
                        "job_type": job.job_type,
                    }),
                    produced_at: Utc::now(),
                    trace_id: None,
                };
                let _ = dlq::insert_dead_letter(
                    db.pool(),
                    &env,
                    &failure.to_string(),
                    1,
                    classification,
                    dlq::NEXT_ACTION_MANUAL_REPLAY,
                )
                .await?;
            }
        }
    }

    Ok(())
}

async fn run_stock_sync_jobs_once(
    db: &Db,
    adapters: &AdapterRegistry,
    batch_limit: i64,
    retry_policy: &dlq::RetryPolicy,
) -> anyhow::Result<()> {
    let jobs_repo = StockSyncJobsRepo::new(db.pool());
    let events_repo = StockSyncJobEventsRepo::new(db.pool());
    let shops_repo = ShopsRepo::new(db.pool());
    let mut config_cache: HashMap<ShopId, SalesShopConfig> = HashMap::new();

    #[derive(sqlx::FromRow)]
    struct MappingRow {
        platform: String,
        platform_sku_id: String,
    }

    let jobs = jobs_repo.list_jobs_for_run(batch_limit).await?;
    for job in jobs {
        let now = Utc::now();
        let Some(job) = jobs_repo
            .try_mark_sending(job.tenant_id, job.job_id, job.updated_at, now)
            .await?
        else {
            continue;
        };

        let attempt = job.attempts.saturating_add(1);
        let config = if let Some(cfg) = config_cache.get(&job.shop_id) {
            cfg.clone()
        } else {
            let cfg = match load_sales_config_for_shop(db.pool(), job.tenant_id, job.shop_id).await
            {
                Ok(v) => v,
                Err(err) => {
                    warn!(
                        tenant_id = %job.tenant_id,
                        shop_id = %job.shop_id,
                        error = %err,
                        "failed to load sales config for stock sync; fallback to defaults"
                    );
                    SalesShopConfig::default()
                }
            };
            config_cache.insert(job.shop_id, cfg.clone());
            cfg
        };
        if !config.enable_stock_sync {
            let error = "stock sync disabled by sales config".to_string();
            let _ = jobs_repo
                .mark_failed(job.tenant_id, job.job_id, error.clone(), "DLQ", now, 0)
                .await?;
            let payload = serde_json::json!({
                "error": error,
                "classification": "DISABLED",
            });
            let _ = events_repo
                .insert_event(erp_repo::stock_sync_job_events::StockSyncJobEventCreate {
                    tenant_id: job.tenant_id,
                    job_id: job.job_id,
                    shop_id: job.shop_id,
                    sku_id: job.sku_id,
                    warehouse_id: job.warehouse_id,
                    status: "ERROR".to_string(),
                    attempt,
                    payload,
                })
                .await?;
            continue;
        }
        let sync_enabled = feature_flag_enabled(
            db,
            job.tenant_id,
            Some(job.shop_id),
            FeatureFlagKey::InventorySync,
        )
        .await?;
        if !sync_enabled {
            let error = "feature disabled: inventory sync".to_string();
            let _ = jobs_repo
                .mark_failed(job.tenant_id, job.job_id, error.clone(), "DLQ", now, 0)
                .await?;
            let payload = serde_json::json!({
                "error": error,
                "classification": "DISABLED",
            });
            let _ = events_repo
                .insert_event(erp_repo::stock_sync_job_events::StockSyncJobEventCreate {
                    tenant_id: job.tenant_id,
                    job_id: job.job_id,
                    shop_id: job.shop_id,
                    sku_id: job.sku_id,
                    warehouse_id: job.warehouse_id,
                    status: "ERROR".to_string(),
                    attempt,
                    payload,
                })
                .await?;
            continue;
        }
        let mapping: Option<MappingRow> = sqlx::query_as(
            r#"
            select m.platform, m.platform_sku_id
            from channel_sku_mappings m
            join shops s on s.shop_id = m.shop_id
            where m.tenant_id = $1
              and m.shop_id = $2
              and m.sku_id = $3
              and m.status = 'ACTIVE'
              and s.status = 'ACTIVE'
            "#,
        )
        .bind(uuid::Uuid::from(job.tenant_id))
        .bind(uuid::Uuid::from(job.shop_id))
        .bind(job.sku_id)
        .fetch_optional(db.pool())
        .await?;

        let Some(mapping) = mapping else {
            let error = "active channel mapping not found".to_string();
            let _ = jobs_repo
                .mark_failed(job.tenant_id, job.job_id, error.clone(), "DLQ", now, 0)
                .await?;
            let payload = serde_json::json!({
                "error": error,
                "classification": "PERMANENT",
            });
            let _ = events_repo
                .insert_event(erp_repo::stock_sync_job_events::StockSyncJobEventCreate {
                    tenant_id: job.tenant_id,
                    job_id: job.job_id,
                    shop_id: job.shop_id,
                    sku_id: job.sku_id,
                    warehouse_id: job.warehouse_id,
                    status: "ERROR".to_string(),
                    attempt,
                    payload,
                })
                .await?;
            continue;
        };

        let platform = match Platform::from_str(&mapping.platform) {
            Ok(platform) => platform,
            Err(err) => {
                let _ = jobs_repo
                    .mark_failed(job.tenant_id, job.job_id, err.to_string(), "DLQ", now, 0)
                    .await?;
                let payload = serde_json::json!({
                    "error": err.to_string(),
                    "classification": "PERMANENT",
                });
                let _ = events_repo
                    .insert_event(erp_repo::stock_sync_job_events::StockSyncJobEventCreate {
                        tenant_id: job.tenant_id,
                        job_id: job.job_id,
                        shop_id: job.shop_id,
                        sku_id: job.sku_id,
                        warehouse_id: job.warehouse_id,
                        status: "ERROR".to_string(),
                        attempt,
                        payload,
                    })
                    .await?;
                continue;
            }
        };

        let shop = match shops_repo.get_shop_auth(job.shop_id).await {
            Ok(shop) => shop,
            Err(err) => {
                let _ = jobs_repo
                    .mark_failed(job.tenant_id, job.job_id, err.to_string(), "DLQ", now, 0)
                    .await?;
                let payload = serde_json::json!({
                    "error": err.to_string(),
                    "classification": "PERMANENT",
                });
                let _ = events_repo
                    .insert_event(erp_repo::stock_sync_job_events::StockSyncJobEventCreate {
                        tenant_id: job.tenant_id,
                        job_id: job.job_id,
                        shop_id: job.shop_id,
                        sku_id: job.sku_id,
                        warehouse_id: job.warehouse_id,
                        status: "ERROR".to_string(),
                        attempt,
                        payload,
                    })
                    .await?;
                continue;
            }
        };

        let request = SyncStockRequest {
            platform_sku_id: mapping.platform_sku_id,
            warehouse_id: job.warehouse_id,
            target_qty: job.target_qty,
        };
        let request_payload = serde_json::json!({
            "platform": platform.to_string(),
            "platform_sku_id": request.platform_sku_id,
            "warehouse_id": request.warehouse_id,
            "target_qty": request.target_qty,
        });
        let _ = events_repo
            .insert_event(erp_repo::stock_sync_job_events::StockSyncJobEventCreate {
                tenant_id: job.tenant_id,
                job_id: job.job_id,
                shop_id: job.shop_id,
                sku_id: job.sku_id,
                warehouse_id: job.warehouse_id,
                status: "REQUEST".to_string(),
                attempt,
                payload: request_payload,
            })
            .await?;

        let adapter = adapters.platform_adapter(platform);
        let started = std::time::Instant::now();
        match adapter.sync_stock(&shop, request).await {
            Ok(response) => {
                let duration_ms = started
                    .elapsed()
                    .as_millis()
                    .min(i64::from(i32::MAX) as u128) as i32;
                let _ = jobs_repo
                    .mark_sent(job.tenant_id, job.job_id, duration_ms)
                    .await?;
                let payload = serde_json::json!({
                    "target_qty": response.target_qty,
                    "duration_ms": duration_ms,
                });
                let _ = events_repo
                    .insert_event(erp_repo::stock_sync_job_events::StockSyncJobEventCreate {
                        tenant_id: job.tenant_id,
                        job_id: job.job_id,
                        shop_id: job.shop_id,
                        sku_id: job.sku_id,
                        warehouse_id: job.warehouse_id,
                        status: "RESPONSE".to_string(),
                        attempt,
                        payload,
                    })
                    .await?;
            }
            Err(err) => {
                let err = anyhow::anyhow!(err);
                let classification = dlq::classify_error(&err);
                let duration_ms = started
                    .elapsed()
                    .as_millis()
                    .min(i64::from(i32::MAX) as u128) as i32;
                let (status, next_retry_at) = if classification == dlq::FailureClass::Transient
                    && (attempt as u32) <= retry_policy.max_attempts
                {
                    let delay = retry_policy.next_delay(attempt as u32);
                    (
                        "FAILED",
                        Utc::now() + ChronoDuration::milliseconds(delay.as_millis() as i64),
                    )
                } else {
                    ("DLQ", Utc::now())
                };

                let _ = jobs_repo
                    .mark_failed(
                        job.tenant_id,
                        job.job_id,
                        err.to_string(),
                        status,
                        next_retry_at,
                        duration_ms,
                    )
                    .await?;
                let payload = serde_json::json!({
                    "error": err.to_string(),
                    "classification": classification.as_str(),
                    "next_action": status,
                    "duration_ms": duration_ms,
                });
                let _ = events_repo
                    .insert_event(erp_repo::stock_sync_job_events::StockSyncJobEventCreate {
                        tenant_id: job.tenant_id,
                        job_id: job.job_id,
                        shop_id: job.shop_id,
                        sku_id: job.sku_id,
                        warehouse_id: job.warehouse_id,
                        status: "ERROR".to_string(),
                        attempt,
                        payload,
                    })
                    .await?;
                warn!(
                    tenant_id = %job.tenant_id,
                    shop_id = %job.shop_id,
                    sku_id = %job.sku_id,
                    error = %err,
                    "inventory sync failed"
                );
            }
        }
    }

    Ok(())
}

const WMS_WORKFLOW_CONFIG_KEY: &str = "wms.workflow.config";

async fn run_wave_jobs_once(db: &Db) -> anyhow::Result<()> {
    let tenant_ids: Vec<uuid::Uuid> = sqlx::query_scalar(
        r#"
        select tenant_id
        from tenants
        "#,
    )
    .fetch_all(db.pool())
    .await?;

    let warehouses_repo = WarehousesRepo::new(db.pool());
    let wms_repo = WmsRepo::new(db.pool());

    for tenant_uuid in tenant_ids {
        let tenant_id = TenantId(tenant_uuid);
        let warehouses = match warehouses_repo
            .list_warehouses(
                tenant_id,
                WarehouseListFilters {
                    keyword: None,
                    status: Some("ACTIVE".to_string()),
                    offset: 0,
                    limit: 200,
                },
            )
            .await
        {
            Ok(v) => v.items,
            Err(err) => {
                warn!(tenant_id = %tenant_id, error = %err, "wave auto run warehouses failed");
                continue;
            }
        };

        let require_audit =
            match fetch_tenant_setting_value(db.pool(), tenant_id, "inv.outbound_require_audit")
                .await
            {
                Ok(value) => value.as_ref().and_then(parse_bool_setting).unwrap_or(false),
                Err(err) => {
                    warn!(tenant_id = %tenant_id, error = %err, "wave auto run setting failed");
                    false
                }
            };
        let status = if require_audit { "AUDITED" } else { "DRAFT" };

        for warehouse in warehouses {
            let workflow_enabled = match fetch_warehouse_workflow_enabled(
                db.pool(),
                tenant_id,
                warehouse.warehouse_id,
            )
            .await
            {
                Ok(v) => v,
                Err(err) => {
                    warn!(
                        tenant_id = %tenant_id,
                        warehouse_id = %warehouse.warehouse_id,
                        error = %err,
                        "wave workflow config load failed"
                    );
                    continue;
                }
            };
            if !workflow_enabled {
                continue;
            }

            let rules = match wms_repo
                .list_wave_rules(
                    tenant_id,
                    erp_repo::wms::WaveRuleListFilters {
                        warehouse_id: Some(warehouse.warehouse_id),
                        carrier_code: None,
                        enabled: Some(true),
                        offset: 0,
                        limit: 50,
                    },
                )
                .await
            {
                Ok(v) => v.items,
                Err(err) => {
                    warn!(
                        tenant_id = %tenant_id,
                        warehouse_id = %warehouse.warehouse_id,
                        error = %err,
                        "wave rule load failed"
                    );
                    continue;
                }
            };
            if rules.is_empty() {
                continue;
            }

            for rule in rules {
                let cutoff_at = resolve_wave_cutoff_at(rule.cutoff_time);
                let max_docs = rule.max_orders.unwrap_or(50).clamp(1, 500) as i64;
                let candidates = match wms_repo
                    .list_wave_candidates(
                        tenant_id,
                        warehouse.warehouse_id,
                        status,
                        cutoff_at,
                        rule.carrier_code.as_deref(),
                        max_docs,
                    )
                    .await
                {
                    Ok(v) => v,
                    Err(err) => {
                        warn!(
                            tenant_id = %tenant_id,
                            warehouse_id = %warehouse.warehouse_id,
                            error = %err,
                            "wave candidate load failed"
                        );
                        continue;
                    }
                };
                let selected_ids = select_wave_candidates(candidates, max_docs, rule.max_qty);
                if selected_ids.is_empty() {
                    continue;
                }

                let input = erp_repo::wms::WaveCreateInput {
                    tenant_id,
                    warehouse_id: warehouse.warehouse_id,
                    cutoff_at,
                    max_docs: selected_ids.len().max(1) as i64,
                    auto_pick: false,
                    outbound_doc_ids: Some(selected_ids),
                };
                let result = if require_audit {
                    wms_repo.create_wave_with_status(input, "AUDITED").await
                } else {
                    wms_repo.create_wave(input).await
                };
                if let Err(err) = result {
                    let message = err.to_string();
                    if message.contains("no outbound docs available") {
                        continue;
                    }
                    warn!(
                        tenant_id = %tenant_id,
                        warehouse_id = %warehouse.warehouse_id,
                        error = %err,
                        "wave auto run failed"
                    );
                }
            }
        }
    }

    Ok(())
}

fn resolve_wave_cutoff_at(cutoff_time: Option<NaiveTime>) -> chrono::DateTime<Utc> {
    let now = Utc::now();
    let Some(time) = cutoff_time else {
        return now;
    };
    let today = now.date_naive();
    let naive_cutoff = chrono::NaiveDateTime::new(today, time);
    let mut cutoff = chrono::DateTime::<Utc>::from_naive_utc_and_offset(naive_cutoff, Utc);
    if cutoff > now {
        cutoff -= ChronoDuration::days(1);
    }
    cutoff
}

fn select_wave_candidates(
    candidates: Vec<erp_repo::wms::WaveCandidate>,
    max_docs: i64,
    max_qty: Option<i64>,
) -> Vec<uuid::Uuid> {
    let max_docs = max_docs.max(1) as usize;
    let mut selected = Vec::new();
    let mut total_qty: i64 = 0;
    for candidate in candidates {
        if selected.len() >= max_docs {
            break;
        }
        if let Some(limit) = max_qty {
            if !selected.is_empty() && total_qty + candidate.total_qty > limit {
                break;
            }
            if selected.is_empty() && candidate.total_qty > limit {
                selected.push(candidate.outbound_doc_id);
                break;
            }
            total_qty += candidate.total_qty;
        }
        selected.push(candidate.outbound_doc_id);
    }
    selected
}

fn parse_bool_setting(value: &serde_json::Value) -> Option<bool> {
    if let Some(result) = value.as_bool() {
        return Some(result);
    }
    if let Some(obj) = value.as_object() {
        if let Some(result) = obj.get("value").and_then(|item| item.as_bool()) {
            return Some(result);
        }
    }
    None
}

fn parse_workflow_config_value(value: &serde_json::Value) -> Option<WarehouseWorkflowConfigLite> {
    if let Ok(config) = serde_json::from_value::<WarehouseWorkflowConfigLite>(value.clone()) {
        return Some(config);
    }
    if let Some(nested) = value.get("value") {
        if let Ok(config) = serde_json::from_value::<WarehouseWorkflowConfigLite>(nested.clone()) {
            return Some(config);
        }
    }
    None
}

async fn fetch_tenant_setting_value(
    pool: &PgPool,
    tenant_id: TenantId,
    key: &str,
) -> anyhow::Result<Option<serde_json::Value>> {
    let repo = SettingsRepo::new(pool);
    let settings = repo
        .list_settings(
            tenant_id,
            SettingListFilters {
                scope_type: Some("TENANT".to_string()),
                scope_id: None,
                key: Some(key.to_string()),
            },
        )
        .await?;
    Ok(settings
        .into_iter()
        .next()
        .map(|setting| setting.value_json))
}

async fn fetch_warehouse_setting_value(
    pool: &PgPool,
    tenant_id: TenantId,
    warehouse_id: uuid::Uuid,
    key: &str,
) -> anyhow::Result<Option<serde_json::Value>> {
    let repo = SettingsRepo::new(pool);
    let settings = repo
        .list_settings(
            tenant_id,
            SettingListFilters {
                scope_type: Some("WAREHOUSE".to_string()),
                scope_id: Some(warehouse_id),
                key: Some(key.to_string()),
            },
        )
        .await?;
    Ok(settings
        .into_iter()
        .next()
        .map(|setting| setting.value_json))
}

async fn fetch_warehouse_workflow_enabled(
    pool: &PgPool,
    tenant_id: TenantId,
    warehouse_id: uuid::Uuid,
) -> anyhow::Result<bool> {
    let value =
        fetch_warehouse_setting_value(pool, tenant_id, warehouse_id, WMS_WORKFLOW_CONFIG_KEY)
            .await?;
    if let Some(value) = value {
        if let Some(config) = parse_workflow_config_value(&value) {
            return Ok(config.enable_wave.unwrap_or(false));
        }
    }
    Ok(false)
}

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
        let metrics_app = Router::new().route("/metrics", get(metrics_handler));
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
mod tests {
    use super::*;
    use anyhow::Result;
    use chrono::{Datelike, Duration as ChronoDuration, Utc};
    use erp_domain::OrderItemProjection;
    use erp_types::{AfterSaleState, OrderId, OrderStatus, Platform, RefundState};
    use sqlx::{postgres::PgPoolOptions, PgPool};
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tracing_subscriber::fmt::MakeWriter;

    #[test]
    fn event_router_module_exports_route_event_symbol() {
        let _ = super::event_router::route_event;
    }

    #[derive(Clone, Default)]
    struct LogCapture {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl LogCapture {
        fn contents(&self) -> String {
            let data = self.buffer.lock().expect("log buffer");
            String::from_utf8_lossy(&data).to_string()
        }
    }

    struct LogWriter {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl Write for LogWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let mut data = self.buffer.lock().expect("log buffer");
            data.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for LogCapture {
        type Writer = LogWriter;

        fn make_writer(&'a self) -> Self::Writer {
            LogWriter {
                buffer: self.buffer.clone(),
            }
        }
    }

    fn test_order_with_refund_case(
        after_sale_id: Option<&str>,
        refund_state: RefundState,
        after_sale_state: AfterSaleState,
        refund_amount_cent: Option<i64>,
    ) -> OrderProjection {
        OrderProjection {
            tenant_id: "00000000-0000-0000-0000-000000000000".parse().unwrap(),
            shop_id: "00000000-0000-0000-0000-000000000000".parse().unwrap(),
            platform: Platform::Jd,
            platform_order_id: "P-ORDER-1".to_string(),
            status: OrderStatus::Shipped,
            total_amount_cent: 100,
            goods_amount_cent: None,
            postage_amount_cent: None,
            discount_amount_cent: None,
            platform_discount_cent: None,
            seller_discount_cent: None,
            coupon_discount_cent: None,
            currency: "CNY".to_string(),
            buyer_name: None,
            receiver_name: None,
            receiver_phone: None,
            receiver_address: None,
            pay_at: None,
            ship_at: None,
            finish_at: None,
            cancel_at: None,
            buyer_message: None,
            seller_remark: None,
            logistics_company: None,
            tracking_no: None,
            order_created_at: Utc::now(),
            platform_updated_at: Some(Utc::now()),
            raw: serde_json::json!({}),
            items: vec![OrderItemProjection {
                line_id: "L1".to_string(),
                sku_id: Some("SKU-1".to_string()),
                title: Some("测试商品".to_string()),
                qty: 1,
                price_cent: 100,
                amount_cent: 100,
                pay_amount_cent: Some(100),
                discount_cent: None,
                refund_state,
                after_sale_state,
                refund_amount_cent,
                after_sale_type: Some("refund".to_string()),
                after_sale_id: after_sale_id.map(|v| v.to_string()),
                raw: serde_json::json!({}),
            }],
        }
    }

    #[test]
    fn refund_sync_dedup_fingerprint_distinguishes_after_sale_case() {
        let order_a = test_order_with_refund_case(
            Some("CASE-A"),
            RefundState::Processing,
            AfterSaleState::Processing,
            Some(1200),
        );
        let order_b = test_order_with_refund_case(
            Some("CASE-B"),
            RefundState::Processing,
            AfterSaleState::Processing,
            Some(1200),
        );
        let fp_a = refund_sync_dedup_fingerprint(&order_a).expect("fingerprint A");
        let fp_b = refund_sync_dedup_fingerprint(&order_b).expect("fingerprint B");
        assert_ne!(fp_a, fp_b);
    }

    #[test]
    fn compute_sync_dedup_key_includes_refund_fingerprint() {
        let k1 = compute_sync_dedup_key(&Platform::Jd, "P-ORDER-1", "REFUND_SYNC", "1", Some("A"));
        let k2 = compute_sync_dedup_key(&Platform::Jd, "P-ORDER-1", "REFUND_SYNC", "1", Some("B"));
        let k3 = compute_sync_dedup_key(&Platform::Jd, "P-ORDER-1", "REFUND_SYNC", "1", None);
        assert_ne!(k1, k2);
        assert_ne!(k1, k3);
    }

    async fn setup_schema(pool: &PgPool) -> Result<()> {
        let statements = [
            "create extension if not exists pgcrypto",
            r#"
            create table if not exists tenants (
              tenant_id uuid primary key,
              name text not null,
              status text not null default 'ACTIVE',
              pdd_only_mode boolean not null default false,
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now()
            )
            "#,
            r#"
            create table if not exists shops (
              shop_id uuid primary key,
              tenant_id uuid not null,
              platform text not null,
              platform_shop_id text not null,
              name text not null default '',
              shop_name text not null default '',
              status text not null default 'AUTHORIZED',
              default_carrier_code text,
              credentials jsonb not null default '{}'::jsonb,
              auth_meta_json jsonb not null default '{}'::jsonb,
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              unique (tenant_id, platform, platform_shop_id)
            )
            "#,
            r#"
            create table if not exists order_key_index (
              tenant_id uuid not null,
              platform text not null,
              platform_order_id text not null,
              order_id uuid not null,
              order_month date not null,
              shop_id uuid not null,
              order_created_at timestamptz not null,
              updated_at timestamptz not null default now(),
              primary key (tenant_id, platform, platform_order_id)
            )
            "#,
            r#"
            create table if not exists order_index (
              tenant_id uuid not null,
              order_id uuid not null,
              order_month date not null,
              platform text not null,
              platform_order_id text not null,
              shop_id uuid not null,
              order_created_at timestamptz not null,
              updated_at timestamptz not null default now(),
              primary key (tenant_id, order_id)
            )
            "#,
            r#"
            create table if not exists orders (
              tenant_id uuid not null,
              order_month date not null,
              order_id uuid not null,
              platform text not null,
              shop_id uuid not null,
                platform_order_id text not null,
                status text not null,
                oms_status text not null default 'NEW',
                order_status text not null default 'PENDING_REVIEW',
                fulfillment_status text not null default 'NOT_ALLOCATED',
                exception_status text not null default 'NONE',
                review_status text not null default 'NOT_REVIEWED',
                exception_flags jsonb not null default '[]'::jsonb,
                version int not null default 0,
              total_amount_cent bigint not null,
              goods_amount_cent bigint,
              postage_amount_cent bigint,
              discount_amount_cent bigint,
              platform_discount_cent bigint,
              seller_discount_cent bigint,
              coupon_discount_cent bigint,
              currency text not null,
              buyer_name text,
              receiver_name text,
              receiver_phone text,
              receiver_address text,
              pay_at timestamptz,
              payment_status text,
              ship_at timestamptz,
              finish_at timestamptz,
              cancel_at timestamptz,
              cancel_reason text,
              buyer_message text,
              seller_remark text,
              logistics_company text,
              tracking_no text,
              order_created_at timestamptz not null,
              platform_updated_at timestamptz,
              raw jsonb not null default '{}'::jsonb,
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              primary key (tenant_id, order_month, order_id),
              unique (tenant_id, order_month, platform, platform_order_id)
            )
            "#,
            r#"
            create table if not exists order_items (
              tenant_id uuid not null,
              order_month date not null,
              order_id uuid not null,
              line_id text not null,
              sku_id text,
              title text,
              qty int not null,
              price_cent bigint not null,
              amount_cent bigint not null,
              pay_amount_cent bigint,
              discount_cent bigint,
              refund_state text not null,
              after_sale_state text not null,
              refund_amount_cent bigint,
              after_sale_type text,
              after_sale_id text,
              raw jsonb not null default '{}'::jsonb,
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              primary key (tenant_id, order_month, order_id, line_id)
            )
            "#,
            r#"
            create table if not exists order_review_rules (
              rule_id uuid primary key default gen_random_uuid(),
              tenant_id uuid not null,
              name text not null,
              enabled boolean not null default true,
              priority int not null default 0,
              conditions_json jsonb not null default '[]'::jsonb,
              action text not null,
              reason text not null default '',
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              unique (tenant_id, name)
            )
            "#,
            r#"
            create table if not exists order_review_hits (
              tenant_id uuid not null,
              order_id uuid not null,
              rule_id uuid not null,
              hit boolean not null default false,
              message text not null default '',
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              primary key (tenant_id, order_id, rule_id)
            )
            "#,
            r#"
            create table if not exists order_reviews (
              tenant_id uuid not null references tenants(tenant_id),
              order_id uuid not null,
              status text not null default 'PENDING',
              reviewed_by uuid,
              reviewed_at timestamptz,
              notes text not null default '',
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              primary key (tenant_id, order_id)
            )
            "#,
            r#"
            create table if not exists order_tags (
              tenant_id uuid not null,
              order_id uuid not null,
              tag text not null,
              created_by uuid,
              created_at timestamptz not null default now(),
              primary key (tenant_id, order_id, tag)
            )
            "#,
            r#"
            create table if not exists order_notes (
              note_id uuid primary key default gen_random_uuid(),
              tenant_id uuid not null,
              order_id uuid not null,
              note text not null,
              created_by uuid,
              created_at timestamptz not null default now()
            )
            "#,
            r#"
            create table if not exists customer_blacklist (
              blacklist_id uuid primary key default gen_random_uuid(),
              tenant_id uuid not null,
              key_type text not null,
              key_value text not null,
              reason text not null default '',
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              unique (tenant_id, key_type, key_value)
            )
            "#,
            r#"
            create table if not exists after_sale_cases (
              tenant_id uuid not null,
              case_id uuid not null default gen_random_uuid(),
              platform text not null,
              shop_id uuid not null,
              after_sale_id text not null,
              after_sale_type text,
              refund_state text not null default 'NONE',
              after_sale_state text not null default 'NONE',
              refund_amount_cent bigint not null default 0,
              item_count int not null default 0,
              total_qty int not null default 0,
              order_count int not null default 0,
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              primary key (tenant_id, case_id),
              unique (tenant_id, platform, shop_id, after_sale_id)
            )
            "#,
            r#"
            create table if not exists after_sale_case_items (
              tenant_id uuid not null,
              case_id uuid not null,
              order_id uuid not null,
              order_month date not null,
              line_id text not null,
              sku_id text,
              title text,
              qty int not null,
              refund_state text not null default 'NONE',
              after_sale_state text not null default 'NONE',
              refund_amount_cent bigint,
              after_sale_type text,
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              primary key (tenant_id, case_id, order_id, line_id)
            )
            "#,
            r#"
            create table if not exists outbox_events (
              tenant_id uuid not null,
              event_month date not null,
              outbox_id uuid not null default gen_random_uuid(),
              event_type text not null,
              payload jsonb not null,
              trace_id text,
              dedup_key text,
              status text not null default 'PENDING',
              attempts int not null default 0,
              next_retry_at timestamptz not null default now(),
              last_error text not null default '',
              last_attempt_at timestamptz not null default now(),
              published_at timestamptz,
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              primary key (tenant_id, event_month, outbox_id)
            )
            "#,
            r#"
            create table if not exists worker_effects (
              tenant_id uuid not null,
              outbox_id uuid not null,
              event_type text not null,
              status text not null default 'PROCESSING',
              last_error text not null default '',
              processed_at timestamptz not null default now(),
              primary key (tenant_id, outbox_id)
            )
            "#,
            r#"
            create table if not exists feature_flags (
              tenant_id uuid not null,
              scope text not null,
              shop_id uuid,
              key text not null,
              enabled boolean not null default true,
              updated_at timestamptz not null default now(),
              unique (tenant_id, scope, shop_id, key)
            )
            "#,
            r#"
            create table if not exists dead_letters (
              dead_letter_id uuid primary key default gen_random_uuid(),
              tenant_id uuid not null,
              outbox_id uuid not null,
              event_type text not null,
              payload jsonb not null,
              error text not null,
              attempts int not null,
              classification text not null,
              next_action text not null,
              status text not null default 'PENDING',
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              replayed_at timestamptz
            )
            "#,
            r#"
            create unique index if not exists idx_dead_letters_outbox
              on dead_letters(tenant_id, outbox_id)
            "#,
            r#"
            create table if not exists channel_sku_mappings (
              mapping_id uuid primary key default gen_random_uuid(),
              tenant_id uuid not null,
              shop_id uuid not null,
              platform text not null,
              platform_sku_id text not null,
              platform_outer_sku_id text,
              sku_id uuid not null,
              status text not null default 'ACTIVE',
              confidence int not null default 0,
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              unique (tenant_id, shop_id, platform, platform_sku_id)
            )
            "#,
            r#"
            create table if not exists platform_tokens (
              tenant_id uuid not null,
              shop_id uuid not null,
              token_status text not null default 'ACTIVE',
              access_token text,
              refresh_token text,
              expires_at timestamptz,
              scopes text,
              last_refreshed_at timestamptz,
              refresh_fail_count int not null default 0,
              last_error text not null default '',
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              primary key (tenant_id, shop_id)
            )
            "#,
            r#"
            create table if not exists sync_jobs (
              job_id uuid primary key default gen_random_uuid(),
              tenant_id uuid not null,
              shop_id uuid not null,
              job_type text not null,
              cursor text,
              watermark timestamptz,
              status text not null default 'PENDING',
              last_error text not null default '',
              last_success_at timestamptz,
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              unique (tenant_id, shop_id, job_type)
            )
            "#,
            r#"
            create table if not exists inbox_raw_payloads (
              raw_id bigserial primary key,
              tenant_id uuid not null,
              platform text not null,
              shop_id uuid not null,
              headers jsonb not null default '{}'::jsonb,
              body bytea not null,
              received_at timestamptz not null default now()
            )
            "#,
            r#"
            create table if not exists inbox_dedup (
              tenant_id uuid not null,
              dedup_key text not null,
              first_seen_at timestamptz not null default now(),
              last_seen_at timestamptz not null default now(),
              seen_count int not null default 1,
              primary key (tenant_id, dedup_key)
            )
            "#,
            r#"
            create table if not exists inbox_events (
              tenant_id uuid not null,
              event_month date not null,
              inbox_event_id uuid not null default gen_random_uuid(),
              platform text not null,
              shop_id uuid not null,
              dedup_key text not null,
              raw_id bigint not null,
              status text not null default 'RECEIVED',
              error text not null default '',
              verify_status text not null default 'PASSED',
              failed_reason text not null default '',
              received_at timestamptz not null default now(),
              processed_at timestamptz,
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              primary key (tenant_id, event_month, inbox_event_id),
              unique (tenant_id, event_month, dedup_key)
            )
            "#,
            r#"
            create table if not exists mapping_rules (
              rule_id uuid primary key default gen_random_uuid(),
              tenant_id uuid not null,
              platform text not null,
              entity_type text not null,
              version int not null,
              enabled boolean not null default true,
              rule_json jsonb not null default '{}'::jsonb,
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              unique (tenant_id, platform, entity_type, version)
            )
            "#,
        ];

        for stmt in statements {
            sqlx::query(stmt).execute(pool).await?;
        }
        Ok(())
    }

    async fn setup_shipping_schema(pool: &PgPool) -> Result<()> {
        setup_schema(pool).await?;
        let statements = [
            r#"
            create table if not exists warehouses (
              warehouse_id uuid primary key,
              tenant_id uuid not null,
              name text not null,
              code text not null,
              is_default boolean not null default false,
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              unique (tenant_id, code)
            )
            "#,
            r#"
            create table if not exists skus (
              sku_id uuid primary key,
              tenant_id uuid not null,
              sku_kind text not null default 'NORMAL'
            )
            "#,
            r#"
            create table if not exists shipments (
              shipment_id uuid primary key,
              tenant_id uuid not null,
              shop_id uuid not null,
              order_id uuid not null,
              platform text not null,
              platform_order_id text not null,
                warehouse_id uuid,
                carrier_code text,
                tracking_no text,
                status text not null default 'REQUESTED',
                shipment_status text not null default 'REQUESTED',
                status_version int not null default 0,
              requested_at timestamptz,
              shipped_at timestamptz,
              updated_at timestamptz not null default now(),
              unique (tenant_id, order_id)
            )
            "#,
            r#"
            create table if not exists shipment_packages (
              package_id uuid primary key default gen_random_uuid(),
              tenant_id uuid not null,
              shipment_id uuid not null,
              package_no text not null,
              carrier_code text,
              tracking_no text,
              weight_gram int,
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              unique (tenant_id, shipment_id, package_no)
            )
            "#,
            r#"
            create table if not exists outbound_docs (
              outbound_doc_id uuid primary key,
              tenant_id uuid not null,
              shipment_id uuid,
              status text not null default 'DRAFT',
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now()
            )
            "#,
            r#"
            create table if not exists inventory_balances (
              tenant_id uuid not null,
              warehouse_id uuid not null,
              sku_id uuid not null,
              on_hand bigint not null default 0,
              reserved bigint not null default 0,
              in_transit bigint not null default 0,
              frozen bigint not null default 0,
              updated_at timestamptz not null default now(),
              primary key (tenant_id, warehouse_id, sku_id)
            )
            "#,
            r#"
            create table if not exists inventory_reservations (
              reservation_id uuid primary key default gen_random_uuid(),
              tenant_id uuid not null,
              warehouse_id uuid not null,
              sku_id uuid not null,
              order_id uuid,
              order_item_id uuid,
              qty bigint not null,
              status text not null default 'ACTIVE',
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              unique (tenant_id, order_item_id)
            )
            "#,
            r#"
            create table if not exists inventory_ledgers (
              ledger_id uuid primary key default gen_random_uuid(),
              tenant_id uuid not null,
              warehouse_id uuid not null,
              sku_id uuid not null,
              delta_on_hand bigint not null,
              delta_reserved bigint not null,
              delta_in_transit bigint not null,
              delta_frozen bigint not null,
              reason_type text not null,
              ref_type text,
              ref_id text,
              occurred_at timestamptz not null default now()
            )
            "#,
            r#"
            create table if not exists shipment_push_attempts (
              attempt_id uuid primary key default gen_random_uuid(),
              tenant_id uuid not null,
              outbox_id uuid not null,
              shipment_id uuid not null,
              shop_id uuid not null,
              platform text not null,
              platform_order_id text not null,
              carrier_code text,
              tracking_no text,
              shipped_at timestamptz,
              status text not null,
              attempt int not null,
              duration_ms int not null,
              error_code text not null default '',
              error_message text not null default '',
              request_json jsonb not null default '{}'::jsonb,
              response_json jsonb not null default '{}'::jsonb,
              next_retry_at timestamptz,
              dead_letter_id uuid,
              trigger text not null default 'AUTO',
              created_at timestamptz not null default now()
            )
            "#,
        ];

        for stmt in statements {
            sqlx::query(stmt).execute(pool).await?;
        }
        Ok(())
    }

    async fn setup_printing_schema(pool: &PgPool) -> Result<()> {
        let statements = [
            "create extension if not exists pgcrypto",
            r#"
            create table if not exists tenants (
              tenant_id uuid primary key,
              name text not null,
              status text not null default 'ACTIVE',
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now()
            )
            "#,
            r#"
            create table if not exists print_templates (
              template_id uuid primary key,
              tenant_id uuid not null,
              shop_id uuid not null,
              platform text not null,
              template_type text not null,
              platform_template_id text,
              version int not null default 1,
              content_format text not null default 'HTML',
              content jsonb not null default '{}'::jsonb,
              is_active boolean not null default true,
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now()
            )
            "#,
            r#"
            create table if not exists print_jobs (
              job_id uuid primary key,
              tenant_id uuid not null,
              type text,
              ref_type text,
              ref_id uuid,
              template_id uuid not null,
              template_version int not null,
              requested_by_user_id uuid not null,
              status text not null default 'PENDING',
              dispatch_status text not null default 'QUEUED',
              payload jsonb not null default '{}'::jsonb,
              output_url text,
              output_content text,
              attempts int not null default 0,
              last_error text not null default '',
              device_fingerprint text,
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now()
            )
            "#,
            r#"
            create table if not exists print_job_events (
              event_id uuid primary key,
              tenant_id uuid not null,
              job_id uuid not null,
              status text not null,
              actor_type text not null,
              actor_user_id uuid,
              error text,
              created_at timestamptz not null default now()
            )
            "#,
            r#"
            create table if not exists audit_logs (
              audit_id uuid primary key default gen_random_uuid(),
              tenant_id uuid not null,
              user_id uuid,
              action text not null,
              entity_type text not null,
              entity_id text,
              before_json jsonb,
              after_json jsonb,
              ip text,
              ua text,
              created_at timestamptz not null default now()
            )
            "#,
        ];

        for stmt in statements {
            sqlx::query(stmt).execute(pool).await?;
        }
        Ok(())
    }

    async fn try_pool() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
        PgPoolOptions::new()
            .acquire_timeout(Duration::from_secs(2))
            .connect(&url)
            .await
            .ok()
    }

    #[tokio::test]
    async fn print_job_renders_text_output() -> Result<()> {
        let Some(pool) = try_pool().await else {
            eprintln!("skipping print_job_renders_text_output: db unavailable");
            return Ok(());
        };
        setup_printing_schema(&pool).await?;

        let tenant_id = TenantId(uuid::Uuid::new_v4());
        sqlx::query(
            r#"
            insert into tenants (tenant_id, name, status)
            values ($1, 'print-tenant', 'ACTIVE')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;

        let template_id = uuid::Uuid::new_v4();
        let shop_id = uuid::Uuid::new_v4();
        sqlx::query(
            r#"
            insert into print_templates (
              template_id, tenant_id, shop_id, platform, template_type, version,
              content_format, content, is_active
            )
            values ($1, $2, $3, 'taobao', 'SHIPPING_LABEL', 1, 'TEXT', $4, true)
            "#,
        )
        .bind(template_id)
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shop_id)
        .bind(serde_json::json!("LABEL {{payload}}"))
        .execute(&pool)
        .await?;

        let job_id = uuid::Uuid::new_v4();
        sqlx::query(
            r#"
            insert into print_jobs (
              job_id, tenant_id, type, ref_type, ref_id, template_id, template_version,
              requested_by_user_id, status, payload, attempts, last_error
            )
            values ($1, $2, 'SHIPPING_LABEL', 'SHIPMENT', $3, $4, 1, $5, 'PENDING', $6, 0, '')
            "#,
        )
        .bind(job_id)
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::new_v4())
        .bind(template_id)
        .bind(uuid::Uuid::new_v4())
        .bind(serde_json::json!({"tracking_no": "TEST-100"}))
        .execute(&pool)
        .await?;

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
        let db = Db::connect_lazy(&database_url)?;
        let env = OutboxEnvelope {
            tenant_id,
            outbox_id: uuid::Uuid::new_v4(),
            event_type: "PRINT_JOB_QUEUED".to_string(),
            payload: serde_json::json!({"job_id": job_id}),
            produced_at: Utc::now(),
            trace_id: None,
        };

        handle_print_job_queued(&db, env).await?;

        #[derive(sqlx::FromRow)]
        struct JobRow {
            status: String,
            output_url: Option<String>,
            output_content: Option<String>,
            attempts: i32,
        }

        let row: JobRow = sqlx::query_as(
            r#"
            select status, output_url, output_content, attempts
            from print_jobs
            where tenant_id = $1 and job_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(job_id)
        .fetch_one(&pool)
        .await?;

        assert_eq!(row.status, "DONE");
        assert_eq!(row.attempts, 1);
        let content = row.output_content.unwrap_or_default();
        assert!(content.contains("tracking_no"));
        assert!(!content.to_lowercase().contains("<html"));
        assert!(row.output_url.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn mapping_missing_emits_outbox() -> Result<()> {
        let Some(pool) = try_pool().await else {
            eprintln!("skipping mapping_missing_emits_outbox: db unavailable");
            return Ok(());
        };
        setup_schema(&pool).await?;

        let tenant_id = TenantId(uuid::Uuid::new_v4());
        let shop_id = ShopId(uuid::Uuid::new_v4());

        sqlx::query(
            r#"
            insert into tenants (tenant_id, name, status)
            values ($1, $2, 'ACTIVE')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind("test-tenant")
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
            values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED')
            "#,
        )
        .bind(uuid::Uuid::from(shop_id))
        .bind(uuid::Uuid::from(tenant_id))
        .bind(Platform::Taobao.to_string())
        .bind("TEST-SHOP")
        .bind("test-shop")
        .execute(&pool)
        .await?;

        let mut order = OrderProjection {
            tenant_id,
            shop_id,
            platform: Platform::Taobao,
            platform_order_id: "ORDER-1".to_string(),
            status: OrderStatus::PendingPayment,
            total_amount_cent: 100,
            goods_amount_cent: None,
            postage_amount_cent: None,
            discount_amount_cent: None,
            platform_discount_cent: None,
            seller_discount_cent: None,
            coupon_discount_cent: None,
            currency: "CNY".to_string(),
            buyer_name: None,
            receiver_name: None,
            receiver_phone: None,
            receiver_address: None,
            pay_at: None,
            ship_at: None,
            finish_at: None,
            cancel_at: None,
            buyer_message: None,
            seller_remark: None,
            logistics_company: None,
            tracking_no: None,
            order_created_at: Utc::now(),
            platform_updated_at: None,
            raw: serde_json::json!({}),
            items: vec![OrderItemProjection {
                line_id: "LINE-1".to_string(),
                sku_id: Some("UNKNOWN-SKU".to_string()),
                title: Some("Test Item".to_string()),
                qty: 1,
                price_cent: 100,
                amount_cent: 100,
                pay_amount_cent: None,
                discount_cent: None,
                refund_state: RefundState::None,
                after_sale_state: AfterSaleState::None,
                refund_amount_cent: None,
                after_sale_type: None,
                after_sale_id: None,
                raw: serde_json::json!({}),
            }],
        };

        let outbox_repo = OutboxRepo::new(&pool);
        apply_sku_mappings(&pool, &outbox_repo, &mut order).await?;

        let orders_repo = OrdersRepo::new(&pool);
        let order_id = orders_repo.upsert_order_projection(&order).await?;

        let stored_sku: Option<String> = sqlx::query_scalar(
            r#"
            select sku_id
            from order_items
            where tenant_id = $1 and order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_one(&pool)
        .await?;
        assert!(stored_sku.is_none());

        #[derive(sqlx::FromRow)]
        struct OutboxRow {
            event_type: String,
            payload: serde_json::Value,
        }

        let row: OutboxRow = sqlx::query_as(
            r#"
            select event_type, payload
            from outbox_events
            where event_type = 'MAPPING_MISSING'
            "#,
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(row.event_type, "MAPPING_MISSING");
        let tenant_id_str = tenant_id.to_string();
        let shop_id_str = shop_id.to_string();

        assert_eq!(
            row.payload.get("tenant_id").and_then(|v| v.as_str()),
            Some(tenant_id_str.as_str())
        );
        assert_eq!(
            row.payload.get("shop_id").and_then(|v| v.as_str()),
            Some(shop_id_str.as_str())
        );
        assert_eq!(
            row.payload.get("platform").and_then(|v| v.as_str()),
            Some("taobao")
        );
        assert_eq!(
            row.payload.get("platform_sku_id").and_then(|v| v.as_str()),
            Some("UNKNOWN-SKU")
        );
        assert_eq!(
            row.payload
                .get("platform_order_id")
                .and_then(|v| v.as_str()),
            Some("ORDER-1")
        );
        Ok(())
    }

    #[tokio::test]
    async fn order_upsert_event_idempotent_for_cases() -> Result<()> {
        let Some(pool) = try_pool().await else {
            eprintln!("skipping order_upsert_event_idempotent_for_cases: db unavailable");
            return Ok(());
        };
        setup_schema(&pool).await?;

        let tenant_id = TenantId(uuid::Uuid::new_v4());
        let shop_id = ShopId(uuid::Uuid::new_v4());

        sqlx::query(
            r#"
            insert into tenants (tenant_id, name, status)
            values ($1, $2, 'ACTIVE')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind("test-tenant")
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
            values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED')
            "#,
        )
        .bind(uuid::Uuid::from(shop_id))
        .bind(uuid::Uuid::from(tenant_id))
        .bind(Platform::Taobao.to_string())
        .bind("SHOP-CASE")
        .bind("shop")
        .execute(&pool)
        .await?;

        let order = OrderProjection {
            tenant_id,
            shop_id,
            platform: Platform::Taobao,
            platform_order_id: "ORDER-CASE-1".to_string(),
            status: OrderStatus::Completed,
            total_amount_cent: 100,
            goods_amount_cent: None,
            postage_amount_cent: None,
            discount_amount_cent: None,
            platform_discount_cent: None,
            seller_discount_cent: None,
            coupon_discount_cent: None,
            currency: "CNY".to_string(),
            buyer_name: None,
            receiver_name: None,
            receiver_phone: None,
            receiver_address: None,
            pay_at: None,
            ship_at: None,
            finish_at: None,
            cancel_at: None,
            buyer_message: None,
            seller_remark: None,
            logistics_company: None,
            tracking_no: None,
            order_created_at: Utc::now(),
            platform_updated_at: None,
            raw: serde_json::json!({}),
            items: vec![OrderItemProjection {
                line_id: "LINE-1".to_string(),
                sku_id: Some("SKU-1".to_string()),
                title: Some("Return Item".to_string()),
                qty: 1,
                price_cent: 100,
                amount_cent: 100,
                pay_amount_cent: None,
                discount_cent: None,
                refund_state: RefundState::Processing,
                after_sale_state: AfterSaleState::Processing,
                refund_amount_cent: None,
                after_sale_type: Some("refund".to_string()),
                after_sale_id: Some("CASE-1".to_string()),
                raw: serde_json::json!({}),
            }],
        };

        let orders_repo = OrdersRepo::new(&pool);
        let order_id = orders_repo.upsert_order_projection(&order).await?;
        let payload = OrderUpsertedPayload {
            tenant_id,
            platform: Platform::Taobao,
            shop_id,
            platform_order_id: "ORDER-CASE-1".to_string(),
            order_id,
        };

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
        let db = Db::connect_lazy(&database_url)?;

        handle_order_upserted(&db, payload.clone()).await?;
        handle_order_upserted(&db, payload).await?;

        let case_count: i64 = sqlx::query_scalar(
            r#"
            select count(*)
            from after_sale_cases
            where tenant_id = $1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_one(&pool)
        .await?;
        assert_eq!(case_count, 1);

        let item_count: i64 = sqlx::query_scalar(
            r#"
            select count(*)
            from after_sale_case_items
            where tenant_id = $1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_one(&pool)
        .await?;
        assert_eq!(item_count, 1);

        Ok(())
    }

    #[tokio::test]
    async fn order_review_rule_hit_on_import() -> Result<()> {
        let Some(pool) = try_pool().await else {
            eprintln!("skipping order_review_rule_hit_on_import: db unavailable");
            return Ok(());
        };
        setup_schema(&pool).await?;

        let tenant_id = TenantId(uuid::Uuid::new_v4());
        let shop_id = ShopId(uuid::Uuid::new_v4());
        let now = Utc::now();

        sqlx::query(
            r#"
            insert into tenants (tenant_id, name, status)
            values ($1, $2, 'ACTIVE')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind("review-tenant")
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
            values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED')
            "#,
        )
        .bind(uuid::Uuid::from(shop_id))
        .bind(uuid::Uuid::from(tenant_id))
        .bind(Platform::Taobao.to_string())
        .bind("SHOP-REVIEW")
        .bind("review-shop")
        .execute(&pool)
        .await?;

        let rule_id = uuid::Uuid::new_v4();
        let conditions_json = serde_json::json!([
            {
                "field_path": "total_cent",
                "op": ">",
                "value": 500
            }
        ]);
        sqlx::query(
            r#"
            insert into order_review_rules (
                rule_id, tenant_id, name, enabled, priority, conditions_json, action, reason
            ) values ($1, $2, $3, true, 10, $4, 'FLAG', 'HIGH_VALUE')
            "#,
        )
        .bind(rule_id)
        .bind(uuid::Uuid::from(tenant_id))
        .bind("high-value")
        .bind(conditions_json)
        .execute(&pool)
        .await?;

        let order = OrderProjection {
            tenant_id,
            shop_id,
            platform: Platform::Taobao,
            platform_order_id: "ORDER-REVIEW-1".to_string(),
            status: OrderStatus::PendingShipment,
            total_amount_cent: 1000,
            goods_amount_cent: None,
            postage_amount_cent: None,
            discount_amount_cent: None,
            platform_discount_cent: None,
            seller_discount_cent: None,
            coupon_discount_cent: None,
            currency: "CNY".to_string(),
            buyer_name: None,
            receiver_name: Some("Alice".to_string()),
            receiver_phone: None,
            receiver_address: Some("Beijing Road 1".to_string()),
            pay_at: None,
            ship_at: None,
            finish_at: None,
            cancel_at: None,
            buyer_message: None,
            seller_remark: None,
            logistics_company: None,
            tracking_no: None,
            order_created_at: now,
            platform_updated_at: Some(now),
            raw: serde_json::json!({}),
            items: vec![OrderItemProjection {
                line_id: "LINE-1".to_string(),
                sku_id: Some("SKU-1".to_string()),
                title: Some("Rule Item".to_string()),
                qty: 1,
                price_cent: 1000,
                amount_cent: 1000,
                pay_amount_cent: None,
                discount_cent: None,
                refund_state: RefundState::None,
                after_sale_state: AfterSaleState::None,
                refund_amount_cent: None,
                after_sale_type: None,
                after_sale_id: None,
                raw: serde_json::json!({}),
            }],
        };

        let orders_repo = OrdersRepo::new(&pool);
        let order_id = orders_repo.upsert_order_projection(&order).await?;
        let payload = OrderUpsertedPayload {
            tenant_id,
            platform: Platform::Taobao,
            shop_id,
            platform_order_id: "ORDER-REVIEW-1".to_string(),
            order_id,
        };

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
        let db = Db::connect_lazy(&database_url)?;
        handle_order_upserted(&db, payload).await?;

        let order_month = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();
        let row: (String, serde_json::Value) = sqlx::query_as(
            r#"
            select review_status, exception_flags
            from orders
            where tenant_id = $1 and order_id = $2 and order_month = $3
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .bind(order_month)
        .fetch_one(&pool)
        .await?;

        assert_eq!(row.0, "PENDING");
        let flags = match row.1.as_array() {
            Some(arr) => arr
                .iter()
                .filter_map(|value| value.as_str())
                .collect::<Vec<_>>(),
            None => vec![],
        };
        assert!(flags.contains(&"HIGH_VALUE"));
        assert!(flags.contains(&"REVIEW_FLAGGED"));

        let review_status_row: (String,) = sqlx::query_as(
            r#"
            select status
            from order_reviews
            where tenant_id = $1 and order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_one(&pool)
        .await?;
        assert_eq!(review_status_row.0, "PENDING");

        let hit_row: (bool, String) = sqlx::query_as(
            r#"
            select hit, message
            from order_review_hits
            where tenant_id = $1 and order_id = $2 and rule_id = $3
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .bind(rule_id)
        .fetch_one(&pool)
        .await?;
        assert!(hit_row.0);
        assert_eq!(hit_row.1, "HIGH_VALUE");

        Ok(())
    }

    #[tokio::test]
    async fn order_upsert_ignores_out_of_order_update() -> Result<()> {
        let Some(pool) = try_pool().await else {
            eprintln!("skipping order_upsert_ignores_out_of_order_update: db unavailable");
            return Ok(());
        };
        setup_schema(&pool).await?;

        let tenant_id = TenantId(uuid::Uuid::new_v4());
        let shop_id = ShopId(uuid::Uuid::new_v4());
        let now = Utc::now();

        sqlx::query(
            r#"
            insert into tenants (tenant_id, name, status)
            values ($1, $2, 'ACTIVE')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind("order-tenant")
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
            values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED')
            "#,
        )
        .bind(uuid::Uuid::from(shop_id))
        .bind(uuid::Uuid::from(tenant_id))
        .bind(Platform::Taobao.to_string())
        .bind("SHOP-ORDER")
        .bind("order-shop")
        .execute(&pool)
        .await?;

        let mut order = OrderProjection {
            tenant_id,
            shop_id,
            platform: Platform::Taobao,
            platform_order_id: "ORDER-OUT-OF-ORDER".to_string(),
            status: OrderStatus::Shipped,
            total_amount_cent: 2000,
            goods_amount_cent: None,
            postage_amount_cent: None,
            discount_amount_cent: None,
            platform_discount_cent: None,
            seller_discount_cent: None,
            coupon_discount_cent: None,
            currency: "CNY".to_string(),
            buyer_name: None,
            receiver_name: None,
            receiver_phone: None,
            receiver_address: Some("Shanghai Road 9".to_string()),
            pay_at: None,
            ship_at: None,
            finish_at: None,
            cancel_at: None,
            buyer_message: None,
            seller_remark: None,
            logistics_company: None,
            tracking_no: None,
            order_created_at: now,
            platform_updated_at: Some(now),
            raw: serde_json::json!({}),
            items: vec![OrderItemProjection {
                line_id: "LINE-1".to_string(),
                sku_id: Some("SKU-1".to_string()),
                title: Some("Out of order".to_string()),
                qty: 1,
                price_cent: 2000,
                amount_cent: 2000,
                pay_amount_cent: None,
                discount_cent: None,
                refund_state: RefundState::None,
                after_sale_state: AfterSaleState::None,
                refund_amount_cent: None,
                after_sale_type: None,
                after_sale_id: None,
                raw: serde_json::json!({}),
            }],
        };

        let orders_repo = OrdersRepo::new(&pool);
        let order_id = orders_repo.upsert_order_projection(&order).await?;

        order.status = OrderStatus::PendingShipment;
        order.platform_updated_at = Some(now - ChronoDuration::hours(1));
        orders_repo.upsert_order_projection(&order).await?;

        let order_month = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();
        let status: String = sqlx::query_scalar(
            r#"
            select status
            from orders
            where tenant_id = $1 and order_id = $2 and order_month = $3
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .bind(order_month)
        .fetch_one(&pool)
        .await?;

        assert_eq!(status, "SHIPPED");

        Ok(())
    }

    #[tokio::test]
    async fn shipment_retry_consumes_once() -> Result<()> {
        let Some(pool) = try_pool().await else {
            eprintln!("skipping shipment_retry_consumes_once: db unavailable");
            return Ok(());
        };
        setup_shipping_schema(&pool).await?;

        let tenant_id = TenantId(uuid::Uuid::new_v4());
        let shop_id = ShopId(uuid::Uuid::new_v4());
        let warehouse_id = uuid::Uuid::new_v4();
        let sku_id = uuid::Uuid::new_v4();
        let order_id = uuid::Uuid::new_v4();
        let order_item_id = uuid::Uuid::new_v4();
        let shipment_id = uuid::Uuid::new_v4();
        let now = Utc::now();
        let order_month = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();

        sqlx::query(
            r#"
            insert into tenants (tenant_id, name, status)
            values ($1, $2, 'ACTIVE')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind("test-tenant")
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
            values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED')
            "#,
        )
        .bind(uuid::Uuid::from(shop_id))
        .bind(uuid::Uuid::from(tenant_id))
        .bind(Platform::Taobao.to_string())
        .bind("SHOP-1")
        .bind("shop")
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into warehouses (warehouse_id, tenant_id, name, code)
            values ($1, $2, $3, $4)
            "#,
        )
        .bind(warehouse_id)
        .bind(uuid::Uuid::from(tenant_id))
        .bind("Main")
        .bind("WH-1")
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into skus (sku_id, tenant_id, sku_kind)
            values ($1, $2, 'NORMAL')
            "#,
        )
        .bind(sku_id)
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into orders (
              tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status, order_created_at
            ) values ($1,$2,$3,$4,$5,$6,'PENDING_SHIPMENT',$7)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(order_month)
        .bind(order_id)
        .bind(Platform::Taobao.to_string())
        .bind(uuid::Uuid::from(shop_id))
        .bind("ORDER-1")
        .bind(now)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into order_index (
              tenant_id, order_id, order_month, platform, platform_order_id, shop_id, order_created_at
            ) values ($1,$2,$3,$4,$5,$6,$7)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(order_id)
        .bind(order_month)
        .bind(Platform::Taobao.to_string())
        .bind("ORDER-1")
        .bind(uuid::Uuid::from(shop_id))
        .bind(now)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into order_items (
              tenant_id, order_month, order_id, line_id, sku_id, qty,
              price_cent, amount_cent, refund_state, after_sale_state, raw
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,'NONE','NONE','{}'::jsonb)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(order_month)
        .bind(order_id)
        .bind(order_item_id.to_string())
        .bind(sku_id.to_string())
        .bind(2_i32)
        .bind(100_i64)
        .bind(200_i64)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into inventory_balances (tenant_id, warehouse_id, sku_id, on_hand, reserved)
            values ($1,$2,$3,$4,$5)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(warehouse_id)
        .bind(sku_id)
        .bind(10_i64)
        .bind(2_i64)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into inventory_reservations (
              tenant_id, warehouse_id, sku_id, order_id, order_item_id, qty, status
            ) values ($1,$2,$3,$4,$5,$6,'ACTIVE')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(warehouse_id)
        .bind(sku_id)
        .bind(order_id)
        .bind(order_item_id)
        .bind(2_i64)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into shipments (
              shipment_id, tenant_id, shop_id, order_id, platform, platform_order_id,
              warehouse_id, status, requested_at
            ) values ($1,$2,$3,$4,$5,$6,$7,'REQUESTED',$8)
            "#,
        )
        .bind(shipment_id)
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(shop_id))
        .bind(order_id)
        .bind(Platform::Taobao.to_string())
        .bind("ORDER-1")
        .bind(warehouse_id)
        .bind(now)
        .execute(&pool)
        .await?;

        let payload = ShipmentRequestedPayload {
            tenant_id,
            shipment_id,
            order_id: erp_types::OrderId(order_id),
            shop_id,
            platform: Platform::Taobao,
            platform_order_id: "ORDER-1".to_string(),
            warehouse_id,
            carrier_code: Some("ZTO".to_string()),
            tracking_no: Some("TRACK-1".to_string()),
            weight_gram: None,
        };

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
        let db = Db::connect_lazy(&database_url)?;
        let adapters = AdapterRegistry::new();

        handle_shipment_requested(&db, &adapters, payload.clone()).await?;
        handle_shipment_requested(&db, &adapters, payload).await?;

        #[derive(sqlx::FromRow)]
        struct BalanceRow {
            on_hand: i64,
            reserved: i64,
        }

        let balance: BalanceRow = sqlx::query_as(
            r#"
            select on_hand, reserved
            from inventory_balances
            where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(warehouse_id)
        .bind(sku_id)
        .fetch_one(&pool)
        .await?;
        assert_eq!(balance.on_hand, 8);
        assert_eq!(balance.reserved, 0);

        let ship_count: i64 = sqlx::query_scalar(
            r#"
            select count(*)
            from inventory_ledgers
            where tenant_id = $1 and sku_id = $2 and kind = 'SHIP'
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(sku_id)
        .fetch_one(&pool)
        .await?;
        assert_eq!(ship_count, 1);

        Ok(())
    }

    #[tokio::test]
    async fn shipment_skipped_when_feature_disabled() -> Result<()> {
        let Some(pool) = try_pool().await else {
            eprintln!("skipping shipment_skipped_when_feature_disabled: db unavailable");
            return Ok(());
        };
        setup_shipping_schema(&pool).await?;

        let tenant_id = TenantId(uuid::Uuid::new_v4());
        let shop_id = ShopId(uuid::Uuid::new_v4());
        let warehouse_id = uuid::Uuid::new_v4();
        let sku_id = uuid::Uuid::new_v4();
        let order_id = uuid::Uuid::new_v4();
        let order_item_id = uuid::Uuid::new_v4();
        let shipment_id = uuid::Uuid::new_v4();
        let now = Utc::now();
        let order_month = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();

        sqlx::query(
            r#"
            insert into tenants (tenant_id, name, status)
            values ($1, $2, 'ACTIVE')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind("test-tenant")
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
            values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED')
            "#,
        )
        .bind(uuid::Uuid::from(shop_id))
        .bind(uuid::Uuid::from(tenant_id))
        .bind(Platform::Taobao.to_string())
        .bind("SHOP-FF")
        .bind("shop")
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into warehouses (warehouse_id, tenant_id, name, code)
            values ($1, $2, $3, $4)
            "#,
        )
        .bind(warehouse_id)
        .bind(uuid::Uuid::from(tenant_id))
        .bind("Main")
        .bind("WH-FF")
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into skus (sku_id, tenant_id, sku_kind)
            values ($1, $2, 'NORMAL')
            "#,
        )
        .bind(sku_id)
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into orders (
              tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status, order_created_at
            ) values ($1,$2,$3,$4,$5,$6,'PENDING_SHIPMENT',$7)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(order_month)
        .bind(order_id)
        .bind(Platform::Taobao.to_string())
        .bind(uuid::Uuid::from(shop_id))
        .bind("ORDER-FF")
        .bind(now)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into order_index (
              tenant_id, order_id, order_month, platform, platform_order_id, shop_id, order_created_at
            ) values ($1,$2,$3,$4,$5,$6,$7)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(order_id)
        .bind(order_month)
        .bind(Platform::Taobao.to_string())
        .bind("ORDER-FF")
        .bind(uuid::Uuid::from(shop_id))
        .bind(now)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into order_items (
              tenant_id, order_month, order_id, line_id, sku_id, qty,
              price_cent, amount_cent, refund_state, after_sale_state, raw
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,'NONE','NONE','{}'::jsonb)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(order_month)
        .bind(order_id)
        .bind(order_item_id.to_string())
        .bind(sku_id.to_string())
        .bind(2_i32)
        .bind(100_i64)
        .bind(200_i64)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into inventory_balances (tenant_id, warehouse_id, sku_id, on_hand, reserved)
            values ($1,$2,$3,$4,$5)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(warehouse_id)
        .bind(sku_id)
        .bind(10_i64)
        .bind(2_i64)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into inventory_reservations (
              tenant_id, warehouse_id, sku_id, order_id, order_item_id, qty, status
            ) values ($1,$2,$3,$4,$5,$6,'ACTIVE')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(warehouse_id)
        .bind(sku_id)
        .bind(order_id)
        .bind(order_item_id)
        .bind(2_i64)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into shipments (
              shipment_id, tenant_id, shop_id, order_id, platform, platform_order_id,
              warehouse_id, status, requested_at
            ) values ($1,$2,$3,$4,$5,$6,$7,'REQUESTED',$8)
            "#,
        )
        .bind(shipment_id)
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(shop_id))
        .bind(order_id)
        .bind(Platform::Taobao.to_string())
        .bind("ORDER-FF")
        .bind(warehouse_id)
        .bind(now)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into feature_flags (tenant_id, scope, shop_id, key, enabled)
            values ($1, 'SHOP', $2, 'SHIPPING_WRITE', false)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(shop_id))
        .execute(&pool)
        .await?;

        let payload = ShipmentRequestedPayload {
            tenant_id,
            shipment_id,
            order_id: erp_types::OrderId(order_id),
            shop_id,
            platform: Platform::Taobao,
            platform_order_id: "ORDER-FF".to_string(),
            warehouse_id,
            carrier_code: Some("ZTO".to_string()),
            tracking_no: Some("TRACK-FF".to_string()),
            weight_gram: None,
        };

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
        let db = Db::connect_lazy(&database_url)?;
        let adapters = AdapterRegistry::new();

        let logs = LogCapture::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(logs.clone())
            .with_ansi(false)
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);

        handle_shipment_requested(&db, &adapters, payload).await?;

        let status: String = sqlx::query_scalar(
            r#"
            select status
            from shipments
            where tenant_id = $1 and shipment_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .fetch_one(&pool)
        .await?;
        assert_eq!(status, "REQUESTED");

        #[derive(sqlx::FromRow)]
        struct BalanceRow {
            on_hand: i64,
            reserved: i64,
        }

        let balance: BalanceRow = sqlx::query_as(
            r#"
            select on_hand, reserved
            from inventory_balances
            where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(warehouse_id)
        .bind(sku_id)
        .fetch_one(&pool)
        .await?;
        assert_eq!(balance.on_hand, 10);
        assert_eq!(balance.reserved, 2);

        let ship_count: i64 = sqlx::query_scalar(
            r#"
            select count(*)
            from inventory_ledgers
            where tenant_id = $1 and sku_id = $2 and kind = 'SHIP'
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(sku_id)
        .fetch_one(&pool)
        .await?;
        assert_eq!(ship_count, 0);

        assert!(logs.contents().contains("feature disabled: shipping write"));

        Ok(())
    }

    #[tokio::test]
    async fn order_upsert_skipped_when_feature_disabled() -> Result<()> {
        let Some(pool) = try_pool().await else {
            eprintln!("skipping order_upsert_skipped_when_feature_disabled: db unavailable");
            return Ok(());
        };
        setup_schema(&pool).await?;

        let tenant_id = TenantId(uuid::Uuid::new_v4());
        let shop_id = ShopId(uuid::Uuid::new_v4());

        sqlx::query(
            r#"
            insert into tenants (tenant_id, name, status)
            values ($1, $2, 'ACTIVE')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind("test-tenant")
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
            values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED')
            "#,
        )
        .bind(uuid::Uuid::from(shop_id))
        .bind(uuid::Uuid::from(tenant_id))
        .bind(Platform::Taobao.to_string())
        .bind("SHOP-FF-CASE")
        .bind("shop")
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into feature_flags (tenant_id, scope, shop_id, key, enabled)
            values ($1, 'SHOP', $2, 'AFTERSALES_WRITE', false)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(shop_id))
        .execute(&pool)
        .await?;

        let mut order = OrderProjection {
            tenant_id,
            shop_id,
            platform: Platform::Taobao,
            platform_order_id: "ORDER-FF-CASE".to_string(),
            status: OrderStatus::Completed,
            total_amount_cent: 100,
            goods_amount_cent: None,
            postage_amount_cent: None,
            discount_amount_cent: None,
            platform_discount_cent: None,
            seller_discount_cent: None,
            coupon_discount_cent: None,
            currency: "CNY".to_string(),
            buyer_name: None,
            receiver_name: None,
            receiver_phone: None,
            receiver_address: None,
            pay_at: None,
            ship_at: None,
            finish_at: None,
            cancel_at: None,
            buyer_message: None,
            seller_remark: None,
            logistics_company: None,
            tracking_no: None,
            order_created_at: Utc::now(),
            platform_updated_at: None,
            raw: serde_json::json!({}),
            items: vec![OrderItemProjection {
                line_id: "LINE-FF".to_string(),
                sku_id: Some("SKU-FF".to_string()),
                title: Some("Test Item".to_string()),
                qty: 1,
                price_cent: 100,
                amount_cent: 100,
                pay_amount_cent: None,
                discount_cent: None,
                refund_state: RefundState::None,
                after_sale_state: AfterSaleState::None,
                refund_amount_cent: None,
                after_sale_type: None,
                after_sale_id: None,
                raw: serde_json::json!({}),
            }],
        };

        let outbox_repo = OutboxRepo::new(&pool);
        apply_sku_mappings(&pool, &outbox_repo, &mut order).await?;

        let orders_repo = OrdersRepo::new(&pool);
        let order_id = orders_repo.upsert_order_projection(&order).await?;

        let payload = OrderUpsertedPayload {
            tenant_id,
            platform: Platform::Taobao,
            shop_id,
            platform_order_id: "ORDER-FF-CASE".to_string(),
            order_id,
        };

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
        let db = Db::connect_lazy(&database_url)?;

        let logs = LogCapture::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(logs.clone())
            .with_ansi(false)
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);

        handle_order_upserted(&db, payload).await?;

        let case_count: i64 = sqlx::query_scalar(
            r#"
            select count(*)
            from after_sale_cases
            where tenant_id = $1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_one(&pool)
        .await?;
        assert_eq!(case_count, 0);

        assert!(logs
            .contents()
            .contains("feature disabled: aftersales write"));

        Ok(())
    }

    async fn apply_effect(pool: &PgPool, env: &OutboxEnvelope) -> Result<()> {
        sqlx::query(
            r#"
            insert into worker_effects (tenant_id, outbox_id, event_type)
            values ($1, $2, $3)
            on conflict (tenant_id, outbox_id) do nothing
            "#,
        )
        .bind(uuid::Uuid::from(env.tenant_id))
        .bind(env.outbox_id)
        .bind(&env.event_type)
        .execute(pool)
        .await?;
        Ok(())
    }

    #[tokio::test]
    async fn transient_failure_moves_to_dlq_after_max_attempts() -> Result<()> {
        let Some(pool) = try_pool().await else {
            eprintln!("skipping transient_failure_moves_to_dlq_after_max_attempts: db unavailable");
            return Ok(());
        };
        setup_schema(&pool).await?;

        let tenant_id = TenantId(uuid::Uuid::new_v4());
        sqlx::query(
            r#"
            insert into tenants (tenant_id, name, status)
            values ($1, $2, 'ACTIVE')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind("test-tenant")
        .execute(&pool)
        .await?;

        let env = OutboxEnvelope {
            tenant_id,
            outbox_id: uuid::Uuid::new_v4(),
            event_type: "DLQ_TRANSIENT".to_string(),
            payload: serde_json::json!({"value": 1}),
            produced_at: Utc::now(),
            trace_id: None,
        };

        let err = anyhow::anyhow!(erp_adapter::AdapterError::Temporary("timeout".to_string()));
        let policy = dlq::RetryPolicy {
            max_attempts: 3,
            base_delay_ms: 1,
            max_delay_ms: 5,
        };

        for attempt in 1..=3 {
            let outcome =
                dlq::handle_processing_failure(&pool, &env, &err, attempt, &policy).await?;
            match outcome {
                dlq::FailureOutcome::Retry { .. } => {}
                _ => panic!("expected retry outcome"),
            }
        }

        let outcome = dlq::handle_processing_failure(&pool, &env, &err, 4, &policy).await?;
        let dead_letter_id = match outcome {
            dlq::FailureOutcome::DeadLettered { dead_letter_id, .. } => dead_letter_id,
            _ => panic!("expected dead letter"),
        };

        let row = dlq::fetch_dead_letter(&pool, uuid::Uuid::from(tenant_id), dead_letter_id)
            .await?
            .expect("dead letter row");
        assert_eq!(row.attempts, 3);
        assert_eq!(row.classification, dlq::FailureClass::Transient.as_str());
        assert_eq!(row.next_action, dlq::NEXT_ACTION_MANUAL_REPLAY);
        assert_eq!(row.payload.get("value").and_then(|v| v.as_i64()), Some(1));

        Ok(())
    }

    #[tokio::test]
    async fn replay_dead_letter_requeues_idempotent() -> Result<()> {
        let Some(pool) = try_pool().await else {
            eprintln!("skipping replay_dead_letter_requeues_idempotent: db unavailable");
            return Ok(());
        };
        setup_schema(&pool).await?;

        sqlx::query(
            r#"
            create table if not exists worker_effects (
              tenant_id uuid not null,
              outbox_id uuid not null,
              event_type text not null,
              status text not null default 'PROCESSING',
              last_error text not null default '',
              processed_at timestamptz not null default now(),
              primary key (tenant_id, outbox_id)
            )
            "#,
        )
        .execute(&pool)
        .await?;

        let tenant_id = TenantId(uuid::Uuid::new_v4());
        sqlx::query(
            r#"
            insert into tenants (tenant_id, name, status)
            values ($1, $2, 'ACTIVE')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind("test-tenant")
        .execute(&pool)
        .await?;

        let env = OutboxEnvelope {
            tenant_id,
            outbox_id: uuid::Uuid::new_v4(),
            event_type: "DLQ_REPLAY".to_string(),
            payload: serde_json::json!({"value": "ok"}),
            produced_at: Utc::now(),
            trace_id: None,
        };

        let dead_letter_id = dlq::insert_dead_letter(
            &pool,
            &env,
            "boom",
            3,
            dlq::FailureClass::Transient,
            dlq::NEXT_ACTION_MANUAL_REPLAY,
        )
        .await?;

        let captured = Arc::new(Mutex::new(None));
        let captured_ref = captured.clone();
        let pool_ref = pool.clone();

        dlq::replay_dead_letter_with(
            &pool,
            uuid::Uuid::from(tenant_id),
            dead_letter_id,
            move |msg| {
                let captured_ref = captured_ref.clone();
                let pool_ref = pool_ref.clone();
                async move {
                    *captured_ref.lock().unwrap() = Some(msg.clone());
                    let env: OutboxEnvelope = serde_json::from_slice(&msg.payload)?;
                    apply_effect(&pool_ref, &env).await?;
                    Ok(())
                }
            },
        )
        .await?;

        let message = captured.lock().unwrap().clone().expect("replay message");
        let expected_subject =
            erp_outbox::publisher::subject_for(&tenant_id.to_string(), &env.event_type);
        assert_eq!(message.subject, expected_subject);

        let replay_env: OutboxEnvelope = serde_json::from_slice(&message.payload)?;
        apply_effect(&pool, &replay_env).await?;

        let count: i64 = sqlx::query_scalar(
            r#"
            select count(*)
            from worker_effects
            where tenant_id = $1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_one(&pool)
        .await?;
        assert_eq!(count, 1);

        let row = dlq::fetch_dead_letter(&pool, uuid::Uuid::from(tenant_id), dead_letter_id)
            .await?
            .expect("dead letter row");
        assert_eq!(row.status, "REPLAYED");

        Ok(())
    }

    #[tokio::test]
    async fn token_refresh_success_updates_token() -> Result<()> {
        let Some(pool) = try_pool().await else {
            eprintln!("skipping token_refresh_success_updates_token: db unavailable");
            return Ok(());
        };
        setup_schema(&pool).await?;

        let tenant_id = TenantId(uuid::Uuid::new_v4());
        let shop_id = ShopId(uuid::Uuid::new_v4());

        sqlx::query(
            r#"
            insert into tenants (tenant_id, name, status)
            values ($1, 'token-tenant', 'ACTIVE')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into shops (
              shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status, credentials, auth_meta_json
            ) values ($1, $2, 'taobao', 'SHOP-TOKEN', 'shop', 'shop', 'AUTHORIZED', '{}'::jsonb, '{}'::jsonb)
            "#,
        )
        .bind(uuid::Uuid::from(shop_id))
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;

        let tokens_repo = PlatformTokensRepo::new(&pool);
        let token = tokens_repo
            .upsert_token(erp_repo::platform_tokens::PlatformTokenUpsert {
                tenant_id,
                shop_id,
                token_status: None,
                access_token: None,
                refresh_token: Some("refresh-token".to_string()),
                expires_at: Some(Utc::now() - ChronoDuration::minutes(10)),
                scopes: Some("read".to_string()),
                last_refreshed_at: None,
                refresh_fail_count: Some(0),
                last_error: Some(String::new()),
            })
            .await?;

        let candidate = erp_repo::platform_tokens::TokenRefreshCandidate {
            token,
            platform: Platform::Taobao,
        };

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
        let db = Db::connect_lazy(&database_url)?;
        let adapters = AdapterRegistry::new();

        token_refresh::refresh_token_for_candidate(&db, &adapters, &candidate).await?;

        let updated = tokens_repo
            .get_token(tenant_id, shop_id)
            .await?
            .expect("token");
        assert!(updated.access_token.is_some());
        assert!(updated.last_refreshed_at.is_some());
        assert!(updated.last_error.is_empty());

        let status: String =
            sqlx::query_scalar("select status from shops where tenant_id = $1 and shop_id = $2")
                .bind(uuid::Uuid::from(tenant_id))
                .bind(uuid::Uuid::from(shop_id))
                .fetch_one(&pool)
                .await?;
        assert_eq!(status, "AUTHORIZED");

        Ok(())
    }

    #[tokio::test]
    async fn token_refresh_failure_emits_outbox() -> Result<()> {
        let Some(pool) = try_pool().await else {
            eprintln!("skipping token_refresh_failure_emits_outbox: db unavailable");
            return Ok(());
        };
        setup_schema(&pool).await?;

        let tenant_id = TenantId(uuid::Uuid::new_v4());
        let shop_id = ShopId(uuid::Uuid::new_v4());

        sqlx::query(
            r#"
            insert into tenants (tenant_id, name, status)
            values ($1, 'token-tenant', 'ACTIVE')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into shops (
              shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status, credentials, auth_meta_json
            ) values ($1, $2, 'taobao', 'SHOP-TOKEN-FAIL', 'shop', 'shop', 'AUTHORIZED', '{}'::jsonb, '{"refresh_fail": true}'::jsonb)
            "#,
        )
        .bind(uuid::Uuid::from(shop_id))
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;

        let tokens_repo = PlatformTokensRepo::new(&pool);
        let token = tokens_repo
            .upsert_token(erp_repo::platform_tokens::PlatformTokenUpsert {
                tenant_id,
                shop_id,
                token_status: None,
                access_token: None,
                refresh_token: Some("refresh-token".to_string()),
                expires_at: Some(Utc::now() - ChronoDuration::minutes(10)),
                scopes: None,
                last_refreshed_at: None,
                refresh_fail_count: Some(0),
                last_error: Some(String::new()),
            })
            .await?;

        let candidate = erp_repo::platform_tokens::TokenRefreshCandidate {
            token,
            platform: Platform::Taobao,
        };

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
        let db = Db::connect_lazy(&database_url)?;
        let adapters = AdapterRegistry::new();

        token_refresh::refresh_token_for_candidate(&db, &adapters, &candidate).await?;

        let updated = tokens_repo
            .get_token(tenant_id, shop_id)
            .await?
            .expect("token");
        assert!(!updated.last_error.is_empty());

        let status: String =
            sqlx::query_scalar("select status from shops where tenant_id = $1 and shop_id = $2")
                .bind(uuid::Uuid::from(tenant_id))
                .bind(uuid::Uuid::from(shop_id))
                .fetch_one(&pool)
                .await?;
        assert_eq!(status, "EXPIRED");

        let outbox_count: i64 = sqlx::query_scalar(
            "select count(*) from outbox_events where event_type = 'TOKEN_REFRESH_FAILED'",
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(outbox_count, 1);

        Ok(())
    }

    #[tokio::test]
    async fn refund_rollback_clears_packages_and_marks_refunded() -> Result<()> {
        let Some(pool) = try_pool().await else {
            eprintln!(
                "skipping refund_rollback_clears_packages_and_marks_refunded: db unavailable"
            );
            return Ok(());
        };
        setup_shipping_schema(&pool).await?;
        setup_printing_schema(&pool).await?;

        let tenant_id = TenantId(uuid::Uuid::new_v4());
        let shop_id = ShopId(uuid::Uuid::new_v4());
        let order_id = uuid::Uuid::new_v4();
        let shipment_id = uuid::Uuid::new_v4();
        let refund_doc_id = uuid::Uuid::new_v4();
        let now = Utc::now();
        let order_month = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();

        sqlx::query(
            r#"
            insert into tenants (tenant_id, name, status)
            values ($1, 'refund-tenant', 'ACTIVE')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
            values ($1, $2, 'taobao', 'SHOP-REFUND', 'shop', 'shop', 'AUTHORIZED')
            "#,
        )
        .bind(uuid::Uuid::from(shop_id))
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into order_index (
              tenant_id, order_id, order_month, platform, platform_order_id, shop_id, order_created_at
            ) values ($1, $2, $3, 'taobao', 'PLAT-REFUND', $4, $5)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(order_id)
        .bind(order_month)
        .bind(uuid::Uuid::from(shop_id))
        .bind(now)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into orders (
              tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status,
              oms_status, order_status, fulfillment_status, exception_status, review_status,
              exception_flags, total_amount_cent, currency, order_created_at
            ) values (
              $1, $2, $3, 'taobao', $4, 'PLAT-REFUND', 'PENDING_SHIPMENT',
              'PENDING_SHIPMENT', 'PENDING_ALLOCATE', 'READY_TO_SHIP', 'NONE', 'APPROVED',
              '[]'::jsonb, 1000, 'CNY', $5
            )
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(order_month)
        .bind(order_id)
        .bind(uuid::Uuid::from(shop_id))
        .bind(now)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into shipments (
              shipment_id, tenant_id, shop_id, order_id, platform, platform_order_id, status
            ) values ($1, $2, $3, $4, 'taobao', 'PLAT-REFUND', 'REQUESTED')
            "#,
        )
        .bind(shipment_id)
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(shop_id))
        .bind(order_id)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into outbound_docs (outbound_doc_id, tenant_id, shipment_id, status)
            values ($1, $2, $3, 'REQUESTED')
            "#,
        )
        .bind(uuid::Uuid::new_v4())
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into shipment_packages (
              tenant_id, shipment_id, package_no, carrier_code
            ) values ($1, $2, 'PKG-REFUND', 'ZTO')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .execute(&pool)
        .await?;

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
        let db = Db::connect_lazy(&database_url)?;
        rollback_refunded_order(&db, tenant_id, OrderId(order_id), refund_doc_id).await?;

        let row: (String, String, String, String, String, Option<String>) = sqlx::query_as(
            r#"
            select status, payment_status, oms_status, order_status, fulfillment_status, cancel_reason
            from orders
            where tenant_id = $1 and order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(order_id)
        .fetch_one(&pool)
        .await?;

        assert_eq!(row.0, "REFUNDED");
        assert_eq!(row.1, "REFUNDED");
        assert_eq!(row.2, "CANCELLED");
        assert_eq!(row.3, "CANCELLED");
        assert_eq!(row.4, "NOT_ALLOCATED");
        let reason = row.5.unwrap_or_default();
        assert!(reason.contains(&refund_doc_id.to_string()));

        let package_count: i64 = sqlx::query_scalar(
            r#"
            select count(*)
            from shipment_packages
            where tenant_id = $1 and shipment_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .fetch_one(&pool)
        .await?;
        assert_eq!(package_count, 0);

        Ok(())
    }

    #[tokio::test]
    async fn sync_job_idempotent_outbox() -> Result<()> {
        let Some(pool) = try_pool().await else {
            eprintln!("skipping sync_job_idempotent_outbox: db unavailable");
            return Ok(());
        };
        setup_schema(&pool).await?;

        let tenant_id = TenantId(uuid::Uuid::new_v4());
        let shop_id = ShopId(uuid::Uuid::new_v4());

        let mock_order = serde_json::json!({
            "tid": "TB-1",
            "created": "2026-01-06 10:00:00",
            "payment": "10.00",
            "status": "WAIT_SELLER_SEND_GOODS",
            "orders": {
                "order": [
                    { "oid": "OID-1", "num": 1, "price": "10.00", "payment": "10.00" }
                ]
            }
        });

        sqlx::query(
            r#"
            insert into tenants (tenant_id, name, status)
            values ($1, 'sync-tenant', 'ACTIVE')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into shops (
              shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status, credentials, auth_meta_json
            ) values ($1, $2, 'taobao', 'SHOP-SYNC', 'shop', 'shop', 'AUTHORIZED', '{}'::jsonb, $3)
            "#,
        )
        .bind(uuid::Uuid::from(shop_id))
        .bind(uuid::Uuid::from(tenant_id))
        .bind(serde_json::json!({ "mock_orders": [mock_order] }))
        .execute(&pool)
        .await?;

        let sync_repo = SyncJobsRepo::new(&pool);
        sync_repo
            .upsert_job(erp_repo::sync_jobs::SyncJobUpsert {
                tenant_id,
                shop_id,
                job_type: "order_sync".to_string(),
                cursor: None,
                watermark: None,
                status: Some("PENDING".to_string()),
                last_error: Some(String::new()),
            })
            .await?;

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
        let db = Db::connect_lazy(&database_url)?;
        let adapters = AdapterRegistry::new();

        run_sync_jobs_once(&db, &adapters, ChronoDuration::minutes(60), 10).await?;
        run_sync_jobs_once(&db, &adapters, ChronoDuration::minutes(60), 10).await?;

        let outbox_count: i64 = sqlx::query_scalar(
            "select count(*) from outbox_events where event_type = 'INBOX_EVENT_RECEIVED'",
        )
        .fetch_one(&pool)
        .await?;
        assert_eq!(outbox_count, 1);

        Ok(())
    }

    #[test]
    fn token_refresh_module_exports_expected_symbols() {
        let handle = token_refresh::handle_token_refresh_requested;
        let batch = token_refresh::refresh_tokens_once;
        let refresh = token_refresh::refresh_token_for_candidate;

        let _ = (handle, batch, refresh);
    }
}
