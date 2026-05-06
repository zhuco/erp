use super::*;

pub(crate) async fn list_inventory(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<InventoryListQuery>,
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

    let sku_id = match query.sku_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad sku_id").into_response(),
        },
        None => None,
    };
    let warehouse_id = match query.warehouse_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    let tag = query.tag.and_then(non_empty);
    let on_hand_negative = query.on_hand_negative.unwrap_or(false);
    let available_below = if query.available_below_safety.unwrap_or(false) {
        match fetch_tenant_setting_value(state.db.pool(), tenant_id, "inventory.safety_stock").await
        {
            Ok(value) => value.as_ref().and_then(parse_numeric_setting).or(Some(0)),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }
    } else {
        None
    };
    if let Some(warehouse_id) = warehouse_id {
        if let Err(resp) = ensure_warehouse_access(&ctx, warehouse_id) {
            return resp.into_response();
        }
    }
    if let Some(warehouse_id) = warehouse_id {
        if let Err(resp) = ensure_warehouse_access(&ctx, warehouse_id) {
            return resp.into_response();
        }
    }
    let cursor = match query.cursor.as_deref() {
        Some(raw) => match InventoryCursor::decode(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad cursor").into_response(),
        },
        None => None,
    };
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let result = match state
        .inventory_repo
        .list_inventory(
            tenant_id,
            InventoryListFilters {
                warehouse_id,
                warehouse_ids: warehouse_scope_filter(&ctx),
                sku_id,
                sku_code: query.sku_code,
                barcode: query.barcode,
                tag,
                on_hand_negative,
                available_below,
                cursor,
                offset: Some(offset),
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = ListResponse {
        data: result
            .items
            .into_iter()
            .map(inventory_record_response)
            .collect(),
        next_cursor: result.next_cursor.map(|c| c.encode()),
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn get_inventory_atp(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<InventoryAtpQuery>,
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

    let warehouse_id = match uuid::Uuid::parse_str(&query.warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    if let Err(resp) = ensure_warehouse_access(&ctx, warehouse_id) {
        return resp.into_response();
    }
    let sku_id = match uuid::Uuid::parse_str(&query.sku_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };

    let request_qty = query.request_qty.unwrap_or(0).max(0);
    let repo = AtpInventoryRepo::new(state.db.pool());
    let quote = match repo
        .quote_fefo(tenant_id, warehouse_id, sku_id, request_qty)
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(inventory_atp_response(quote))).into_response()
}

pub(crate) async fn check_inventory_reconciliation(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<InventoryReconciliationCheckRequest>,
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
    if let Err(resp) = ensure_warehouse_access(&ctx, warehouse_id) {
        return resp.into_response();
    }
    let sku_id = match uuid::Uuid::parse_str(&req.sku_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };

    let repo = InventoryReconciliationCenterRepo::new(state.db.pool());
    let record = match repo
        .check_and_record(InventoryReconciliationCheckInput {
            tenant_id,
            warehouse_id,
            sku_id,
            expected_on_hand: req.expected_on_hand,
            source_ref: req.source_ref.and_then(non_empty),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(inventory_reconciliation_response(record)),
    )
        .into_response()
}

pub(crate) async fn list_inventory_summary(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<InventorySummaryListQuery>,
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

    let sku_id = match query.sku_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad sku_id").into_response(),
        },
        None => None,
    };
    let warehouse_id = match query.warehouse_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    if let Some(warehouse_id) = warehouse_id {
        if let Err(resp) = ensure_warehouse_access(&ctx, warehouse_id) {
            return resp.into_response();
        }
    }

    let tag = query.tag.and_then(non_empty);
    let on_hand_negative = query.on_hand_negative.unwrap_or(false);
    let available_below = if query.available_below_safety.unwrap_or(false) {
        match fetch_tenant_setting_value(state.db.pool(), tenant_id, "inventory.safety_stock").await
        {
            Ok(value) => value.as_ref().and_then(parse_numeric_setting).or(Some(0)),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }
    } else {
        None
    };

    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = InventoryRepoCore::new(state.db.pool());
    let result = match repo
        .list_inventory_summary(
            tenant_id,
            InventorySummaryListFilters {
                warehouse_id,
                warehouse_ids: warehouse_scope_filter(&ctx),
                sku_id,
                sku_code: query.sku_code.and_then(non_empty),
                barcode: query.barcode.and_then(non_empty),
                tag,
                on_hand_negative,
                available_below,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = InventorySummaryListResponse {
        items: result
            .items
            .into_iter()
            .map(inventory_summary_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn list_inventory_costs(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<InventoryCostListQuery>,
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

    let cost_enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::CostManagement).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !cost_enabled {
        return feature_flag_forbidden(FeatureFlagKey::CostManagement).into_response();
    }

    let sku_id = match query.sku_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad sku_id").into_response(),
        },
        None => None,
    };
    let warehouse_id = match query.warehouse_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    if let Some(warehouse_id) = warehouse_id {
        if let Err(resp) = ensure_warehouse_access(&ctx, warehouse_id) {
            return resp.into_response();
        }
    }

    let tag = query.tag.and_then(non_empty);
    let on_hand_negative = query.on_hand_negative.unwrap_or(false);
    let available_below = if query.available_below_safety.unwrap_or(false) {
        match fetch_tenant_setting_value(state.db.pool(), tenant_id, "inventory.safety_stock").await
        {
            Ok(value) => value.as_ref().and_then(parse_numeric_setting).or(Some(0)),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }
    } else {
        None
    };

    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = InventoryRepoCore::new(state.db.pool());
    let result = match repo
        .list_inventory_costs(
            tenant_id,
            InventoryCostListFilters {
                warehouse_id,
                warehouse_ids: warehouse_scope_filter(&ctx),
                sku_id,
                sku_code: query.sku_code.and_then(non_empty),
                barcode: query.barcode.and_then(non_empty),
                tag,
                on_hand_negative,
                available_below,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = InventoryCostListResponse {
        items: result
            .items
            .into_iter()
            .map(inventory_cost_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn list_inventory_ledgers(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<InventoryLedgerListQuery>,
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

    let sku_id = match query.sku_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad sku_id").into_response(),
        },
        None => None,
    };
    let warehouse_id = match query.warehouse_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    let occurred_from = match query.occurred_from.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad occurred_from").into_response(),
        },
        None => None,
    };
    let occurred_to = match query.occurred_to.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad occurred_to").into_response(),
        },
        None => None,
    };
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = InventoryRepoCore::new(state.db.pool());
    let result = match repo
        .list_ledgers(
            tenant_id,
            InventoryLedgerListFilters {
                warehouse_id,
                warehouse_ids: warehouse_scope_filter(&ctx),
                sku_id,
                sku_code: query.sku_code,
                occurred_from,
                occurred_to,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = InventoryLedgerListResponse {
        items: result
            .items
            .into_iter()
            .map(inventory_ledger_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn list_wms_inout_records(
    state: State<AppState>,
    path: Path<String>,
    query: Query<InventoryLedgerListQuery>,
    ctx: Extension<AuthContext>,
) -> impl IntoResponse {
    list_inventory_ledgers(state, path, query, ctx).await
}

pub(crate) async fn list_wms_stock_change_details(
    state: State<AppState>,
    path: Path<String>,
    query: Query<InventoryLedgerListQuery>,
    ctx: Extension<AuthContext>,
) -> impl IntoResponse {
    list_inventory_ledgers(state, path, query, ctx).await
}

pub(crate) async fn get_wms_inbound_settings(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.inbounds.read") {
        return resp.into_response();
    }

    let settings = match load_wms_inbound_settings(state.db.pool(), tenant_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(settings)).into_response()
}

pub(crate) async fn update_wms_inbound_settings(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<WmsInboundSettingsUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.inbounds.write") {
        return resp.into_response();
    }

    if req.require_audit.is_none()
        && req.require_inspection.is_none()
        && req.require_putaway_location.is_none()
        && req.enable_return_inbound_link.is_none()
    {
        return bad_request("at least one setting required").into_response();
    }

    let before = match load_wms_inbound_settings(state.db.pool(), tenant_id).await {
        Ok(v) => serde_json::to_value(v).ok(),
        Err(_) => None,
    };
    let repo = SettingsRepo::new(state.db.pool());
    let upsert = |key: &str, value: bool| SettingUpsert {
        tenant_id,
        scope_type: "TENANT".to_string(),
        scope_id: None,
        key: key.to_string(),
        value_json: Value::Bool(value),
        expected_version: None,
    };

    if let Some(value) = req.require_audit {
        if let Err(e) = repo
            .upsert_setting(upsert("inv.inbound_require_audit", value))
            .await
        {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
        }
    }
    if let Some(value) = req.require_inspection {
        if let Err(e) = repo
            .upsert_setting(upsert("wms.inbound_require_inspection", value))
            .await
        {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
        }
    }
    if let Some(value) = req.require_putaway_location {
        if let Err(e) = repo
            .upsert_setting(upsert("wms.inbound_require_location", value))
            .await
        {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
        }
    }
    if let Some(value) = req.enable_return_inbound_link {
        if let Err(e) = repo
            .upsert_setting(upsert("service.enable_return_inbound_link", value))
            .await
        {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
        }
    }

    let settings = match load_wms_inbound_settings(state.db.pool(), tenant_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "WMS.INBOUND_SETTINGS.UPDATE",
        "setting",
        None,
        before,
        Some(serde_json::to_value(&settings).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(settings)).into_response()
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn apply_inventory_movement(
    state: &AppState,
    ctx: &AuthContext,
    tenant_id: TenantId,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    batch_id: Option<uuid::Uuid>,
    qty_delta: i64,
    reason: Option<String>,
    reference_id: Option<String>,
    reason_type: &str,
    log_action: &str,
) -> Result<InventoryBalance, Response> {
    if let Err(resp) = ensure_tenant_access(ctx, tenant_id) {
        return Err(resp.into_response());
    }
    if let Err(resp) = ensure_warehouse_access(ctx, warehouse_id) {
        return Err(resp.into_response());
    }
    if let Err(resp) = ensure_permission(ctx, "inventory.write") {
        return Err(resp.into_response());
    }

    let enabled =
        match feature_flag_enabled(state, tenant_id, None, FeatureFlagKey::InventoryWrite).await {
            Ok(v) => v,
            Err(resp) => return Err(resp.into_response()),
        };
    if !enabled {
        warn!(
            tenant_id = %tenant_id,
            "feature disabled: inventory write"
        );
        return Err(feature_flag_forbidden(FeatureFlagKey::InventoryWrite).into_response());
    }

    let balance = match InventoryRepoCore::new(state.db.pool())
        .apply_adjustment(
            InventoryAdjustment {
                tenant_id,
                warehouse_id,
                sku_id,
                batch_id,
                qty_delta,
                ref_type: reason.clone(),
                ref_id: reference_id.clone(),
            },
            reason_type,
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return Err(inventory_error_response(e).into_response()),
    };

    let log_metadata = serde_json::json!({
        "warehouse_id": warehouse_id,
        "sku_id": sku_id,
        "qty_delta": qty_delta,
        "reason": reason,
        "reference_id": reference_id,
        "reason_type": reason_type,
    });
    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: log_action.to_string(),
            ref_type: Some("INVENTORY".to_string()),
            ref_id: Some(sku_id.to_string()),
            metadata: Some(log_metadata.clone()),
        })
        .await
    {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response());
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(e) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: format!("INVENTORY.{reason_type}"),
            entity_type: "INVENTORY_BALANCE".to_string(),
            entity_id: Some(sku_id.to_string()),
            before_json: None,
            after_json: Some(log_metadata),
            ip: None,
            ua: None,
        })
        .await
    {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response());
    }

    Ok(balance)
}

pub(crate) async fn adjust_inventory(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<InventoryAdjustmentRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
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
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad batch_id").into_response(),
        },
        None => None,
    };
    let balance = match apply_inventory_movement(
        &state,
        &ctx,
        tenant_id,
        warehouse_id,
        sku_id,
        batch_id,
        req.quantity_delta,
        req.reason.clone(),
        req.reference_id.clone(),
        "STOCK_COUNT_ADJUST",
        OP_ACTION_INVENTORY_ADJUSTED,
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
