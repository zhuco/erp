use super::*;

pub(crate) async fn list_wms_outbounds(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<WmsOutboundListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.read") {
        return resp.into_response();
    }

    let warehouse_id = match query.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    let shipment_id = match query.shipment_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad shipment_id").into_response(),
        },
        None => None,
    };
    let shop_id = match query.shop_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };
    let created_from = match query.created_from.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad created_from").into_response(),
        },
        None => None,
    };
    let created_to = match query.created_to.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad created_to").into_response(),
        },
        None => None,
    };
    let outbound_status = match query.outbound_status.as_deref() {
        Some(raw) => match OutboundStatus::from_str(raw) {
            Ok(v) => Some(v.to_string()),
            Err(_) => return bad_request("bad outbound_status").into_response(),
        },
        None => None,
    };
    let pick_status = match query.pick_status.as_deref() {
        Some(raw) => match PickStatus::from_str(raw) {
            Ok(v) => Some(v.to_string()),
            Err(_) => return bad_request("bad pick_status").into_response(),
        },
        None => None,
    };
    let verify_status = match query.verify_status.as_deref() {
        Some(raw) => match VerifyStatus::from_str(raw) {
            Ok(v) => Some(v.to_string()),
            Err(_) => return bad_request("bad verify_status").into_response(),
        },
        None => None,
    };
    let pack_status = match query.pack_status.as_deref() {
        Some(raw) => match PackStatus::from_str(raw) {
            Ok(v) => Some(v.to_string()),
            Err(_) => return bad_request("bad pack_status").into_response(),
        },
        None => None,
    };
    let ship_status = match query.ship_status.as_deref() {
        Some(raw) => match ShipStatus::from_str(raw) {
            Ok(v) => Some(v.to_string()),
            Err(_) => return bad_request("bad ship_status").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = WmsRepo::new(state.db.pool());
    let result = match repo
        .list_outbound_docs(
            tenant_id,
            OutboundDocListFilters {
                warehouse_id,
                status: query.status.filter(|v| !v.trim().is_empty()),
                outbound_status,
                pick_status,
                verify_status,
                pack_status,
                ship_status,
                doc_type: query.doc_type.filter(|v| !v.trim().is_empty()),
                doc_no: query.doc_no.and_then(non_empty),
                doc_no_keyword: query.doc_no_keyword.and_then(non_empty),
                shipment_id,
                shop_id,
                carrier_code: query.carrier_code.and_then(non_empty),
                tracking_no: query.tracking_no.and_then(non_empty),
                tag: query.tag.and_then(non_empty),
                exception_flag: query.exception_flag.and_then(non_empty),
                created_from,
                created_to,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let items = result
        .items
        .into_iter()
        .map(wms_outbound_doc_response)
        .collect();
    let response = WmsOutboundListResponse {
        items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn list_wms_waiting_ship_outbounds(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<WmsWaitingShipListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.waiting_ship.read") {
        return resp.into_response();
    }

    let warehouse_id = match query.warehouse_id.as_deref().and_then(non_empty) {
        Some(raw) => match uuid::Uuid::parse_str(&raw) {
            Ok(v) => {
                if let Err(resp) = ensure_warehouse_access(&ctx, v) {
                    return resp.into_response();
                }
                Some(v)
            }
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    let shipment_id = match query.shipment_id.as_deref().and_then(non_empty) {
        Some(raw) => match uuid::Uuid::parse_str(&raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad shipment_id").into_response(),
        },
        None => None,
    };
    let shop_id = match query.shop_id.as_deref().and_then(non_empty) {
        Some(raw) => match uuid::Uuid::parse_str(&raw) {
            Ok(v) => {
                if let Err(resp) = ensure_shop_access(&ctx, ShopId(v)) {
                    return resp.into_response();
                }
                Some(v)
            }
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = WmsRepo::new(state.db.pool());
    let result = match repo
        .list_outbound_docs(
            tenant_id,
            OutboundDocListFilters {
                warehouse_id,
                status: None,
                outbound_status: Some(OutboundStatus::PendingShip.to_string()),
                pick_status: None,
                verify_status: None,
                pack_status: None,
                ship_status: Some(ShipStatus::NotShipped.to_string()),
                doc_type: None,
                doc_no: None,
                doc_no_keyword: query.doc_no_keyword.and_then(non_empty),
                shipment_id,
                shop_id,
                carrier_code: query.carrier_code.and_then(non_empty),
                tracking_no: query.tracking_no.and_then(non_empty),
                tag: None,
                exception_flag: None,
                created_from: None,
                created_to: None,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let items = result
        .items
        .into_iter()
        .map(wms_outbound_doc_response)
        .collect();
    let response = WmsOutboundListResponse {
        items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn get_wms_outbound(
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
    if let Err(resp) = ensure_permission(&ctx, "wms.read") {
        return resp.into_response();
    }

    let repo = WmsRepo::new(state.db.pool());
    let detail = match repo.get_outbound_doc(tenant_id, outbound_doc_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("outbound doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = wms_outbound_detail_response(detail);
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn print_wms_outbound(
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
    if let Err(resp) = ensure_permission(&ctx, "wms.read") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "print_jobs.write") {
        return resp.into_response();
    }

    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::Printing).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::Printing).into_response();
    }

    let repo = WmsRepo::new(state.db.pool());
    let detail = match repo.get_outbound_doc(tenant_id, outbound_doc_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("outbound doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let (template_id, device_fingerprint) =
        match resolve_print_mapping(&state, tenant_id, "PACK_LIST").await {
            Ok(v) => v,
            Err(resp) => return resp,
        };

    let payload = match serde_json::to_value(wms_outbound_detail_response(detail)) {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let job = match state
        .print_jobs_repo
        .create_job(PrintJobCreate {
            tenant_id,
            requested_by_user_id: ctx.user_id,
            job_type: "PACK_LIST".to_string(),
            ref_type: "OUTBOUND_DOC".to_string(),
            ref_id: outbound_doc_id,
            template_id,
            device_fingerprint,
            payload,
            trace_id: Some(req_ctx.trace_id.clone()),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "PRINT_JOB.CREATE",
        "print_job",
        Some(job.job_id.to_string()),
        None,
        Some(serde_json::to_value(&job).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(job)).into_response()
}

pub(crate) async fn audit_wms_outbound(
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

    let repo = WmsRepo::new(state.db.pool());
    let detail = match repo.audit_outbound_doc(tenant_id, outbound_doc_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("{e}")).into_response(),
    };

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_WMS_OUTBOUND_AUDITED.to_string(),
            ref_type: Some("OUTBOUND_DOC".to_string()),
            ref_id: Some(detail.doc.outbound_doc_id.to_string()),
            metadata: Some(serde_json::json!({
                "doc_no": detail.doc.doc_no,
                "warehouse_id": detail.doc.warehouse_id,
            })),
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(e) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.OUTBOUND_AUDITED".to_string(),
            entity_type: "OUTBOUND_DOC".to_string(),
            entity_id: Some(detail.doc.outbound_doc_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::to_value(&detail).unwrap_or(Value::Null)),
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

pub(crate) async fn create_wms_outbound(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<WmsOutboundCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.write") {
        return resp.into_response();
    }

    let sales_order_id = match req.sales_order_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("sales_order_id 鏃犳晥").into_response(),
        },
        None => None,
    };
    let sales_order = if let Some(sales_order_id) = sales_order_id {
        let sales_repo = SalesOrdersRepo::new(state.db.pool());
        match sales_repo.get_sales_order(tenant_id, sales_order_id).await {
            Ok(Some(order)) => Some(order),
            Ok(None) => return not_found("sales_order 不存在").into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }
    } else {
        None
    };

    if req.doc_no.trim().is_empty() {
        return bad_request("doc_no required").into_response();
    }

    let warehouse_id = match uuid::Uuid::parse_str(&req.warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    if let Err(resp) = ensure_warehouse_access(&ctx, warehouse_id) {
        return resp.into_response();
    }
    let shipment_id = match req.shipment_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad shipment_id").into_response(),
        },
        None => None,
    };
    let workflow_config =
        match fetch_warehouse_workflow_config(state.db.pool(), tenant_id, warehouse_id).await {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };

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
    let mut detail = match repo
        .create_outbound_doc(OutboundDocCreateInput {
            tenant_id,
            warehouse_id,
            doc_type: req.doc_type.trim().to_string(),
            shipment_id,
            doc_no: req.doc_no.trim().to_string(),
            items,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return bad_request(&e.to_string()).into_response(),
    };

    if !workflow_config.enable_wave {
        if let Err(e) = repo
            .create_pick_tasks_for_outbound(tenant_id, detail.doc.outbound_doc_id, false)
            .await
        {
            return bad_request(&e.to_string()).into_response();
        }
        if let Ok(Some(updated)) = repo
            .get_outbound_doc(tenant_id, detail.doc.outbound_doc_id)
            .await
        {
            detail = updated;
        }
    }

    let logs_repo = AuditLogsRepo::new(state.db.pool());
    let doc_no = detail.doc.doc_no.clone();
    let doc_type = detail.doc.doc_type.clone();
    if let Err(e) = logs_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.OUTBOUND_CREATED".to_string(),
            entity_type: "OUTBOUND_DOC".to_string(),
            entity_id: Some(detail.doc.outbound_doc_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({
                "doc_no": doc_no.clone(),
                "shipment_id": detail.doc.shipment_id,
                "warehouse_id": detail.doc.warehouse_id,
                "doc_type": doc_type.clone(),
                "sales_order_id": sales_order_id,
            })),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %e, "audit log insert failed");
    }

    let op_logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = op_logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_WMS_OUTBOUND_CREATED.to_string(),
            ref_type: Some("OUTBOUND_DOC".to_string()),
            ref_id: Some(detail.doc.outbound_doc_id.to_string()),
            metadata: Some(serde_json::json!({
                "doc_no": doc_no,
                "doc_type": doc_type,
                "warehouse_id": detail.doc.warehouse_id,
                "shipment_id": detail.doc.shipment_id,
                "sales_order_id": sales_order_id,
            })),
        })
        .await
    {
        warn!(error = %e, "operation log insert failed");
    }

    if let Some(sales_order_id) = sales_order_id {
        let sales_repo = SalesOrdersRepo::new(state.db.pool());
        let delivery_status =
            derive_delivery_status(&detail.doc.outbound_status, &detail.doc.ship_status);
        let updated = match sales_repo
            .bind_wms_outbound(
                tenant_id,
                sales_order_id,
                detail.doc.outbound_doc_id,
                Some(detail.doc.outbound_status.clone()),
                Some(detail.doc.ship_status.clone()),
                Some(delivery_status.clone()),
            )
            .await
        {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        if let Some(updated) = updated {
            let from_status = sales_order.as_ref().and_then(|order| {
                format_sales_order_status(
                    "WMS_OUTBOUND_STATUS",
                    order.wms_outbound_status.as_deref(),
                )
            });
            let to_status = format_sales_order_status(
                "WMS_OUTBOUND_STATUS",
                updated.wms_outbound_status.as_deref(),
            );
            record_sales_order_state_logs(
                &state,
                &req_ctx,
                tenant_id,
                sales_order_id,
                "SALES_ORDER.WMS_OUTBOUND_BOUND",
                from_status,
                to_status,
                Some(ctx.user_id),
                None,
                serde_json::json!({
                    "sales_order_id": sales_order_id,
                    "outbound_doc_id": detail.doc.outbound_doc_id,
                    "doc_no": detail.doc.doc_no,
                    "doc_type": detail.doc.doc_type,
                    "outbound_status": detail.doc.outbound_status,
                    "ship_status": detail.doc.ship_status,
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
    }

    let response = wms_outbound_detail_response(detail);
    (StatusCode::OK, Json(response)).into_response()
}
