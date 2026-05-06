use super::*;

pub(crate) async fn list_skus(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<SkuListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.read") {
        return resp.into_response();
    }

    let product_id = match query.product_id.as_deref() {
        Some(v) => match uuid::Uuid::parse_str(v) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad product_id").into_response(),
        },
        None => None,
    };
    let cursor = match query.cursor.as_deref() {
        Some(raw) => match SkuCursor::decode(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad cursor").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let result = match state
        .skus_repo
        .list_skus(
            tenant_id,
            SkuListFilters {
                product_id,
                sku_code: query.sku_code.and_then(non_empty),
                barcode: query.barcode.and_then(non_empty),
                keyword: query.keyword.and_then(non_empty),
                is_active: query.is_active,
                cursor,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response: ListResponse<Sku> = ListResponse {
        data: result.items,
        next_cursor: result.next_cursor.map(|c| c.encode()),
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn get_sku_detail(
    State(state): State<AppState>,
    Path((tenant_id, sku_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.read") {
        return resp.into_response();
    }

    let sku_id = match uuid::Uuid::parse_str(&sku_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };

    let sku = match state.skus_repo.get_sku(tenant_id, sku_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("sku not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let product = match state
        .products_repo
        .get_product(tenant_id, sku.product_id)
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let suppliers = match state
        .sku_suppliers_repo
        .list_sku_suppliers(tenant_id, sku.sku_id)
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(SkuDetailResponse {
            sku,
            product,
            suppliers,
        }),
    )
        .into_response()
}

pub(crate) async fn list_product_packaging_specs(
    State(state): State<AppState>,
    Path((tenant_id, sku_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.read") {
        return resp.into_response();
    }

    let sku_id = match uuid::Uuid::parse_str(&sku_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };

    let repo = ProductPackagingSpecsRepo::new(state.db.pool());
    let items = match repo.list_by_sku(tenant_id, sku_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = ProductPackagingSpecListResponse {
        items: items
            .into_iter()
            .map(product_packaging_spec_response)
            .collect(),
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn upsert_default_product_packaging_spec(
    State(state): State<AppState>,
    Path((tenant_id, sku_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProductPackagingSpecUpsertRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    if req.level_code.trim().is_empty() {
        return bad_request("level_code required").into_response();
    }
    if req.unit_name.trim().is_empty() {
        return bad_request("unit_name required").into_response();
    }
    if req.units_per_level <= 0 {
        return bad_request("units_per_level must be positive").into_response();
    }

    let sku_id = match uuid::Uuid::parse_str(&sku_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };

    let repo = ProductPackagingSpecsRepo::new(state.db.pool());
    let spec = match repo
        .upsert_default(ProductPackagingSpecUpsert {
            tenant_id,
            sku_id,
            level_code: req.level_code.trim().to_string(),
            unit_name: req.unit_name.trim().to_string(),
            units_per_level: req.units_per_level,
            length_mm: req.length_mm,
            width_mm: req.width_mm,
            height_mm: req.height_mm,
            weight_gram: req.weight_gram,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(product_packaging_spec_response(spec))).into_response()
}

pub(crate) async fn create_sku(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<SkuCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let product_id = match uuid::Uuid::parse_str(&req.product_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad product_id").into_response(),
    };
    let tags = normalize_string_list(req.tags.unwrap_or_default());
    let image_urls = normalize_string_list(req.image_urls.unwrap_or_default());
    let supplier_ids = match req.supplier_ids {
        Some(values) => match parse_uuid_list(&values) {
            Ok(list) => Some(list),
            Err(resp) => return resp.into_response(),
        },
        None => None,
    };
    let primary_supplier_id = match req.primary_supplier_id.as_deref() {
        Some(value) => match uuid::Uuid::parse_str(value) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad primary_supplier_id").into_response(),
        },
        None => None,
    };

    let sku = match state
        .skus_repo
        .create_sku(SkuCreate {
            tenant_id,
            product_id,
            sku_code: req.sku_code,
            barcode: req.barcode,
            name: req.name,
            note: req.note,
            spec: req.spec.unwrap_or_else(|| serde_json::json!({})),
            tags,
            image_urls,
            weight_gram: req.weight_gram,
            length_mm: req.length_mm,
            width_mm: req.width_mm,
            height_mm: req.height_mm,
            volume_cm3: req.volume_cm3,
            sku_kind: req.sku_kind.unwrap_or_else(|| "NORMAL".to_string()),
            cost_cent: req.cost_cent,
            is_active: req.is_active.unwrap_or(true),
            status: req.status,
            batch_enabled: req.batch_enabled.unwrap_or(false),
            serial_enabled: req.serial_enabled.unwrap_or(false),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    if let Some(supplier_ids) = supplier_ids {
        if let Err(e) = state
            .sku_suppliers_repo
            .replace_sku_suppliers(tenant_id, sku.sku_id, supplier_ids, primary_supplier_id)
            .await
        {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
        }
    }

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SKUS.CREATE",
        "SKU",
        Some(sku.sku_id.to_string()),
        None,
        Some(serde_json::json!({
            "sku_id": sku.sku_id,
            "product_id": sku.product_id,
            "sku_code": sku.sku_code,
            "barcode": sku.barcode,
            "sku_kind": sku.sku_kind,
        })),
    )
    .await;

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_SKU_CREATED.to_string(),
            ref_type: Some("SKU".to_string()),
            ref_id: Some(sku.sku_id.to_string()),
            metadata: Some(serde_json::json!({
                "sku_id": sku.sku_id,
                "product_id": sku.product_id,
                "sku_code": sku.sku_code.clone(),
                "barcode": sku.barcode.clone(),
                "sku_kind": sku.sku_kind.clone(),
            })),
        })
        .await;

    (StatusCode::OK, Json(sku)).into_response()
}

pub(crate) async fn update_sku(
    State(state): State<AppState>,
    Path((tenant_id, sku_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<SkuUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }
    let sku_id = match uuid::Uuid::parse_str(&sku_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };
    let tags = req.tags.map(normalize_string_list);
    let image_urls = req.image_urls.map(normalize_string_list);
    let supplier_ids = match req.supplier_ids {
        Some(values) => match parse_uuid_list(&values) {
            Ok(list) => Some(list),
            Err(resp) => return resp.into_response(),
        },
        None => None,
    };
    let primary_supplier_id = match req.primary_supplier_id.as_deref() {
        Some(value) => match uuid::Uuid::parse_str(value) {
            Ok(id) => Some(id),
            Err(_) => return bad_request("bad primary_supplier_id").into_response(),
        },
        None => None,
    };

    let sku = match state
        .skus_repo
        .update_sku(
            tenant_id,
            sku_id,
            SkuUpdate {
                sku_code: req.sku_code,
                barcode: req.barcode,
                name: req.name,
                note: req.note,
                spec: req.spec,
                tags,
                image_urls,
                weight_gram: req.weight_gram,
                length_mm: req.length_mm,
                width_mm: req.width_mm,
                height_mm: req.height_mm,
                volume_cm3: req.volume_cm3,
                sku_kind: req.sku_kind,
                cost_cent: req.cost_cent,
                is_active: req.is_active,
                status: req.status,
                batch_enabled: req.batch_enabled,
                serial_enabled: req.serial_enabled,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("sku not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    if let Some(supplier_ids) = supplier_ids {
        if let Err(e) = state
            .sku_suppliers_repo
            .replace_sku_suppliers(tenant_id, sku.sku_id, supplier_ids, primary_supplier_id)
            .await
        {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
        }
    }

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SKUS.UPDATE",
        "SKU",
        Some(sku.sku_id.to_string()),
        None,
        Some(serde_json::json!({
            "sku_id": sku.sku_id,
            "product_id": sku.product_id,
            "sku_code": sku.sku_code,
            "barcode": sku.barcode,
            "sku_kind": sku.sku_kind,
        })),
    )
    .await;

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_SKU_UPDATED.to_string(),
            ref_type: Some("SKU".to_string()),
            ref_id: Some(sku.sku_id.to_string()),
            metadata: Some(serde_json::json!({
                "sku_id": sku.sku_id,
                "product_id": sku.product_id,
                "sku_code": sku.sku_code.clone(),
                "barcode": sku.barcode.clone(),
                "sku_kind": sku.sku_kind.clone(),
            })),
        })
        .await;

    (StatusCode::OK, Json(sku)).into_response()
}

pub(crate) async fn delete_sku(
    State(state): State<AppState>,
    Path((tenant_id, sku_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let sku_id = match uuid::Uuid::parse_str(&sku_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };

    let sku = match state.skus_repo.delete_sku(tenant_id, sku_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("sku not found").into_response(),
        Err(e) => {
            let message = e.to_string();
            if message.contains("kit has active orders") {
                return conflict("kit has active orders").into_response();
            }
            return (StatusCode::INTERNAL_SERVER_ERROR, message).into_response();
        }
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SKUS.DELETE",
        "SKU",
        Some(sku.sku_id.to_string()),
        None,
        Some(serde_json::json!({
            "sku_id": sku.sku_id,
            "sku_code": sku.sku_code,
        })),
    )
    .await;

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_SKU_DELETED.to_string(),
            ref_type: Some("SKU".to_string()),
            ref_id: Some(sku.sku_id.to_string()),
            metadata: Some(serde_json::json!({
                "sku_id": sku.sku_id,
                "sku_code": sku.sku_code.clone(),
            })),
        })
        .await;

    (StatusCode::OK, Json(sku)).into_response()
}

pub(crate) async fn copy_sku(
    State(state): State<AppState>,
    Path((tenant_id, sku_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let sku_id = match uuid::Uuid::parse_str(&sku_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };
    let sku = match state.skus_repo.get_sku(tenant_id, sku_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("sku not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let new_sku_code = format!("{}-COPY-{}", sku.sku_code, &suffix[..8]);

    let copied = match state
        .skus_repo
        .create_sku(SkuCreate {
            tenant_id,
            product_id: sku.product_id,
            sku_code: new_sku_code,
            barcode: sku.barcode.clone(),
            name: sku.name.clone(),
            note: sku.note.clone(),
            spec: sku.spec.clone(),
            tags: sku.tags.clone(),
            image_urls: sku.image_urls.clone(),
            weight_gram: sku.weight_gram,
            length_mm: sku.length_mm,
            width_mm: sku.width_mm,
            height_mm: sku.height_mm,
            volume_cm3: sku.volume_cm3,
            sku_kind: sku.sku_kind.clone(),
            cost_cent: sku.cost_cent,
            is_active: sku.is_active,
            status: Some(sku.status.clone()),
            batch_enabled: sku.batch_enabled,
            serial_enabled: sku.serial_enabled,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let suppliers = match state
        .sku_suppliers_repo
        .list_sku_suppliers(tenant_id, sku.sku_id)
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if !suppliers.is_empty() {
        let supplier_ids = suppliers.iter().map(|s| s.supplier_id).collect::<Vec<_>>();
        let primary_supplier_id = suppliers
            .iter()
            .find(|s| s.is_primary)
            .map(|s| s.supplier_id);
        if let Err(e) = state
            .sku_suppliers_repo
            .replace_sku_suppliers(tenant_id, copied.sku_id, supplier_ids, primary_supplier_id)
            .await
        {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
        }
    }

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SKUS.COPY",
        "SKU",
        Some(copied.sku_id.to_string()),
        None,
        Some(serde_json::json!({
            "source_sku_id": sku.sku_id,
            "sku_code": copied.sku_code,
        })),
    )
    .await;

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_SKU_COPIED.to_string(),
            ref_type: Some("SKU".to_string()),
            ref_id: Some(copied.sku_id.to_string()),
            metadata: Some(serde_json::json!({
                "source_sku_id": sku.sku_id,
                "sku_code": copied.sku_code.clone(),
            })),
        })
        .await;

    (StatusCode::OK, Json(copied)).into_response()
}

pub(crate) async fn search_skus(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<SkuSearchQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.read") {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(20).clamp(1, 200);
    let items = match state
        .skus_repo
        .search_skus(
            tenant_id,
            SkuSearchFilters {
                keyword: query.keyword.and_then(non_empty),
                sku_kind: query.sku_kind.and_then(non_empty),
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = SkuSearchResponse {
        items: items
            .into_iter()
            .map(|item| SkuSearchItem {
                sku_id: item.sku_id,
                sku_code: item.sku_code,
                name: item.name,
                barcode: item.barcode,
                sku_kind: item.sku_kind,
            })
            .collect(),
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn precheck_sku_import(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SkuImportPrecheckRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let file_type =
        match resolve_import_file_type(req.file_type.as_deref(), req.file_name.as_deref()) {
            Some(v) => v,
            None => return bad_request("unsupported file type").into_response(),
        };

    let content = match decode_base64_payload(&req.content_base64) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };

    let rows = match parse_sku_import_rows(&content, &file_type) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };

    let total = rows.len() as i64;
    let mut errors = Vec::new();
    let mut invalid_rows = HashSet::new();
    let mut preview = Vec::new();
    let mut seen_sku_codes = HashSet::new();

    let sku_codes: Vec<String> = rows
        .iter()
        .map(|row| row.sku_code.trim().to_string())
        .filter(|code| !code.is_empty())
        .collect();
    let existing_sku_codes =
        match fetch_existing_sku_codes(state.db.pool(), tenant_id, &sku_codes).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };

    let product_ids: Vec<uuid::Uuid> = rows
        .iter()
        .filter_map(|row| row.product_id.as_deref())
        .filter_map(|raw| uuid::Uuid::parse_str(raw.trim()).ok())
        .collect();
    let existing_product_ids =
        match fetch_existing_product_ids(state.db.pool(), tenant_id, &product_ids).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    let existing_product_categories =
        match fetch_product_categories_by_id(state.db.pool(), tenant_id, &product_ids).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    let existing_categories =
        match fetch_existing_product_categories(state.db.pool(), tenant_id).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };

    let supplier_ids: Vec<uuid::Uuid> = rows
        .iter()
        .flat_map(|row| row.supplier_ids.iter())
        .filter_map(|raw| uuid::Uuid::parse_str(raw.trim()).ok())
        .collect();
    let existing_supplier_ids =
        match fetch_existing_supplier_ids(state.db.pool(), tenant_id, &supplier_ids).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };

    for row in &rows {
        let row_number = row.row;
        let sku_code = row.sku_code.trim();
        if sku_code.is_empty() {
            push_import_issue(
                &mut errors,
                &mut invalid_rows,
                row_number,
                Some("sku_code"),
                "sku_code required",
            );
        } else {
            let normalized = sku_code.to_uppercase();
            if !seen_sku_codes.insert(normalized) {
                push_import_issue(
                    &mut errors,
                    &mut invalid_rows,
                    row_number,
                    Some("sku_code"),
                    "duplicate sku_code in file",
                );
            }
            if existing_sku_codes.contains(sku_code) {
                push_import_issue(
                    &mut errors,
                    &mut invalid_rows,
                    row_number,
                    Some("sku_code"),
                    "sku_code already exists",
                );
            }
        }

        if row
            .product_id
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .is_none()
            && row
                .product_name
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .is_none()
        {
            push_import_issue(
                &mut errors,
                &mut invalid_rows,
                row_number,
                Some("product_id"),
                "product_id or product_name required",
            );
        }

        if let Some(raw) = row
            .product_id
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            match uuid::Uuid::parse_str(raw) {
                Ok(id) => {
                    if !existing_product_ids.contains(&id) {
                        push_import_issue(
                            &mut errors,
                            &mut invalid_rows,
                            row_number,
                            Some("product_id"),
                            "product not found",
                        );
                    }
                }
                Err(_) => push_import_issue(
                    &mut errors,
                    &mut invalid_rows,
                    row_number,
                    Some("product_id"),
                    "invalid product_id",
                ),
            }
        }

        let unit = row.unit.as_deref().map(str::trim).filter(|v| !v.is_empty());
        if unit.is_none() {
            push_import_issue(
                &mut errors,
                &mut invalid_rows,
                row_number,
                Some("unit"),
                "unit required",
            );
        }

        let category_id = row
            .category_id
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty());
        let category_name = row
            .category_name
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty());
        let category = match (category_id, category_name) {
            (Some(id), Some(name)) if id != name => {
                push_import_issue(
                    &mut errors,
                    &mut invalid_rows,
                    row_number,
                    Some("category"),
                    "category_id and category_name mismatch",
                );
                None
            }
            (Some(id), _) => Some(id),
            (None, Some(name)) => Some(name),
            (None, None) => None,
        };
        let category_value = category.map(|value| value.to_string());

        if category_value.is_none() {
            push_import_issue(
                &mut errors,
                &mut invalid_rows,
                row_number,
                Some("category"),
                "category required",
            );
        } else if let Some(value) = category_value.as_deref() {
            if !existing_categories.contains(value) {
                push_import_issue(
                    &mut errors,
                    &mut invalid_rows,
                    row_number,
                    Some("category"),
                    "category not found",
                );
            }
        }

        if let (Some(raw_product_id), Some(category)) = (
            row.product_id
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty()),
            category_value.as_deref(),
        ) {
            if let Ok(product_id) = uuid::Uuid::parse_str(raw_product_id) {
                if let Some(existing_category) = existing_product_categories.get(&product_id) {
                    match existing_category.as_deref().map(str::trim) {
                        Some(value) if !value.is_empty() && value != category => {
                            push_import_issue(
                                &mut errors,
                                &mut invalid_rows,
                                row_number,
                                Some("category"),
                                "product category mismatch",
                            );
                        }
                        None => {
                            push_import_issue(
                                &mut errors,
                                &mut invalid_rows,
                                row_number,
                                Some("category"),
                                "product category missing",
                            );
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Some(kind) = row
            .sku_kind
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            let normalized = kind.to_uppercase();
            if normalized != "NORMAL" && normalized != "KIT" {
                push_import_issue(
                    &mut errors,
                    &mut invalid_rows,
                    row_number,
                    Some("sku_kind"),
                    "sku_kind must be NORMAL or KIT",
                );
            }
        }

        let cost_cent = parse_optional_i64_field(
            row.cost_cent.as_deref(),
            "cost_cent",
            row_number,
            &mut errors,
            &mut invalid_rows,
        );
        if let Some(cost_cent) = cost_cent {
            if cost_cent < 0 {
                push_import_issue(
                    &mut errors,
                    &mut invalid_rows,
                    row_number,
                    Some("cost_cent"),
                    "cost_cent must be >= 0",
                );
            }
        }

        parse_optional_i32_field(
            row.weight_gram.as_deref(),
            "weight_gram",
            row_number,
            &mut errors,
            &mut invalid_rows,
        );
        parse_optional_i32_field(
            row.length_mm.as_deref(),
            "length_mm",
            row_number,
            &mut errors,
            &mut invalid_rows,
        );
        parse_optional_i32_field(
            row.width_mm.as_deref(),
            "width_mm",
            row_number,
            &mut errors,
            &mut invalid_rows,
        );
        parse_optional_i32_field(
            row.height_mm.as_deref(),
            "height_mm",
            row_number,
            &mut errors,
            &mut invalid_rows,
        );
        parse_optional_i32_field(
            row.volume_cm3.as_deref(),
            "volume_cm3",
            row_number,
            &mut errors,
            &mut invalid_rows,
        );

        for raw in &row.supplier_ids {
            if raw.trim().is_empty() {
                continue;
            }
            match uuid::Uuid::parse_str(raw.trim()) {
                Ok(id) => {
                    if !existing_supplier_ids.contains(&id) {
                        push_import_issue(
                            &mut errors,
                            &mut invalid_rows,
                            row_number,
                            Some("supplier_ids"),
                            "supplier not found",
                        );
                    }
                }
                Err(_) => push_import_issue(
                    &mut errors,
                    &mut invalid_rows,
                    row_number,
                    Some("supplier_ids"),
                    "invalid supplier_id",
                ),
            }
        }

        if preview.len() < 10 {
            preview.push(SkuImportPreviewRow {
                row: row_number,
                product_id: row.product_id.clone(),
                product_name: row.product_name.clone(),
                sku_code: sku_code.to_string(),
                barcode: row.barcode.clone(),
                name: row.name.clone(),
                sku_kind: row
                    .sku_kind
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(|v| v.to_uppercase()),
                tags: row.tags.clone(),
                image_urls: row.image_urls.clone(),
                cost_cent,
                supplier_ids: row.supplier_ids.clone(),
            });
        }
    }

    let valid = total - invalid_rows.len() as i64;
    let response = SkuImportPrecheckResponse {
        total,
        valid,
        errors,
        preview,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_sku_import(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SkuImportRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let file_type =
        match resolve_import_file_type(req.file_type.as_deref(), req.file_name.as_deref()) {
            Some(v) => v,
            None => return bad_request("unsupported file type").into_response(),
        };

    let content = match decode_base64_payload(&req.content_base64) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };

    let file_path = match write_sku_import_file(&content, tenant_id, &file_type) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };

    let params = serde_json::json!({
        "file_path": file_path.to_string_lossy(),
        "file_type": file_type,
        "file_name": req.file_name,
    });

    let repo = ExportJobsRepo::new(state.db.pool());
    let job = match repo
        .create_job(ExportJobCreateInput {
            tenant_id,
            job_type: "SKU_IMPORT".to_string(),
            params_json: params,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(export_job_response(job))).into_response()
}

pub(crate) async fn download_sku_import_template(
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
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let template_path = sku_import_template_path();
    let file_bytes = match tokio::fs::read(&template_path).await {
        Ok(bytes) => bytes,
        Err(_) => return not_found("template not found").into_response(),
    };
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ),
    );
    let disposition = "attachment; filename=\"sku-import-template.xlsx\"";
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(disposition).unwrap_or_else(|_| HeaderValue::from_static("")),
    );

    (StatusCode::OK, headers, file_bytes).into_response()
}

pub(crate) async fn upload_sku_import_file(
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let mut file_name = None;
    let mut file_bytes = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        if let Some(name) = field.file_name().map(|value| value.to_string()) {
            file_name = Some(name);
            file_bytes = field.bytes().await.ok().map(|value| value.to_vec());
            break;
        }
    }

    let file_name = match file_name {
        Some(name) => name,
        None => return bad_request("missing file").into_response(),
    };
    let content = match file_bytes {
        Some(bytes) => bytes,
        None => return bad_request("empty file").into_response(),
    };
    let file_type = match resolve_import_file_type(None, Some(&file_name)) {
        Some(value) => value,
        None => return bad_request("unsupported file type").into_response(),
    };
    let file_id = uuid::Uuid::new_v4();
    let file_path = sku_import_file_path(tenant_id, file_id, &file_type);
    if let Some(parent) = file_path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to create import directory: {err}"),
            )
                .into_response();
        }
    }
    if let Err(err) = fs::write(&file_path, &content) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to write import file: {err}"),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(SkuImportUploadResponse {
            file_id,
            file_name,
            file_type,
        }),
    )
        .into_response()
}

pub(crate) async fn create_sku_import_job(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<SkuImportJobCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let file_id = match uuid::Uuid::parse_str(&req.file_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad file_id").into_response(),
    };
    let file_type =
        match resolve_import_file_type(req.file_type.as_deref(), req.file_name.as_deref()) {
            Some(v) => v,
            None => return bad_request("unsupported file type").into_response(),
        };
    let duplicate_strategy = match normalize_duplicate_strategy(req.duplicate_strategy.as_deref()) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    let file_path = sku_import_file_path(tenant_id, file_id, &file_type);
    if !file_path.exists() {
        return not_found("import file not found").into_response();
    }

    let params = serde_json::json!({
        "file_path": file_path.to_string_lossy(),
        "file_type": file_type,
        "file_name": req.file_name,
        "file_id": file_id.to_string(),
        "duplicate_strategy": duplicate_strategy,
    });

    let repo = ExportJobsRepo::new(state.db.pool());
    let job = match repo
        .create_job(ExportJobCreateInput {
            tenant_id,
            job_type: "SKU_IMPORT".to_string(),
            params_json: params,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(build_sku_import_job_response(job))).into_response()
}

pub(crate) async fn get_sku_import_job(
    State(state): State<AppState>,
    Path((tenant_id, job_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let job_id = match uuid::Uuid::parse_str(&job_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad job_id").into_response(),
    };
    let repo = ExportJobsRepo::new(state.db.pool());
    let job = match repo.get_job(tenant_id, job_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("import job not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    if job.job_type != "SKU_IMPORT" {
        return bad_request("invalid import job type").into_response();
    }

    (StatusCode::OK, Json(build_sku_import_job_response(job))).into_response()
}

pub(crate) async fn list_sku_import_failures(
    State(_state): State<AppState>,
    Path((tenant_id, job_id)): Path<(String, String)>,
    Query(query): Query<SkuImportFailureListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }
    let job_id = match uuid::Uuid::parse_str(&job_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad job_id").into_response(),
    };

    let file_path = sku_import_failure_file_path(tenant_id, job_id);
    if !file_path.exists() {
        return (
            StatusCode::OK,
            Json(SkuImportFailureListResponse {
                items: vec![],
                total: 0,
            }),
        )
            .into_response();
    }

    let mut bytes = match tokio::fs::read(&file_path).await {
        Ok(v) => v,
        Err(_) => return not_found("failure log not found").into_response(),
    };
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        bytes.drain(..3);
    }

    let mut reader = ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(Cursor::new(bytes));
    let mut items = Vec::new();
    for record in reader.records().flatten() {
        let row = record
            .get(0)
            .and_then(|value| value.trim().parse::<i64>().ok())
            .unwrap_or_default();
        let sku_code = record.get(1).map(|value| value.trim().to_string());
        let field = record.get(2).map(|value| value.trim().to_string());
        let message = record
            .get(3)
            .map(|value| value.trim().to_string())
            .unwrap_or_default();
        items.push(SkuImportFailureRow {
            row,
            sku_code: sku_code.filter(|value| !value.is_empty()),
            field: field.filter(|value| !value.is_empty()),
            message,
        });
    }

    let offset = query.offset.unwrap_or(0).max(0) as usize;
    let limit = query.limit.unwrap_or(50).clamp(1, 200) as usize;
    let total = items.len() as i64;
    let page_items = items.into_iter().skip(offset).take(limit).collect();

    (
        StatusCode::OK,
        Json(SkuImportFailureListResponse {
            items: page_items,
            total,
        }),
    )
        .into_response()
}

pub(crate) async fn download_sku_import_failures(
    Path((tenant_id, job_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }
    let job_id = match uuid::Uuid::parse_str(&job_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad job_id").into_response(),
    };

    let file_path = sku_import_failure_file_path(tenant_id, job_id);
    let file_bytes = match tokio::fs::read(&file_path).await {
        Ok(bytes) => bytes,
        Err(_) => return not_found("failure log not found").into_response(),
    };
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/csv"));
    let filename = format!("sku-import-failures-{job_id}.csv");
    let disposition = format!("attachment; filename=\"{filename}\"");
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&disposition).unwrap_or_else(|_| HeaderValue::from_static("")),
    );

    (StatusCode::OK, headers, file_bytes).into_response()
}

pub(crate) async fn list_sku_cost_ledgers(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<SkuCostLedgerListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.read") {
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
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad sku_id").into_response(),
        },
        None => None,
    };
    let start_at = match query.start_at.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad start_at").into_response(),
        },
        None => None,
    };
    let end_at = match query.end_at.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad end_at").into_response(),
        },
        None => None,
    };
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let result = match state
        .sku_costs_repo
        .list_cost_ledgers(
            tenant_id,
            SkuCostLedgerListFilters {
                sku_id,
                start_at,
                end_at,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(SkuCostLedgerListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn adjust_sku_cost(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<SkuCostAdjustRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
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

    let sku_id = match uuid::Uuid::parse_str(req.sku_id.trim()) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };

    let ledger = match state
        .sku_costs_repo
        .adjust_cost(SkuCostAdjustInput {
            tenant_id,
            sku_id,
            new_cost_cent: req.new_cost_cent,
            ref_type: req.ref_type.clone(),
            ref_id: req.ref_id.clone(),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return bad_request(&e.to_string()).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SKUS.COST_ADJUST",
        "SKU",
        Some(ledger.sku_id.to_string()),
        None,
        Some(serde_json::json!({
            "sku_id": ledger.sku_id,
            "previous_cost_cent": ledger.previous_cost_cent,
            "new_cost_cent": ledger.new_cost_cent,
        })),
    )
    .await;

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_SKU_COST_ADJUSTED.to_string(),
            ref_type: Some("SKU".to_string()),
            ref_id: Some(ledger.sku_id.to_string()),
            metadata: Some(serde_json::json!({
                "sku_id": ledger.sku_id,
                "previous_cost_cent": ledger.previous_cost_cent,
                "new_cost_cent": ledger.new_cost_cent,
                "ref_type": req.ref_type,
                "ref_id": req.ref_id,
            })),
        })
        .await;

    (StatusCode::OK, Json(ledger)).into_response()
}

pub(crate) async fn list_kits(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<KitListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.read") {
        return resp.into_response();
    }

    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let result = match state
        .kits_repo
        .list_kits(
            tenant_id,
            KitListFilters {
                keyword: query.keyword.and_then(non_empty),
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(result)).into_response()
}

pub(crate) async fn get_kit_detail(
    State(state): State<AppState>,
    Path((tenant_id, kit_sku_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.read") {
        return resp.into_response();
    }

    let kit_sku_id = match uuid::Uuid::parse_str(&kit_sku_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad kit_sku_id").into_response(),
    };

    let detail = match state.kits_repo.get_kit_detail(tenant_id, kit_sku_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("kit not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(detail)).into_response()
}

pub(crate) async fn save_kit_bom(
    State(state): State<AppState>,
    Path((tenant_id, kit_sku_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<KitBomSaveRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let kit_sku_id = match uuid::Uuid::parse_str(&kit_sku_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad kit_sku_id").into_response(),
    };

    let mut components = Vec::with_capacity(req.components.len());
    for component in req.components {
        if component.qty <= 0 {
            return bad_request("component qty must be > 0").into_response();
        }
        let component_sku_id = match uuid::Uuid::parse_str(&component.component_sku_id) {
            Ok(v) => v,
            Err(_) => return bad_request("bad component_sku_id").into_response(),
        };
        let expire_at = match component.expire_at.as_deref() {
            Some(raw) => match DateTime::parse_from_rfc3339(raw) {
                Ok(v) => Some(v.with_timezone(&Utc)),
                Err(_) => return bad_request("bad expire_at").into_response(),
            },
            None => None,
        };
        components.push(erp_repo::kits::KitBomComponentInput {
            component_sku_id,
            qty: component.qty,
            expire_at,
        });
    }

    let log_components: Vec<Value> = components
        .iter()
        .map(|component| {
            serde_json::json!({
                "component_sku_id": component.component_sku_id,
                "qty": component.qty,
                "expire_at": component.expire_at,
            })
        })
        .collect();

    let detail = match state
        .kits_repo
        .save_kit_bom(KitBomSaveInput {
            tenant_id,
            kit_sku_id,
            components,
        })
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("kit not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let log_metadata = serde_json::json!({
        "kit_sku_id": detail.kit_sku.sku_id,
        "kit_sku_code": detail.kit_sku.sku_code.clone(),
        "component_count": detail.components.len(),
        "components": log_components,
    });

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "KITS.BOM_SAVE",
        "KIT",
        Some(detail.kit_sku.sku_id.to_string()),
        None,
        Some(log_metadata.clone()),
    )
    .await;

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_KIT_BOM_SAVED.to_string(),
            ref_type: Some("KIT".to_string()),
            ref_id: Some(detail.kit_sku.sku_id.to_string()),
            metadata: Some(log_metadata),
        })
        .await;

    (StatusCode::OK, Json(detail)).into_response()
}

pub(crate) async fn list_sku_batches(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<SkuBatchListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.read") {
        return resp.into_response();
    }

    let sku_id = match query.sku_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad sku_id").into_response(),
        },
        None => None,
    };
    let cursor = match query.cursor.as_deref() {
        Some(raw) => match SkuBatchCursor::decode(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad cursor").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let result = match state
        .sku_batches_repo
        .list_batches(
            tenant_id,
            SkuBatchListFilters {
                sku_id,
                batch_no: query.batch_no.and_then(non_empty),
                cursor,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = ListResponse {
        data: result.items,
        next_cursor: result.next_cursor.map(|c| c.encode()),
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn list_sku_batch_strategies(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<SkuBatchStrategyListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.read") {
        return resp.into_response();
    }

    let sku_id = match query.sku_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad sku_id").into_response(),
        },
        None => None,
    };
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let result = match SkuBatchStrategiesRepo::new(state.db.pool())
        .list_strategies(
            tenant_id,
            SkuBatchStrategyListFilters {
                sku_id,
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

    (
        StatusCode::OK,
        Json(SkuBatchStrategyListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn create_sku_batch_strategy(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<SkuBatchStrategyCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }
    if req.name.trim().is_empty() {
        return bad_request("name required").into_response();
    }
    let sku_id = match req.sku_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad sku_id").into_response(),
        },
        None => None,
    };

    let strategy = match SkuBatchStrategiesRepo::new(state.db.pool())
        .create_strategy(SkuBatchStrategyCreate {
            tenant_id,
            sku_id,
            name: req.name.trim().to_string(),
            enabled: req.enabled.unwrap_or(true),
            expire_threshold_days: req.expire_threshold_days.unwrap_or(0),
            fifo_enabled: req.fifo_enabled.unwrap_or(false),
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
        "SKU_BATCH_STRATEGIES.CREATE",
        "SKU_BATCH_STRATEGY",
        Some(strategy.strategy_id.to_string()),
        None,
        Some(serde_json::json!({
            "name": strategy.name,
            "sku_id": strategy.sku_id,
            "enabled": strategy.enabled,
        })),
    )
    .await;

    (StatusCode::OK, Json(strategy)).into_response()
}

pub(crate) async fn update_sku_batch_strategy(
    State(state): State<AppState>,
    Path((tenant_id, strategy_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<SkuBatchStrategyUpdateRequest>,
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
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let sku_id = match req.sku_id {
        Some(Some(raw)) => match uuid::Uuid::parse_str(&raw) {
            Ok(v) => Some(Some(v)),
            Err(_) => return bad_request("bad sku_id").into_response(),
        },
        Some(None) => Some(None),
        None => None,
    };

    let strategy = match SkuBatchStrategiesRepo::new(state.db.pool())
        .update_strategy(
            tenant_id,
            strategy_id,
            SkuBatchStrategyUpdate {
                sku_id,
                name: req.name.and_then(non_empty),
                enabled: req.enabled,
                expire_threshold_days: req.expire_threshold_days,
                fifo_enabled: req.fifo_enabled,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("strategy not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SKU_BATCH_STRATEGIES.UPDATE",
        "SKU_BATCH_STRATEGY",
        Some(strategy.strategy_id.to_string()),
        None,
        Some(serde_json::json!({
            "name": strategy.name,
            "sku_id": strategy.sku_id,
            "enabled": strategy.enabled,
        })),
    )
    .await;

    (StatusCode::OK, Json(strategy)).into_response()
}

pub(crate) async fn delete_sku_batch_strategy(
    State(state): State<AppState>,
    Path((tenant_id, strategy_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
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
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let strategy = match SkuBatchStrategiesRepo::new(state.db.pool())
        .delete_strategy(tenant_id, strategy_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("strategy not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SKU_BATCH_STRATEGIES.DELETE",
        "SKU_BATCH_STRATEGY",
        Some(strategy.strategy_id.to_string()),
        None,
        Some(serde_json::json!({
            "name": strategy.name,
            "sku_id": strategy.sku_id,
            "enabled": strategy.enabled,
        })),
    )
    .await;

    (StatusCode::OK, Json(strategy)).into_response()
}

pub(crate) async fn get_sku_batch(
    State(state): State<AppState>,
    Path((tenant_id, batch_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let batch_id = match uuid::Uuid::parse_str(&batch_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad batch_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.read") {
        return resp.into_response();
    }

    let batch = match state.sku_batches_repo.get_batch(tenant_id, batch_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("sku batch not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(batch)).into_response()
}

pub(crate) async fn create_sku_batch(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<SkuBatchCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }
    if req.batch_no.trim().is_empty() {
        return bad_request("batch_no required").into_response();
    }

    let sku_id = match uuid::Uuid::parse_str(&req.sku_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };
    let expire_at = match req.expire_at.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad expire_at").into_response(),
        },
        None => None,
    };
    let production_date = match req.production_date.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad production_date").into_response(),
        },
        None => None,
    };

    let batch = match state
        .sku_batches_repo
        .create_batch(SkuBatchCreate {
            tenant_id,
            sku_id,
            batch_no: req.batch_no.trim().to_string(),
            production_date,
            expire_at,
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
        "SKU_BATCHES.CREATE",
        "SKU_BATCH",
        Some(batch.batch_id.to_string()),
        None,
        Some(serde_json::json!({
            "batch_id": batch.batch_id,
            "sku_id": batch.sku_id,
            "batch_no": batch.batch_no,
            "production_date": batch.production_date,
            "expire_at": batch.expire_at,
        })),
    )
    .await;

    (StatusCode::OK, Json(batch)).into_response()
}

pub(crate) async fn update_sku_batch(
    State(state): State<AppState>,
    Path((tenant_id, batch_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<SkuBatchUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let batch_id = match uuid::Uuid::parse_str(&batch_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad batch_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let expire_at = match req.expire_at.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad expire_at").into_response(),
        },
        None => None,
    };
    let production_date = match req.production_date.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad production_date").into_response(),
        },
        None => None,
    };

    let batch = match state
        .sku_batches_repo
        .update_batch(
            tenant_id,
            batch_id,
            SkuBatchUpdate {
                batch_no: req.batch_no.and_then(non_empty),
                production_date,
                expire_at,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("sku batch not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SKU_BATCHES.UPDATE",
        "SKU_BATCH",
        Some(batch.batch_id.to_string()),
        None,
        Some(serde_json::json!({
            "batch_id": batch.batch_id,
            "sku_id": batch.sku_id,
            "batch_no": batch.batch_no,
            "production_date": batch.production_date,
            "expire_at": batch.expire_at,
        })),
    )
    .await;

    (StatusCode::OK, Json(batch)).into_response()
}

pub(crate) async fn list_sku_serials(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<SkuSerialListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.read") {
        return resp.into_response();
    }

    let sku_id = match query.sku_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad sku_id").into_response(),
        },
        None => None,
    };
    let cursor = match query.cursor.as_deref() {
        Some(raw) => match SkuSerialCursor::decode(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad cursor").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let result = match state
        .sku_serials_repo
        .list_serials(
            tenant_id,
            SkuSerialListFilters {
                sku_id,
                status: query.status.and_then(non_empty),
                serial_no: query.serial_no.and_then(non_empty),
                cursor,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = ListResponse {
        data: result.items,
        next_cursor: result.next_cursor.map(|c| c.encode()),
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn list_sku_serial_rules(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<SkuSerialRuleListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.read") {
        return resp.into_response();
    }

    let sku_id = match query.sku_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad sku_id").into_response(),
        },
        None => None,
    };
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let result = match SkuSerialRulesRepo::new(state.db.pool())
        .list_rules(
            tenant_id,
            SkuSerialRuleListFilters {
                sku_id,
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

    (
        StatusCode::OK,
        Json(SkuSerialRuleListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn create_sku_serial_rule(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<SkuSerialRuleCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }
    if req.name.trim().is_empty() {
        return bad_request("name required").into_response();
    }
    if req.prefix.trim().is_empty() {
        return bad_request("prefix required").into_response();
    }
    let sku_id = match req.sku_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad sku_id").into_response(),
        },
        None => None,
    };

    let rule = match SkuSerialRulesRepo::new(state.db.pool())
        .create_rule(SkuSerialRuleCreate {
            tenant_id,
            sku_id,
            name: req.name.trim().to_string(),
            prefix: req.prefix.trim().to_string(),
            seq_length: req.seq_length.unwrap_or(6),
            increment_step: req.increment_step.unwrap_or(1),
            enabled: req.enabled.unwrap_or(true),
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
        "SKU_SERIAL_RULES.CREATE",
        "SKU_SERIAL_RULE",
        Some(rule.rule_id.to_string()),
        None,
        Some(serde_json::json!({
            "name": rule.name,
            "sku_id": rule.sku_id,
            "enabled": rule.enabled,
        })),
    )
    .await;

    (StatusCode::OK, Json(rule)).into_response()
}

pub(crate) async fn update_sku_serial_rule(
    State(state): State<AppState>,
    Path((tenant_id, rule_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<SkuSerialRuleUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let rule_id = match uuid::Uuid::parse_str(&rule_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad rule_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let sku_id = match req.sku_id {
        Some(Some(raw)) => match uuid::Uuid::parse_str(&raw) {
            Ok(v) => Some(Some(v)),
            Err(_) => return bad_request("bad sku_id").into_response(),
        },
        Some(None) => Some(None),
        None => None,
    };

    let rule = match SkuSerialRulesRepo::new(state.db.pool())
        .update_rule(
            tenant_id,
            rule_id,
            SkuSerialRuleUpdate {
                sku_id,
                name: req.name.and_then(non_empty),
                prefix: req.prefix.and_then(non_empty),
                seq_length: req.seq_length,
                increment_step: req.increment_step,
                enabled: req.enabled,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("rule not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SKU_SERIAL_RULES.UPDATE",
        "SKU_SERIAL_RULE",
        Some(rule.rule_id.to_string()),
        None,
        Some(serde_json::json!({
            "name": rule.name,
            "sku_id": rule.sku_id,
            "enabled": rule.enabled,
        })),
    )
    .await;

    (StatusCode::OK, Json(rule)).into_response()
}

pub(crate) async fn delete_sku_serial_rule(
    State(state): State<AppState>,
    Path((tenant_id, rule_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let rule_id = match uuid::Uuid::parse_str(&rule_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad rule_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let rule = match SkuSerialRulesRepo::new(state.db.pool())
        .delete_rule(tenant_id, rule_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("rule not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SKU_SERIAL_RULES.DELETE",
        "SKU_SERIAL_RULE",
        Some(rule.rule_id.to_string()),
        None,
        Some(serde_json::json!({
            "name": rule.name,
            "sku_id": rule.sku_id,
            "enabled": rule.enabled,
        })),
    )
    .await;

    (StatusCode::OK, Json(rule)).into_response()
}

pub(crate) async fn generate_sku_serials(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<SkuSerialGenerateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }
    let sku_id = match uuid::Uuid::parse_str(&req.sku_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad sku_id").into_response(),
    };
    let rule_id = match req.rule_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad rule_id").into_response(),
        },
        None => None,
    };

    let rule = if let Some(rule_id) = rule_id {
        match SkuSerialRulesRepo::new(state.db.pool())
            .get_rule(tenant_id, rule_id)
            .await
        {
            Ok(Some(rule)) => Some(rule),
            Ok(None) => return not_found("serial rule not found").into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }
    } else {
        match SkuSerialRulesRepo::new(state.db.pool())
            .find_enabled_rule_for_sku(tenant_id, sku_id)
            .await
        {
            Ok(rule) => rule,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }
    };

    let prefix = match req.prefix.as_deref().and_then(non_empty) {
        Some(prefix) => prefix.to_string(),
        None => match rule.as_ref() {
            Some(rule) => rule.prefix.clone(),
            None => return bad_request("prefix required").into_response(),
        },
    };
    let seq_length = rule.as_ref().map(|rule| rule.seq_length).unwrap_or(6);
    let increment_step = rule.as_ref().map(|rule| rule.increment_step).unwrap_or(1);

    let serials = match state
        .sku_serials_repo
        .generate_serials(SkuSerialGenerateInput {
            tenant_id,
            sku_id,
            prefix,
            count: req.count,
            seq_length,
            increment_step,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return bad_request(&e.to_string()).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SKU_SERIALS.GENERATE",
        "SKU_SERIAL",
        None,
        None,
        Some(serde_json::json!({
            "sku_id": sku_id,
            "count": serials.len(),
        })),
    )
    .await;

    (StatusCode::OK, Json(serials)).into_response()
}

pub(crate) async fn print_sku_serials(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<SkuSerialPrintRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "print_jobs.write") {
        return resp.into_response();
    }

    let template_id = match uuid::Uuid::parse_str(&req.template_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad template_id").into_response(),
    };
    if req.serial_ids.is_empty() {
        return bad_request("serial_ids required").into_response();
    }
    let serial_ids: Vec<uuid::Uuid> = match req
        .serial_ids
        .iter()
        .map(|raw| uuid::Uuid::parse_str(raw))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(v) => v,
        Err(_) => return bad_request("bad serial_id").into_response(),
    };

    #[derive(sqlx::FromRow)]
    struct SerialRow {
        serial_id: uuid::Uuid,
        serial_no: String,
    }

    let rows: Vec<SerialRow> = match sqlx::query_as(
        r#"
        select serial_id, serial_no
        from sku_serials
        where tenant_id = $1 and serial_id = any($2)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(&serial_ids)
    .fetch_all(state.db.pool())
    .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    if rows.len() != serial_ids.len() {
        return not_found("serial not found").into_response();
    }

    let output_content = rows
        .iter()
        .map(|row| row.serial_no.clone())
        .collect::<Vec<_>>()
        .join("\n");

    let job = match state
        .print_jobs_repo
        .create_job(PrintJobCreate {
            tenant_id,
            requested_by_user_id: ctx.user_id,
            job_type: "SKU_SERIAL_LABEL".to_string(),
            ref_type: "SKU_SERIAL".to_string(),
            ref_id: rows[0].serial_id,
            template_id,
            device_fingerprint: req.device_fingerprint.clone(),
            payload: serde_json::json!({
                "serial_ids": req.serial_ids,
                "device_fingerprint": req.device_fingerprint,
            }),
            trace_id: Some(req_ctx.trace_id.clone()),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let job = match state
        .print_jobs_repo
        .update_job(
            tenant_id,
            job.job_id,
            PrintJobUpdate {
                status: Some("DONE".to_string()),
                payload: None,
                output_url: None,
                output_content: Some(output_content.clone()),
                attempts: None,
                last_error: None,
                actor_user_id: Some(ctx.user_id),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("print job not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SKU_SERIALS.PRINT",
        "PRINT_JOB",
        Some(job.job_id.to_string()),
        None,
        Some(serde_json::json!({
            "job_id": job.job_id,
            "serial_ids": serial_ids,
        })),
    )
    .await;

    (StatusCode::OK, Json(job)).into_response()
}

pub(crate) async fn bind_shipment_serials(
    State(state): State<AppState>,
    Path((tenant_id, shipment_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<ShipmentSerialBindRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let shipment_id = match uuid::Uuid::parse_str(&shipment_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shipment_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {
        return resp.into_response();
    }
    if req.serials.is_empty() {
        return bad_request("serials required").into_response();
    }

    let shipments_repo = ShipmentsRepo::new(state.db.pool());
    let shipment = match shipments_repo.get_by_id(tenant_id, shipment_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("shipment not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let bound = match bind_serials_for_shipment(
        &state,
        tenant_id,
        shipment_id,
        OrderId(shipment.order_id),
        req.serials,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SHIPMENT.SERIAL_BIND",
        "SHIPMENT",
        Some(shipment_id.to_string()),
        None,
        Some(serde_json::json!({
            "shipment_id": shipment_id,
            "bound_count": bound.len(),
        })),
    )
    .await;

    (StatusCode::OK, Json(bound)).into_response()
}
