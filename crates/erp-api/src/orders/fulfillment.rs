use super::*;

fn oms_shipment_types() -> Vec<String> {
    vec![
        SHIPMENT_TYPE_OMS.to_string(),
        SHIPMENT_TYPE_EXCHANGE.to_string(),
        SHIPMENT_TYPE_RESEND.to_string(),
    ]
}

pub(super) fn derive_allocation_status(shortage_count: usize) -> &'static str {
    if shortage_count == 0 {
        "ALLOCATED"
    } else {
        "SHORTAGE"
    }
}

fn derive_submit_shipment_status(shortage_count: usize) -> &'static str {
    if shortage_count == 0 {
        "PENDING_OUTBOUND"
    } else {
        "SHORTAGE"
    }
}

pub(super) fn parse_shipped_at(
    raw: Option<&str>,
    default_now: DateTime<Utc>,
) -> Result<DateTime<Utc>, Response> {
    match raw {
        Some(value) => match DateTime::parse_from_rfc3339(value) {
            Ok(parsed) => Ok(parsed.with_timezone(&Utc)),
            Err(_) => Err(bad_request("bad shipped_at").into_response()),
        },
        None => Ok(default_now),
    }
}

async fn load_shipment_summaries(
    state: &AppState,
    tenant_id: TenantId,
    raw_shipments: Vec<erp_repo::fulfillment::OmsShipment>,
) -> Result<Vec<OmsShipmentSummary>, Response> {
    let mut shipments = Vec::new();
    for shipment in raw_shipments {
        let shipment_id = shipment.shipment_id;
        let items = load_oms_shipment_items(state.db.pool(), tenant_id, shipment_id).await?;
        shipments.push(oms_shipment_summary(
            uuid::Uuid::from(tenant_id),
            shipment,
            Some(items),
        ));
    }
    Ok(shipments)
}

