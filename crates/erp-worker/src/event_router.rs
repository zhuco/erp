use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) async fn route_event(
    db: &Db,
    adapters: &AdapterRegistry,
    nats: &async_nats::Client,
    env: OutboxEnvelope,
    stock_sync_debounce: ChronoDuration,
    attempt: u32,
    trigger: &str,
    replay_dead_letter_id: Option<uuid::Uuid>,
) -> anyhow::Result<()> {
    match env.event_type.as_str() {
        "INBOX_EVENT_RECEIVED" => {
            let payload: InboxReceivedPayload = serde_json::from_value(env.payload)?;
            handle_inbox_received(db, adapters, payload).await
        }
        SHIPMENT_REQUESTED_EVENT => {
            let payload: ShipmentRequestedPayload = serde_json::from_value(env.payload)?;
            handle_shipment_requested(db, adapters, payload).await
        }
        SHIPMENT_SHIPPED_EVENT | SHIPMENT_CONFIRMED_EVENT => {
            let payload: ShipmentShippedPayload = serde_json::from_value(env.payload)?;
            handle_shipment_shipped(
                db,
                adapters,
                payload,
                env.outbox_id,
                attempt,
                trigger,
                replay_dead_letter_id,
            )
            .await
        }
        "PRINT_JOB_QUEUED" => handle_print_job_queued(db, env).await,
        INVENTORY_CHANGED_EVENT => {
            let payload: InventoryChangedPayload = serde_json::from_value(env.payload)?;
            handle_inventory_changed(db, payload, stock_sync_debounce).await
        }
        REFUND_REQUESTED_EVENT => {
            let payload: RefundRequestedPayload = serde_json::from_value(env.payload)?;
            handle_refund_requested(db, payload).await
        }
        "TOKEN_REFRESH_REQUESTED" => {
            let payload: erp_domain::TokenRefreshRequestedPayload =
                serde_json::from_value(env.payload)?;
            handle_token_refresh_requested(db, adapters, payload).await
        }
        "ORDER_IMPORTED" | "ORDER_UPSERTED" => {
            let payload: OrderUpsertedPayload = serde_json::from_value(env.payload)?;
            handle_order_upserted(db, payload).await
        }
        "ORDER_REVIEWED" => {
            let payload: OrderReviewedPayload = serde_json::from_value(env.payload)?;
            handle_order_reviewed(db, payload).await
        }
        "SYNC_JOB_RUN" => {
            let payload: SyncJobReplayPayload = serde_json::from_value(env.payload)?;
            handle_sync_job_run(db, payload).await
        }
        PROFIT_COMPUTE_REQUESTED_EVENT => {
            let payload: ProfitComputeRequestedPayload = serde_json::from_value(env.payload)?;
            handle_profit_compute_requested(db, payload).await
        }
        DLQ_REPLAY_REQUESTED_EVENT => {
            let payload: DeadLetterReplayPayload = serde_json::from_value(env.payload)?;
            handle_dead_letter_replay(db, nats, payload).await
        }
        other => {
            warn!(event_type = %other, "unknown event_type - ignored");
            Ok(())
        }
    }
}
