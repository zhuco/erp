use super::*;

pub(crate) async fn list_wms_labor_tasks(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<WmsLaborTaskListQuery>,
    Extension(ctx): Extension<AuthContext>,
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

    let mut items = match load_wms_labor_tasks(state.db.pool(), tenant_id).await {
        Ok(v) => v,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    if let Some(warehouse_id) = query.warehouse_id.as_deref() {
        items.retain(|item| item.warehouse_id == warehouse_id);
    }
    if let Some(status) = query.status.as_deref() {
        let normalized = status.trim().to_ascii_uppercase();
        items.retain(|item| item.status == normalized);
    }
    let total = items.len() as i64;
    let offset = query.offset.unwrap_or(0).max(0) as usize;
    let limit = query.limit.unwrap_or(50).clamp(1, 200) as usize;
    let items = items.into_iter().skip(offset).take(limit).collect();

    (
        StatusCode::OK,
        Json(WmsLaborTaskListResponse { items, total }),
    )
        .into_response()
}

pub(crate) async fn assign_wms_labor_task(
    State(state): State<AppState>,
    Path((tenant_id, task_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WmsLaborTaskAssignRequest>,
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
    if req.assignee.trim().is_empty() {
        return bad_request("assignee required").into_response();
    }

    let mut items = match load_wms_labor_tasks(state.db.pool(), tenant_id).await {
        Ok(v) => v,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    let Some(item) = items.iter_mut().find(|item| item.task_id == task_id) else {
        return not_found("labor task not found").into_response();
    };
    item.assignee = Some(req.assignee.trim().to_string());
    if item.status != "DONE" {
        item.status = "ASSIGNED".to_string();
    }
    item.updated_at = Utc::now().to_rfc3339();
    let response = item.clone();

    if let Err(err) = save_wms_labor_tasks(state.db.pool(), tenant_id, &items).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn start_wms_labor_task(
    State(state): State<AppState>,
    Path((tenant_id, task_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
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

    let mut items = match load_wms_labor_tasks(state.db.pool(), tenant_id).await {
        Ok(v) => v,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    let Some(item) = items.iter_mut().find(|item| item.task_id == task_id) else {
        return not_found("labor task not found").into_response();
    };
    if item.status != "DONE" {
        item.status = "IN_PROGRESS".to_string();
    }
    item.updated_at = Utc::now().to_rfc3339();
    let response = item.clone();

    if let Err(err) = save_wms_labor_tasks(state.db.pool(), tenant_id, &items).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn complete_wms_labor_task(
    State(state): State<AppState>,
    Path((tenant_id, task_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
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

    let mut items = match load_wms_labor_tasks(state.db.pool(), tenant_id).await {
        Ok(v) => v,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    let Some(item) = items.iter_mut().find(|item| item.task_id == task_id) else {
        return not_found("labor task not found").into_response();
    };
    item.status = "DONE".to_string();
    item.finished_qty = item.planned_qty.max(item.finished_qty);
    item.updated_at = Utc::now().to_rfc3339();
    let response = item.clone();

    if let Err(err) = save_wms_labor_tasks(state.db.pool(), tenant_id, &items).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn list_wms_slotting_suggestions(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<WmsSlottingSuggestionListQuery>,
    Extension(ctx): Extension<AuthContext>,
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

    let mut items = match load_wms_slotting_suggestions(state.db.pool(), tenant_id).await {
        Ok(v) => v,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    if let Some(warehouse_id) = query.warehouse_id.as_deref() {
        items.retain(|item| item.warehouse_id == warehouse_id);
    }
    if let Some(status) = query.status.as_deref() {
        let normalized = status.trim().to_ascii_uppercase();
        items.retain(|item| item.status == normalized);
    }
    let total = items.len() as i64;
    let offset = query.offset.unwrap_or(0).max(0) as usize;
    let limit = query.limit.unwrap_or(50).clamp(1, 200) as usize;
    let items = items.into_iter().skip(offset).take(limit).collect();

    (
        StatusCode::OK,
        Json(WmsSlottingSuggestionListResponse { items, total }),
    )
        .into_response()
}

pub(crate) async fn apply_wms_slotting_suggestion(
    State(state): State<AppState>,
    Path((tenant_id, suggestion_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
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

    let mut items = match load_wms_slotting_suggestions(state.db.pool(), tenant_id).await {
        Ok(v) => v,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    let Some(item) = items
        .iter_mut()
        .find(|item| item.suggestion_id == suggestion_id)
    else {
        return not_found("slotting suggestion not found").into_response();
    };
    item.status = "APPLIED".to_string();
    item.updated_at = Utc::now().to_rfc3339();
    let response = item.clone();

    if let Err(err) = save_wms_slotting_suggestions(state.db.pool(), tenant_id, &items).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn reject_wms_slotting_suggestion(
    State(state): State<AppState>,
    Path((tenant_id, suggestion_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
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

    let mut items = match load_wms_slotting_suggestions(state.db.pool(), tenant_id).await {
        Ok(v) => v,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    let Some(item) = items
        .iter_mut()
        .find(|item| item.suggestion_id == suggestion_id)
    else {
        return not_found("slotting suggestion not found").into_response();
    };
    item.status = "REJECTED".to_string();
    item.updated_at = Utc::now().to_rfc3339();
    let response = item.clone();

    if let Err(err) = save_wms_slotting_suggestions(state.db.pool(), tenant_id, &items).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn list_wms_replenishment_tasks(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<WmsReplenishmentTaskListQuery>,
    Extension(ctx): Extension<AuthContext>,
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

    let mut items = match load_wms_replenishment_tasks(state.db.pool(), tenant_id).await {
        Ok(v) => v,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    if let Some(warehouse_id) = query.warehouse_id.as_deref() {
        items.retain(|item| item.warehouse_id == warehouse_id);
    }
    if let Some(status) = query.status.as_deref() {
        let normalized = status.trim().to_ascii_uppercase();
        items.retain(|item| item.status == normalized);
    }
    let total = items.len() as i64;
    let offset = query.offset.unwrap_or(0).max(0) as usize;
    let limit = query.limit.unwrap_or(50).clamp(1, 200) as usize;
    let items = items.into_iter().skip(offset).take(limit).collect();

    (
        StatusCode::OK,
        Json(WmsReplenishmentTaskListResponse { items, total }),
    )
        .into_response()
}

pub(crate) async fn claim_wms_replenishment_task(
    State(state): State<AppState>,
    Path((tenant_id, task_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WmsReplenishmentTaskClaimRequest>,
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
    if req.assignee.trim().is_empty() {
        return bad_request("assignee required").into_response();
    }

    let mut items = match load_wms_replenishment_tasks(state.db.pool(), tenant_id).await {
        Ok(v) => v,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    let Some(item) = items.iter_mut().find(|item| item.task_id == task_id) else {
        return not_found("replenishment task not found").into_response();
    };
    item.assignee = Some(req.assignee.trim().to_string());
    item.status = "IN_PROGRESS".to_string();
    item.updated_at = Utc::now().to_rfc3339();
    let response = item.clone();

    if let Err(err) = save_wms_replenishment_tasks(state.db.pool(), tenant_id, &items).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn complete_wms_replenishment_task(
    State(state): State<AppState>,
    Path((tenant_id, task_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
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

    let mut items = match load_wms_replenishment_tasks(state.db.pool(), tenant_id).await {
        Ok(v) => v,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    let Some(item) = items.iter_mut().find(|item| item.task_id == task_id) else {
        return not_found("replenishment task not found").into_response();
    };
    item.status = "DONE".to_string();
    item.updated_at = Utc::now().to_rfc3339();
    let response = item.clone();

    if let Err(err) = save_wms_replenishment_tasks(state.db.pool(), tenant_id, &items).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn list_wms_dock_appointments(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<WmsDockAppointmentListQuery>,
    Extension(ctx): Extension<AuthContext>,
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

    let mut items = match load_wms_dock_appointments(state.db.pool(), tenant_id).await {
        Ok(v) => v,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    if let Some(warehouse_id) = query.warehouse_id.as_deref() {
        items.retain(|item| item.warehouse_id == warehouse_id);
    }
    if let Some(status) = query.status.as_deref() {
        let normalized = status.trim().to_ascii_uppercase();
        items.retain(|item| item.status == normalized);
    }
    let total = items.len() as i64;
    let offset = query.offset.unwrap_or(0).max(0) as usize;
    let limit = query.limit.unwrap_or(50).clamp(1, 200) as usize;
    let items = items.into_iter().skip(offset).take(limit).collect();

    (
        StatusCode::OK,
        Json(WmsDockAppointmentListResponse { items, total }),
    )
        .into_response()
}

pub(crate) async fn check_in_wms_dock_appointment(
    State(state): State<AppState>,
    Path((tenant_id, appointment_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
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

    let mut items = match load_wms_dock_appointments(state.db.pool(), tenant_id).await {
        Ok(v) => v,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    let Some(item) = items
        .iter_mut()
        .find(|item| item.appointment_id == appointment_id)
    else {
        return not_found("dock appointment not found").into_response();
    };
    item.status = "CHECKED_IN".to_string();
    item.updated_at = Utc::now().to_rfc3339();
    let response = item.clone();

    if let Err(err) = save_wms_dock_appointments(state.db.pool(), tenant_id, &items).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn start_loading_wms_dock_appointment(
    State(state): State<AppState>,
    Path((tenant_id, appointment_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
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

    let mut items = match load_wms_dock_appointments(state.db.pool(), tenant_id).await {
        Ok(v) => v,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    let Some(item) = items
        .iter_mut()
        .find(|item| item.appointment_id == appointment_id)
    else {
        return not_found("dock appointment not found").into_response();
    };
    item.status = "LOADING".to_string();
    item.updated_at = Utc::now().to_rfc3339();
    let response = item.clone();

    if let Err(err) = save_wms_dock_appointments(state.db.pool(), tenant_id, &items).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn depart_wms_dock_appointment(
    State(state): State<AppState>,
    Path((tenant_id, appointment_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
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

    let mut items = match load_wms_dock_appointments(state.db.pool(), tenant_id).await {
        Ok(v) => v,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    let Some(item) = items
        .iter_mut()
        .find(|item| item.appointment_id == appointment_id)
    else {
        return not_found("dock appointment not found").into_response();
    };
    item.status = "DEPARTED".to_string();
    item.updated_at = Utc::now().to_rfc3339();
    let response = item.clone();

    if let Err(err) = save_wms_dock_appointments(state.db.pool(), tenant_id, &items).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn get_wms_workflow_config(
    State(state): State<AppState>,
    Path((tenant_id, warehouse_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let warehouse_id = match uuid::Uuid::parse_str(&warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.read") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_warehouse_access(&ctx, warehouse_id) {
        return resp.into_response();
    }

    let config =
        match fetch_warehouse_workflow_config(state.db.pool(), tenant_id, warehouse_id).await {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };

    (
        StatusCode::OK,
        Json(workflow_config_response(warehouse_id, config)),
    )
        .into_response()
}

pub(crate) async fn update_wms_workflow_config(
    State(state): State<AppState>,
    Path((tenant_id, warehouse_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WarehouseWorkflowConfigPayload>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let warehouse_id = match uuid::Uuid::parse_str(&warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.write") {
        return resp.into_response();
    }
    if let Err(resp) = ensure_warehouse_access(&ctx, warehouse_id) {
        return resp.into_response();
    }

    if !req.require_pack && req.require_weighing {
        return bad_request("require_pack required for weighing").into_response();
    }
    if !req.require_pack && req.require_inspection {
        return bad_request("require_pack required for inspection").into_response();
    }

    let value_json = match serde_json::to_value(&req) {
        Ok(v) => v,
        Err(_) => return bad_request("invalid workflow config").into_response(),
    };

    let repo = SettingsRepo::new(state.db.pool());
    let result = repo
        .upsert_setting(SettingUpsert {
            tenant_id,
            scope_type: "WAREHOUSE".to_string(),
            scope_id: Some(warehouse_id),
            key: WMS_WORKFLOW_CONFIG_KEY.to_string(),
            value_json,
            expected_version: None,
        })
        .await;
    if let Err(err) = result {
        if err.to_string().contains("version conflict") {
            return conflict("settings version conflict").into_response();
        }
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    (
        StatusCode::OK,
        Json(workflow_config_response(warehouse_id, req)),
    )
        .into_response()
}

pub(crate) async fn list_shipment_preprocess_strategies(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ShipmentPreprocessStrategyListQuery>,
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
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = ShipmentPreprocessStrategiesRepo::new(state.db.pool());
    let result = match repo
        .list_strategies(
            tenant_id,
            ShipmentPreprocessStrategyListFilters {
                enabled: query.enabled,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = ShipmentPreprocessStrategyListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn get_shipment_preprocess_strategy(
    State(state): State<AppState>,
    Path((tenant_id, strategy_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let strategy_id = match uuid::Uuid::parse_str(&strategy_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad strategy_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.read") {
        return resp.into_response();
    }

    let repo = ShipmentPreprocessStrategiesRepo::new(state.db.pool());
    let strategy = match repo.get_strategy(tenant_id, strategy_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("strategy not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(strategy)).into_response()
}

pub(crate) async fn create_shipment_preprocess_strategy(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ShipmentPreprocessStrategyRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let effective_from = match req.effective_from.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad effective_from").into_response(),
        },
        None => None,
    };
    let effective_to = match req.effective_to.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad effective_to").into_response(),
        },
        None => None,
    };
    let sender_address_id = match req.sender_address_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad sender_address_id").into_response(),
        },
        None => None,
    };

    let repo = ShipmentPreprocessStrategiesRepo::new(state.db.pool());
    let strategy = match repo
        .create_strategy(ShipmentPreprocessStrategyCreate {
            tenant_id,
            name: req.name,
            enabled: req.enabled.unwrap_or(true),
            priority: req.priority.unwrap_or(0),
            effective_from,
            effective_to,
            conditions_json: req.conditions_json.unwrap_or_else(|| serde_json::json!([])),
            rules_json: req.rules_json.unwrap_or_else(|| serde_json::json!({})),
            auto_freeze: req.auto_freeze.unwrap_or(false),
            auto_waybill_reserve: req.auto_waybill_reserve.unwrap_or(false),
            sender_address_id,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(strategy)).into_response()
}

pub(crate) async fn update_shipment_preprocess_strategy(
    State(state): State<AppState>,
    Path((tenant_id, strategy_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ShipmentPreprocessStrategyUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let strategy_id = match uuid::Uuid::parse_str(&strategy_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad strategy_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let effective_from = match req.effective_from.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad effective_from").into_response(),
        },
        None => None,
    };
    let effective_to = match req.effective_to.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad effective_to").into_response(),
        },
        None => None,
    };
    let sender_address_id = match req.sender_address_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad sender_address_id").into_response(),
        },
        None => None,
    };

    let repo = ShipmentPreprocessStrategiesRepo::new(state.db.pool());
    let updated = match repo
        .update_strategy(
            tenant_id,
            strategy_id,
            ShipmentPreprocessStrategyUpdate {
                name: req.name,
                enabled: req.enabled,
                priority: req.priority,
                effective_from,
                effective_to,
                conditions_json: req.conditions_json,
                rules_json: req.rules_json,
                auto_freeze: req.auto_freeze,
                auto_waybill_reserve: req.auto_waybill_reserve,
                sender_address_id,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("strategy not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(updated)).into_response()
}

pub(crate) async fn delete_shipment_preprocess_strategy(
    State(state): State<AppState>,
    Path((tenant_id, strategy_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let strategy_id = match uuid::Uuid::parse_str(&strategy_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad strategy_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let repo = ShipmentPreprocessStrategiesRepo::new(state.db.pool());
    let deleted = match repo.delete_strategy(tenant_id, strategy_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("strategy not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(deleted)).into_response()
}

pub(crate) async fn list_shipment_preprocess_hits(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ShipmentPreprocessHitListQuery>,
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

    let strategy_id = match query.strategy_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad strategy_id").into_response(),
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
    let order_id = match query.order_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad order_id").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = ShipmentPreprocessHitsRepo::new(state.db.pool());
    let result = match repo
        .list_hits(
            tenant_id,
            ShipmentPreprocessHitListFilters {
                strategy_id,
                shipment_id,
                order_id,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = ShipmentPreprocessHitListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn list_pick_recommend_strategies(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<PickRecommendStrategyListQuery>,
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

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = PickRecommendStrategiesRepo::new(state.db.pool());
    let result = match repo
        .list_strategies(
            tenant_id,
            PickRecommendStrategyListFilters {
                enabled: query.enabled,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = PickRecommendStrategyListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn get_pick_recommend_strategy(
    State(state): State<AppState>,
    Path((tenant_id, strategy_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let strategy_id = match uuid::Uuid::parse_str(&strategy_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad strategy_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.read") {
        return resp.into_response();
    }

    let repo = PickRecommendStrategiesRepo::new(state.db.pool());
    let strategy = match repo.get_strategy(tenant_id, strategy_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("strategy not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(strategy)).into_response()
}

pub(crate) async fn create_pick_recommend_strategy(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PickRecommendStrategyRequest>,
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

    let repo = PickRecommendStrategiesRepo::new(state.db.pool());
    let strategy = match repo
        .create_strategy(PickRecommendStrategyCreate {
            tenant_id,
            name: req.name,
            enabled: req.enabled.unwrap_or(true),
            priority: req.priority.unwrap_or(0),
            method: req.method.unwrap_or_else(|| "FIFO".to_string()),
            prefer_location_priority: req.prefer_location_priority.unwrap_or(false),
            prefer_pick_zone: req.prefer_pick_zone.unwrap_or(false),
            prefer_sufficient: req.prefer_sufficient.unwrap_or(false),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(e) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.PICK_RECOMMEND_STRATEGY_CREATED".to_string(),
            entity_type: "PICK_RECOMMEND_STRATEGY".to_string(),
            entity_id: Some(strategy.strategy_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({
                "name": strategy.name,
                "enabled": strategy.enabled,
                "priority": strategy.priority,
                "method": strategy.method,
            })),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %e, "audit log insert failed");
    }

    (StatusCode::OK, Json(strategy)).into_response()
}

pub(crate) async fn update_pick_recommend_strategy(
    State(state): State<AppState>,
    Path((tenant_id, strategy_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PickRecommendStrategyUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let strategy_id = match uuid::Uuid::parse_str(&strategy_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad strategy_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.write") {
        return resp.into_response();
    }

    let repo = PickRecommendStrategiesRepo::new(state.db.pool());
    let updated = match repo
        .update_strategy(
            tenant_id,
            strategy_id,
            PickRecommendStrategyUpdate {
                name: req.name,
                enabled: req.enabled,
                priority: req.priority,
                method: req.method,
                prefer_location_priority: req.prefer_location_priority,
                prefer_pick_zone: req.prefer_pick_zone,
                prefer_sufficient: req.prefer_sufficient,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("strategy not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(e) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.PICK_RECOMMEND_STRATEGY_UPDATED".to_string(),
            entity_type: "PICK_RECOMMEND_STRATEGY".to_string(),
            entity_id: Some(updated.strategy_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({
                "name": updated.name,
                "enabled": updated.enabled,
                "priority": updated.priority,
                "method": updated.method,
            })),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %e, "audit log insert failed");
    }

    (StatusCode::OK, Json(updated)).into_response()
}

pub(crate) async fn delete_pick_recommend_strategy(
    State(state): State<AppState>,
    Path((tenant_id, strategy_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let strategy_id = match uuid::Uuid::parse_str(&strategy_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad strategy_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.write") {
        return resp.into_response();
    }

    let repo = PickRecommendStrategiesRepo::new(state.db.pool());
    let deleted = match repo.delete_strategy(tenant_id, strategy_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("strategy not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(e) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.PICK_RECOMMEND_STRATEGY_DELETED".to_string(),
            entity_type: "PICK_RECOMMEND_STRATEGY".to_string(),
            entity_id: Some(deleted.strategy_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::json!({
                "name": deleted.name,
                "enabled": deleted.enabled,
                "priority": deleted.priority,
                "method": deleted.method,
            })),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %e, "audit log insert failed");
    }

    (StatusCode::OK, Json(deleted)).into_response()
}

pub(crate) async fn list_pick_recommend_hits(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<PickRecommendHitListQuery>,
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

    let strategy_id = match query.strategy_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad strategy_id").into_response(),
        },
        None => None,
    };
    let pick_task_id = match query.pick_task_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad pick_task_id").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = PickRecommendHitsRepo::new(state.db.pool());
    let result = match repo
        .list_hits(
            tenant_id,
            PickRecommendHitListFilters {
                strategy_id,
                pick_task_id,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = PickRecommendHitListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn apply_pick_recommendation_for_wave(
    state: &AppState,
    tenant_id: TenantId,
    wave_id: uuid::Uuid,
) -> anyhow::Result<()> {
    let enabled =
        match feature_flag_enabled(state, tenant_id, None, FeatureFlagKey::PickRecommend).await {
            Ok(v) => v,
            Err(_) => return Ok(()),
        };
    if !enabled {
        return Ok(());
    }
    let repo = PickRecommendStrategiesRepo::new(state.db.pool());
    let strategies = repo.list_enabled_strategies(tenant_id).await?;
    let Some(strategy) = strategies.into_iter().next() else {
        return Ok(());
    };

    #[derive(sqlx::FromRow)]
    struct TaskRow {
        pick_task_id: uuid::Uuid,
        sku_id: uuid::Uuid,
        qty: i64,
        warehouse_id: uuid::Uuid,
    }

    let tasks: Vec<TaskRow> = sqlx::query_as(
        r#"
        select t.pick_task_id, t.sku_id, t.qty, d.warehouse_id
        from outbound_pick_tasks t
        join outbound_docs d
          on d.tenant_id = t.tenant_id
         and d.outbound_doc_id = t.outbound_doc_id
        join wave_docs wd
          on wd.tenant_id = d.tenant_id
         and wd.outbound_doc_id = d.outbound_doc_id
        where t.tenant_id = $1
          and wd.wave_id = $2
          and t.from_location_id is null
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(wave_id)
    .fetch_all(state.db.pool())
    .await?;

    #[derive(sqlx::FromRow, Clone)]
    struct LocationRow {
        location_id: uuid::Uuid,
        on_hand: i64,
        updated_at: DateTime<Utc>,
        priority: i32,
        pick_zone: Option<String>,
    }

    let hits_repo = PickRecommendHitsRepo::new(state.db.pool());
    let method = strategy.method.to_uppercase();

    for task in tasks {
        let mut locations: Vec<LocationRow> = sqlx::query_as(
            r#"
            select
                ilb.location_id,
                ilb.on_hand,
                ilb.updated_at,
                coalesce(wl.priority, 0) as priority,
                wl.pick_zone
            from inventory_location_balances ilb
            left join warehouse_locations wl
              on wl.location_id = ilb.location_id
            where ilb.tenant_id = $1
              and ilb.warehouse_id = $2
              and ilb.sku_id = $3
              and ilb.on_hand > 0
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(task.warehouse_id)
        .bind(task.sku_id)
        .fetch_all(state.db.pool())
        .await?;

        let mut selected: Option<LocationRow> = None;
        if !locations.is_empty() {
            if strategy.prefer_sufficient {
                let sufficient: Vec<LocationRow> = locations
                    .iter()
                    .filter(|&row| row.on_hand >= task.qty)
                    .cloned()
                    .collect();
                if !sufficient.is_empty() {
                    locations = sufficient;
                }
            }

            locations.sort_by(|a, b| {
                if strategy.prefer_pick_zone {
                    let zone_cmp = match (a.pick_zone.as_deref(), b.pick_zone.as_deref()) {
                        (Some(a), Some(b)) => a.cmp(b),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        _ => std::cmp::Ordering::Equal,
                    };
                    if zone_cmp != std::cmp::Ordering::Equal {
                        return zone_cmp;
                    }
                }

                if strategy.prefer_location_priority {
                    let priority_cmp = b.priority.cmp(&a.priority);
                    if priority_cmp != std::cmp::Ordering::Equal {
                        return priority_cmp;
                    }
                }

                match method.as_str() {
                    "FEFO" | "FIFO" => {
                        let fifo_cmp = a.updated_at.cmp(&b.updated_at);
                        if fifo_cmp != std::cmp::Ordering::Equal {
                            return fifo_cmp;
                        }
                    }
                    _ => {}
                }

                b.on_hand.cmp(&a.on_hand)
            });

            selected = locations.first().cloned();
        }

        if let Some(location) = selected {
            sqlx::query(
                r#"
                update outbound_pick_tasks
                set from_location_id = $3,
                    updated_at = now()
                where tenant_id = $1 and pick_task_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(task.pick_task_id)
            .bind(location.location_id)
            .execute(state.db.pool())
            .await?;

            let reason = format!(
                "method={}, pick_zone={}, location_priority={}, sufficient_first={}",
                method,
                strategy.prefer_pick_zone,
                strategy.prefer_location_priority,
                strategy.prefer_sufficient
            );
            let _ = hits_repo
                .insert_hit(PickRecommendHitCreate {
                    tenant_id,
                    strategy_id: Some(strategy.strategy_id),
                    pick_task_id: Some(task.pick_task_id),
                    sku_id: Some(task.sku_id),
                    from_location_id: Some(location.location_id),
                    reason: Some(reason),
                    success: true,
                })
                .await;
        } else {
            let _ = hits_repo
                .insert_hit(PickRecommendHitCreate {
                    tenant_id,
                    strategy_id: Some(strategy.strategy_id),
                    pick_task_id: Some(task.pick_task_id),
                    sku_id: Some(task.sku_id),
                    from_location_id: None,
                    reason: Some("no candidate locations".to_string()),
                    success: false,
                })
                .await;
        }
    }

    Ok(())
}
