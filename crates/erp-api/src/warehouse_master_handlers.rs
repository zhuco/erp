use super::*;

pub(crate) async fn list_warehouses(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<WarehouseListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicWarehouseRead.as_str()) {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let keyword = query
        .keyword
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = query
        .status
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let result = match state
        .warehouses_repo
        .list_warehouses(
            tenant_id,
            WarehouseListFilters {
                keyword,
                status,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = WarehouseListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_warehouse(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WarehouseCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicWarehouseWrite.as_str()) {
        return resp.into_response();
    }

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name required").into_response();
    }
    let code = req.code.trim();
    if code.is_empty() {
        return bad_request("code required").into_response();
    }

    let status = req
        .status
        .unwrap_or_else(|| "ACTIVE".to_string())
        .trim()
        .to_string();
    if status.is_empty() {
        return bad_request("status required").into_response();
    }

    let warehouse = match state
        .warehouses_repo
        .create_warehouse(WarehouseCreate {
            tenant_id,
            name: name.to_string(),
            code: code.to_string(),
            status,
            is_default: req.is_default.unwrap_or(false),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(warehouse)).into_response()
}

pub(crate) async fn update_warehouse(
    State(state): State<AppState>,
    Path((tenant_id, warehouse_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WarehouseUpdateRequest>,
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
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicWarehouseWrite.as_str()) {
        return resp.into_response();
    }

    let name = req
        .name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let code = req
        .code
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = req
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());

    if let Some(status) = status.as_deref() {
        if status != "ACTIVE" && status != "DISABLED" {
            return bad_request("unsupported status").into_response();
        }
        if status == "DISABLED" {
            let is_default: Option<bool> = sqlx::query_scalar(
                r#"
                select is_default
                from warehouses
                where tenant_id = $1 and warehouse_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(warehouse_id)
            .fetch_optional(state.db.pool())
            .await
            .unwrap_or(None);
            if matches!(is_default, Some(true)) {
                return conflict("cannot disable default warehouse").into_response();
            }
        }
    }

    if name.is_none() && code.is_none() && status.is_none() {
        return bad_request("no fields to update").into_response();
    }

    let warehouse = match state
        .warehouses_repo
        .update_warehouse(
            tenant_id,
            warehouse_id,
            WarehouseUpdate { name, code, status },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("warehouse not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(warehouse)).into_response()
}

pub(crate) async fn delete_warehouse(
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
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicWarehouseWrite.as_str()) {
        return resp.into_response();
    }

    let is_default: Option<bool> = sqlx::query_scalar(
        r#"
        select is_default
        from warehouses
        where tenant_id = $1 and warehouse_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(warehouse_id)
    .fetch_optional(state.db.pool())
    .await
    .unwrap_or(None);
    if matches!(is_default, Some(true)) {
        return conflict("cannot delete default warehouse").into_response();
    }

    let location_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from warehouse_locations
        where tenant_id = $1 and warehouse_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(warehouse_id)
    .fetch_one(state.db.pool())
    .await
    .unwrap_or(0);
    if location_count > 0 {
        return conflict("warehouse in use by locations").into_response();
    }

    let inventory_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from inventory_balances
        where tenant_id = $1 and warehouse_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(warehouse_id)
    .fetch_one(state.db.pool())
    .await
    .unwrap_or(0);
    if inventory_count > 0 {
        return conflict("warehouse in use by inventory").into_response();
    }

    let shipment_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from shipments
        where tenant_id = $1 and warehouse_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(warehouse_id)
    .fetch_one(state.db.pool())
    .await
    .unwrap_or(0);
    if shipment_count > 0 {
        return conflict("warehouse in use by shipments").into_response();
    }

    let warehouse = match state
        .warehouses_repo
        .delete_warehouse(tenant_id, warehouse_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("warehouse not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(warehouse)).into_response()
}

pub(crate) async fn set_default_warehouse(
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
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicWarehouseWrite.as_str()) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_warehouse_access(&ctx, warehouse_id) {
        return resp.into_response();
    }

    let warehouse = match state
        .warehouses_repo
        .set_default_warehouse(tenant_id, warehouse_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("warehouse not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(warehouse)).into_response()
}

pub(crate) async fn list_warehouse_areas(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<WarehouseAreaListQuery>,
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

    let warehouse_id = match query.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
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
    let area_type = match query.area_type.as_deref() {
        Some(raw) => match normalize_warehouse_area_type(raw) {
            Some(v) => Some(v),
            None => return bad_request("bad area_type").into_response(),
        },
        None => None,
    };
    let status = match query.status.as_deref() {
        Some(raw) => match normalize_warehouse_area_status(raw) {
            Some(v) => Some(v),
            None => return bad_request("bad status").into_response(),
        },
        None => None,
    };
    let keyword = query
        .keyword
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = WarehouseAreasRepo::new(state.db.pool());
    let result = match repo
        .list_areas(
            tenant_id,
            WarehouseAreaListFilters {
                warehouse_id,
                area_type,
                status,
                keyword,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = WarehouseAreaListResponse {
        items: result
            .items
            .into_iter()
            .map(warehouse_area_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_warehouse_area(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WarehouseAreaCreateRequest>,
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

    let warehouse_id = match uuid::Uuid::parse_str(&req.warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    if let Err(resp) = ensure_warehouse_access(&ctx, warehouse_id) {
        return resp.into_response();
    }

    let code = req.code.trim();
    if code.is_empty() {
        return bad_request("code required").into_response();
    }
    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name required").into_response();
    }
    let area_type = req
        .area_type
        .as_deref()
        .and_then(normalize_warehouse_area_type)
        .unwrap_or_else(|| "STORAGE".to_string());
    let status = req
        .status
        .as_deref()
        .and_then(normalize_warehouse_area_status)
        .unwrap_or_else(|| "ACTIVE".to_string());

    let repo = WarehouseAreasRepo::new(state.db.pool());
    let area = match repo
        .create_area(WarehouseAreaCreate {
            tenant_id,
            warehouse_id,
            code: code.to_string(),
            name: name.to_string(),
            area_type,
            status,
            priority: req.priority.unwrap_or(0),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(warehouse_area_response(area))).into_response()
}

pub(crate) async fn update_warehouse_area(
    State(state): State<AppState>,
    Path((tenant_id, area_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WarehouseAreaUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let area_id = match uuid::Uuid::parse_str(&area_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad area_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let area_type = match req.area_type {
        Some(raw) => match normalize_warehouse_area_type(&raw) {
            Some(v) => Some(v),
            None => return bad_request("bad area_type").into_response(),
        },
        None => None,
    };
    let status = match req.status {
        Some(raw) => match normalize_warehouse_area_status(&raw) {
            Some(v) => Some(v),
            None => return bad_request("bad status").into_response(),
        },
        None => None,
    };

    let repo = WarehouseAreasRepo::new(state.db.pool());
    let area = match repo
        .update_area(
            tenant_id,
            area_id,
            WarehouseAreaUpdate {
                name: req.name.and_then(non_empty),
                area_type,
                status,
                priority: req.priority,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("area not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(warehouse_area_response(area))).into_response()
}

pub(crate) async fn delete_warehouse_area(
    State(state): State<AppState>,
    Path((tenant_id, area_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let area_id = match uuid::Uuid::parse_str(&area_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad area_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }

    let repo = WarehouseAreasRepo::new(state.db.pool());
    let area = match repo.delete_area(tenant_id, area_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("area not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(warehouse_area_response(area))).into_response()
}

pub(crate) async fn list_warehouse_locations(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<WarehouseLocationListQuery>,
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
    let area_id = match query.area_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad area_id").into_response(),
        },
        None => None,
    };
    let keyword = query
        .keyword
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = WarehouseLocationsRepo::new(state.db.pool());
    let result = match repo
        .list_locations(
            tenant_id,
            WarehouseLocationListFilters {
                warehouse_id,
                area_id,
                is_picking: query.is_picking,
                is_disabled: query.is_disabled,
                keyword,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = WarehouseLocationListResponse {
        items: result
            .items
            .into_iter()
            .map(warehouse_location_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_warehouse_location(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WarehouseLocationCreateRequest>,
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
    let area_id = match req.area_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad area_id").into_response(),
        },
        None => None,
    };

    let code = req.code.trim();
    if code.is_empty() {
        return bad_request("code required").into_response();
    }

    let repo = WarehouseLocationsRepo::new(state.db.pool());
    let location = match repo
        .create_location(WarehouseLocationCreate {
            tenant_id,
            warehouse_id,
            area_id,
            code: code.to_string(),
            name: req.name.and_then(non_empty),
            barcode: req.barcode.and_then(non_empty),
            volume: req.volume,
            max_weight: req.max_weight,
            is_picking: req.is_picking.unwrap_or(false),
            is_disabled: req.is_disabled.unwrap_or(false),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(warehouse_location_response(location))).into_response()
}

pub(crate) async fn batch_create_warehouse_locations(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WarehouseLocationBatchRequest>,
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
    if req.locations.is_empty() {
        return bad_request("locations required").into_response();
    }

    let default_area_id = match req.area_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad area_id").into_response(),
        },
        None => None,
    };

    let mut locations = Vec::with_capacity(req.locations.len());
    for item in req.locations {
        let code = item.code.trim();
        if code.is_empty() {
            return bad_request("location code required").into_response();
        }
        let area_id = match item.area_id.as_deref() {
            Some(raw) => match uuid::Uuid::parse_str(raw) {
                Ok(v) => Some(v),
                Err(_) => return bad_request("bad area_id").into_response(),
            },
            None => default_area_id,
        };
        locations.push(WarehouseLocationCreate {
            tenant_id,
            warehouse_id,
            area_id,
            code: code.to_string(),
            name: item.name.and_then(non_empty),
            barcode: item.barcode.and_then(non_empty),
            volume: item.volume,
            max_weight: item.max_weight,
            is_picking: item.is_picking.unwrap_or(false),
            is_disabled: item.is_disabled.unwrap_or(false),
        });
    }

    let repo = WarehouseLocationsRepo::new(state.db.pool());
    let created = match repo.create_locations(tenant_id, locations).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let total = created.len() as i64;
    let response = WarehouseLocationListResponse {
        items: created
            .into_iter()
            .map(warehouse_location_response)
            .collect(),
        total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn update_warehouse_location(
    State(state): State<AppState>,
    Path((tenant_id, location_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<WarehouseLocationUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let location_id = match uuid::Uuid::parse_str(&location_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad location_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }

    let area_id = match req.area_id {
        Some(Some(raw)) => match uuid::Uuid::parse_str(&raw) {
            Ok(v) => Some(Some(v)),
            Err(_) => return bad_request("bad area_id").into_response(),
        },
        Some(None) => Some(None),
        None => None,
    };
    let name = match req.name {
        Some(Some(raw)) => Some(non_empty(raw)),
        Some(None) => Some(None),
        None => None,
    };
    let barcode = match req.barcode {
        Some(Some(raw)) => Some(non_empty(raw)),
        Some(None) => Some(None),
        None => None,
    };

    let repo = WarehouseLocationsRepo::new(state.db.pool());
    let location = match repo
        .update_location(
            tenant_id,
            location_id,
            WarehouseLocationUpdate {
                area_id,
                name,
                barcode,
                volume: req.volume,
                max_weight: req.max_weight,
                is_picking: req.is_picking,
                is_disabled: req.is_disabled,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("location not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(warehouse_location_response(location))).into_response()
}

pub(crate) async fn delete_warehouse_location(
    State(state): State<AppState>,
    Path((tenant_id, location_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let location_id = match uuid::Uuid::parse_str(&location_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad location_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }

    let repo = WarehouseLocationsRepo::new(state.db.pool());
    let location = match repo.delete_location(tenant_id, location_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("location not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(warehouse_location_response(location))).into_response()
}

pub(crate) async fn list_location_balances(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<LocationBalanceListQuery>,
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
    let location_id = match query.location_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad location_id").into_response(),
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
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = WarehouseLocationsRepo::new(state.db.pool());
    let result = match repo
        .list_location_balances(
            tenant_id,
            LocationBalanceListFilters {
                warehouse_id,
                location_id,
                sku_id,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = LocationBalanceListResponse {
        items: result
            .items
            .into_iter()
            .map(location_balance_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn list_containers(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ContainerListQuery>,
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

    let keyword = query
        .keyword
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = ContainersRepo::new(state.db.pool());
    let result = match repo
        .list_containers(
            tenant_id,
            ContainerListFilters {
                keyword,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = ContainerListResponse {
        items: result.items.into_iter().map(container_response).collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_container(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ContainerCreateRequest>,
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

    let code = req.code.trim();
    if code.is_empty() {
        return bad_request("code required").into_response();
    }
    let container_type = req.container_type.trim();
    if container_type.is_empty() {
        return bad_request("container_type required").into_response();
    }

    let repo = ContainersRepo::new(state.db.pool());
    let container = match repo
        .create_container(ContainerCreate {
            tenant_id,
            code: code.to_string(),
            container_type: container_type.to_string(),
            capacity: req.capacity,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(container_response(container))).into_response()
}

pub(crate) async fn update_container(
    State(state): State<AppState>,
    Path((tenant_id, container_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ContainerUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let container_id = match uuid::Uuid::parse_str(&container_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad container_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }

    let container_type = req.container_type.and_then(non_empty);

    let repo = ContainersRepo::new(state.db.pool());
    let container = match repo
        .update_container(
            tenant_id,
            container_id,
            ContainerUpdate {
                container_type,
                capacity: req.capacity,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("container not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(container_response(container))).into_response()
}

pub(crate) async fn delete_container(
    State(state): State<AppState>,
    Path((tenant_id, container_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let container_id = match uuid::Uuid::parse_str(&container_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad container_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }

    let repo = ContainersRepo::new(state.db.pool());
    let container = match repo.delete_container(tenant_id, container_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("container not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(container_response(container))).into_response()
}

pub(crate) async fn list_sender_addresses(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<SenderAddressListQuery>,
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

    let keyword = query
        .keyword
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = SenderAddressesRepo::new(state.db.pool());
    let result = match repo
        .list_sender_addresses(
            tenant_id,
            SenderAddressListFilters {
                keyword,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = SenderAddressListResponse {
        items: result
            .items
            .into_iter()
            .map(sender_address_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_sender_address(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SenderAddressCreateRequest>,
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

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name required").into_response();
    }
    let phone = req.phone.trim();
    if phone.is_empty() {
        return bad_request("phone required").into_response();
    }
    let province = req.province.trim();
    if province.is_empty() {
        return bad_request("province required").into_response();
    }
    let city = req.city.trim();
    if city.is_empty() {
        return bad_request("city required").into_response();
    }
    let district = req.district.trim();
    if district.is_empty() {
        return bad_request("district required").into_response();
    }
    let address = req.address.trim();
    if address.is_empty() {
        return bad_request("address required").into_response();
    }

    let repo = SenderAddressesRepo::new(state.db.pool());
    let sender = match repo
        .create_sender_address(SenderAddressCreate {
            tenant_id,
            name: name.to_string(),
            phone: phone.to_string(),
            province: province.to_string(),
            city: city.to_string(),
            district: district.to_string(),
            address: address.to_string(),
            is_default: req.is_default.unwrap_or(false),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(sender_address_response(sender))).into_response()
}

pub(crate) async fn update_sender_address(
    State(state): State<AppState>,
    Path((tenant_id, sender_address_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SenderAddressUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let sender_address_id = match uuid::Uuid::parse_str(&sender_address_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sender_address_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }

    let repo = SenderAddressesRepo::new(state.db.pool());
    let sender = match repo
        .update_sender_address(
            tenant_id,
            sender_address_id,
            SenderAddressUpdate {
                name: req.name.and_then(non_empty),
                phone: req.phone.and_then(non_empty),
                province: req.province.and_then(non_empty),
                city: req.city.and_then(non_empty),
                district: req.district.and_then(non_empty),
                address: req.address.and_then(non_empty),
                is_default: req.is_default,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("sender address not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(sender_address_response(sender))).into_response()
}

pub(crate) async fn delete_sender_address(
    State(state): State<AppState>,
    Path((tenant_id, sender_address_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let sender_address_id = match uuid::Uuid::parse_str(&sender_address_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sender_address_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {
        return resp.into_response();
    }

    let repo = SenderAddressesRepo::new(state.db.pool());
    let sender = match repo
        .delete_sender_address(tenant_id, sender_address_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("sender address not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(sender_address_response(sender))).into_response()
}

pub(crate) async fn list_couriers(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<CourierListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicCourierRead.as_str()) {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let keyword = query
        .keyword
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = query
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());

    let result = match state
        .couriers_repo
        .list_couriers(
            tenant_id,
            CourierListFilters {
                keyword,
                status,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = CourierListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_courier(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CourierCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicCourierWrite.as_str()) {
        return resp.into_response();
    }

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name is required").into_response();
    }
    let code = req.code.trim();
    if code.is_empty() {
        return bad_request("code is required").into_response();
    }
    let status = req
        .status
        .unwrap_or_else(|| "ACTIVE".to_string())
        .trim()
        .to_uppercase();
    if status.is_empty() {
        return bad_request("status is required").into_response();
    }

    let courier = match state
        .couriers_repo
        .create_courier(CourierCreate {
            tenant_id,
            name: name.to_string(),
            code: code.to_string(),
            status,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(courier)).into_response()
}

pub(crate) async fn update_courier(
    State(state): State<AppState>,
    Path((tenant_id, courier_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CourierUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let courier_id = match uuid::Uuid::parse_str(&courier_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad courier_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicCourierWrite.as_str()) {
        return resp.into_response();
    }

    let name = req
        .name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let code = req
        .code
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = req
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());
    if name.is_none() && code.is_none() && status.is_none() {
        return bad_request("no fields to update").into_response();
    }

    let courier = match state
        .couriers_repo
        .update_courier(tenant_id, courier_id, CourierUpdate { name, code, status })
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("courier not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(courier)).into_response()
}

pub(crate) async fn delete_courier(
    State(state): State<AppState>,
    Path((tenant_id, courier_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let courier_id = match uuid::Uuid::parse_str(&courier_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad courier_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicCourierWrite.as_str()) {
        return resp.into_response();
    }

    let courier = match state
        .couriers_repo
        .delete_courier(tenant_id, courier_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("courier not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(courier)).into_response()
}

pub(crate) async fn list_carriers(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<CarrierListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicCarrierRead.as_str()) {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let keyword = query
        .keyword
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let repo = CarriersRepo::new(state.db.pool());
    let result = match repo
        .list_carriers(
            tenant_id,
            CarrierListFilters {
                keyword,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = CarrierListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_carrier(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CarrierCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicCarrierWrite.as_str()) {
        return resp.into_response();
    }

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name is required").into_response();
    }
    let carrier_code = req.carrier_code.trim();
    if carrier_code.is_empty() {
        return bad_request("carrier_code is required").into_response();
    }

    let repo = CarriersRepo::new(state.db.pool());
    let carrier = match repo
        .create_carrier(CarrierCreate {
            tenant_id,
            carrier_code: carrier_code.to_string(),
            name: name.to_string(),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let logs_repo = AuditLogsRepo::new(state.db.pool());
    let after_json = serde_json::to_value(&carrier).unwrap_or(serde_json::json!({}));
    if let Err(err) = logs_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "CARRIER.CREATE".to_string(),
            entity_type: "carrier".to_string(),
            entity_id: Some(carrier.carrier_code.clone()),
            before_json: None,
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("audit log failed: {err}"),
        )
            .into_response();
    }

    (StatusCode::OK, Json(carrier)).into_response()
}

pub(crate) async fn update_carrier(
    State(state): State<AppState>,
    Path((tenant_id, carrier_code)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<CarrierUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicCarrierWrite.as_str()) {
        return resp.into_response();
    }

    let name = req
        .name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if name.is_none() {
        return bad_request("name is required").into_response();
    }

    let repo = CarriersRepo::new(state.db.pool());
    let carrier = match repo
        .update_carrier(tenant_id, carrier_code.trim(), CarrierUpdate { name })
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("carrier not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let logs_repo = AuditLogsRepo::new(state.db.pool());
    let after_json = serde_json::to_value(&carrier).unwrap_or(serde_json::json!({}));
    if let Err(err) = logs_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "CARRIER.UPDATE".to_string(),
            entity_type: "carrier".to_string(),
            entity_id: Some(carrier.carrier_code.clone()),
            before_json: None,
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("audit log failed: {err}"),
        )
            .into_response();
    }

    (StatusCode::OK, Json(carrier)).into_response()
}

pub(crate) async fn delete_carrier(
    State(state): State<AppState>,
    Path((tenant_id, carrier_code)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicCarrierWrite.as_str()) {
        return resp.into_response();
    }

    let repo = CarriersRepo::new(state.db.pool());
    let carrier = match repo.delete_carrier(tenant_id, carrier_code.trim()).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("carrier not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let logs_repo = AuditLogsRepo::new(state.db.pool());
    let before_json = serde_json::to_value(&carrier).unwrap_or(serde_json::json!({}));
    if let Err(err) = logs_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "CARRIER.DELETE".to_string(),
            entity_type: "carrier".to_string(),
            entity_id: Some(carrier.carrier_code.clone()),
            before_json: Some(before_json),
            after_json: None,
            ip: None,
            ua: None,
        })
        .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("audit log failed: {err}"),
        )
            .into_response();
    }

    (StatusCode::OK, Json(carrier)).into_response()
}

pub(crate) async fn list_logistics_products(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<LogisticsProductListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicLogisticsProductRead.as_str()) {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let keyword = query
        .keyword
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let carrier_code = query
        .carrier_code
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = query
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());

    let repo = LogisticsProductsRepo::new(state.db.pool());
    let result = match repo
        .list_products(
            tenant_id,
            LogisticsProductListFilters {
                keyword,
                carrier_code,
                status,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = LogisticsProductListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_logistics_product(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LogisticsProductCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicLogisticsProductWrite.as_str())
    {
        return resp.into_response();
    }

    let carrier_code = req.carrier_code.trim();
    if carrier_code.is_empty() {
        return bad_request("carrier_code required").into_response();
    }
    let code = req.code.trim();
    if code.is_empty() {
        return bad_request("code required").into_response();
    }
    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name required").into_response();
    }
    let status = req
        .status
        .unwrap_or_else(|| "ACTIVE".to_string())
        .trim()
        .to_uppercase();
    if status.is_empty() {
        return bad_request("status required").into_response();
    }

    let repo = LogisticsProductsRepo::new(state.db.pool());
    let product = match repo
        .create_product(LogisticsProductCreate {
            tenant_id,
            carrier_code: carrier_code.to_string(),
            code: code.to_string(),
            name: name.to_string(),
            status,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "LOGISTICS_PRODUCT.CREATE".to_string(),
            entity_type: "logistics_product".to_string(),
            entity_id: Some(product.product_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&product).ok(),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(product)).into_response()
}

pub(crate) async fn update_logistics_product(
    State(state): State<AppState>,
    Path((tenant_id, product_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LogisticsProductUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let product_id = match uuid::Uuid::parse_str(&product_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad product_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicLogisticsProductWrite.as_str())
    {
        return resp.into_response();
    }

    let carrier_code = req
        .carrier_code
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let code = req
        .code
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let name = req
        .name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = req
        .status
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty());

    if carrier_code.is_none() && code.is_none() && name.is_none() && status.is_none() {
        return bad_request("no fields to update").into_response();
    }

    let repo = LogisticsProductsRepo::new(state.db.pool());
    let product = match repo
        .update_product(
            tenant_id,
            product_id,
            LogisticsProductUpdate {
                carrier_code,
                code,
                name,
                status,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("logistics product not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "LOGISTICS_PRODUCT.UPDATE".to_string(),
            entity_type: "logistics_product".to_string(),
            entity_id: Some(product.product_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&product).ok(),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(product)).into_response()
}

pub(crate) async fn delete_logistics_product(
    State(state): State<AppState>,
    Path((tenant_id, product_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let product_id = match uuid::Uuid::parse_str(&product_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad product_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicLogisticsProductWrite.as_str())
    {
        return resp.into_response();
    }

    let price_count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from logistics_prices
        where tenant_id = $1 and product_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(product_id)
    .fetch_one(state.db.pool())
    .await
    .unwrap_or(0);
    if price_count > 0 {
        return conflict("logistics product in use by prices").into_response();
    }

    let repo = LogisticsProductsRepo::new(state.db.pool());
    let product = match repo.delete_product(tenant_id, product_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("logistics product not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "LOGISTICS_PRODUCT.DELETE".to_string(),
            entity_type: "logistics_product".to_string(),
            entity_id: Some(product.product_id.to_string()),
            before_json: serde_json::to_value(&product).ok(),
            after_json: None,
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(product)).into_response()
}

pub(crate) async fn list_logistics_prices(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<LogisticsPriceListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicLogisticsPriceRead.as_str()) {
        return resp.into_response();
    }
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::LogisticsPrice).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::LogisticsPrice).into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let carrier_code = query
        .carrier_code
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let region_code = query
        .region_code
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let product_id = match query.product_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad product_id").into_response(),
        },
        None => None,
    };

    let repo = LogisticsPricesRepo::new(state.db.pool());
    let result = match repo
        .list_prices(
            tenant_id,
            LogisticsPriceListFilters {
                carrier_code,
                product_id,
                region_code,
                weight_kg: query.weight_kg,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = LogisticsPriceListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn estimate_logistics_price(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LogisticsPriceEstimateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicLogisticsPriceRead.as_str()) {
        return resp.into_response();
    }
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::LogisticsPrice).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::LogisticsPrice).into_response();
    }

    let shipment_id = match req.shipment_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad shipment_id").into_response(),
        },
        None => None,
    };
    let mut order_id = match req.order_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad order_id").into_response(),
        },
        None => None,
    };

    let mut carrier_code = req
        .carrier_code
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let mut region_code = req
        .region_code
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if let Some(weight) = req.weight_kg {
        if weight < 0.0 {
            return bad_request("weight_kg must be positive").into_response();
        }
    }
    let mut weight_kg = req.weight_kg;

    if let Some(shipment_id) = shipment_id {
        let repo = ShipmentsRepo::new(state.db.pool());
        let shipment = match repo.get_by_id(tenant_id, shipment_id).await {
            Ok(Some(v)) => v,
            Ok(None) => return not_found("shipment not found").into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        if carrier_code.is_none() {
            carrier_code = shipment
                .carrier_code
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
        }
        if weight_kg.is_none() {
            weight_kg = shipment.weight_gram.map(|value| value as f64 / 1000.0);
        }
        if order_id.is_none() {
            order_id = Some(shipment.order_id);
        }
    }

    let mut product_id = match req
        .product_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad product_id").into_response(),
        },
        None => None,
    };

    if product_id.is_none() {
        match load_latest_logistics_match_product_id(
            state.db.pool(),
            tenant_id,
            shipment_id,
            order_id,
        )
        .await
        {
            Ok(value) => product_id = value,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }
    }

    if region_code.is_none() {
        if let Some(order_id) = order_id {
            let orders_repo = OrdersRepo::new(state.db.pool());
            let detail = match orders_repo
                .get_order_detail(tenant_id, OrderId(order_id))
                .await
            {
                Ok(Some(v)) => v,
                Ok(None) => return not_found("order not found").into_response(),
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()
                }
            };
            region_code = detail
                .order
                .receiver_address
                .as_deref()
                .and_then(extract_province);
        }
    }

    let Some(carrier_code) = carrier_code else {
        return bad_request("carrier_code required").into_response();
    };
    let Some(region_code) = region_code else {
        return bad_request("region_code required").into_response();
    };
    let Some(weight_kg) = weight_kg else {
        return bad_request("weight_kg required").into_response();
    };

    let repo = LogisticsPricesRepo::new(state.db.pool());
    let price = match repo
        .find_price(
            tenant_id,
            &carrier_code,
            product_id,
            &region_code,
            weight_kg,
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = LogisticsPriceEstimateResponse {
        matched: price.is_some(),
        carrier_code,
        product_id,
        region_code,
        weight_kg,
        price,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_logistics_price(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LogisticsPriceCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicLogisticsPriceWrite.as_str()) {
        return resp.into_response();
    }
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::LogisticsPrice).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::LogisticsPrice).into_response();
    }

    let carrier_code = req.carrier_code.trim();
    if carrier_code.is_empty() {
        return bad_request("carrier_code required").into_response();
    }
    let region_code = req.region_code.trim();
    if region_code.is_empty() {
        return bad_request("region_code required").into_response();
    }
    if req.weight_min_kg < 0.0 || req.weight_max_kg < 0.0 {
        return bad_request("weight must be positive").into_response();
    }
    if req.weight_max_kg > 0.0 && req.weight_min_kg > req.weight_max_kg {
        return bad_request("weight_min_kg must be <= weight_max_kg").into_response();
    }

    let product_id = match req.product_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad product_id").into_response(),
        },
        None => None,
    };

    let repo = LogisticsPricesRepo::new(state.db.pool());
    let price = match repo
        .create_price(LogisticsPriceCreate {
            tenant_id,
            carrier_code: carrier_code.to_string(),
            product_id,
            region_code: region_code.to_string(),
            weight_min_kg: req.weight_min_kg,
            weight_max_kg: req.weight_max_kg,
            price_cent: req.price_cent,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "LOGISTICS_PRICE.CREATE".to_string(),
            entity_type: "logistics_price".to_string(),
            entity_id: Some(price.price_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&price).ok(),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(price)).into_response()
}

pub(crate) async fn update_logistics_price(
    State(state): State<AppState>,
    Path((tenant_id, price_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<LogisticsPriceUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let price_id = match uuid::Uuid::parse_str(&price_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad price_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicLogisticsPriceWrite.as_str()) {
        return resp.into_response();
    }
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::LogisticsPrice).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::LogisticsPrice).into_response();
    }

    let carrier_code = req
        .carrier_code
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let region_code = req
        .region_code
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let product_id = match req.product_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad product_id").into_response(),
        },
        None => None,
    };
    if let Some(weight_min) = req.weight_min_kg {
        if weight_min < 0.0 {
            return bad_request("weight_min_kg must be positive").into_response();
        }
    }
    if let Some(weight_max) = req.weight_max_kg {
        if weight_max < 0.0 {
            return bad_request("weight_max_kg must be positive").into_response();
        }
    }

    if carrier_code.is_none()
        && region_code.is_none()
        && product_id.is_none()
        && req.weight_min_kg.is_none()
        && req.weight_max_kg.is_none()
        && req.price_cent.is_none()
    {
        return bad_request("no fields to update").into_response();
    }

    let repo = LogisticsPricesRepo::new(state.db.pool());
    let price = match repo
        .update_price(
            tenant_id,
            price_id,
            LogisticsPriceUpdate {
                carrier_code,
                product_id,
                region_code,
                weight_min_kg: req.weight_min_kg,
                weight_max_kg: req.weight_max_kg,
                price_cent: req.price_cent,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("logistics price not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "LOGISTICS_PRICE.UPDATE".to_string(),
            entity_type: "logistics_price".to_string(),
            entity_id: Some(price.price_id.to_string()),
            before_json: None,
            after_json: serde_json::to_value(&price).ok(),
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(price)).into_response()
}

pub(crate) async fn delete_logistics_price(
    State(state): State<AppState>,
    Path((tenant_id, price_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let price_id = match uuid::Uuid::parse_str(&price_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad price_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::BasicLogisticsPriceWrite.as_str()) {
        return resp.into_response();
    }
    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::LogisticsPrice).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::LogisticsPrice).into_response();
    }

    let repo = LogisticsPricesRepo::new(state.db.pool());
    let price = match repo.delete_price(tenant_id, price_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("logistics price not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let _ = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "LOGISTICS_PRICE.DELETE".to_string(),
            entity_type: "logistics_price".to_string(),
            entity_id: Some(price.price_id.to_string()),
            before_json: serde_json::to_value(&price).ok(),
            after_json: None,
            ip: None,
            ua: None,
        })
        .await;

    (StatusCode::OK, Json(price)).into_response()
}

pub(crate) async fn list_shop_carrier_accounts(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ShopCarrierAccountListQuery>,
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

    let shop_id = match query.shop_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let carrier_code = query
        .carrier_code
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let repo = ShopCarrierAccountsRepo::new(state.db.pool());
    let result = match repo
        .list_accounts(
            tenant_id,
            ShopCarrierAccountListFilters {
                shop_id,
                carrier_code,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = ShopCarrierAccountListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_shop_carrier_account(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ShopCarrierAccountCreateRequest>,
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

    let shop_id = match uuid::Uuid::parse_str(req.shop_id.trim()) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    let carrier_code = req.carrier_code.trim();
    if carrier_code.is_empty() {
        return bad_request("carrier_code is required").into_response();
    }

    let repo = ShopCarrierAccountsRepo::new(state.db.pool());
    let account = match repo
        .create_account(ShopCarrierAccountCreate {
            tenant_id,
            shop_id,
            carrier_code: carrier_code.to_string(),
            account_meta_json: req
                .account_meta_json
                .unwrap_or_else(|| serde_json::json!({})),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let logs_repo = AuditLogsRepo::new(state.db.pool());
    let after_json = serde_json::to_value(&account).unwrap_or(serde_json::json!({}));
    if let Err(err) = logs_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "SHOP_CARRIER_ACCOUNT.CREATE".to_string(),
            entity_type: "shop_carrier_account".to_string(),
            entity_id: Some(account.account_id.to_string()),
            before_json: None,
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("audit log failed: {err}"),
        )
            .into_response();
    }

    (StatusCode::OK, Json(account)).into_response()
}

pub(crate) async fn update_shop_carrier_account(
    State(state): State<AppState>,
    Path((tenant_id, account_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ShopCarrierAccountUpdateRequest>,
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

    let account_id = match uuid::Uuid::parse_str(&account_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad account_id").into_response(),
    };
    if req.account_meta_json.is_none() {
        return bad_request("account_meta_json required").into_response();
    }

    let repo = ShopCarrierAccountsRepo::new(state.db.pool());
    let account = match repo
        .update_account(
            tenant_id,
            account_id,
            ShopCarrierAccountUpdate {
                account_meta_json: req.account_meta_json,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("account not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let logs_repo = AuditLogsRepo::new(state.db.pool());
    let after_json = serde_json::to_value(&account).unwrap_or(serde_json::json!({}));
    if let Err(err) = logs_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "SHOP_CARRIER_ACCOUNT.UPDATE".to_string(),
            entity_type: "shop_carrier_account".to_string(),
            entity_id: Some(account.account_id.to_string()),
            before_json: None,
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("audit log failed: {err}"),
        )
            .into_response();
    }

    (StatusCode::OK, Json(account)).into_response()
}

pub(crate) async fn delete_shop_carrier_account(
    State(state): State<AppState>,
    Path((tenant_id, account_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
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

    let account_id = match uuid::Uuid::parse_str(&account_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad account_id").into_response(),
    };

    let repo = ShopCarrierAccountsRepo::new(state.db.pool());
    let account = match repo.delete_account(tenant_id, account_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("account not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let logs_repo = AuditLogsRepo::new(state.db.pool());
    let before_json = serde_json::to_value(&account).unwrap_or(serde_json::json!({}));
    if let Err(err) = logs_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "SHOP_CARRIER_ACCOUNT.DELETE".to_string(),
            entity_type: "shop_carrier_account".to_string(),
            entity_id: Some(account.account_id.to_string()),
            before_json: Some(before_json),
            after_json: None,
            ip: None,
            ua: None,
        })
        .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("audit log failed: {err}"),
        )
            .into_response();
    }

    (StatusCode::OK, Json(account)).into_response()
}

pub(crate) async fn list_channels(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ChannelListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "channels.read") {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let keyword = query
        .keyword
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let status = match parse_channel_status(query.status) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };

    let result = match state
        .channels_repo
        .list_channels(
            tenant_id,
            ChannelListFilters {
                keyword,
                status,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = ChannelListResponse {
        items: result.items,
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_channel(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ChannelCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "channels.write") {
        return resp.into_response();
    }

    let platform = req.platform.trim();
    if platform.is_empty() {
        return bad_request("platform is required").into_response();
    }
    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name is required").into_response();
    }
    let department = req
        .department
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let owner = req
        .owner
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let allow_manual_order = req.allow_manual_order.unwrap_or(false);
    let allow_manual_after_sales = req.allow_manual_after_sales.unwrap_or(false);
    let default_warehouse_id = match req.default_warehouse_id.as_deref() {
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                match uuid::Uuid::parse_str(trimmed) {
                    Ok(v) => Some(v),
                    Err(_) => return bad_request("bad default_warehouse_id").into_response(),
                }
            }
        }
        None => None,
    };
    let commission_formula = req
        .commission_formula
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let channel = match state
        .channels_repo
        .create_channel(ChannelCreate {
            tenant_id,
            platform: platform.to_string(),
            name: name.to_string(),
            department,
            owner,
            allow_manual_order,
            allow_manual_after_sales,
            default_warehouse_id,
            commission_formula,
            status: "ACTIVE".to_string(),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(channel)).into_response()
}

pub(crate) async fn update_channel(
    State(state): State<AppState>,
    Path((tenant_id, channel_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ChannelUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let channel_id = match uuid::Uuid::parse_str(&channel_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad channel_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "channels.write") {
        return resp.into_response();
    }

    let platform = req
        .platform
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let name = req
        .name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let department = req
        .department
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let owner = req
        .owner
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let allow_manual_order = req.allow_manual_order;
    let allow_manual_after_sales = req.allow_manual_after_sales;
    let default_warehouse_id = match req.default_warehouse_id {
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                Some(None)
            } else {
                match uuid::Uuid::parse_str(trimmed) {
                    Ok(v) => Some(Some(v)),
                    Err(_) => return bad_request("bad default_warehouse_id").into_response(),
                }
            }
        }
        None => None,
    };
    let commission_formula = req.commission_formula.map(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    let status = match parse_channel_status(req.status) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if platform.is_none()
        && name.is_none()
        && department.is_none()
        && owner.is_none()
        && allow_manual_order.is_none()
        && allow_manual_after_sales.is_none()
        && default_warehouse_id.is_none()
        && commission_formula.is_none()
        && status.is_none()
    {
        return bad_request("no fields to update").into_response();
    }

    let channel = match state
        .channels_repo
        .update_channel(
            tenant_id,
            channel_id,
            ChannelUpdate {
                platform,
                name,
                department,
                owner,
                allow_manual_order,
                allow_manual_after_sales,
                default_warehouse_id,
                commission_formula,
                status,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("channel not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(channel)).into_response()
}

pub(crate) async fn delete_channel(
    State(state): State<AppState>,
    Path((tenant_id, channel_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let channel_id = match uuid::Uuid::parse_str(&channel_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad channel_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "channels.write") {
        return resp.into_response();
    }

    let channel = match state
        .channels_repo
        .delete_channel(tenant_id, channel_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("channel not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(channel)).into_response()
}