pub(super) async fn submit_order_shipment(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OmsSubmitShipmentRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.submit") {
        return resp.into_response();
    }

    let warehouse_id = match req.warehouse_id.as_deref().and_then(non_empty) {
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

    let orders_repo = OrdersRepo::new(state.db.pool());
    let state_row = match orders_repo
        .get_order_review_state(tenant_id, order_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("order not found").into_response(),
        Err(e) => return crate::inventory_views::inventory_error_response(e).into_response(),
    };
    if !state_row.review_status.is_approved() {
        return bad_request("order not approved").into_response();
    }

    let carrier_code = req.carrier_code.and_then(non_empty);
    let fulfillment_repo =
        FulfillmentRepo::new(state.db.pool()).with_actor(ctx.fulfillment_actor(&req_ctx));
    let allocation = match fulfillment_repo
        .allocate_order(tenant_id, order_id, carrier_code.clone())
        .await
    {
        Ok(v) => v,
        Err(e) => return crate::inventory_views::inventory_error_response(e).into_response(),
    };

    let wms_repo = WmsRepo::new(state.db.pool());
    let mut outbound_doc_id: Option<String> = None;
    if let Some(first_shipment) = allocation.shipments.first() {
        if let Ok(result) = wms_repo
            .list_outbound_docs(
                tenant_id,
                OutboundDocListFilters {
                    warehouse_id: None,
                    status: None,
                    outbound_status: None,
                    pick_status: None,
                    verify_status: None,
                    pack_status: None,
                    ship_status: None,
                    doc_type: None,
                    doc_no: None,
                    doc_no_keyword: None,
                    shipment_id: Some(first_shipment.shipment_id),
                    shop_id: None,
                    carrier_code: None,
                    tracking_no: None,
                    tag: None,
                    exception_flag: None,
                    created_from: None,
                    created_to: None,
                    offset: 0,
                    limit: 1,
                },
            )
            .await
        {
            outbound_doc_id = result
                .items
                .first()
                .map(|item| item.outbound_doc_id.to_string());
        }
    }

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "OMS.ORDER_SUBMIT_SHIPMENT_REQUESTED",
        "ORDER",
        Some(order_id.to_string()),
        None,
        Some(serde_json::json!({
            "warehouse_id": warehouse_id,
            "carrier_code": carrier_code,
            "logistics_product_code": req.logistics_product_code,
            "priority": req.priority,
            "remark": req.remark,
            "shipment_count": allocation.shipments.len(),
            "shortage_count": allocation.shortages.len(),
            "outbound_doc_id": outbound_doc_id,
        })),
    )
    .await;

    let response = OmsSubmitShipmentResponse {
        order_id: order_id.to_string(),
        outbound_doc_id,
        wms_status: derive_submit_shipment_status(allocation.shortages.len()).to_string(),
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn list_oms_shipments(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<OmsShipmentListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.read") {
        return resp.into_response();
    }

    let order_id = match query.order_id.as_deref() {
        Some(raw) => match OrderId::from_str(raw) {
            Ok(v) => Some(uuid::Uuid::from(v)),
            Err(_) => return bad_request("bad order_id").into_response(),
        },
        None => None,
    };
    let warehouse_id = match query.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = FulfillmentRepo::new(state.db.pool());
    let result = match repo
        .list_oms_shipments(
            tenant_id,
            OmsShipmentListFilters {
                order_id,
                status: query.status.and_then(non_empty),
                warehouse_id,
                shipment_types: oms_shipment_types(),
                limit,
                offset,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return crate::inventory_views::inventory_error_response(e).into_response(),
    };

    let items = result
        .items
        .into_iter()
        .map(|shipment| oms_shipment_summary(uuid::Uuid::from(tenant_id), shipment, None))
        .collect();
    let response = OmsShipmentListResponse {
        items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn list_oms_order_shipments(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.read") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => uuid::Uuid::from(v),
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    let repo = FulfillmentRepo::new(state.db.pool());
    let result = match repo
        .list_oms_shipments(
            tenant_id,
            OmsShipmentListFilters {
                order_id: Some(order_id),
                status: None,
                warehouse_id: None,
                shipment_types: oms_shipment_types(),
                limit: 200,
                offset: 0,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return crate::inventory_views::inventory_error_response(e).into_response(),
    };

    let items = match load_shipment_summaries(&state, tenant_id, result.items).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let total = items.len() as i64;
    let response = OmsShipmentListResponse { items, total };
    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn get_oms_shipment(
    State(state): State<AppState>,
    Path((tenant_id, shipment_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.read") {
        return resp.into_response();
    }
    let shipment_id = match uuid::Uuid::parse_str(&shipment_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shipment_id").into_response(),
    };

    let repo = FulfillmentRepo::new(state.db.pool());
    let shipment = match repo.get_oms_shipment(tenant_id, shipment_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shipment not found").into_response(),
        Err(e) => return crate::inventory_views::inventory_error_response(e).into_response(),
    };

    let items = match load_oms_shipment_items(state.db.pool(), tenant_id, shipment_id).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let response = oms_shipment_summary(uuid::Uuid::from(tenant_id), shipment, Some(items));
    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn ship_oms_shipment(
    State(state): State<AppState>,
    Path((tenant_id, shipment_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OmsShipRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    // Cross-tenant shipment writes recheck the authenticated tenant inside
    // ship_oms_shipment. Blocking the mismatch here would leave that check unproven.
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }
    let shipment_id = match uuid::Uuid::parse_str(&shipment_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shipment_id").into_response(),
    };

    let shipping_enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::ShippingWrite).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !shipping_enabled {
        return feature_flag_forbidden(FeatureFlagKey::ShippingWrite).into_response();
    }

    let inventory_enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::InventoryWrite).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !inventory_enabled {
        return feature_flag_forbidden(FeatureFlagKey::InventoryWrite).into_response();
    }

    let shipped_at = match parse_shipped_at(req.shipped_at.as_deref(), Utc::now()) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let repo = FulfillmentRepo::new(state.db.pool()).with_actor(ctx.fulfillment_actor(&req_ctx));
    let (shipment, shipped_now) = match repo
        .ship_oms_shipment(
            tenant_id,
            shipment_id,
            req.carrier_code.clone(),
            req.tracking_no.clone(),
            shipped_at,
            req.weight_gram,
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return crate::inventory_views::inventory_error_response(e).into_response(),
    };

    if shipped_now {
        let waybills_repo = WaybillsRepo::new(state.db.pool());
        if let Err(err) = waybills_repo
            .mark_used(tenant_id, shipment.shipment_id)
            .await
        {
            tracing::warn!(
                tenant_id = %tenant_id,
                shipment_id = %shipment.shipment_id,
                error = %err,
                "failed to mark waybill used"
            );
        }
    }

    let items =
        match load_oms_shipment_items(state.db.pool(), tenant_id, shipment.shipment_id).await {
            Ok(v) => v,
            Err(resp) => return resp,
        };
    let response = oms_shipment_summary(uuid::Uuid::from(tenant_id), shipment, Some(items));
    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn handle_oms_allocation(
    state: &AppState,
    ctx: &AuthContext,
    req_ctx: &RequestContext,
    tenant_id: &str,
    order_id: &str,
    carrier_code: Option<String>,
) -> Response {
    let tenant_id = match TenantId::from_str(tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(ctx, "orders.write") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(ctx, "shipping.write") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(ctx, "inventory.write") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    let inventory_enabled =
        match feature_flag_enabled(state, tenant_id, None, FeatureFlagKey::InventoryWrite).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !inventory_enabled {
        return feature_flag_forbidden(FeatureFlagKey::InventoryWrite).into_response();
    }

    let repo = FulfillmentRepo::new(state.db.pool()).with_actor(ctx.fulfillment_actor(req_ctx));
    let result = match repo.allocate_order(tenant_id, order_id, carrier_code).await {
        Ok(v) => v,
        Err(e) => return crate::inventory_views::inventory_error_response(e).into_response(),
    };

    let shortages = result
        .shortages
        .into_iter()
        .map(|shortage| OmsAllocationShortageView {
            order_item_id: shortage.order_item_id,
            sku_id: shortage.sku_id,
            requested: shortage.requested,
            available: shortage.available,
        })
        .collect::<Vec<_>>();

    let shipments = match load_shipment_summaries(state, tenant_id, result.shipments).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let response = OmsAllocationResponse {
        status: derive_allocation_status(shortages.len()).to_string(),
        shipments,
        shortages,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn handle_oms_resend(
    state: &AppState,
    ctx: &AuthContext,
    req_ctx: &RequestContext,
    tenant_id: &str,
    order_id: &str,
    carrier_code: Option<String>,
    origin_shipment_id: Option<String>,
) -> Response {
    let tenant_id = match TenantId::from_str(tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(ctx, "orders.write") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(ctx, "shipping.write") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(ctx, "inventory.write") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    let origin_shipment_id = match origin_shipment_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad origin_shipment_id").into_response(),
        },
        None => None,
    };

    let inventory_enabled =
        match feature_flag_enabled(state, tenant_id, None, FeatureFlagKey::InventoryWrite).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !inventory_enabled {
        return feature_flag_forbidden(FeatureFlagKey::InventoryWrite).into_response();
    }

    let repo = FulfillmentRepo::new(state.db.pool()).with_actor(ctx.fulfillment_actor(req_ctx));
    let result = match repo
        .create_resend_shipments(tenant_id, order_id, carrier_code, origin_shipment_id)
        .await
    {
        Ok(v) => v,
        Err(e) => return crate::inventory_views::inventory_error_response(e).into_response(),
    };

    let shortages = result
        .shortages
        .into_iter()
        .map(|shortage| OmsAllocationShortageView {
            order_item_id: shortage.order_item_id,
            sku_id: shortage.sku_id,
            requested: shortage.requested,
            available: shortage.available,
        })
        .collect::<Vec<_>>();

    let shipments = match load_shipment_summaries(state, tenant_id, result.shipments).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let response = OmsAllocationResponse {
        status: derive_allocation_status(shortages.len()).to_string(),
        shipments,
        shortages,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn handle_oms_release(
    state: &AppState,
    ctx: &AuthContext,
    req_ctx: &RequestContext,
    tenant_id: &str,
    order_id: &str,
) -> Response {
    let tenant_id = match TenantId::from_str(tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(ctx, "orders.write") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(ctx, "inventory.write") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    let inventory_enabled =
        match feature_flag_enabled(state, tenant_id, None, FeatureFlagKey::InventoryWrite).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !inventory_enabled {
        return feature_flag_forbidden(FeatureFlagKey::InventoryWrite).into_response();
    }

    let repo = FulfillmentRepo::new(state.db.pool()).with_actor(ctx.fulfillment_actor(req_ctx));
    let result = match repo.release_order_reservations(tenant_id, order_id).await {
        Ok(v) => v,
        Err(e) => return crate::inventory_views::inventory_error_response(e).into_response(),
    };

    let response = OmsReleaseResponse {
        released: result.released,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn handle_oms_cancel_request(
    state: &AppState,
    ctx: &AuthContext,
    req_ctx: &RequestContext,
    tenant_id: &str,
    shipment_id: &str,
    reason: Option<String>,
) -> Response {
    let tenant_id = match TenantId::from_str(tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(ctx, "shipping.write") {
        return resp.into_response();
    }
    let shipment_id = match uuid::Uuid::parse_str(shipment_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shipment_id").into_response(),
    };

    let repo = FulfillmentRepo::new(state.db.pool());
    let cancel_request = match repo
        .request_shipment_cancel(
            tenant_id,
            shipment_id,
            reason.clone(),
            Some(ctx.user_id.into()),
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return crate::inventory_views::inventory_error_response(e).into_response(),
    };

    let log_metadata = serde_json::json!({
        "shipment_id": cancel_request.shipment_id,
        "order_id": cancel_request.order_id,
        "reason": cancel_request.reason,
    });
    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_OMS_SHIPMENT_CANCEL_REQUESTED.to_string(),
            ref_type: Some("OMS_SHIPMENT".to_string()),
            ref_id: Some(cancel_request.shipment_id.to_string()),
            metadata: Some(log_metadata),
        })
        .await;

    record_audit_log(
        state,
        ctx,
        req_ctx,
        "OMS.SHIPMENT.CANCEL_REQUESTED",
        "shipment",
        Some(cancel_request.shipment_id.to_string()),
        None,
        Some(serde_json::json!({
            "shipment_id": cancel_request.shipment_id,
            "order_id": cancel_request.order_id,
            "reason": cancel_request.reason,
            "status": cancel_request.status,
        })),
    )
    .await;

    let response = OmsShipmentCancelResponse {
        cancel_request_id: cancel_request.cancel_request_id,
        status: cancel_request.status,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn get_order_raw(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, "bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read_raw") {
        return resp.into_response();
    }

    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, "bad order_id").into_response(),
    };

    let orders_repo = OrdersRepo::new(state.db.pool());
    let detail = match orders_repo.get_order_detail(tenant_id, order_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return crate::inventory_views::inventory_error_response(e).into_response(),
    };
    if let Err(resp) = ensure_shop_access(&ctx, detail.order.shop_id) {
        return resp.into_response();
    }
    match orders_repo.get_order_raw(tenant_id, order_id).await {
        Ok(Some(raw)) => (StatusCode::OK, Json(raw)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    }
}

pub(super) async fn ship_order(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    headers: HeaderMap,
    Json(req): Json<ShipOrderRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };
    let warehouse_id = match req.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };

    let idempotency_key = headers
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| order_id.to_string());

    let _ = req.print_label;
    let shipment = match handle_shipment_request(
        &state,
        &ctx,
        &req_ctx,
        ShipmentRequestArgs {
            tenant_id,
            order_id,
            warehouse_id,
            carrier_code: req.carrier_code,
            tracking_no: req.tracking_no,
            weight_gram: req.weight_gram,
            serials: req.serials.unwrap_or_default(),
            idempotency_key,
        },
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    (StatusCode::OK, Json(shipment)).into_response()
}

pub(super) async fn allocate_oms_order(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OmsAllocationRequest>,
) -> impl IntoResponse {
    handle_oms_allocation(
        &state,
        &ctx,
        &req_ctx,
        &tenant_id,
        &order_id,
        req.carrier_code,
    )
    .await
}

pub(super) async fn create_oms_shipments(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OmsAllocationRequest>,
) -> impl IntoResponse {
    handle_oms_allocation(
        &state,
        &ctx,
        &req_ctx,
        &tenant_id,
        &order_id,
        req.carrier_code,
    )
    .await
}

pub(super) async fn resend_oms_order(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OmsResendRequest>,
) -> impl IntoResponse {
    handle_oms_resend(
        &state,
        &ctx,
        &req_ctx,
        &tenant_id,
        &order_id,
        req.carrier_code,
        req.origin_shipment_id,
    )
    .await
}

pub(super) async fn release_oms_order(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    handle_oms_release(&state, &ctx, &req_ctx, &tenant_id, &order_id).await
}

pub(super) async fn request_oms_shipment_cancel(
    State(state): State<AppState>,
    Path((tenant_id, shipment_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<OmsShipmentCancelRequest>,
) -> impl IntoResponse {
    handle_oms_cancel_request(&state, &ctx, &req_ctx, &tenant_id, &shipment_id, req.reason).await
}
