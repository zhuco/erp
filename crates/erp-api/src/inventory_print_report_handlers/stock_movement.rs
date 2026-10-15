use super::*;

pub(crate) async fn list_stock_transfers(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<StockTransferListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.read") {
        return resp.into_response();
    }

    let warehouse_id = match query.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = StockTransfersRepo::new(state.db.pool());
    let result = match repo
        .list_transfers(
            tenant_id,
            StockTransferListFilters {
                warehouse_id,
                status: query.status,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = StockTransferListResponse {
        items: result
            .items
            .into_iter()
            .map(stock_transfer_summary_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_stock_transfer(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<StockTransferCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }

    let from_warehouse_id = match uuid::Uuid::parse_str(&req.from_warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad from_warehouse_id").into_response(),
    };
    let to_warehouse_id = match uuid::Uuid::parse_str(&req.to_warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad to_warehouse_id").into_response(),
    };

    let mut items = Vec::with_capacity(req.items.len());
    for item in req.items {
        let sku_id = match uuid::Uuid::parse_str(&item.sku_id) {
            Ok(v) => v,
            Err(_) => return bad_request("bad sku_id").into_response(),
        };
        items.push(StockTransferItemInput {
            sku_id,
            qty: item.qty,
        });
    }

    let repo = StockTransfersRepo::new(state.db.pool());
    let transfer = match repo
        .create_transfer(StockTransferCreateInput {
            tenant_id,
            from_warehouse_id,
            to_warehouse_id,
            items,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let log_payload = serde_json::json!({
        "transfer_id": transfer.transfer_id,
        "from_warehouse_id": transfer.from_warehouse_id,
        "to_warehouse_id": transfer.to_warehouse_id,
        "status": transfer.status,
    });
    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_STOCK_TRANSFER_CREATED.to_string(),
            ref_type: Some("STOCK_TRANSFER".to_string()),
            ref_id: Some(transfer.transfer_id.to_string()),
            metadata: Some(log_payload.clone()),
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
            action: "WMS.STOCK_TRANSFER.CREATE".to_string(),
            entity_type: "STOCK_TRANSFER".to_string(),
            entity_id: Some(transfer.transfer_id.to_string()),
            before_json: None,
            after_json: Some(log_payload),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let detail = match repo.get_detail(tenant_id, transfer.transfer_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("transfer not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(stock_transfer_detail_response(detail))).into_response()
}

pub(crate) async fn get_stock_transfer(
    State(state): State<AppState>,
    Path((tenant_id, transfer_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.read") {
        return resp.into_response();
    }
    let transfer_id = match uuid::Uuid::parse_str(&transfer_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad transfer_id").into_response(),
    };

    let repo = StockTransfersRepo::new(state.db.pool());
    let detail = match repo.get_detail(tenant_id, transfer_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("transfer not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    (StatusCode::OK, Json(stock_transfer_detail_response(detail))).into_response()
}

pub(crate) async fn confirm_stock_transfer_outbound(
    State(state): State<AppState>,
    Path((tenant_id, transfer_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }
    let transfer_id = match uuid::Uuid::parse_str(&transfer_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad transfer_id").into_response(),
    };

    let repo = StockTransfersRepo::new(state.db.pool());
    let transfer = match repo.confirm_outbound(tenant_id, transfer_id).await {
        Ok(v) => v,
        Err(e) => return inventory_error_response(e).into_response(),
    };

    let log_payload = serde_json::json!({
        "transfer_id": transfer.transfer_id,
        "status": transfer.status,
    });
    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_STOCK_TRANSFER_OUT.to_string(),
            ref_type: Some("STOCK_TRANSFER".to_string()),
            ref_id: Some(transfer.transfer_id.to_string()),
            metadata: Some(log_payload.clone()),
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
            action: "WMS.STOCK_TRANSFER.OUT_CONFIRMED".to_string(),
            entity_type: "STOCK_TRANSFER".to_string(),
            entity_id: Some(transfer.transfer_id.to_string()),
            before_json: None,
            after_json: Some(log_payload),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let detail = match repo.get_detail(tenant_id, transfer.transfer_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("transfer not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    (StatusCode::OK, Json(stock_transfer_detail_response(detail))).into_response()
}

pub(crate) async fn confirm_stock_transfer_inbound(
    State(state): State<AppState>,
    Path((tenant_id, transfer_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }
    let transfer_id = match uuid::Uuid::parse_str(&transfer_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad transfer_id").into_response(),
    };

    let repo = StockTransfersRepo::new(state.db.pool());
    let transfer = match repo.confirm_inbound(tenant_id, transfer_id).await {
        Ok(v) => v,
        Err(e) => return inventory_error_response(e).into_response(),
    };

    let log_payload = serde_json::json!({
        "transfer_id": transfer.transfer_id,
        "status": transfer.status,
    });
    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_STOCK_TRANSFER_IN.to_string(),
            ref_type: Some("STOCK_TRANSFER".to_string()),
            ref_id: Some(transfer.transfer_id.to_string()),
            metadata: Some(log_payload.clone()),
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
            action: "WMS.STOCK_TRANSFER.IN_CONFIRMED".to_string(),
            entity_type: "STOCK_TRANSFER".to_string(),
            entity_id: Some(transfer.transfer_id.to_string()),
            before_json: None,
            after_json: Some(log_payload),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let detail = match repo.get_detail(tenant_id, transfer.transfer_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("transfer not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    (StatusCode::OK, Json(stock_transfer_detail_response(detail))).into_response()
}

pub(crate) async fn list_inventory_freezes(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<InventoryFreezeListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.read") {
        return resp.into_response();
    }

    let warehouse_id = match query.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    let sku_id = match query.sku_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad sku_id").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = InventoryFreezesRepo::new(state.db.pool());
    let result = match repo
        .list_freezes(
            tenant_id,
            InventoryFreezeListFilters {
                warehouse_id,
                sku_id,
                status: query.status,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = InventoryFreezeListResponse {
        items: result
            .items
            .into_iter()
            .map(inventory_freeze_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_inventory_freeze(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<InventoryFreezeCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }

    if req.qty <= 0 {
        return bad_request("qty must be positive").into_response();
    }

    let warehouse_id = match uuid::Uuid::parse_str(&req.warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(&req.sku_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };

    let repo = InventoryFreezesRepo::new(state.db.pool());
    let freeze = match repo
        .create_freeze(InventoryFreezeCreateInput {
            tenant_id,
            warehouse_id,
            sku_id,
            qty: req.qty,
            reason: req.reason.clone(),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return inventory_error_response(e).into_response(),
    };

    let log_payload = serde_json::json!({
        "freeze_id": freeze.freeze_id,
        "warehouse_id": warehouse_id,
        "sku_id": sku_id,
        "qty": freeze.qty,
        "status": freeze.status,
    });
    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_INVENTORY_FREEZE.to_string(),
            ref_type: Some("INVENTORY_FREEZE".to_string()),
            ref_id: Some(freeze.freeze_id.to_string()),
            metadata: Some(log_payload.clone()),
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
            action: "INVENTORY.FREEZE".to_string(),
            entity_type: "INVENTORY_FREEZE".to_string(),
            entity_id: Some(freeze.freeze_id.to_string()),
            before_json: None,
            after_json: Some(log_payload),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    (StatusCode::OK, Json(inventory_freeze_response(freeze))).into_response()
}

pub(crate) async fn cancel_inventory_freeze(
    State(state): State<AppState>,
    Path((tenant_id, freeze_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }
    let freeze_id = match uuid::Uuid::parse_str(&freeze_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad freeze_id").into_response(),
    };

    let repo = InventoryFreezesRepo::new(state.db.pool());
    let freeze = match repo.cancel_freeze(tenant_id, freeze_id).await {
        Ok(v) => v,
        Err(e) => return inventory_error_response(e).into_response(),
    };

    let log_payload = serde_json::json!({
        "freeze_id": freeze.freeze_id,
        "warehouse_id": freeze.warehouse_id,
        "sku_id": freeze.sku_id,
        "qty": freeze.qty,
        "status": freeze.status,
    });
    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_INVENTORY_UNFREEZE.to_string(),
            ref_type: Some("INVENTORY_FREEZE".to_string()),
            ref_id: Some(freeze.freeze_id.to_string()),
            metadata: Some(log_payload.clone()),
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
            action: "INVENTORY.UNFREEZE".to_string(),
            entity_type: "INVENTORY_FREEZE".to_string(),
            entity_id: Some(freeze.freeze_id.to_string()),
            before_json: None,
            after_json: Some(log_payload),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    (StatusCode::OK, Json(inventory_freeze_response(freeze))).into_response()
}

pub(crate) async fn list_stock_adjustments(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<StockAdjustListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.read") {
        return resp.into_response();
    }

    let warehouse_id = match query.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = StockAdjustmentsRepo::new(state.db.pool());
    let result = match repo
        .list_adjustments(
            tenant_id,
            StockAdjustListFilters {
                warehouse_id,
                status: query.status,
                reason: query.reason,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = StockAdjustListResponse {
        items: result
            .items
            .into_iter()
            .map(stock_adjust_summary_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_stock_adjustment(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<StockAdjustCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }

    let warehouse_id = match uuid::Uuid::parse_str(&req.warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, req.request_id.as_deref()) {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    if let Some(request_id) = request_id {
        let location_id = match req
            .location_id
            .as_deref()
            .and_then(|value| uuid::Uuid::parse_str(value).ok())
        {
            Some(value) => value,
            None => return bad_request("bad location_id").into_response(),
        };
        let sku_id = match req
            .sku_id
            .as_deref()
            .and_then(|value| uuid::Uuid::parse_str(value).ok())
        {
            Some(value) => value,
            None => return bad_request("bad sku_id").into_response(),
        };
        let batch_code = req
            .batch_code
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let cutoff_at = req
            .cutoff_at
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let (Some(batch_code), Some(cutoff_at), Some(counted_qty)) =
            (batch_code, cutoff_at, req.counted_qty)
        else {
            return bad_request("batch, cutoff, and counted quantity are required").into_response();
        };
        let saved = SharedContractRepo::new(state.db.pool())
            .confirm_count_adjustment(
                tenant_id,
                &request_id,
                &request_id,
                warehouse_id,
                location_id,
                sku_id,
                batch_code,
                cutoff_at,
                counted_qty,
            )
            .await;
        let (write, delta) = match saved {
            Ok(Ok(value)) => value,
            Ok(Err(_)) => return conflict("payload conflict").into_response(),
            Err(error) => return (StatusCode::CONFLICT, format!("{error}")).into_response(),
        };
        let (receipt, replay) = match write {
            SharedWrite::Stored(receipt) => (receipt, false),
            SharedWrite::Replay(receipt) => (receipt, true),
        };
        return (
            StatusCode::OK,
            Json(serde_json::json!({
                "request_id": receipt.request_id,
                "status": receipt.status.as_str(),
                "replay": replay,
                "adjustment_micros": delta,
                "counted_qty": counted_qty,
            })),
        )
            .into_response();
    }
    let mut items = Vec::with_capacity(req.items.len());
    for item in req.items {
        let sku_id = match uuid::Uuid::parse_str(&item.sku_id) {
            Ok(v) => v,
            Err(_) => return bad_request("bad sku_id").into_response(),
        };
        items.push(StockAdjustItemInput {
            sku_id,
            qty: item.qty,
        });
    }

    let repo = StockAdjustmentsRepo::new(state.db.pool());
    let doc = match repo
        .create_adjustment(StockAdjustCreateInput {
            tenant_id,
            warehouse_id,
            reason: req.reason.clone(),
            items,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let log_payload = serde_json::json!({
        "adjust_id": doc.adjust_id,
        "warehouse_id": warehouse_id,
        "reason": doc.reason,
        "status": doc.status,
    });
    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_STOCK_ADJUST_CREATED.to_string(),
            ref_type: Some("STOCK_ADJUST".to_string()),
            ref_id: Some(doc.adjust_id.to_string()),
            metadata: Some(log_payload.clone()),
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
            action: "WMS.STOCK_ADJUST.CREATE".to_string(),
            entity_type: "STOCK_ADJUST".to_string(),
            entity_id: Some(doc.adjust_id.to_string()),
            before_json: None,
            after_json: Some(log_payload),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let detail = match repo.get_detail(tenant_id, doc.adjust_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("adjust doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    (StatusCode::OK, Json(stock_adjust_detail_response(detail))).into_response()
}

pub(crate) async fn get_stock_adjustment(
    State(state): State<AppState>,
    Path((tenant_id, adjust_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.read") {
        return resp.into_response();
    }
    let adjust_id = match uuid::Uuid::parse_str(&adjust_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad adjust_id").into_response(),
    };

    let repo = StockAdjustmentsRepo::new(state.db.pool());
    let detail = match repo.get_detail(tenant_id, adjust_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("adjust doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    (StatusCode::OK, Json(stock_adjust_detail_response(detail))).into_response()
}

pub(crate) async fn apply_stock_adjustment(
    State(state): State<AppState>,
    Path((tenant_id, adjust_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }
    let adjust_id = match uuid::Uuid::parse_str(&adjust_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad adjust_id").into_response(),
    };

    let repo = StockAdjustmentsRepo::new(state.db.pool());
    let doc = match repo.apply_adjustment(tenant_id, adjust_id).await {
        Ok(v) => v,
        Err(e) => return inventory_error_response(e).into_response(),
    };

    let log_payload = serde_json::json!({
        "adjust_id": doc.adjust_id,
        "status": doc.status,
        "reason": doc.reason,
    });
    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_STOCK_ADJUST_APPLIED.to_string(),
            ref_type: Some("STOCK_ADJUST".to_string()),
            ref_id: Some(doc.adjust_id.to_string()),
            metadata: Some(log_payload.clone()),
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
            action: "WMS.STOCK_ADJUST.APPLY".to_string(),
            entity_type: "STOCK_ADJUST".to_string(),
            entity_id: Some(doc.adjust_id.to_string()),
            before_json: None,
            after_json: Some(log_payload),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let detail = match repo.get_detail(tenant_id, adjust_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("adjust doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    (StatusCode::OK, Json(stock_adjust_detail_response(detail))).into_response()
}

pub(crate) async fn list_stock_moves(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<StockMoveListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.read") {
        return resp.into_response();
    }

    let warehouse_id = match query.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = StockMovesRepo::new(state.db.pool());
    let result = match repo
        .list_moves(
            tenant_id,
            StockMoveListFilters {
                warehouse_id,
                status: query.status,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = StockMoveListResponse {
        items: result
            .items
            .into_iter()
            .map(stock_move_summary_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_stock_move(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<StockMoveCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }

    let warehouse_id = match uuid::Uuid::parse_str(&req.warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    let from_location_id = match req.from_location_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad from_location_id").into_response(),
        },
        None => None,
    };
    let to_location_id = match req.to_location_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad to_location_id").into_response(),
        },
        None => None,
    };

    let mut items = Vec::with_capacity(req.items.len());
    for item in req.items {
        let sku_id = match uuid::Uuid::parse_str(&item.sku_id) {
            Ok(v) => v,
            Err(_) => return bad_request("bad sku_id").into_response(),
        };
        items.push(StockMoveItemInput {
            sku_id,
            qty: item.qty,
        });
    }

    let repo = StockMovesRepo::new(state.db.pool());
    let doc = match repo
        .create_move(StockMoveCreateInput {
            tenant_id,
            warehouse_id,
            from_location_id,
            to_location_id,
            items,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let log_payload = serde_json::json!({
        "move_id": doc.move_id,
        "warehouse_id": doc.warehouse_id,
        "from_location_id": doc.from_location_id,
        "to_location_id": doc.to_location_id,
        "status": doc.status,
    });
    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_STOCK_MOVE_CREATED.to_string(),
            ref_type: Some("STOCK_MOVE".to_string()),
            ref_id: Some(doc.move_id.to_string()),
            metadata: Some(log_payload.clone()),
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
            action: "WMS.STOCK_MOVE.CREATE".to_string(),
            entity_type: "STOCK_MOVE".to_string(),
            entity_id: Some(doc.move_id.to_string()),
            before_json: None,
            after_json: Some(log_payload),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let detail = match repo.get_detail(tenant_id, doc.move_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("move doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    (StatusCode::OK, Json(stock_move_detail_response(detail))).into_response()
}

pub(crate) async fn get_stock_move(
    State(state): State<AppState>,
    Path((tenant_id, move_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.read") {
        return resp.into_response();
    }
    let move_id = match uuid::Uuid::parse_str(&move_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad move_id").into_response(),
    };

    let repo = StockMovesRepo::new(state.db.pool());
    let detail = match repo.get_detail(tenant_id, move_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("move doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    (StatusCode::OK, Json(stock_move_detail_response(detail))).into_response()
}

pub(crate) async fn apply_stock_move(
    State(state): State<AppState>,
    Path((tenant_id, move_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }
    let move_id = match uuid::Uuid::parse_str(&move_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad move_id").into_response(),
    };

    let repo = StockMovesRepo::new(state.db.pool());
    let doc = match repo.apply_move(tenant_id, move_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let log_payload = serde_json::json!({
        "move_id": doc.move_id,
        "status": doc.status,
    });
    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_STOCK_MOVE_APPLIED.to_string(),
            ref_type: Some("STOCK_MOVE".to_string()),
            ref_id: Some(doc.move_id.to_string()),
            metadata: Some(log_payload.clone()),
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
            action: "WMS.STOCK_MOVE.APPLY".to_string(),
            entity_type: "STOCK_MOVE".to_string(),
            entity_id: Some(doc.move_id.to_string()),
            before_json: None,
            after_json: Some(log_payload),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let detail = match repo.get_detail(tenant_id, move_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("move doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    (StatusCode::OK, Json(stock_move_detail_response(detail))).into_response()
}

pub(crate) async fn inbound_inventory(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<InventoryMovementRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if req.quantity <= 0 {
        return bad_request("quantity must be positive").into_response();
    }
    let warehouse_id = match uuid::Uuid::parse_str(&req.warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(&req.sku_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };
    let batch_id = match req.batch_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) if !v.is_nil() => Some(v),
            _ => return bad_request("bad batch_id").into_response(),
        },
        None => None,
    };

    let request_id = match shared_request_id(&headers, req.request_id.as_deref()) {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    if let Some(request_id) = request_id {
        if batch_id.is_some()
            && (req.location_id.is_some()
                || req.batch_code.is_some()
                || req.quality_status.is_some())
        {
            return bad_request("batch UUID cannot be combined with location batch code")
                .into_response();
        }
        if req.location_id.is_some() || req.batch_code.is_some() || req.quality_status.is_some() {
            return shared_stock_ledger_inbound(
                &state,
                &ctx,
                tenant_id,
                warehouse_id,
                sku_id,
                &req,
                &request_id,
            )
            .await;
        }
        return execute_inventory_command(
            &state,
            &ctx,
            erp_repo::inventory_movement_commands::InventoryMovementCommandInput {
                tenant_id,
                actor_user_id: ctx.user_id,
                session_id: ctx.session_id,
                request_id,
                kind: erp_repo::inventory_movement_commands::InventoryMovementKind::Inbound,
                warehouse_id,
                sku_id,
                batch_id,
                quantity_delta: req.quantity,
                reason: req.reason.clone(),
                reference_id: req.reference_id.clone(),
                location: None,
                reservation_source: None,
            },
        )
        .await;
    }

    if req.location_id.is_some() || req.batch_code.is_some() || req.quality_status.is_some() {
        return bad_request("structured inbound requires an original request_id").into_response();
    }

    let balance = match apply_inventory_movement(
        &state,
        &ctx,
        tenant_id,
        warehouse_id,
        sku_id,
        batch_id,
        req.quantity,
        req.reason.clone(),
        req.reference_id.clone(),
        "PURCHASE_RECEIPT",
        OP_ACTION_INVENTORY_INBOUND,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let response = InventoryMutationResponse {
        record: inventory_balance_response(balance),
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) fn shared_request_id(
    headers: &HeaderMap,
    body_request_id: Option<&str>,
) -> Result<Option<String>, Response> {
    if headers.get_all("Idempotency-Key").iter().count() > 1 {
        return Err(bad_request("multiple Idempotency-Key headers").into_response());
    }
    let validate = |value: &str| {
        erp_repo::inventory_movement_commands::normalize_inventory_request_id(value)
            .map_err(inventory_command_error)
    };
    let header = headers
        .get("Idempotency-Key")
        .map(|value| {
            value
                .to_str()
                .map_err(|_| bad_request("invalid Idempotency-Key").into_response())
                .and_then(validate)
        })
        .transpose()?;
    let body = body_request_id.map(validate).transpose()?;
    match (header, body) {
        (Some(header), Some(body)) if header != body => {
            Err(conflict("Idempotency-Key disagrees with request_id").into_response())
        }
        (Some(header), _) => Ok(Some(header)),
        (None, Some(body)) => Ok(Some(body)),
        (None, None) => Ok(None),
    }
}

async fn shared_stock_ledger_inbound(
    state: &AppState,
    ctx: &AuthContext,
    tenant_id: TenantId,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    req: &InventoryMovementRequest,
    request_id: &str,
) -> Response {
    let location = match inventory_movement_location(req) {
        Ok(location) => location,
        Err(response) => return response,
    };
    execute_inventory_command(
        state,
        ctx,
        erp_repo::inventory_movement_commands::InventoryMovementCommandInput {
            tenant_id,
            actor_user_id: ctx.user_id,
            session_id: ctx.session_id,
            request_id: request_id.to_owned(),
            kind: erp_repo::inventory_movement_commands::InventoryMovementKind::Inbound,
            warehouse_id,
            sku_id,
            batch_id: None,
            quantity_delta: req.quantity,
            reason: req.reason.clone(),
            reference_id: req.reference_id.clone(),
            location: Some(location),
            reservation_source: None,
        },
    )
    .await
}

fn inventory_movement_location(
    req: &InventoryMovementRequest,
) -> Result<erp_repo::inventory_movement_commands::InventoryInboundLocation, Response> {
    let location_id = match req
        .location_id
        .as_deref()
        .and_then(|value| uuid::Uuid::parse_str(value).ok())
    {
        Some(value) => value,
        None => return Err(bad_request("bad location_id").into_response()),
    };
    let batch_code = req
        .batch_code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let quality_status = req
        .quality_status
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let (Some(batch_code), Some(quality_status)) = (batch_code, quality_status) else {
        return Err(bad_request("batch and quality status are required").into_response());
    };
    Ok(
        erp_repo::inventory_movement_commands::InventoryInboundLocation {
            location_id,
            batch_code: batch_code.to_owned(),
            quality_status: quality_status.to_owned(),
        },
    )
}

pub(crate) async fn outbound_inventory(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<InventoryMovementRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if req.quantity <= 0 {
        return bad_request("quantity must be positive").into_response();
    }
    let warehouse_id = match uuid::Uuid::parse_str(&req.warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(&req.sku_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };
    let batch_id = match req.batch_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) if !v.is_nil() => Some(v),
            _ => return bad_request("bad batch_id").into_response(),
        },
        None => None,
    };

    let request_id = match shared_request_id(&headers, req.request_id.as_deref()) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if let Some(request_id) = request_id {
        let location = if req.location_id.is_some()
            || req.batch_code.is_some()
            || req.quality_status.is_some()
        {
            match inventory_movement_location(&req) {
                Ok(location) => Some(location),
                Err(response) => return response,
            }
        } else {
            None
        };
        return execute_inventory_command(
            &state,
            &ctx,
            erp_repo::inventory_movement_commands::InventoryMovementCommandInput {
                tenant_id,
                actor_user_id: ctx.user_id,
                session_id: ctx.session_id,
                request_id,
                kind: erp_repo::inventory_movement_commands::InventoryMovementKind::Outbound,
                warehouse_id,
                sku_id,
                batch_id,
                quantity_delta: -req.quantity,
                reason: req.reason.clone(),
                reference_id: req.reference_id.clone(),
                location,
                reservation_source: None,
            },
        )
        .await;
    }
    if req.location_id.is_some() || req.batch_code.is_some() || req.quality_status.is_some() {
        return bad_request("structured outbound requires an original request_id").into_response();
    }
    let balance = match apply_inventory_movement(
        &state,
        &ctx,
        tenant_id,
        warehouse_id,
        sku_id,
        batch_id,
        -req.quantity,
        req.reason.clone(),
        req.reference_id.clone(),
        "TRANSFER_OUT",
        OP_ACTION_INVENTORY_OUTBOUND,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let response = InventoryMutationResponse {
        record: inventory_balance_response(balance),
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn reserve_inventory(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<InventoryReservationRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }

    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::InventoryWrite).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        warn!(
            tenant_id = %tenant_id,
            "feature disabled: inventory write"
        );
        return feature_flag_forbidden(FeatureFlagKey::InventoryWrite).into_response();
    }

    if req.quantity <= 0 || req.quantity.checked_mul(1_000_000).is_none() {
        return bad_request("quantity must be positive and within exact range").into_response();
    }

    let warehouse_id = match uuid::Uuid::parse_str(&req.warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(&req.sku_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };
    let order_id = match req.order_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) if !id.is_nil() => Some(id),
            _ => return bad_request("bad order_id").into_response(),
        },
        None => None,
    };
    let request_id = match shared_request_id(&headers, req.request_id.as_deref()) {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    if let Some(request_id) = request_id {
        return execute_source_inventory_command(
            &state,
            &ctx,
            tenant_id,
            erp_repo::inventory_movement_commands::InventoryMovementKind::Reserve,
            request_id,
            InventorySourceCommandRequest {
                warehouse_id,
                sku_id,
                quantity: req.quantity,
                order_id,
                order_item_id: req.order_item_id.as_deref(),
                reference_id: req.reference_id.as_deref(),
                location_id: req.location_id.as_deref(),
                batch_code: req.batch_code.as_deref(),
                quality_status: req.quality_status.as_deref(),
            },
        )
        .await;
    }
    if req.location_id.is_some() || req.batch_code.is_some() || req.quality_status.is_some() {
        return bad_request("structured reservation requires an original request_id")
            .into_response();
    }
    if let Err(response) =
        inventory_command_warehouse_access(&state, &ctx, tenant_id, warehouse_id).await
    {
        return response;
    }
    let order_item_id = match legacy_reservation_source_id(
        req.order_item_id.as_deref(),
        req.reference_id.as_deref(),
    ) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let balance = match state
        .inventory_repo
        .reserve_for_order_item(
            InventoryReservationInput {
                tenant_id,
                warehouse_id,
                sku_id,
                order_id,
                order_item_id,
                qty: req.quantity,
            },
            ctx.user_id,
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return inventory_error_response(e).into_response(),
    };

    let response = InventoryMutationResponse {
        record: inventory_balance_response(balance),
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn release_inventory(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<InventoryReleaseRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }

    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::InventoryWrite).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        warn!(
            tenant_id = %tenant_id,
            "feature disabled: inventory write"
        );
        return feature_flag_forbidden(FeatureFlagKey::InventoryWrite).into_response();
    }

    if req.quantity <= 0 || req.quantity.checked_mul(1_000_000).is_none() {
        return bad_request("quantity must be positive and within exact range").into_response();
    }

    let warehouse_id = match uuid::Uuid::parse_str(&req.warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    let sku_id = match uuid::Uuid::parse_str(&req.sku_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };
    let request_id = match shared_request_id(&headers, req.request_id.as_deref()) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if let Some(request_id) = request_id {
        let order_id = match req
            .order_id
            .as_deref()
            .map(uuid::Uuid::parse_str)
            .transpose()
        {
            Ok(value) => value,
            Err(_) => return bad_request("bad order_id").into_response(),
        };
        return execute_source_inventory_command(
            &state,
            &ctx,
            tenant_id,
            erp_repo::inventory_movement_commands::InventoryMovementKind::Release,
            request_id,
            InventorySourceCommandRequest {
                warehouse_id,
                sku_id,
                quantity: req.quantity,
                order_id,
                order_item_id: req.order_item_id.as_deref(),
                reference_id: req.reference_id.as_deref(),
                location_id: req.location_id.as_deref(),
                batch_code: req.batch_code.as_deref(),
                quality_status: req.quality_status.as_deref(),
            },
        )
        .await;
    }
    if req.location_id.is_some()
        || req.batch_code.is_some()
        || req.quality_status.is_some()
        || req.order_id.is_some()
    {
        return bad_request("structured release requires an original request_id").into_response();
    }
    if let Err(response) =
        inventory_command_warehouse_access(&state, &ctx, tenant_id, warehouse_id).await
    {
        return response;
    }
    let order_item_id = match legacy_reservation_source_id(
        req.order_item_id.as_deref(),
        req.reference_id.as_deref(),
    ) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let balance = match state
        .inventory_repo
        .release_reservation(
            InventoryReservationAction {
                tenant_id,
                warehouse_id,
                sku_id,
                order_item_id,
                qty: req.quantity,
            },
            ctx.user_id,
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return inventory_error_response(e).into_response(),
    };

    let response = InventoryMutationResponse {
        record: inventory_balance_response(balance),
    };
    (StatusCode::OK, Json(response)).into_response()
}

fn legacy_reservation_source_id(
    order_item: Option<&str>,
    reference: Option<&str>,
) -> Result<uuid::Uuid, Response> {
    let parse = |value: &str| {
        uuid::Uuid::parse_str(value.trim())
            .ok()
            .filter(|id| !id.is_nil())
            .ok_or_else(|| bad_request("bad reservation source UUID").into_response())
    };
    let order_item = order_item.map(parse).transpose()?;
    let reference = reference.map(parse).transpose()?;
    match (order_item, reference) {
        (Some(a), Some(b)) if a != b => {
            Err(bad_request("source aliases must agree").into_response())
        }
        (Some(id), _) | (_, Some(id)) => Ok(id),
        (None, None) => Err(bad_request("missing order_item_id").into_response()),
    }
}

struct InventorySourceCommandRequest<'a> {
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    quantity: i64,
    order_id: Option<uuid::Uuid>,
    order_item_id: Option<&'a str>,
    reference_id: Option<&'a str>,
    location_id: Option<&'a str>,
    batch_code: Option<&'a str>,
    quality_status: Option<&'a str>,
}

async fn execute_source_inventory_command(
    state: &AppState,
    ctx: &AuthContext,
    tenant_id: TenantId,
    kind: erp_repo::inventory_movement_commands::InventoryMovementKind,
    request_id: String,
    req: InventorySourceCommandRequest<'_>,
) -> Response {
    use erp_repo::{
        inventory_location_reservations::InventoryReservationSource,
        inventory_movement_commands::{
            InventoryInboundLocation, InventoryMovementCommandInput, InventoryMovementKind,
        },
    };
    let aliases = (
        req.order_item_id.map(str::trim),
        req.reference_id.map(str::trim),
    );
    if aliases.0.is_some_and(str::is_empty)
        || aliases.1.is_some_and(str::is_empty)
        || matches!(aliases,(Some(a),Some(b)) if a!=b)
    {
        return bad_request("source aliases must be nonempty and agree").into_response();
    }
    let Some(source_line) = aliases.0.or(aliases.1) else {
        return bad_request("missing reservation source").into_response();
    };
    let Some(location_id) = req.location_id.and_then(|v| uuid::Uuid::parse_str(v).ok()) else {
        return bad_request("bad location_id").into_response();
    };
    let (Some(batch_code), Some(quality_status)) = (req.batch_code, req.quality_status) else {
        return bad_request("batch and quality status are required").into_response();
    };
    execute_inventory_command(
        state,
        ctx,
        InventoryMovementCommandInput {
            tenant_id,
            actor_user_id: ctx.user_id,
            session_id: ctx.session_id,
            request_id,
            kind,
            warehouse_id: req.warehouse_id,
            sku_id: req.sku_id,
            batch_id: None,
            quantity_delta: if kind == InventoryMovementKind::Release {
                -req.quantity
            } else {
                req.quantity
            },
            reason: None,
            reference_id: req.reference_id.map(str::to_owned),
            location: Some(InventoryInboundLocation {
                location_id,
                batch_code: batch_code.to_owned(),
                quality_status: quality_status.to_owned(),
            }),
            reservation_source: Some(InventoryReservationSource {
                source_line: source_line.to_owned(),
                order_id: req.order_id,
            }),
        },
    )
    .await
}

pub(crate) async fn list_inventory_sync_jobs(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<StockSyncJobListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.read") {
        return resp.into_response();
    }

    let shop_id = match query.shop_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(ShopId(v)),
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };
    let sync_enabled =
        match feature_flag_enabled(&state, tenant_id, shop_id, FeatureFlagKey::InventorySync).await
        {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !sync_enabled {
        return feature_flag_forbidden(FeatureFlagKey::InventorySync).into_response();
    }
    let sku_id = match query.sku_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad sku_id").into_response(),
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
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = StockSyncJobsRepo::new(state.db.pool());
    let result = match repo
        .list_jobs(
            tenant_id,
            StockSyncJobListFilters {
                shop_id,
                sku_id,
                warehouse_id,
                status: query.status,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = StockSyncJobListResponse {
        items: result
            .items
            .into_iter()
            .map(stock_sync_job_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn retry_inventory_sync_job(
    State(state): State<AppState>,
    Path((tenant_id, job_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let job_id = match uuid::Uuid::parse_str(&job_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad job_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }
    let sync_enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::InventorySync).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !sync_enabled {
        return feature_flag_forbidden(FeatureFlagKey::InventorySync).into_response();
    }

    let repo = StockSyncJobsRepo::new(state.db.pool());
    let job = match repo.retry_failed_job(tenant_id, job_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("sync job not retryable").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let log_metadata = serde_json::json!({
        "job_id": job.job_id,
        "shop_id": job.shop_id,
        "sku_id": job.sku_id,
        "warehouse_id": job.warehouse_id,
        "target_qty": job.target_qty,
    });
    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_INVENTORY_SYNC_RETRY.to_string(),
            ref_type: Some("STOCK_SYNC_JOB".to_string()),
            ref_id: Some(job.job_id.to_string()),
            metadata: Some(log_metadata.clone()),
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
            action: "INVENTORY.SYNC_RETRY".to_string(),
            entity_type: "STOCK_SYNC_JOB".to_string(),
            entity_id: Some(job.job_id.to_string()),
            before_json: None,
            after_json: Some(log_metadata),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    (StatusCode::OK, Json(stock_sync_job_response(job))).into_response()
}

pub(crate) async fn run_inventory_sync_job(
    State(state): State<AppState>,
    Path((tenant_id, job_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let job_id = match uuid::Uuid::parse_str(&job_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad job_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }
    let sync_enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::InventorySync).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !sync_enabled {
        return feature_flag_forbidden(FeatureFlagKey::InventorySync).into_response();
    }

    let repo = StockSyncJobsRepo::new(state.db.pool());
    let job = match repo.request_job_run(tenant_id, job_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("sync job not runnable").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let log_metadata = serde_json::json!({
        "job_id": job.job_id,
        "shop_id": job.shop_id,
        "sku_id": job.sku_id,
        "warehouse_id": job.warehouse_id,
        "target_qty": job.target_qty,
    });
    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_INVENTORY_SYNC_RUN.to_string(),
            ref_type: Some("STOCK_SYNC_JOB".to_string()),
            ref_id: Some(job.job_id.to_string()),
            metadata: Some(log_metadata.clone()),
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
            action: "INVENTORY.SYNC_RUN".to_string(),
            entity_type: "STOCK_SYNC_JOB".to_string(),
            entity_id: Some(job.job_id.to_string()),
            before_json: None,
            after_json: Some(log_metadata),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    (StatusCode::OK, Json(stock_sync_job_response(job))).into_response()
}
