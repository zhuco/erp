use super::*;

pub(crate) async fn list_stock_counts(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<StockCountListQuery>,
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
    let parse_dt = |raw: &str, label: &str| -> Result<DateTime<Utc>, Response> {
        match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Ok(v.with_timezone(&Utc)),
            Err(_) => Err(bad_request(&format!("bad {label}")).into_response()),
        }
    };
    let created_from = match query.created_from.as_deref() {
        Some(raw) => match parse_dt(raw, "created_from") {
            Ok(v) => Some(v),
            Err(resp) => return resp,
        },
        None => None,
    };
    let created_to = match query.created_to.as_deref() {
        Some(raw) => match parse_dt(raw, "created_to") {
            Ok(v) => Some(v),
            Err(resp) => return resp,
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = StockCountsRepo::new(state.db.pool());
    let result = match repo
        .list_counts(
            tenant_id,
            StockCountListFilters {
                warehouse_id,
                status: query.status,
                count_type: query.count_type,
                scope_type: query.scope_type,
                keyword: query.keyword,
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

    let response = StockCountListResponse {
        items: result
            .items
            .into_iter()
            .map(stock_count_summary_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_stock_count(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<StockCountCreateRequest>,
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
    let auto_freeze_enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::AutoFreeze).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !auto_freeze_enabled {
        return feature_flag_forbidden(FeatureFlagKey::AutoFreeze).into_response();
    }

    let warehouse_id = match uuid::Uuid::parse_str(&req.warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    let strategy = match normalize_stock_count_strategy(req.strategy.as_deref()) {
        Some(value) => value,
        None => return bad_request("bad strategy").into_response(),
    };
    let mut dimension = match normalize_stock_count_dimension(req.dimension.as_deref()) {
        Some(value) => value,
        None => return bad_request("bad dimension").into_response(),
    };
    let scope_type = match normalize_stock_count_scope_type(req.scope_type.as_deref()) {
        Some(value) => Some(value),
        None => return bad_request("bad scope_type").into_response(),
    };
    if matches!(scope_type.as_deref(), Some("AREA") | Some("LOCATION")) {
        dimension = "LOCATION".to_string();
    }
    if matches!(scope_type.as_deref(), Some("SKU")) {
        dimension = "SKU".to_string();
    }
    let scope_ids: Option<Vec<uuid::Uuid>> = match req.scope_ids {
        Some(values) => {
            let mut ids = Vec::with_capacity(values.len());
            for raw in values {
                match uuid::Uuid::parse_str(&raw) {
                    Ok(v) => ids.push(v),
                    Err(_) => return bad_request("bad scope_ids").into_response(),
                }
            }
            Some(ids)
        }
        None => None,
    };
    let count_mode = match normalize_stock_count_mode(req.count_mode.as_deref()) {
        Some(value) => Some(value),
        None => return bad_request("bad count_mode").into_response(),
    };
    let count_type = match normalize_stock_count_type(req.count_type.as_deref()) {
        Some(value) => Some(value),
        None => return bad_request("bad count_type").into_response(),
    };
    let mut recent_days = req.recent_days;
    let mut brand = req
        .brand
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let mut category = req
        .category
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    match strategy.as_str() {
        "RECENT_DAYS" => {
            if recent_days.unwrap_or(0) <= 0 {
                return bad_request("recent_days required").into_response();
            }
            brand = None;
            category = None;
        }
        "BRAND" => {
            if brand.is_none() {
                return bad_request("brand required").into_response();
            }
            recent_days = None;
            category = None;
        }
        "CATEGORY" => {
            if category.is_none() {
                return bad_request("category required").into_response();
            }
            recent_days = None;
            brand = None;
        }
        _ => {
            recent_days = None;
            brand = None;
            category = None;
        }
    }
    let repo = StockCountsRepo::new(state.db.pool());
    let count = match repo
        .create_count(StockCountCreateInput {
            tenant_id,
            warehouse_id,
            strategy: strategy.clone(),
            dimension: dimension.clone(),
            recent_days,
            brand,
            category,
            scope_type,
            scope_ids,
            count_mode,
            count_type,
            freeze_stock: req.freeze_stock,
            created_by: None,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let log_payload = serde_json::json!({
        "warehouse_id": warehouse_id,
        "count_id": count.count_id,
        "status": count.status,
    });
    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_STOCK_COUNT_CREATED.to_string(),
            ref_type: Some("STOCK_COUNT".to_string()),
            ref_id: Some(count.count_id.to_string()),
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
            action: "WMS.STOCK_COUNT.CREATE".to_string(),
            entity_type: "STOCK_COUNT".to_string(),
            entity_id: Some(count.count_id.to_string()),
            before_json: None,
            after_json: Some(log_payload),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let detail = match repo.get_detail(tenant_id, count.count_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("stock count not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let audit_required = match fetch_stock_count_audit_required(state.db.pool(), tenant_id).await {
        Ok(value) => value,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    (
        StatusCode::OK,
        Json(stock_count_detail_response(detail, audit_required)),
    )
        .into_response()
}

pub(crate) async fn import_stock_count(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<StockCountImportRequest>,
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
    if req.items.is_empty() {
        return bad_request("items required").into_response();
    }

    let has_location = req.items.iter().any(|item| item.location_id.is_some());
    let raw_dimension = req.dimension.as_deref().or({
        if has_location {
            Some("LOCATION")
        } else {
            Some("SKU")
        }
    });
    let dimension = match normalize_stock_count_dimension(raw_dimension) {
        Some(value) => value,
        None => return bad_request("bad dimension").into_response(),
    };

    let scope_type = match req.scope_type.as_deref() {
        Some(raw) => match normalize_stock_count_scope_type(Some(raw)) {
            Some(value) => Some(value),
            None => return bad_request("bad scope_type").into_response(),
        },
        None => Some(if dimension == "LOCATION" {
            "LOCATION".to_string()
        } else {
            "SKU".to_string()
        }),
    };
    let count_mode = match normalize_stock_count_mode(req.count_mode.as_deref()) {
        Some(value) => Some(value),
        None => return bad_request("bad count_mode").into_response(),
    };
    let count_type = match req.count_type.as_deref() {
        Some(raw) => match normalize_stock_count_type(Some(raw)) {
            Some(value) => value,
            None => return bad_request("bad count_type").into_response(),
        },
        None => "IMPORT".to_string(),
    };

    let mut items = Vec::with_capacity(req.items.len());
    for item in req.items {
        let sku_id = match uuid::Uuid::parse_str(&item.sku_id) {
            Ok(v) => v,
            Err(_) => return bad_request("bad sku_id").into_response(),
        };
        let location_id = match item.location_id.as_deref() {
            Some(raw) => match uuid::Uuid::parse_str(raw) {
                Ok(v) => Some(v),
                Err(_) => return bad_request("bad location_id").into_response(),
            },
            None => None,
        };
        items.push(StockCountImportItemInput {
            sku_id,
            location_id,
            counted_qty: item.counted_qty,
            diff_reason: item
                .diff_reason
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
        });
    }

    let filters_json = serde_json::json!({
        "scope_type": scope_type,
        "count_mode": count_mode,
        "count_type": count_type,
        "freeze_stock": req.freeze_stock,
    });

    let repo = StockCountsRepo::new(state.db.pool());
    let detail = match repo
        .create_count_with_import_items(StockCountImportInput {
            tenant_id,
            warehouse_id,
            strategy: "IMPORT".to_string(),
            dimension: dimension.clone(),
            filters_json,
            items,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let log_payload = serde_json::json!({
        "warehouse_id": warehouse_id,
        "count_id": detail.count.count_id,
        "status": detail.count.status,
        "count_type": "IMPORT",
    });
    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_STOCK_COUNT_CREATED.to_string(),
            ref_type: Some("STOCK_COUNT".to_string()),
            ref_id: Some(detail.count.count_id.to_string()),
            metadata: Some(log_payload.clone()),
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let audit_required = match fetch_stock_count_audit_required(state.db.pool(), tenant_id).await {
        Ok(value) => value,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    (
        StatusCode::OK,
        Json(stock_count_detail_response(detail, audit_required)),
    )
        .into_response()
}

pub(crate) async fn get_stock_count(
    State(state): State<AppState>,
    Path((tenant_id, count_id)): Path<(String, String)>,
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

    let count_id = match uuid::Uuid::parse_str(&count_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad count_id").into_response(),
    };
    let repo = StockCountsRepo::new(state.db.pool());
    let detail = match repo.get_detail(tenant_id, count_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("stock count not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let audit_required = match fetch_stock_count_audit_required(state.db.pool(), tenant_id).await {
        Ok(value) => value,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(stock_count_detail_response(detail, audit_required)),
    )
        .into_response()
}

pub(crate) async fn update_stock_count_items(
    State(state): State<AppState>,
    Path((tenant_id, count_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<StockCountItemsUpdateRequest>,
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
    let count_id = match uuid::Uuid::parse_str(&count_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad count_id").into_response(),
    };
    if req.items.is_empty() {
        return bad_request("items required").into_response();
    }

    let item_count = req.items.len();
    let mut updates = Vec::with_capacity(item_count);
    for item in &req.items {
        let count_item_id = match uuid::Uuid::parse_str(&item.count_item_id) {
            Ok(v) => v,
            Err(_) => return bad_request("bad count_item_id").into_response(),
        };
        if item.counted_qty < 0 {
            return bad_request("counted_qty must be non-negative").into_response();
        }
        updates.push(StockCountItemUpdate {
            count_item_id,
            counted_qty: item.counted_qty,
            diff_reason: item
                .diff_reason
                .as_deref()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
        });
    }

    let repo = StockCountsRepo::new(state.db.pool());
    if let Err(e) = repo.update_items(tenant_id, count_id, updates).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let log_payload = serde_json::json!({
        "count_id": count_id,
        "items": item_count,
    });
    if let Err(e) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.STOCK_COUNT.UPDATE".to_string(),
            entity_type: "STOCK_COUNT".to_string(),
            entity_id: Some(count_id.to_string()),
            before_json: None,
            after_json: Some(log_payload),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let detail = match repo.get_detail(tenant_id, count_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("stock count not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let audit_required = match fetch_stock_count_audit_required(state.db.pool(), tenant_id).await {
        Ok(value) => value,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    (
        StatusCode::OK,
        Json(stock_count_detail_response(detail, audit_required)),
    )
        .into_response()
}

pub(crate) async fn confirm_stock_count_diff(
    State(state): State<AppState>,
    Path((tenant_id, count_id)): Path<(String, String)>,
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
    let count_id = match uuid::Uuid::parse_str(&count_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad count_id").into_response(),
    };

    let repo = StockCountsRepo::new(state.db.pool());
    let count = match repo.confirm_diff(tenant_id, count_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let log_payload = serde_json::json!({
        "count_id": count_id,
        "status": count.status,
    });
    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_STOCK_COUNT_CONFIRMED.to_string(),
            ref_type: Some("STOCK_COUNT".to_string()),
            ref_id: Some(count_id.to_string()),
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
            action: "WMS.STOCK_COUNT.CONFIRM_DIFF".to_string(),
            entity_type: "STOCK_COUNT".to_string(),
            entity_id: Some(count_id.to_string()),
            before_json: None,
            after_json: Some(log_payload),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let detail = match repo.get_detail(tenant_id, count_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("stock count not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let audit_required = match fetch_stock_count_audit_required(state.db.pool(), tenant_id).await {
        Ok(value) => value,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    (
        StatusCode::OK,
        Json(stock_count_detail_response(detail, audit_required)),
    )
        .into_response()
}

pub(crate) async fn apply_stock_count(
    State(state): State<AppState>,
    Path((tenant_id, count_id)): Path<(String, String)>,
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
    let count_id = match uuid::Uuid::parse_str(&count_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad count_id").into_response(),
    };

    let repo = StockCountsRepo::new(state.db.pool());
    let audit_required = match fetch_stock_count_audit_required(state.db.pool(), tenant_id).await {
        Ok(value) => value,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if !audit_required {
        let detail = match repo.get_detail(tenant_id, count_id).await {
            Ok(Some(v)) => v,
            Ok(None) => return not_found("stock count not found").into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        if detail.count.status != "DIFF_CONFIRMED" && detail.count.status != "APPLIED" {
            if let Err(e) = repo.confirm_diff(tenant_id, count_id).await {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
            }
        }
    }

    let count = match repo.apply_count(tenant_id, count_id).await {
        Ok(v) => v,
        Err(e) => return inventory_error_response(e).into_response(),
    };

    let log_payload = serde_json::json!({
        "count_id": count_id,
        "status": count.status,
    });
    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_STOCK_COUNT_APPLIED.to_string(),
            ref_type: Some("STOCK_COUNT".to_string()),
            ref_id: Some(count_id.to_string()),
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
            action: "WMS.STOCK_COUNT.APPLY".to_string(),
            entity_type: "STOCK_COUNT".to_string(),
            entity_id: Some(count_id.to_string()),
            before_json: None,
            after_json: Some(log_payload),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let detail = match repo.get_detail(tenant_id, count_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("stock count not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    (
        StatusCode::OK,
        Json(stock_count_detail_response(detail, audit_required)),
    )
        .into_response()
}

pub(crate) async fn cancel_stock_count(
    State(state): State<AppState>,
    Path((tenant_id, count_id)): Path<(String, String)>,
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
    let count_id = match uuid::Uuid::parse_str(&count_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad count_id").into_response(),
    };

    let status: Option<String> = match sqlx::query_scalar(
        r#"
        select status
        from stock_counts
        where tenant_id = $1 and count_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(count_id)
    .fetch_optional(state.db.pool())
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let Some(status) = status else {
        return not_found("stock count not found").into_response();
    };
    if status == "COUNTING" {
        return bad_request("stock count in progress").into_response();
    }

    let repo = StockCountsRepo::new(state.db.pool());
    let count = match repo.cancel_count(tenant_id, count_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let log_payload = serde_json::json!({
        "count_id": count_id,
        "status": count.status,
    });
    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_STOCK_COUNT_CANCELLED.to_string(),
            ref_type: Some("STOCK_COUNT".to_string()),
            ref_id: Some(count_id.to_string()),
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
            action: "WMS.STOCK_COUNT.CANCEL".to_string(),
            entity_type: "STOCK_COUNT".to_string(),
            entity_id: Some(count_id.to_string()),
            before_json: None,
            after_json: Some(log_payload),
            ip: None,
            ua: None,
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let detail = match repo.get_detail(tenant_id, count_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("stock count not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let audit_required = match fetch_stock_count_audit_required(state.db.pool(), tenant_id).await {
        Ok(value) => value,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    (
        StatusCode::OK,
        Json(stock_count_detail_response(detail, audit_required)),
    )
        .into_response()
}

pub(crate) async fn list_stock_count_strategies(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<StockCountStrategyListQuery>,
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

    if let Err(e) = ensure_default_stock_count_strategies(&state, tenant_id).await {
        warn!(tenant_id = %tenant_id, error = %e, "seed stock count strategies failed");
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = StockCountStrategiesRepo::new(state.db.pool());
    let result = match repo
        .list_strategies(
            tenant_id,
            StockCountStrategyListFilters {
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

    let response = StockCountStrategyListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn get_stock_count_strategy(
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
    if let Err(resp) = ensure_permission(&ctx, "inventory.read") {
        return resp.into_response();
    }

    let repo = StockCountStrategiesRepo::new(state.db.pool());
    let strategy = match repo.get_strategy(tenant_id, strategy_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("strategy not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(strategy)).into_response()
}

pub(crate) async fn create_stock_count_strategy(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<StockCountStrategyRequest>,
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

    let repo = StockCountStrategiesRepo::new(state.db.pool());
    let strategy = match repo
        .create_strategy(StockCountStrategyCreate {
            tenant_id,
            name: req.name,
            enabled: req.enabled.unwrap_or(true),
            strategy_type: req.strategy_type,
            config_json: req.config_json.unwrap_or_else(|| serde_json::json!({})),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(strategy)).into_response()
}

pub(crate) async fn update_stock_count_strategy(
    State(state): State<AppState>,
    Path((tenant_id, strategy_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<StockCountStrategyUpdateRequest>,
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
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }

    let repo = StockCountStrategiesRepo::new(state.db.pool());
    let updated = match repo
        .update_strategy(
            tenant_id,
            strategy_id,
            StockCountStrategyUpdate {
                name: req.name,
                enabled: req.enabled,
                strategy_type: req.strategy_type,
                config_json: req.config_json,
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

pub(crate) async fn delete_stock_count_strategy(
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
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }

    let repo = StockCountStrategiesRepo::new(state.db.pool());
    let deleted = match repo.delete_strategy(tenant_id, strategy_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("strategy not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(deleted)).into_response()
}

#[derive(Debug, Serialize, Deserialize)]
struct StockCountSuggestionItem {
    sku_id: uuid::Uuid,
    reason: Option<String>,
}

pub(crate) async fn refresh_stock_count_suggestions(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<StockCountSuggestionRefreshRequest>,
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

    let suggest_enabled = match feature_flag_enabled(
        &state,
        tenant_id,
        None,
        FeatureFlagKey::StockCountSuggest,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if !suggest_enabled {
        return feature_flag_forbidden(FeatureFlagKey::StockCountSuggest).into_response();
    }

    if let Err(e) = ensure_default_stock_count_strategies(&state, tenant_id).await {
        warn!(tenant_id = %tenant_id, error = %e, "seed stock count strategies failed");
    }

    let warehouse_filter = match req.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };

    let strategy_repo = StockCountStrategiesRepo::new(state.db.pool());
    let strategies = match strategy_repo.list_enabled_strategies(tenant_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let warehouse_ids: Vec<uuid::Uuid> = if let Some(warehouse_id) = warehouse_filter {
        vec![warehouse_id]
    } else {
        sqlx::query_scalar(
            r#"
            select warehouse_id
            from warehouses
            where tenant_id = $1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_all(state.db.pool())
        .await
        .unwrap_or_default()
    };

    let suggestions_repo = StockCountSuggestionsRepo::new(state.db.pool());
    if let Err(e) = suggestions_repo.clear_open(tenant_id).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let mut created = 0_i64;
    for strategy in strategies {
        let mut warehouse_items: HashMap<uuid::Uuid, Vec<StockCountSuggestionItem>> =
            HashMap::new();
        match strategy.strategy_type.to_uppercase().as_str() {
            "WEEKLY_FULL" => {
                #[derive(sqlx::FromRow)]
                struct Row {
                    warehouse_id: uuid::Uuid,
                    sku_id: uuid::Uuid,
                }
                let rows: Vec<Row> = sqlx::query_as(
                    r#"
                    select warehouse_id, sku_id
                    from inventory_balances
                    where tenant_id = $1 and warehouse_id = any($2)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(&warehouse_ids)
                .fetch_all(state.db.pool())
                .await
                .unwrap_or_default();
                for row in rows {
                    warehouse_items.entry(row.warehouse_id).or_default().push(
                        StockCountSuggestionItem {
                            sku_id: row.sku_id,
                            reason: Some("WEEKLY_FULL".to_string()),
                        },
                    );
                }
            }
            "DAILY_ACTIVE_SKU" => {
                let recent_days = strategy
                    .config_json
                    .get("recent_days")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(7)
                    .max(1);
                let since = Utc::now() - Duration::days(recent_days);
                #[derive(sqlx::FromRow)]
                struct Row {
                    warehouse_id: uuid::Uuid,
                    sku_id: uuid::Uuid,
                }
                let rows: Vec<Row> = sqlx::query_as(
                    r#"
                    select warehouse_id, sku_id
                    from inventory_ledgers
                    where tenant_id = $1
                      and occurred_at >= $2
                      and warehouse_id = any($3)
                    group by warehouse_id, sku_id
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(since)
                .bind(&warehouse_ids)
                .fetch_all(state.db.pool())
                .await
                .unwrap_or_default();
                for row in rows {
                    warehouse_items.entry(row.warehouse_id).or_default().push(
                        StockCountSuggestionItem {
                            sku_id: row.sku_id,
                            reason: Some("RECENT_MOVEMENT".to_string()),
                        },
                    );
                }
            }
            "NEGATIVE_STOCK" => {
                #[derive(sqlx::FromRow)]
                struct Row {
                    warehouse_id: uuid::Uuid,
                    sku_id: uuid::Uuid,
                }
                let rows: Vec<Row> = sqlx::query_as(
                    r#"
                    select warehouse_id, sku_id
                    from inventory_balances
                    where tenant_id = $1
                      and warehouse_id = any($2)
                      and on_hand <= 0
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(&warehouse_ids)
                .fetch_all(state.db.pool())
                .await
                .unwrap_or_default();
                for row in rows {
                    warehouse_items.entry(row.warehouse_id).or_default().push(
                        StockCountSuggestionItem {
                            sku_id: row.sku_id,
                            reason: Some("NEGATIVE_STOCK".to_string()),
                        },
                    );
                }
            }
            _ => {}
        }

        for (warehouse_id, items) in warehouse_items {
            if items.is_empty() {
                continue;
            }
            let item_count = items.len() as i32;
            let items_json = serde_json::to_value(&items).unwrap_or_else(|_| serde_json::json!([]));
            let created_result = suggestions_repo
                .insert_suggestion(StockCountSuggestionCreate {
                    tenant_id,
                    warehouse_id,
                    strategy_id: Some(strategy.strategy_id),
                    strategy_name: Some(strategy.name.clone()),
                    reason: Some(strategy.strategy_type.clone()),
                    item_count,
                    items_json,
                    status: "OPEN".to_string(),
                })
                .await;
            if created_result.is_ok() {
                created += 1;
            }
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "created": created
        })),
    )
        .into_response()
}

pub(crate) async fn list_stock_count_suggestions(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<StockCountSuggestionListQuery>,
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

    let suggest_enabled = match feature_flag_enabled(
        &state,
        tenant_id,
        None,
        FeatureFlagKey::StockCountSuggest,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if !suggest_enabled {
        return feature_flag_forbidden(FeatureFlagKey::StockCountSuggest).into_response();
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

    let repo = StockCountSuggestionsRepo::new(state.db.pool());
    let result = match repo
        .list_suggestions(
            tenant_id,
            StockCountSuggestionListFilters {
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

    let response = StockCountSuggestionListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn apply_stock_count_suggestion(
    State(state): State<AppState>,
    Path((tenant_id, suggestion_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let suggestion_id = match uuid::Uuid::parse_str(&suggestion_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad suggestion_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }

    let suggest_enabled = match feature_flag_enabled(
        &state,
        tenant_id,
        None,
        FeatureFlagKey::StockCountSuggest,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if !suggest_enabled {
        return feature_flag_forbidden(FeatureFlagKey::StockCountSuggest).into_response();
    }

    let suggestions_repo = StockCountSuggestionsRepo::new(state.db.pool());
    let suggestion = match suggestions_repo
        .get_suggestion(tenant_id, suggestion_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("suggestion not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let items: Vec<StockCountSuggestionItem> =
        serde_json::from_value(suggestion.items_json.clone()).unwrap_or_default();
    if items.is_empty() {
        return bad_request("suggestion items empty").into_response();
    }
    let sku_ids: Vec<uuid::Uuid> = items.into_iter().map(|item| item.sku_id).collect();

    let repo = StockCountsRepo::new(state.db.pool());
    let count = match repo
        .create_count_with_items(StockCountCreateWithItemsInput {
            tenant_id,
            warehouse_id: suggestion.warehouse_id,
            sku_ids,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    if let Err(e) = suggestions_repo
        .mark_applied(tenant_id, suggestion_id)
        .await
    {
        warn!(error = %e, "mark suggestion applied failed");
    }

    let log_payload = serde_json::json!({
        "warehouse_id": suggestion.warehouse_id,
        "count_id": count.count_id,
        "suggestion_id": suggestion_id,
        "item_count": suggestion.item_count,
    });
    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_STOCK_COUNT_CREATED.to_string(),
            ref_type: Some("STOCK_COUNT".to_string()),
            ref_id: Some(count.count_id.to_string()),
            metadata: Some(log_payload.clone()),
        })
        .await
    {
        warn!(error = %e, "stock count apply log insert failed");
    }

    (StatusCode::OK, Json(stock_count_summary_response(count))).into_response()
}
