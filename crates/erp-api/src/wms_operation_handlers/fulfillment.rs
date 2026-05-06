use super::*;

pub(crate) async fn pack_wms_outbound(
    State(state): State<AppState>,
    Path((tenant_id, outbound_doc_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<WmsPackRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let outbound_doc_id = match uuid::Uuid::parse_str(&outbound_doc_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad outbound_doc_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.write") {
        return resp.into_response();
    }

    let pack_enabled = match feature_flag_enabled(
        &state,
        tenant_id,
        None,
        FeatureFlagKey::PackPerformance,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if !pack_enabled {
        return feature_flag_forbidden(FeatureFlagKey::PackPerformance).into_response();
    }

    let WmsPackRequest {
        package_no,
        weight_gram,
        operator_id,
        items: req_items,
        serials,
        batches: req_batches,
    } = req;

    if package_no.trim().is_empty() {
        return bad_request("package_no required").into_response();
    }

    let operator_id = match operator_id.and_then(non_empty) {
        Some(raw) => match uuid::Uuid::parse_str(&raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad operator_id").into_response(),
        },
        None => Some(uuid::Uuid::from(ctx.user_id)),
    };

    let mut items = Vec::with_capacity(req_items.len());
    for item in &req_items {
        if item.packed_qty <= 0 {
            return bad_request("packed_qty must be positive").into_response();
        }
        let sku_id = match uuid::Uuid::parse_str(&item.sku_id) {
            Ok(v) => v,
            Err(_) => return bad_request("bad sku_id").into_response(),
        };
        items.push(OutboundPackItemInput {
            sku_id,
            packed_qty: item.packed_qty,
        });
    }

    let mut batches = Vec::new();
    if let Some(entries) = req_batches {
        if entries.is_empty() {
            return bad_request("batches required").into_response();
        }
        for entry in entries {
            if entry.qty <= 0 {
                return bad_request("batch qty must be positive").into_response();
            }
            let sku_id = match uuid::Uuid::parse_str(&entry.sku_id) {
                Ok(v) => v,
                Err(_) => return bad_request("bad sku_id").into_response(),
            };
            let batch_id = match entry.batch_id.as_deref() {
                Some(raw) => match uuid::Uuid::parse_str(raw) {
                    Ok(v) => Some(v),
                    Err(_) => return bad_request("bad batch_id").into_response(),
                },
                None => None,
            };
            let batch_no = entry.batch_no.and_then(non_empty);
            if batch_id.is_none() && batch_no.is_none() {
                return bad_request("batch_id or batch_no required").into_response();
            }
            batches.push(OutboundPackBatchInput {
                sku_id,
                batch_id,
                batch_no,
                qty: entry.qty,
            });
        }
    }

    let repo = WmsRepo::new(state.db.pool());
    let require_audit =
        match fetch_tenant_setting_value(state.db.pool(), tenant_id, "inv.outbound_require_audit")
            .await
        {
            Ok(value) => value.as_ref().and_then(parse_bool_setting).unwrap_or(false),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
    let detail = match repo.get_outbound_doc(tenant_id, outbound_doc_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("outbound doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let _workflow_config =
        match fetch_warehouse_workflow_config(state.db.pool(), tenant_id, detail.doc.warehouse_id)
            .await
        {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
    if require_audit && detail.doc.status == "DRAFT" {
        return bad_request("outbound doc not audited").into_response();
    }
    if let Some(weight_gram) = weight_gram {
        if weight_gram <= 0 {
            return bad_request("weight_gram must be positive").into_response();
        }
    }

    if let Some(serials) = serials {
        if serials.is_empty() {
            return bad_request("serials required").into_response();
        }
        let shipment_id = match detail.doc.shipment_id {
            Some(v) => v,
            None => return bad_request("shipment_id required for serials").into_response(),
        };
        let order_id = match detail.doc.order_id {
            Some(v) => v,
            None => return bad_request("order_id required for serials").into_response(),
        };

        let fulfillment_repo = FulfillmentRepo::new(state.db.pool());
        let shipment_items = match fulfillment_repo
            .list_oms_shipment_items(tenant_id, shipment_id)
            .await
        {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        if shipment_items.is_empty() {
            return bad_request("shipment items missing").into_response();
        }

        let mut items_by_sku: std::collections::HashMap<uuid::Uuid, Vec<(uuid::Uuid, i64)>> =
            std::collections::HashMap::new();
        for item in shipment_items {
            items_by_sku
                .entry(item.sku_id)
                .or_default()
                .push((item.order_item_id, item.qty));
        }

        for entry in serials {
            if entry.serial_nos.is_empty() {
                return bad_request("serial_nos required").into_response();
            }
            let sku_id = match uuid::Uuid::parse_str(&entry.sku_id) {
                Ok(v) => v,
                Err(_) => return bad_request("bad sku_id").into_response(),
            };
            let Some(items) = items_by_sku.get(&sku_id) else {
                return bad_request("sku not in shipment").into_response();
            };
            let total_required: i64 = items.iter().map(|(_, qty)| *qty).sum();
            if entry.serial_nos.len() as i64 != total_required {
                return conflict("serial count mismatch").into_response();
            }

            let mut remaining = entry.serial_nos;
            for (order_item_id, qty) in items {
                let take = (*qty).max(0) as usize;
                if take == 0 {
                    continue;
                }
                let chunk: Vec<String> = remaining.drain(0..take).collect();
                if let Err(e) = state
                    .sku_serials_repo
                    .bind_serials_to_shipment(RepoShipmentSerialBindInput {
                        tenant_id,
                        shipment_id,
                        order_id,
                        order_item_id: *order_item_id,
                        sku_id,
                        serial_nos: chunk,
                    })
                    .await
                {
                    return bad_request(&e.to_string()).into_response();
                }
            }
            if !remaining.is_empty() {
                return conflict("serial count mismatch").into_response();
            }
        }
    }

    if let Err(e) = repo
        .pack_outbound(OutboundPackInput {
            tenant_id,
            outbound_doc_id,
            package_no: package_no.trim().to_string(),
            weight_gram,
            operator_id,
            items,
            batches,
        })
        .await
    {
        return bad_request(&e.to_string()).into_response();
    }

    let logs_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.PACKED".to_string(),
            entity_type: "OUTBOUND_DOC".to_string(),
            entity_id: Some(outbound_doc_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({
                "package_no": package_no.clone(),
                "weight_gram": weight_gram,
            })),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %e, "audit log insert failed");
    }

    if let Ok(Some(detail)) = repo.get_outbound_doc(tenant_id, outbound_doc_id).await {
        let package_count = detail.pack_records.len().min(i32::MAX as usize) as i32;
        let delivery_status =
            derive_delivery_status(&detail.doc.outbound_status, &detail.doc.ship_status);
        let sales_repo = SalesOrdersRepo::new(state.db.pool());
        let before_order = match sales_repo
            .get_sales_order_by_outbound(tenant_id, outbound_doc_id)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                warn!(error = %e, "閿€鍞崟鑱斿姩鑾峰彇失败");
                None
            }
        };
        if let Some(before_order) = before_order {
            let updated = sales_repo
                .update_wms_status_by_outbound(
                    tenant_id,
                    outbound_doc_id,
                    SalesOrderWmsUpdate {
                        wms_outbound_status: Some(detail.doc.outbound_status.clone()),
                        wms_wave_id: None,
                        wms_wave_status: None,
                        wms_ship_status: Some(detail.doc.ship_status.clone()),
                        delivery_status: Some(delivery_status.clone()),
                        package_count: Some(package_count),
                        waybill_nos: None,
                        carrier_code: None,
                        logistics_service: None,
                    },
                )
                .await;
            match updated {
                Ok(Some(updated)) => {
                    let from_status = format_sales_order_status(
                        "WMS_OUTBOUND_STATUS",
                        before_order.wms_outbound_status.as_deref(),
                    );
                    let to_status = format_sales_order_status(
                        "WMS_OUTBOUND_STATUS",
                        updated.wms_outbound_status.as_deref(),
                    );
                    record_sales_order_state_logs(
                        &state,
                        &req_ctx,
                        tenant_id,
                        updated.sales_order_id,
                        "SALES_ORDER.WMS_PACKED",
                        from_status,
                        to_status,
                        Some(ctx.user_id),
                        None,
                        serde_json::json!({
                            "outbound_doc_id": outbound_doc_id,
                            "outbound_status": detail.doc.outbound_status,
                            "ship_status": detail.doc.ship_status,
                            "package_no": package_no.clone(),
                            "weight_gram": weight_gram,
                        }),
                        serde_json::json!({
                            "delivery_status": updated.delivery_status,
                            "wms_outbound_status": updated.wms_outbound_status,
                            "wms_ship_status": updated.wms_ship_status,
                            "package_count": updated.package_count,
                        }),
                    )
                    .await;
                }
                Ok(None) => {}
                Err(e) => {
                    warn!(error = %e, "閿€鍞崟鑱斿姩更新失败");
                }
            }
        }
    }

    StatusCode::NO_CONTENT.into_response()
}

pub(crate) async fn inspect_wms_outbound(
    State(state): State<AppState>,
    Path((tenant_id, outbound_doc_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let outbound_doc_id = match uuid::Uuid::parse_str(&outbound_doc_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad outbound_doc_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.write") {
        return resp.into_response();
    }

    let inspect_enabled = match feature_flag_enabled(
        &state,
        tenant_id,
        None,
        FeatureFlagKey::WmsOutboundInspection,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if !inspect_enabled {
        return feature_flag_forbidden(FeatureFlagKey::WmsOutboundInspection).into_response();
    }

    let repo = WmsRepo::new(state.db.pool());
    let detail = match repo.get_outbound_doc(tenant_id, outbound_doc_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("outbound doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let workflow_config =
        match fetch_warehouse_workflow_config(state.db.pool(), tenant_id, detail.doc.warehouse_id)
            .await
        {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
    if !workflow_config.require_inspection {
        return bad_request("inspection disabled").into_response();
    }
    if let Err(e) = repo
        .inspect_outbound(OutboundInspectInput {
            tenant_id,
            outbound_doc_id,
        })
        .await
    {
        return bad_request(&e.to_string()).into_response();
    }

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_WMS_OUTBOUND_INSPECTED.to_string(),
            ref_type: Some("OUTBOUND_DOC".to_string()),
            ref_id: Some(outbound_doc_id.to_string()),
            metadata: Some(serde_json::json!({ "status": "INSPECTED" })),
        })
        .await
    {
        warn!(error = %e, "operation log insert failed");
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(e) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.INSPECTED".to_string(),
            entity_type: "OUTBOUND_DOC".to_string(),
            entity_id: Some(outbound_doc_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({ "status": "INSPECTED" })),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %e, "audit log insert failed");
    }

    StatusCode::NO_CONTENT.into_response()
}

pub(crate) async fn check_wms_outbound(
    State(state): State<AppState>,
    Path((tenant_id, outbound_doc_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WmsOutboundCheckRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let outbound_doc_id = match uuid::Uuid::parse_str(&outbound_doc_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad outbound_doc_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.write") {
        return resp.into_response();
    }
    if req.package_no.trim().is_empty() {
        return bad_request("package_no required").into_response();
    }
    if !req.passed
        && req
            .reason
            .as_deref()
            .map(|v| v.trim().is_empty())
            .unwrap_or(true)
    {
        return bad_request("reason required").into_response();
    }

    let inspect_enabled = match feature_flag_enabled(
        &state,
        tenant_id,
        None,
        FeatureFlagKey::WmsOutboundInspection,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if !inspect_enabled {
        return feature_flag_forbidden(FeatureFlagKey::WmsOutboundInspection).into_response();
    }

    let repo = WmsRepo::new(state.db.pool());
    let detail = match repo.get_outbound_doc(tenant_id, outbound_doc_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("outbound doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let workflow_config =
        match fetch_warehouse_workflow_config(state.db.pool(), tenant_id, detail.doc.warehouse_id)
            .await
        {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
    if !workflow_config.require_inspection {
        return bad_request("inspection disabled").into_response();
    }
    if detail.pack_records.is_empty() {
        return bad_request("pack records missing").into_response();
    }
    let package_no = req.package_no.trim().to_string();
    if !detail
        .pack_records
        .iter()
        .any(|record| record.package_no == package_no)
    {
        return bad_request("package_no not found").into_response();
    }
    if detail.doc.status != "PACKED" && detail.doc.status != "INSPECTED" {
        return bad_request("outbound doc not packed").into_response();
    }

    if let Err(e) = repo
        .record_pack_check_event(PackCheckEventInput {
            tenant_id,
            outbound_doc_id,
            shipment_id: detail.doc.shipment_id,
            package_no: package_no.clone(),
            event_type: "CHECK".to_string(),
            result: if req.passed {
                "PASSED".to_string()
            } else {
                "FAILED".to_string()
            },
            weight_gram: None,
            reason: req.reason.clone(),
            operator_id: Some(uuid::Uuid::from(ctx.user_id)),
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    if req.passed {
        match repo.has_unchecked_packs(tenant_id, outbound_doc_id).await {
            Ok(true) => {}
            Ok(false) => {
                if let Err(e) = repo
                    .inspect_outbound(OutboundInspectInput {
                        tenant_id,
                        outbound_doc_id,
                    })
                    .await
                {
                    return bad_request(&e.to_string()).into_response();
                }
            }
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }
    } else if detail.doc.status == "INSPECTED" {
        if let Err(e) = repo
            .reset_outbound_inspection(tenant_id, outbound_doc_id)
            .await
        {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
        }
    }

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_WMS_OUTBOUND_CHECKED.to_string(),
            ref_type: Some("OUTBOUND_DOC".to_string()),
            ref_id: Some(outbound_doc_id.to_string()),
            metadata: Some(serde_json::json!({
                "package_no": package_no,
                "passed": req.passed,
                "reason": req.reason,
            })),
        })
        .await
    {
        warn!(error = %e, "operation log insert failed");
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(e) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.OUTBOUND_CHECKED".to_string(),
            entity_type: "OUTBOUND_DOC".to_string(),
            entity_id: Some(outbound_doc_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({
                "package_no": package_no,
                "passed": req.passed,
                "reason": req.reason,
            })),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %e, "audit log insert failed");
    }

    StatusCode::NO_CONTENT.into_response()
}

pub(crate) async fn start_wms_outbound_verify(
    State(state): State<AppState>,
    Path((tenant_id, outbound_doc_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<WmsOutboundVerifyStartRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let outbound_doc_id = match uuid::Uuid::parse_str(&outbound_doc_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad outbound_doc_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.verify.write") {
        return resp.into_response();
    }
    if req.shipment_no.trim().is_empty() {
        return bad_request("shipment_no required").into_response();
    }

    let repo = WmsRepo::new(state.db.pool());
    let detail = match repo.get_outbound_doc(tenant_id, outbound_doc_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("outbound doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if let Err(resp) = ensure_warehouse_access(&ctx, detail.doc.warehouse_id) {
        return resp.into_response();
    }

    let verify_state = match repo
        .start_outbound_verify(OutboundVerifyStartInput {
            tenant_id,
            outbound_doc_id,
            shipment_no: req.shipment_no.clone(),
            tracking_no: req.tracking_no.clone(),
            package_no: req
                .package_no
                .clone()
                .unwrap_or_else(|| "DEFAULT".to_string()),
            started_by: Some(uuid::Uuid::from(ctx.user_id)),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("{e}")).into_response(),
    };

    let response = WmsOutboundVerifyStateResponse {
        outbound_doc_id: verify_state.outbound_doc_id,
        verify_status: verify_state.verify_status.clone(),
        scanned_total_qty: verify_state.scanned_total_qty,
        missing_total_qty: verify_state.missing_total_qty,
        extra_total_qty: verify_state.extra_total_qty,
    };

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: "WMS.OUTBOUND_VERIFY_STARTED".to_string(),
            ref_type: Some("OUTBOUND_DOC".to_string()),
            ref_id: Some(outbound_doc_id.to_string()),
            metadata: Some(serde_json::json!({
                "session_id": verify_state.session_id,
                "shipment_no": req.shipment_no,
                "tracking_no": req.tracking_no,
                "package_no": req.package_no,
                "verify_status": verify_state.verify_status,
                "scanned_total_qty": verify_state.scanned_total_qty,
                "missing_total_qty": verify_state.missing_total_qty,
                "extra_total_qty": verify_state.extra_total_qty,
            })),
        })
        .await
    {
        warn!(error = %e, "operation log insert failed");
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(e) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.OUTBOUND_VERIFY_STARTED".to_string(),
            entity_type: "OUTBOUND_DOC".to_string(),
            entity_id: Some(outbound_doc_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({
                "session_id": verify_state.session_id,
                "shipment_no": req.shipment_no,
                "tracking_no": req.tracking_no,
                "package_no": req.package_no,
                "verify_status": verify_state.verify_status,
                "scanned_total_qty": verify_state.scanned_total_qty,
                "missing_total_qty": verify_state.missing_total_qty,
                "extra_total_qty": verify_state.extra_total_qty,
            })),
            ip: req_ctx.ip.clone(),
            ua: req_ctx.user_agent.clone(),
        })
        .await
    {
        warn!(error = %e, "audit log insert failed");
    }

    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn scan_wms_outbound_verify_item(
    State(state): State<AppState>,
    Path((tenant_id, outbound_doc_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<WmsOutboundVerifyScanItemRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let outbound_doc_id = match uuid::Uuid::parse_str(&outbound_doc_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad outbound_doc_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.verify.write") {
        return resp.into_response();
    }
    if req.barcode.trim().is_empty() {
        return bad_request("barcode required").into_response();
    }
    if req.qty <= 0 {
        return bad_request("qty must be positive").into_response();
    }

    let repo = WmsRepo::new(state.db.pool());
    let detail = match repo.get_outbound_doc(tenant_id, outbound_doc_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("outbound doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if let Err(resp) = ensure_warehouse_access(&ctx, detail.doc.warehouse_id) {
        return resp.into_response();
    }

    let verify_state = match repo
        .scan_outbound_verify_item(OutboundVerifyScanInput {
            tenant_id,
            outbound_doc_id,
            barcode: req.barcode.clone(),
            qty: req.qty,
            scanned_by: Some(uuid::Uuid::from(ctx.user_id)),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("{e}")).into_response(),
    };
    let response = WmsOutboundVerifyStateResponse {
        outbound_doc_id: verify_state.outbound_doc_id,
        verify_status: verify_state.verify_status.clone(),
        scanned_total_qty: verify_state.scanned_total_qty,
        missing_total_qty: verify_state.missing_total_qty,
        extra_total_qty: verify_state.extra_total_qty,
    };

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: "WMS.OUTBOUND_VERIFY_SCANNED".to_string(),
            ref_type: Some("OUTBOUND_DOC".to_string()),
            ref_id: Some(outbound_doc_id.to_string()),
            metadata: Some(serde_json::json!({
                "session_id": verify_state.session_id,
                "barcode": req.barcode,
                "qty": req.qty,
                "verify_status": verify_state.verify_status,
                "scanned_total_qty": verify_state.scanned_total_qty,
                "missing_total_qty": verify_state.missing_total_qty,
                "extra_total_qty": verify_state.extra_total_qty,
            })),
        })
        .await
    {
        warn!(error = %e, "operation log insert failed");
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(e) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.OUTBOUND_VERIFY_SCANNED".to_string(),
            entity_type: "OUTBOUND_DOC".to_string(),
            entity_id: Some(outbound_doc_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({
                "barcode": req.barcode,
                "qty": req.qty,
                "session_id": verify_state.session_id,
                "verify_status": verify_state.verify_status,
                "scanned_total_qty": verify_state.scanned_total_qty,
                "missing_total_qty": verify_state.missing_total_qty,
                "extra_total_qty": verify_state.extra_total_qty,
            })),
            ip: req_ctx.ip.clone(),
            ua: req_ctx.user_agent.clone(),
        })
        .await
    {
        warn!(error = %e, "audit log insert failed");
    }

    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn finish_wms_outbound_verify(
    State(state): State<AppState>,
    Path((tenant_id, outbound_doc_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let outbound_doc_id = match uuid::Uuid::parse_str(&outbound_doc_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad outbound_doc_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.verify.write") {
        return resp.into_response();
    }

    let repo = WmsRepo::new(state.db.pool());
    let detail = match repo.get_outbound_doc(tenant_id, outbound_doc_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("outbound doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if let Err(resp) = ensure_warehouse_access(&ctx, detail.doc.warehouse_id) {
        return resp.into_response();
    }

    let verify_state = match repo
        .finish_outbound_verify(OutboundVerifyFinishInput {
            tenant_id,
            outbound_doc_id,
            finished_by: Some(uuid::Uuid::from(ctx.user_id)),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("{e}")).into_response(),
    };
    let response = WmsOutboundVerifyStateResponse {
        outbound_doc_id: verify_state.outbound_doc_id,
        verify_status: verify_state.verify_status.clone(),
        scanned_total_qty: verify_state.scanned_total_qty,
        missing_total_qty: verify_state.missing_total_qty,
        extra_total_qty: verify_state.extra_total_qty,
    };

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: "WMS.OUTBOUND_VERIFY_FINISHED".to_string(),
            ref_type: Some("OUTBOUND_DOC".to_string()),
            ref_id: Some(outbound_doc_id.to_string()),
            metadata: Some(serde_json::json!({
                "session_id": verify_state.session_id,
                "verify_status": verify_state.verify_status,
                "scanned_total_qty": verify_state.scanned_total_qty,
                "missing_total_qty": verify_state.missing_total_qty,
                "extra_total_qty": verify_state.extra_total_qty,
            })),
        })
        .await
    {
        warn!(error = %e, "operation log insert failed");
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(e) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.OUTBOUND_VERIFY_FINISHED".to_string(),
            entity_type: "OUTBOUND_DOC".to_string(),
            entity_id: Some(outbound_doc_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({
                "session_id": verify_state.session_id,
                "verify_status": verify_state.verify_status,
                "scanned_total_qty": verify_state.scanned_total_qty,
                "missing_total_qty": verify_state.missing_total_qty,
                "extra_total_qty": verify_state.extra_total_qty,
            })),
            ip: req_ctx.ip.clone(),
            ua: req_ctx.user_agent.clone(),
        })
        .await
    {
        warn!(error = %e, "audit log insert failed");
    }

    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn weigh_wms_outbound(
    State(state): State<AppState>,
    Path((tenant_id, outbound_doc_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WmsOutboundWeighRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let outbound_doc_id = match uuid::Uuid::parse_str(&outbound_doc_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad outbound_doc_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.write") {
        return resp.into_response();
    }
    if req.package_no.trim().is_empty() {
        return bad_request("package_no required").into_response();
    }
    if req.weight_gram <= 0 {
        return bad_request("weight_gram must be positive").into_response();
    }

    let repo = WmsRepo::new(state.db.pool());
    let detail = match repo.get_outbound_doc(tenant_id, outbound_doc_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("outbound doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let workflow_config =
        match fetch_warehouse_workflow_config(state.db.pool(), tenant_id, detail.doc.warehouse_id)
            .await
        {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
    if !workflow_config.require_weighing {
        return bad_request("weighing disabled").into_response();
    }

    if let Err(e) = repo
        .update_pack_record_weight(
            tenant_id,
            outbound_doc_id,
            req.package_no.trim(),
            req.weight_gram,
            Some(uuid::Uuid::from(ctx.user_id)),
        )
        .await
    {
        return bad_request(&e.to_string()).into_response();
    }
    if let Err(e) = repo
        .record_pack_check_event(PackCheckEventInput {
            tenant_id,
            outbound_doc_id,
            shipment_id: detail.doc.shipment_id,
            package_no: req.package_no.trim().to_string(),
            event_type: "WEIGH".to_string(),
            result: "PASSED".to_string(),
            weight_gram: Some(req.weight_gram),
            reason: None,
            operator_id: Some(uuid::Uuid::from(ctx.user_id)),
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_WMS_OUTBOUND_WEIGHED.to_string(),
            ref_type: Some("OUTBOUND_DOC".to_string()),
            ref_id: Some(outbound_doc_id.to_string()),
            metadata: Some(serde_json::json!({
                "package_no": req.package_no,
                "weight_gram": req.weight_gram,
            })),
        })
        .await
    {
        warn!(error = %e, "operation log insert failed");
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(e) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.OUTBOUND_WEIGHED".to_string(),
            entity_type: "OUTBOUND_DOC".to_string(),
            entity_id: Some(outbound_doc_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({
                "package_no": req.package_no,
                "weight_gram": req.weight_gram,
            })),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %e, "audit log insert failed");
    }

    StatusCode::NO_CONTENT.into_response()
}

pub(crate) async fn ship_wms_outbound(
    State(state): State<AppState>,
    Path((tenant_id, outbound_doc_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<WmsShipRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let outbound_doc_id = match uuid::Uuid::parse_str(&outbound_doc_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad outbound_doc_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.write") {
        return resp.into_response();
    }

    let inspect_enabled = match feature_flag_enabled(
        &state,
        tenant_id,
        None,
        FeatureFlagKey::WmsOutboundInspection,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };

    let shipped_at = match req.shipped_at.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad shipped_at").into_response(),
        },
        None => None,
    };
    let shipped_at_log = shipped_at;

    let repo = WmsRepo::new(state.db.pool());
    let detail = match repo.get_outbound_doc(tenant_id, outbound_doc_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("outbound doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let workflow_config =
        match fetch_warehouse_workflow_config(state.db.pool(), tenant_id, detail.doc.warehouse_id)
            .await
        {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
    let inspect_required = inspect_enabled && workflow_config.require_inspection;
    let allow_unpacked = !workflow_config.require_pack;
    if workflow_config.require_weighing {
        match repo.has_unweighed_packs(tenant_id, outbound_doc_id).await {
            Ok(true) => return bad_request("outbound doc not weighed").into_response(),
            Ok(false) => {}
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }
    }
    if inspect_required {
        match repo.has_unchecked_packs(tenant_id, outbound_doc_id).await {
            Ok(true) => return bad_request("outbound doc not checked").into_response(),
            Ok(false) => {}
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }
    }

    if let Some(serials) = req.serials {
        if serials.is_empty() {
            return bad_request("serials required").into_response();
        }
        let shipment_id = match detail.doc.shipment_id {
            Some(v) => v,
            None => return bad_request("shipment_id required for serials").into_response(),
        };
        let order_id = match detail.doc.order_id {
            Some(v) => v,
            None => return bad_request("order_id required for serials").into_response(),
        };

        let fulfillment_repo = FulfillmentRepo::new(state.db.pool());
        let shipment_items = match fulfillment_repo
            .list_oms_shipment_items(tenant_id, shipment_id)
            .await
        {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        if shipment_items.is_empty() {
            return bad_request("shipment items missing").into_response();
        }

        let mut items_by_sku: std::collections::HashMap<uuid::Uuid, Vec<(uuid::Uuid, i64)>> =
            std::collections::HashMap::new();
        for item in shipment_items {
            items_by_sku
                .entry(item.sku_id)
                .or_default()
                .push((item.order_item_id, item.qty));
        }

        for entry in serials {
            if entry.serial_nos.is_empty() {
                return bad_request("serial_nos required").into_response();
            }
            let sku_id = match uuid::Uuid::parse_str(&entry.sku_id) {
                Ok(v) => v,
                Err(_) => return bad_request("bad sku_id").into_response(),
            };
            let Some(items) = items_by_sku.get(&sku_id) else {
                return bad_request("sku not in shipment").into_response();
            };
            let total_required: i64 = items.iter().map(|(_, qty)| *qty).sum();
            if entry.serial_nos.len() as i64 != total_required {
                return conflict("serial count mismatch").into_response();
            }

            let mut remaining = entry.serial_nos;
            for (order_item_id, qty) in items {
                let take = (*qty).max(0) as usize;
                if take == 0 {
                    continue;
                }
                let chunk: Vec<String> = remaining.drain(0..take).collect();
                if let Err(e) = state
                    .sku_serials_repo
                    .bind_serials_to_shipment(RepoShipmentSerialBindInput {
                        tenant_id,
                        shipment_id,
                        order_id,
                        order_item_id: *order_item_id,
                        sku_id,
                        serial_nos: chunk,
                    })
                    .await
                {
                    return bad_request(&e.to_string()).into_response();
                }
            }
            if !remaining.is_empty() {
                return conflict("serial count mismatch").into_response();
            }
        }
    }

    let sku_ids: Vec<uuid::Uuid> = detail.items.iter().map(|item| item.sku_id).collect();
    if !sku_ids.is_empty() {
        let required_serials: Vec<uuid::Uuid> = match sqlx::query_scalar(
            r#"
            select sku_id
            from skus
            where tenant_id = $1 and sku_id = any($2) and serial_enabled = true
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(&sku_ids)
        .fetch_all(state.db.pool())
        .await
        {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };

        if !required_serials.is_empty() {
            let shipment_id = match detail.doc.shipment_id {
                Some(v) => v,
                None => return conflict("shipment_id required for serial check").into_response(),
            };
            let counts = match state
                .sku_serials_repo
                .count_shipment_serials(tenant_id, shipment_id)
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()
                }
            };
            let mut count_map = std::collections::HashMap::new();
            for count in counts {
                count_map.insert(count.sku_id, count.qty);
            }
            for item in &detail.items {
                if required_serials.contains(&item.sku_id) {
                    let bound_qty = count_map.get(&item.sku_id).copied().unwrap_or(0);
                    if bound_qty < item.planned_qty {
                        return conflict("serials required before shipping").into_response();
                    }
                }
            }
        }
    }

    if let Err(e) = repo
        .ship_outbound(OutboundShipInput {
            tenant_id,
            outbound_doc_id,
            shipped_at,
            allow_unpacked,
            inspect_required,
        })
        .await
    {
        return wms_ship_error_response(e);
    }

    sync_shipment_serials_after_wms_ship(&state, tenant_id, detail.doc.shipment_id).await;

    if let Ok(Some(detail)) = repo.get_outbound_doc(tenant_id, outbound_doc_id).await {
        let delivery_status =
            derive_delivery_status(&detail.doc.outbound_status, &detail.doc.ship_status);
        let package_count = detail.pack_records.len().min(i32::MAX as usize) as i32;
        let sales_repo = SalesOrdersRepo::new(state.db.pool());
        let before_order = match sales_repo
            .get_sales_order_by_outbound(tenant_id, outbound_doc_id)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                warn!(error = %e, "閿€鍞崟鑱斿姩鑾峰彇失败");
                None
            }
        };
        if let Some(before_order) = before_order {
            let updated = sales_repo
                .update_wms_status_by_outbound(
                    tenant_id,
                    outbound_doc_id,
                    SalesOrderWmsUpdate {
                        wms_outbound_status: Some(detail.doc.outbound_status.clone()),
                        wms_wave_id: None,
                        wms_wave_status: None,
                        wms_ship_status: Some(detail.doc.ship_status.clone()),
                        delivery_status: Some(delivery_status.clone()),
                        package_count: Some(package_count),
                        waybill_nos: None,
                        carrier_code: None,
                        logistics_service: None,
                    },
                )
                .await;
            match updated {
                Ok(Some(updated)) => {
                    let from_status = format_sales_order_status(
                        "DELIVERY_STATUS",
                        Some(before_order.delivery_status.as_str()),
                    );
                    let to_status = format_sales_order_status(
                        "DELIVERY_STATUS",
                        Some(updated.delivery_status.as_str()),
                    );
                    record_sales_order_state_logs(
                        &state,
                        &req_ctx,
                        tenant_id,
                        updated.sales_order_id,
                        "SALES_ORDER.WMS_SHIPPED",
                        from_status,
                        to_status,
                        Some(ctx.user_id),
                        None,
                        serde_json::json!({
                            "outbound_doc_id": outbound_doc_id,
                            "outbound_status": detail.doc.outbound_status,
                            "ship_status": detail.doc.ship_status,
                            "shipped_at": shipped_at_log.clone(),
                        }),
                        serde_json::json!({
                            "delivery_status": updated.delivery_status,
                            "wms_outbound_status": updated.wms_outbound_status,
                            "wms_ship_status": updated.wms_ship_status,
                            "package_count": updated.package_count,
                        }),
                    )
                    .await;
                }
                Ok(None) => {}
                Err(e) => {
                    warn!(error = %e, "閿€鍞崟鑱斿姩更新失败");
                }
            }
        }
    }

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_WMS_OUTBOUND_SHIPPED.to_string(),
            ref_type: Some("OUTBOUND_DOC".to_string()),
            ref_id: Some(outbound_doc_id.to_string()),
            metadata: Some(serde_json::json!({
                "shipped_at": shipped_at_log.clone(),
                "allow_unpacked": allow_unpacked,
                "inspect_required": inspect_required,
            })),
        })
        .await
    {
        warn!(error = %e, "operation log insert failed");
    }

    let logs_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.SHIPPED".to_string(),
            entity_type: "OUTBOUND_DOC".to_string(),
            entity_id: Some(outbound_doc_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({
                "shipped_at": shipped_at_log,
            })),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %e, "audit log insert failed");
    }

    StatusCode::NO_CONTENT.into_response()
}

pub(crate) async fn direct_ship_wms_outbound(
    State(state): State<AppState>,
    Path((tenant_id, outbound_doc_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<WmsDirectShipRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let outbound_doc_id = match uuid::Uuid::parse_str(&outbound_doc_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad outbound_doc_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.direct_ship") {
        return resp.into_response();
    }

    let shipped_at = match req.shipped_at.as_deref().and_then(non_empty) {
        Some(raw) => match DateTime::parse_from_rfc3339(&raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad shipped_at").into_response(),
        },
        None => None,
    };
    let reason = req.reason.and_then(non_empty);

    let repo = WmsRepo::new(state.db.pool());
    let detail = match repo.get_outbound_doc(tenant_id, outbound_doc_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("outbound doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if let Err(resp) = ensure_warehouse_access(&ctx, detail.doc.warehouse_id) {
        return resp.into_response();
    }

    if let Some(serials) = req.serials {
        if let Err(resp) =
            bind_shipment_serials_for_outbound(&state, tenant_id, outbound_doc_id, &detail, serials)
                .await
        {
            return resp;
        }
    }
    if let Err(resp) = ensure_required_serials_bound_for_outbound(&state, tenant_id, &detail).await
    {
        return resp;
    }

    if let Err(e) = repo
        .ship_outbound(OutboundShipInput {
            tenant_id,
            outbound_doc_id,
            shipped_at,
            allow_unpacked: true,
            inspect_required: false,
        })
        .await
    {
        return wms_ship_error_response(e);
    }

    sync_shipment_serials_after_wms_ship(&state, tenant_id, detail.doc.shipment_id).await;

    sync_sales_order_status_after_wms_ship(
        &state,
        &req_ctx,
        tenant_id,
        outbound_doc_id,
        ctx.user_id,
        shipped_at,
    )
    .await;

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: "WMS.DIRECT_SHIPPED".to_string(),
            ref_type: Some("OUTBOUND_DOC".to_string()),
            ref_id: Some(outbound_doc_id.to_string()),
            metadata: Some(serde_json::json!({
                "shipped_at": shipped_at,
                "reason": reason.clone(),
            })),
        })
        .await
    {
        warn!(error = %e, "operation log insert failed");
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(e) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.DIRECT_SHIPPED".to_string(),
            entity_type: "OUTBOUND_DOC".to_string(),
            entity_id: Some(outbound_doc_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({
                "shipped_at": shipped_at,
                "reason": reason,
            })),
            ip: req_ctx.ip.clone(),
            ua: req_ctx.user_agent.clone(),
        })
        .await
    {
        warn!(error = %e, "audit log insert failed");
    }

    StatusCode::NO_CONTENT.into_response()
}

pub(crate) async fn register_wms_outbound_operators(
    State(state): State<AppState>,
    Path((tenant_id, outbound_doc_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<WmsOutboundOperatorRegisterRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let outbound_doc_id = match uuid::Uuid::parse_str(&outbound_doc_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad outbound_doc_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.operator.register") {
        return resp.into_response();
    }

    let picker_user_id = match req.picker_user_id.as_deref().and_then(non_empty) {
        Some(raw) => match uuid::Uuid::parse_str(&raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad picker_user_id").into_response(),
        },
        None => None,
    };
    let packer_user_id = match req.packer_user_id.as_deref().and_then(non_empty) {
        Some(raw) => match uuid::Uuid::parse_str(&raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad packer_user_id").into_response(),
        },
        None => None,
    };
    if picker_user_id.is_none() && packer_user_id.is_none() {
        return bad_request("picker_user_id or packer_user_id required").into_response();
    }
    let note = req.note.and_then(non_empty);

    let repo = WmsRepo::new(state.db.pool());
    let detail = match repo.get_outbound_doc(tenant_id, outbound_doc_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("outbound doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if let Err(resp) = ensure_warehouse_access(&ctx, detail.doc.warehouse_id) {
        return resp.into_response();
    }

    let registered_at = Utc::now();
    let response = WmsOutboundOperatorRegisterResponse {
        outbound_doc_id,
        picker_user_id,
        packer_user_id,
        note: note.clone(),
        registered_by: uuid::Uuid::from(ctx.user_id),
        registered_at,
    };

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: "WMS.OUTBOUND_OPERATOR_REGISTERED".to_string(),
            ref_type: Some("OUTBOUND_DOC".to_string()),
            ref_id: Some(outbound_doc_id.to_string()),
            metadata: Some(serde_json::json!({
                "picker_user_id": picker_user_id,
                "packer_user_id": packer_user_id,
                "note": note.clone(),
                "registered_at": registered_at,
            })),
        })
        .await
    {
        warn!(error = %e, "operation log insert failed");
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(e) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.OUTBOUND_OPERATOR_REGISTERED".to_string(),
            entity_type: "OUTBOUND_DOC".to_string(),
            entity_id: Some(outbound_doc_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({
                "picker_user_id": picker_user_id,
                "packer_user_id": packer_user_id,
                "note": note,
                "registered_at": registered_at,
            })),
            ip: req_ctx.ip.clone(),
            ua: req_ctx.user_agent.clone(),
        })
        .await
    {
        warn!(error = %e, "audit log insert failed");
    }

    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn bind_shipment_serials_for_outbound(
    state: &AppState,
    tenant_id: TenantId,
    outbound_doc_id: uuid::Uuid,
    detail: &erp_repo::wms::OutboundDocDetail,
    serials: Vec<WmsShipmentSerialEntry>,
) -> Result<(), Response> {
    if serials.is_empty() {
        return Err(bad_request("serials required").into_response());
    }

    let shipment_id = match detail.doc.shipment_id {
        Some(v) => v,
        None => return Err(bad_request("shipment_id required for serials").into_response()),
    };
    let order_id = match detail.doc.order_id {
        Some(v) => v,
        None => return Err(bad_request("order_id required for serials").into_response()),
    };

    let fulfillment_repo = FulfillmentRepo::new(state.db.pool());
    let shipment_items = match fulfillment_repo
        .list_oms_shipment_items(tenant_id, shipment_id)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response());
        }
    };
    if shipment_items.is_empty() {
        return Err(bad_request("shipment items missing").into_response());
    }

    let mut items_by_sku: std::collections::HashMap<uuid::Uuid, Vec<(uuid::Uuid, i64)>> =
        std::collections::HashMap::new();
    for item in shipment_items {
        items_by_sku
            .entry(item.sku_id)
            .or_default()
            .push((item.order_item_id, item.qty));
    }

    for entry in serials {
        if entry.serial_nos.is_empty() {
            return Err(bad_request("serial_nos required").into_response());
        }
        let sku_id = match uuid::Uuid::parse_str(&entry.sku_id) {
            Ok(v) => v,
            Err(_) => return Err(bad_request("bad sku_id").into_response()),
        };
        let Some(items) = items_by_sku.get(&sku_id) else {
            return Err(bad_request("sku not in shipment").into_response());
        };
        let total_required: i64 = items.iter().map(|(_, qty)| *qty).sum();
        if entry.serial_nos.len() as i64 != total_required {
            return Err(conflict("serial count mismatch").into_response());
        }

        let mut remaining = entry.serial_nos;
        for (order_item_id, qty) in items {
            let take = (*qty).max(0) as usize;
            if take == 0 {
                continue;
            }
            let chunk: Vec<String> = remaining.drain(0..take).collect();
            if let Err(e) = state
                .sku_serials_repo
                .bind_serials_to_shipment(RepoShipmentSerialBindInput {
                    tenant_id,
                    shipment_id,
                    order_id,
                    order_item_id: *order_item_id,
                    sku_id,
                    serial_nos: chunk,
                })
                .await
            {
                return Err(serial_bind_error_response(e));
            }
        }
        if !remaining.is_empty() {
            return Err(conflict("serial count mismatch").into_response());
        }
    }

    let _ = outbound_doc_id;
    Ok(())
}

pub(crate) fn serial_bind_error_response(error: anyhow::Error) -> Response {
    let message = error.to_string();
    if message.contains("serial not available") {
        return conflict(&message).into_response();
    }
    bad_request(&message).into_response()
}

pub(crate) fn wms_ship_error_response(error: anyhow::Error) -> Response {
    let message = error.to_string();
    if message.contains("serials required before shipping")
        || message.contains("batch info required before shipping")
        || message.contains("shipment_id required for batch check")
        || message.contains("batch expired before shipping")
        || message.contains("batch production date in future before shipping")
    {
        return conflict(&message).into_response();
    }
    bad_request(&message).into_response()
}

pub(crate) async fn ensure_required_serials_bound_for_outbound(
    state: &AppState,
    tenant_id: TenantId,
    detail: &erp_repo::wms::OutboundDocDetail,
) -> Result<(), Response> {
    let sku_ids: Vec<uuid::Uuid> = detail.items.iter().map(|item| item.sku_id).collect();
    if sku_ids.is_empty() {
        return Ok(());
    }

    let required_serials: Vec<uuid::Uuid> = match sqlx::query_scalar(
        r#"
        select sku_id
        from skus
        where tenant_id = $1 and sku_id = any($2) and serial_enabled = true
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(&sku_ids)
    .fetch_all(state.db.pool())
    .await
    {
        Ok(v) => v,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()),
    };

    if required_serials.is_empty() {
        return Ok(());
    }

    let shipment_id = match detail.doc.shipment_id {
        Some(v) => v,
        None => return Err(conflict("shipment_id required for serial check").into_response()),
    };
    let counts = match state
        .sku_serials_repo
        .count_shipment_serials(tenant_id, shipment_id)
        .await
    {
        Ok(v) => v,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()),
    };
    let mut count_map = std::collections::HashMap::new();
    for count in counts {
        count_map.insert(count.sku_id, count.qty);
    }
    for item in &detail.items {
        if required_serials.contains(&item.sku_id) {
            let bound_qty = count_map.get(&item.sku_id).copied().unwrap_or(0);
            if bound_qty < item.planned_qty {
                return Err(conflict("serials required before shipping").into_response());
            }
        }
    }

    Ok(())
}

pub(crate) async fn sync_shipment_serials_after_wms_ship(
    state: &AppState,
    tenant_id: TenantId,
    shipment_id: Option<uuid::Uuid>,
) {
    let Some(shipment_id) = shipment_id else {
        return;
    };

    if let Err(e) = state
        .sku_serials_repo
        .mark_shipment_serials_shipped(tenant_id, shipment_id)
        .await
    {
        warn!(error = %e, shipment_id = %shipment_id, "shipment serial sync after wms ship failed");
    }
}

pub(crate) async fn sync_sales_order_status_after_wms_ship(
    state: &AppState,
    req_ctx: &RequestContext,
    tenant_id: TenantId,
    outbound_doc_id: uuid::Uuid,
    user_id: UserId,
    shipped_at_log: Option<DateTime<Utc>>,
) {
    let repo = WmsRepo::new(state.db.pool());
    let detail = match repo.get_outbound_doc(tenant_id, outbound_doc_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return,
        Err(e) => {
            warn!(error = %e, "sales order sync load outbound failed");
            return;
        }
    };

    let delivery_status =
        derive_delivery_status(&detail.doc.outbound_status, &detail.doc.ship_status);
    let package_count = detail.pack_records.len().min(i32::MAX as usize) as i32;
    let sales_repo = SalesOrdersRepo::new(state.db.pool());
    let before_order = match sales_repo
        .get_sales_order_by_outbound(tenant_id, outbound_doc_id)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "sales order linkage fetch failed");
            None
        }
    };
    let Some(before_order) = before_order else {
        return;
    };

    let updated = sales_repo
        .update_wms_status_by_outbound(
            tenant_id,
            outbound_doc_id,
            SalesOrderWmsUpdate {
                wms_outbound_status: Some(detail.doc.outbound_status.clone()),
                wms_wave_id: None,
                wms_wave_status: None,
                wms_ship_status: Some(detail.doc.ship_status.clone()),
                delivery_status: Some(delivery_status.clone()),
                package_count: Some(package_count),
                waybill_nos: None,
                carrier_code: None,
                logistics_service: None,
            },
        )
        .await;

    match updated {
        Ok(Some(updated)) => {
            let from_status = format_sales_order_status(
                "DELIVERY_STATUS",
                Some(before_order.delivery_status.as_str()),
            );
            let to_status = format_sales_order_status(
                "DELIVERY_STATUS",
                Some(updated.delivery_status.as_str()),
            );
            record_sales_order_state_logs(
                state,
                req_ctx,
                tenant_id,
                updated.sales_order_id,
                "SALES_ORDER.WMS_SHIPPED",
                from_status,
                to_status,
                Some(user_id),
                None,
                serde_json::json!({
                    "outbound_doc_id": outbound_doc_id,
                    "outbound_status": detail.doc.outbound_status,
                    "ship_status": detail.doc.ship_status,
                    "shipped_at": shipped_at_log,
                }),
                serde_json::json!({
                    "delivery_status": updated.delivery_status,
                    "wms_outbound_status": updated.wms_outbound_status,
                    "wms_ship_status": updated.wms_ship_status,
                    "package_count": updated.package_count,
                }),
            )
            .await;
        }
        Ok(None) => {}
        Err(e) => {
            warn!(error = %e, "sales order linkage update failed");
        }
    }
}

pub(crate) async fn reserve_wms_tracking(
    State(state): State<AppState>,
    Path((tenant_id, outbound_doc_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WmsReserveTrackingRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let outbound_doc_id = match uuid::Uuid::parse_str(&outbound_doc_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad outbound_doc_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.write") {
        return resp.into_response();
    }

    let waybill_enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::WaybillAutoReserve)
            .await
        {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !waybill_enabled {
        return feature_flag_forbidden(FeatureFlagKey::WaybillAutoReserve).into_response();
    }

    let repo = WmsRepo::new(state.db.pool());
    let detail = match repo.get_outbound_doc(tenant_id, outbound_doc_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("outbound doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let shipment_id = match detail.doc.shipment_id {
        Some(v) => v,
        None => return bad_request("shipment_id required").into_response(),
    };

    let shipments_repo = ShipmentsRepo::new(state.db.pool());
    let shipment = match shipments_repo.get_by_id(tenant_id, shipment_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shipment not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let carrier_code = req
        .carrier_code
        .and_then(non_empty)
        .or_else(|| detail.doc.carrier_code.clone().and_then(non_empty))
        .or_else(|| shipment.carrier_code.clone().and_then(non_empty));
    let carrier_code = if let Some(code) = carrier_code {
        Some(code)
    } else {
        let shops_repo = ShopsRepo::new(state.db.pool());
        match shops_repo
            .get_default_carrier_code(tenant_id, ShopId::from(shipment.shop_id))
            .await
        {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }
    };
    let Some(carrier_code) = carrier_code else {
        return bad_request("carrier_code required").into_response();
    };

    let preferred_waybill = req
        .waybill_no
        .and_then(non_empty)
        .or_else(|| shipment.tracking_no.clone().and_then(non_empty));

    let waybills_repo = WaybillsRepo::new(state.db.pool());
    let reserved = match waybills_repo
        .reserve_waybill(
            tenant_id,
            shipment.shop_id,
            &carrier_code,
            shipment_id,
            preferred_waybill.as_deref(),
        )
        .await
    {
        Ok(v) => v,
        Err(err) => {
            let message = err.to_string();
            if message.contains("no available waybill")
                || message.contains("shipment already reserved")
            {
                return conflict("waybill not available").into_response();
            }
            return (StatusCode::INTERNAL_SERVER_ERROR, message).into_response();
        }
    };

    if let Err(e) = shipments_repo
        .update_logistics(
            tenant_id,
            shipment_id,
            Some(carrier_code.clone()),
            Some(reserved.waybill_no.clone()),
        )
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_WMS_TRACKING_RESERVED.to_string(),
            ref_type: Some("OUTBOUND_DOC".to_string()),
            ref_id: Some(outbound_doc_id.to_string()),
            metadata: Some(serde_json::json!({
                "carrier_code": carrier_code.clone(),
                "tracking_no": reserved.waybill_no.clone(),
            })),
        })
        .await
    {
        warn!(error = %e, "operation log insert failed");
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(e) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.TRACKING_RESERVED".to_string(),
            entity_type: "OUTBOUND_DOC".to_string(),
            entity_id: Some(outbound_doc_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({
                "carrier_code": carrier_code,
                "tracking_no": reserved.waybill_no,
            })),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %e, "audit log insert failed");
    }

    let detail = match repo.get_outbound_doc(tenant_id, outbound_doc_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("outbound doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let response = wms_outbound_detail_response(detail);
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn update_wms_outbound_logistics(
    State(state): State<AppState>,
    Path((tenant_id, outbound_doc_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<WmsLogisticsUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let outbound_doc_id = match uuid::Uuid::parse_str(&outbound_doc_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad outbound_doc_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.write") {
        return resp.into_response();
    }

    let carrier_code = req.carrier_code.and_then(non_empty);
    let tracking_no = req.tracking_no.and_then(non_empty);
    if carrier_code.is_none() && tracking_no.is_none() {
        return bad_request("carrier_code or tracking_no required").into_response();
    }

    let repo = WmsRepo::new(state.db.pool());
    let detail = match repo.get_outbound_doc(tenant_id, outbound_doc_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("outbound doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let shipment_id = match detail.doc.shipment_id {
        Some(v) => v,
        None => return bad_request("shipment_id required").into_response(),
    };

    let shipments_repo = ShipmentsRepo::new(state.db.pool());
    let shipment = match shipments_repo
        .update_logistics(
            tenant_id,
            shipment_id,
            carrier_code.clone(),
            tracking_no.clone(),
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    if detail.doc.status == "SHIPPED" {
        let sent: Option<uuid::Uuid> = match sqlx::query_scalar(
            r#"
            select outbox_id
            from outbox_events
            where tenant_id = $1
              and event_type = $2
              and payload->>'shipment_id' = $3
              and status = 'SENT'
            limit 1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(SHIPMENT_SHIPPED_EVENT)
        .bind(shipment_id.to_string())
        .fetch_optional(state.db.pool())
        .await
        {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };

        if sent.is_some() {
            let payload = serde_json::json!({
                "tenant_id": tenant_id,
                "shipment_id": shipment.shipment_id,
                "order_id": shipment.order_id,
                "shop_id": shipment.shop_id,
                "platform": shipment.platform,
                "platform_order_id": shipment.platform_order_id,
                "warehouse_id": shipment.warehouse_id.unwrap_or(detail.doc.warehouse_id),
                "carrier_code": shipment.carrier_code,
                "tracking_no": shipment.tracking_no,
                "shipped_at": shipment.shipped_at,
                "outbound_doc_id": outbound_doc_id,
            });
            let outbox_repo = OutboxRepo::new(state.db.pool());
            if let Err(e) = outbox_repo
                .enqueue(tenant_id, SHIPMENT_SHIPPED_EVENT, &payload)
                .await
            {
                warn!(error = %e, "enqueue shipment repush failed");
            }
        }
    }

    let delivery_status =
        derive_delivery_status(&detail.doc.outbound_status, &detail.doc.ship_status);
    let waybill_nos = tracking_no.as_ref().map(|value| vec![value.clone()]);
    let sales_repo = SalesOrdersRepo::new(state.db.pool());
    let before_order = match sales_repo
        .get_sales_order_by_outbound(tenant_id, outbound_doc_id)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "閿€鍞崟鑱斿姩鑾峰彇失败");
            None
        }
    };
    if let Some(before_order) = before_order {
        let updated = sales_repo
            .update_wms_status_by_outbound(
                tenant_id,
                outbound_doc_id,
                SalesOrderWmsUpdate {
                    wms_outbound_status: Some(detail.doc.outbound_status.clone()),
                    wms_wave_id: None,
                    wms_wave_status: None,
                    wms_ship_status: Some(detail.doc.ship_status.clone()),
                    delivery_status: Some(delivery_status.clone()),
                    package_count: None,
                    waybill_nos: waybill_nos.clone(),
                    carrier_code: carrier_code.clone(),
                    logistics_service: None,
                },
            )
            .await;
        match updated {
            Ok(Some(updated)) => {
                let from_status = format_sales_order_status(
                    "DELIVERY_STATUS",
                    Some(before_order.delivery_status.as_str()),
                );
                let to_status = format_sales_order_status(
                    "DELIVERY_STATUS",
                    Some(updated.delivery_status.as_str()),
                );
                record_sales_order_state_logs(
                    &state,
                    &req_ctx,
                    tenant_id,
                    updated.sales_order_id,
                    "SALES_ORDER.WMS_LOGISTICS_UPDATED",
                    from_status,
                    to_status,
                    Some(ctx.user_id),
                    None,
                    serde_json::json!({
                        "outbound_doc_id": outbound_doc_id,
                        "carrier_code": carrier_code.clone(),
                        "tracking_no": tracking_no.clone(),
                    }),
                    serde_json::json!({
                        "delivery_status": updated.delivery_status,
                        "waybill_nos": updated.waybill_nos,
                        "carrier_code": updated.carrier_code,
                    }),
                )
                .await;
            }
            Ok(None) => {}
            Err(e) => {
                warn!(error = %e, "閿€鍞崟鑱斿姩更新失败");
            }
        }
    }

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_WMS_OUTBOUND_LOGISTICS_UPDATED.to_string(),
            ref_type: Some("OUTBOUND_DOC".to_string()),
            ref_id: Some(outbound_doc_id.to_string()),
            metadata: Some(serde_json::json!({
                "shipment_id": shipment_id,
                "carrier_code": carrier_code.clone(),
                "tracking_no": tracking_no.clone(),
            })),
        })
        .await
    {
        warn!(error = %e, "operation log insert failed");
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(e) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.LOGISTICS_UPDATED".to_string(),
            entity_type: "OUTBOUND_DOC".to_string(),
            entity_id: Some(outbound_doc_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({
                "carrier_code": carrier_code.clone(),
                "tracking_no": tracking_no.clone(),
            })),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %e, "audit log insert failed");
    }

    let detail = match repo.get_outbound_doc(tenant_id, outbound_doc_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("outbound doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let response = wms_outbound_detail_response(detail);
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_wms_add_package(
    State(state): State<AppState>,
    Path((tenant_id, outbound_doc_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WmsAddPackageRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let outbound_doc_id = match uuid::Uuid::parse_str(&outbound_doc_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad outbound_doc_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.write") {
        return resp.into_response();
    }

    if req.doc_no.trim().is_empty() {
        return bad_request("doc_no required").into_response();
    }
    if req.items.is_empty() {
        return bad_request("items required").into_response();
    }

    let mut items = Vec::with_capacity(req.items.len());
    for item in &req.items {
        if item.planned_qty <= 0 {
            return bad_request("planned_qty must be positive").into_response();
        }
        let sku_id = match uuid::Uuid::parse_str(&item.sku_id) {
            Ok(v) => v,
            Err(_) => return bad_request("bad sku_id").into_response(),
        };
        items.push(OutboundDocItemInput {
            sku_id,
            planned_qty: item.planned_qty,
        });
    }

    let repo = WmsRepo::new(state.db.pool());
    let origin_detail = match repo.get_outbound_doc(tenant_id, outbound_doc_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("outbound doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let origin_shipment_id = match origin_detail.doc.shipment_id {
        Some(v) => v,
        None => return bad_request("shipment_id required").into_response(),
    };

    let shipments_repo = ShipmentsRepo::new(state.db.pool());
    let origin_shipment = match shipments_repo
        .get_by_id(tenant_id, origin_shipment_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shipment not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let mut shipment_type = req
        .shipment_type
        .and_then(non_empty)
        .unwrap_or_else(|| SHIPMENT_TYPE_RESEND.to_string());
    shipment_type = shipment_type.trim().to_uppercase();
    if shipment_type != "RESEND" && shipment_type != "ADD_PACKAGE" {
        return bad_request("unsupported shipment_type").into_response();
    }

    let carrier_code = req
        .carrier_code
        .and_then(non_empty)
        .or_else(|| origin_shipment.carrier_code.clone().and_then(non_empty));
    let tracking_no = req.tracking_no.and_then(non_empty);
    let warehouse_id = origin_shipment
        .warehouse_id
        .unwrap_or(origin_detail.doc.warehouse_id);

    let shipment_id = uuid::Uuid::new_v4();
    if let Err(e) = shipments_repo
        .create_requested(ShipmentCreateInput {
            shipment_id,
            tenant_id,
            shipment_type,
            shop_id: origin_shipment.shop_id,
            order_id: origin_shipment.order_id,
            platform: origin_shipment.platform.clone(),
            platform_order_id: origin_shipment.platform_order_id.clone(),
            warehouse_id,
            shipment_no: None,
            carrier_code,
            tracking_no,
            requested_at: Utc::now(),
            weight_gram: None,
            sender_address_id: origin_shipment.sender_address_id,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let detail = match repo
        .create_outbound_doc(OutboundDocCreateInput {
            tenant_id,
            warehouse_id,
            doc_type: origin_detail.doc.doc_type,
            shipment_id: Some(shipment_id),
            doc_no: req.doc_no.trim().to_string(),
            items,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return bad_request(&e.to_string()).into_response(),
    };

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_WMS_OUTBOUND_ADD_PACKAGE.to_string(),
            ref_type: Some("OUTBOUND_DOC".to_string()),
            ref_id: Some(detail.doc.outbound_doc_id.to_string()),
            metadata: Some(serde_json::json!({
                "origin_outbound_doc_id": outbound_doc_id,
                "doc_no": detail.doc.doc_no.clone(),
                "shipment_id": detail.doc.shipment_id,
            })),
        })
        .await
    {
        warn!(error = %e, "operation log insert failed");
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(e) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.ADD_PACKAGE".to_string(),
            entity_type: "OUTBOUND_DOC".to_string(),
            entity_id: Some(detail.doc.outbound_doc_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({
                "doc_no": detail.doc.doc_no,
                "shipment_id": detail.doc.shipment_id,
            })),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %e, "audit log insert failed");
    }

    let response = wms_outbound_detail_response(detail);
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn request_wms_shipment_cancel(
    State(state): State<AppState>,
    Path((tenant_id, outbound_doc_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WmsShipmentCancelRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let outbound_doc_id = match uuid::Uuid::parse_str(&outbound_doc_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad outbound_doc_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.write") {
        return resp.into_response();
    }

    let enabled = match feature_flag_enabled(
        &state,
        tenant_id,
        None,
        FeatureFlagKey::WmsShipmentCancel,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::WmsShipmentCancel).into_response();
    }

    let repo = WmsRepo::new(state.db.pool());
    let detail = match repo.get_outbound_doc(tenant_id, outbound_doc_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("outbound doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let shipment_id = match detail.doc.shipment_id {
        Some(v) => v,
        None => return bad_request("shipment_id required").into_response(),
    };

    let shipments_repo = ShipmentsRepo::new(state.db.pool());
    let reason = req.reason.clone();
    if let Err(e) = shipments_repo
        .request_cancel(
            tenant_id,
            shipment_id,
            reason.clone().and_then(non_empty),
            Some(uuid::Uuid::from(ctx.user_id)),
        )
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_WMS_SHIPMENT_CANCEL_REQUESTED.to_string(),
            ref_type: Some("OUTBOUND_DOC".to_string()),
            ref_id: Some(outbound_doc_id.to_string()),
            metadata: Some(serde_json::json!({
                "shipment_id": shipment_id,
                "reason": reason.clone(),
            })),
        })
        .await
    {
        warn!(error = %e, "operation log insert failed");
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(e) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.SHIPMENT_CANCEL_REQUESTED".to_string(),
            entity_type: "OUTBOUND_DOC".to_string(),
            entity_id: Some(outbound_doc_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({ "reason": reason })),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %e, "audit log insert failed");
    }

    StatusCode::NO_CONTENT.into_response()
}
