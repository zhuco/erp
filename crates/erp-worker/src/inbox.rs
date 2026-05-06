use super::*;

pub(super) async fn handle_inbox_received(
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

    // 1) ??????
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

    // 2) ?????
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

    // 3) ??????
    for mut order in orders {
        annotate_mapping_version(&mut order, mapping_version);
        apply_sku_mappings(db.pool(), &outbox_repo, &mut order).await?;
        let order_id = orders_repo.upsert_order_projection(&order).await?;
        // ??????????? UI??????
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

pub(super) async fn record_inbox_dedup_hits(
    db: &Db,
    tenant_id: TenantId,
    inbox_event_id: uuid::Uuid,
) {
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
