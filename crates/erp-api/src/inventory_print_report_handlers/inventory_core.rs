use super::*;

fn parse_inventory_safety_setting(value: Option<&Value>) -> Result<i128, &'static str> {
    let Some(value) = value else {
        return Ok(0);
    };
    let value = value
        .as_object()
        .and_then(|object| object.get("value"))
        .unwrap_or(value);
    let invalid = "Invalid inventory.safety_stock quantity";
    let quantity = if let Some(text) = value.as_str() {
        erp_grid_core::decimal_scaled(text, 6)
            .map_err(|_| invalid)?
            .ok_or(invalid)?
    } else if let Some(whole) = value.as_i64() {
        i128::from(whole).checked_mul(1_000_000).ok_or(invalid)?
    } else if let Some(whole) = value.as_u64() {
        i128::from(whole).checked_mul(1_000_000).ok_or(invalid)?
    } else {
        return Err(invalid);
    };
    if quantity < 0 {
        return Err(invalid);
    }
    Ok(quantity)
}

fn parse_inventory_available_range(
    min: Option<&str>,
    max: Option<&str>,
) -> Result<(Option<i128>, Option<i128>), &'static str> {
    let parse = |value: Option<&str>| {
        value
            .filter(|v| !v.trim().is_empty())
            .map(|value| {
                erp_grid_core::decimal_scaled(value, 6).map_err(|_| "bad available quantity range")
            })
            .transpose()
            .map(Option::flatten)
    };
    let min = parse(min)?;
    let max = parse(max)?;
    if min.zip(max).is_some_and(|(min, max)| min > max) {
        return Err("available minimum exceeds maximum");
    }
    Ok((min, max))
}

