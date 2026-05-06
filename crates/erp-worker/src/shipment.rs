use super::*;

pub(super) async fn handle_shipment_requested(
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

pub(super) async fn handle_shipment_shipped(
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

pub(super) fn shipment_push_error_summary(err: &AdapterError) -> (String, serde_json::Value) {
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