fn inventory_read_error(error: anyhow::Error) -> Response {
    use erp_repo::inventory_reads::InventoryReadError;
    match error.downcast_ref::<InventoryReadError>() {
        Some(InventoryReadError::ReconciliationRequired(count)) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "code": "inventory_quantity_reconciliation_required", "unreconciled_count": count })),
        ).into_response(),
        Some(InventoryReadError::ValueOverflow) => (
            StatusCode::UNPROCESSABLE_ENTITY, Json(serde_json::json!({ "code": "inventory_read_value_overflow" })),
        ).into_response(),
        None => {
            tracing::error!(%error, "inventory read storage failed");
            (StatusCode::INTERNAL_SERVER_ERROR, "Inventory read storage failed").into_response()
        }
    }
}

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
    let (available_min, available_max) = match parse_inventory_available_range(
        query.available_min.as_deref(),
        query.available_max.as_deref(),
    ) {
        Ok(range) => range,
        Err(message) => return bad_request(message).into_response(),
    };
    let tag = query.tag.and_then(non_empty);
    let on_hand_negative = query.on_hand_negative.unwrap_or(false);
    let available_below = if query.available_below_safety.unwrap_or(false) {
        match fetch_tenant_setting_value(state.db.pool(), tenant_id, "inventory.safety_stock").await
        {
            Ok(value) => match parse_inventory_safety_setting(value.as_ref()) {
                Ok(quantity) => Some(quantity),
                Err(message) => return (StatusCode::UNPROCESSABLE_ENTITY,
                    Json(serde_json::json!({"code":"inventory_safety_stock_invalid","message":message}))).into_response(),
            },
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
    let cursor = match query.cursor.as_deref() {
        Some(raw) => match InventoryCursor::decode(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad cursor").into_response(),
        },
        None => None,
    };
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query
        .limit
        .unwrap_or(50)
        .clamp(1, erp_repo::inventory_reads::MAX_READ_PAGE);

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
                available_below_micros: available_below,
                available_min,
                available_max,
                cursor,
                offset: Some(offset),
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return inventory_read_error(e),
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

    let request_qty = match erp_domain::quantity::parse_qty(
        query.request_qty.as_deref().unwrap_or("0"),
    ) {
        Ok(quantity) if !quantity.is_negative() => quantity,
        _ => return bad_request(
            "request_qty must be a nonnegative decimal quantity with at most six fractional digits",
        )
        .into_response(),
    };
    let repo = AtpInventoryRepo::new(state.db.pool());
    let quote = match repo
        .quote_fefo_exact(tenant_id, warehouse_id, sku_id, request_qty)
        .await
    {
        Ok(v) => v,
        Err(e) => return inventory_error_response(e).into_response(),
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

    let (available_min, available_max) = match parse_inventory_available_range(
        query.available_min.as_deref(),
        query.available_max.as_deref(),
    ) {
        Ok(range) => range,
        Err(message) => return bad_request(message).into_response(),
    };
    let tag = query.tag.and_then(non_empty);
    let on_hand_negative = query.on_hand_negative.unwrap_or(false);
    let available_below = if query.available_below_safety.unwrap_or(false) {
        match fetch_tenant_setting_value(state.db.pool(), tenant_id, "inventory.safety_stock").await
        {
            Ok(value) => match parse_inventory_safety_setting(value.as_ref()) {
                Ok(quantity) => Some(quantity),
                Err(message) => return (StatusCode::UNPROCESSABLE_ENTITY,
                    Json(serde_json::json!({"code":"inventory_safety_stock_invalid","message":message}))).into_response(),
            },
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }
    } else {
        None
    };

    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query
        .limit
        .unwrap_or(50)
        .clamp(1, erp_repo::inventory_reads::MAX_READ_PAGE);

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
                available_below_micros: available_below,
                available_min,
                available_max,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return inventory_read_error(e),
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

    let (available_min, available_max) = match parse_inventory_available_range(
        query.available_min.as_deref(),
        query.available_max.as_deref(),
    ) {
        Ok(range) => range,
        Err(message) => return bad_request(message).into_response(),
    };
    let tag = query.tag.and_then(non_empty);
    let on_hand_negative = query.on_hand_negative.unwrap_or(false);
    let available_below = if query.available_below_safety.unwrap_or(false) {
        match fetch_tenant_setting_value(state.db.pool(), tenant_id, "inventory.safety_stock").await
        {
            Ok(value) => match parse_inventory_safety_setting(value.as_ref()) {
                Ok(quantity) => Some(quantity),
                Err(message) => return (StatusCode::UNPROCESSABLE_ENTITY,
                    Json(serde_json::json!({"code":"inventory_safety_stock_invalid","message":message}))).into_response(),
            },
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }
    } else {
        None
    };

    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query
        .limit
        .unwrap_or(50)
        .clamp(1, erp_repo::inventory_reads::MAX_READ_PAGE);

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
                available_below_micros: available_below,
                available_min,
                available_max,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return inventory_read_error(e),
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
    if let Some(warehouse_id) = warehouse_id
        && let Err(response) = ensure_warehouse_access(&ctx, warehouse_id)
    {
        return response.into_response();
    }
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
    let limit = query
        .limit
        .unwrap_or(50)
        .clamp(1, erp_repo::inventory_reads::MAX_READ_PAGE);

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
        Err(e) => return inventory_read_error(e),
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

    // Commit the movement and its required operation/audit facts together.
    // Returning an error before commit drops and rolls back this transaction.
    let mut tx = match state.db.pool().begin().await {
        Ok(tx) => tx,
        Err(error) => return Err(inventory_error_response(error.into()).into_response()),
    };
    let balance = match InventoryRepoCore::new(state.db.pool())
        .apply_adjustment_with_tx(
            &mut tx,
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
        "batch_id": batch_id,
        "qty_delta": qty_delta,
        "reason": reason,
        "reference_id": reference_id,
        "reason_type": reason_type,
        "quantities": &balance.quantities,
    });
    if let Err(e) = OperationLogsRepo::insert_with_tx(
        &mut *tx,
        OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: log_action.to_string(),
            ref_type: Some("INVENTORY".to_string()),
            ref_id: Some(sku_id.to_string()),
            metadata: Some(log_metadata.clone()),
        },
    )
    .await
    {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response());
    }

    if let Err(e) = AuditLogsRepo::insert_with_tx(
        &mut *tx,
        AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: format!("INVENTORY.{reason_type}"),
            entity_type: "INVENTORY_BALANCE".to_string(),
            entity_id: Some(sku_id.to_string()),
            before_json: None,
            after_json: Some(log_metadata),
            ip: None,
            ua: None,
        },
    )
    .await
    {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response());
    }

    if let Err(error) = tx.commit().await {
        return Err(inventory_error_response(error.into()).into_response());
    }
    Ok(balance)
}

pub(crate) async fn adjust_inventory(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    headers: HeaderMap,
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
        return execute_inventory_command(
            &state,
            &ctx,
            erp_repo::inventory_movement_commands::InventoryMovementCommandInput {
                tenant_id,
                actor_user_id: ctx.user_id,
                session_id: ctx.session_id,
                request_id,
                kind: erp_repo::inventory_movement_commands::InventoryMovementKind::Adjustment,
                warehouse_id,
                sku_id,
                batch_id,
                quantity_delta: req.quantity_delta,
                reason: req.reason.clone(),
                reference_id: req.reference_id.clone(),
                location: None,
                reservation_source: None,
            },
        )
        .await;
    }
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
