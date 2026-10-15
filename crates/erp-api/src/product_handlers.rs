use super::*;

#[cfg(test)]
#[path = "product_grid_tests.rs"]
mod product_grid_tests;

pub(crate) async fn find_product_cell(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(request): Json<erp_repo::product_grid_find::FindRequest>,
) -> impl IntoResponse {
    let tenant = match TenantId::from_str(&tenant_id) {
        Ok(tenant) => tenant,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(response) = ensure_tenant_access(&ctx, tenant) {
        return response.into_response();
    }
    if let Err(response) = ensure_permission(&ctx, "skus.read") {
        return response.into_response();
    }
    if let Err(message) = erp_repo::product_grid_find::validate(&request) {
        return bad_request(message).into_response();
    }
    match erp_repo::product_grid_find::find(state.db.pool(), tenant, &request).await {
        Ok(hit) => (StatusCode::OK, Json(serde_json::json!({"hit": hit}))).into_response(),
        Err(error) => {
            tracing::error!(%error, "product grid find failed");
            (StatusCode::INTERNAL_SERVER_ERROR, "Product find failed").into_response()
        }
    }
}

pub(crate) async fn product_action(
    State(state): State<AppState>,
    Path(tenant): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(command): Json<erp_grid_core::actions::ActionCommand>,
) -> impl IntoResponse {
    let tenant = match TenantId::from_str(&tenant) {
        Ok(tenant) => tenant,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(response) = ensure_tenant_access(&ctx, tenant) {
        return response.into_response();
    }
    for permission in ["skus.read", "skus.write"] {
        if let Err(response) = ensure_permission(&ctx, permission) {
            return response.into_response();
        }
    }
    if let Err(error) = erp_repo::product_actions::validate(&command) {
        return product_action_error(error);
    }
    match state
        .products_repo
        .execute_action(tenant, ctx.user_id, &command)
        .await
    {
        Ok(receipt) => (StatusCode::OK, Json(receipt)).into_response(),
        Err(error) => product_action_error(error),
    }
}

pub(crate) async fn product_action_receipt(
    State(state): State<AppState>,
    Path((tenant, operation)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant = match TenantId::from_str(&tenant) {
        Ok(tenant) => tenant,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let operation = match erp_grid_core::RowId::from_uuid(&operation) {
        Some(id) => id,
        None => return bad_request("bad operation_id").into_response(),
    };
    if let Err(response) = ensure_tenant_access(&ctx, tenant) {
        return response.into_response();
    }
    if let Err(response) = ensure_permission(&ctx, "skus.read") {
        return response.into_response();
    }
    match state
        .products_repo
        .action_receipt(tenant, ctx.user_id, operation)
        .await
    {
        Ok(Some(receipt)) => (StatusCode::OK, Json(receipt)).into_response(),
        Ok(None) => {
            not_found("Operation receipt not found; any resend must use the original identity")
                .into_response()
        }
        Err(error) => product_action_error(error),
    }
}

fn product_action_error(error: erp_repo::product_actions::ActionError) -> axum::response::Response {
    use erp_repo::product_actions::ActionError;
    match error {
        ActionError::Invalid(message) => bad_request(message).into_response(),
        ActionError::IdentityConflict => {
            (StatusCode::CONFLICT, "Operation identity conflict").into_response()
        }
        error => {
            tracing::error!(%error, "product action storage failed; caller must check receipt");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "Operation result unknown; check the original operation receipt",
            )
                .into_response()
        }
    }
}

pub(crate) async fn query_products(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(query): Json<erp_grid_core::paged_query::PageQuery>,
) -> impl IntoResponse {
    let tenant = match TenantId::from_str(&tenant_id) {
        Ok(tenant) => tenant,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(response) = ensure_tenant_access(&ctx, tenant) {
        return response.into_response();
    }
    if let Err(response) = ensure_permission(&ctx, "skus.read") {
        return response.into_response();
    }
    if let Err(message) = erp_repo::product_grid::validate(&query) {
        return bad_request(message).into_response();
    }
    match state.products_repo.query_products(tenant, &query).await {
        Ok(page) => (StatusCode::OK, Json(page)).into_response(),
        Err(error) => {
            tracing::error!(%error, "product grid query failed");
            (StatusCode::INTERNAL_SERVER_ERROR, "Product query failed").into_response()
        }
    }
}

pub(crate) async fn product_options(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(query): Json<erp_repo::product_grid_options::OptionQuery>,
) -> impl IntoResponse {
    let tenant = match TenantId::from_str(&tenant_id) {
        Ok(tenant) => tenant,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(response) = ensure_tenant_access(&ctx, tenant) {
        return response.into_response();
    }
    if let Err(response) = ensure_permission(&ctx, "skus.read") {
        return response.into_response();
    }
    if let Err(error) = erp_repo::product_grid_options::validate(&query) {
        return bad_request(&error.to_string()).into_response();
    }
    match state.products_repo.product_options(tenant, &query).await {
        Ok(page) => (StatusCode::OK, Json(page)).into_response(),
        Err(error) => {
            tracing::error!(%error, "product options query failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Product options query failed",
            )
                .into_response()
        }
    }
}

pub(crate) async fn list_products(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ProductListQuery>,
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

    let cursor = match query.cursor.as_deref() {
        Some(raw) => match ProductCursor::decode(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad cursor").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let result = match state
        .products_repo
        .list_products(
            tenant_id,
            ProductListFilters {
                keyword: query.keyword.and_then(non_empty),
                brand: query.brand.and_then(non_empty),
                category: query.category.and_then(non_empty),
                status: query.status.and_then(non_empty),
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

pub(crate) async fn get_product(
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
    if let Err(resp) = ensure_permission(&ctx, "skus.read") {
        return resp.into_response();
    }

    let product = match state.products_repo.get_product(tenant_id, product_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("product not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(product)).into_response()
}

pub(crate) async fn create_product(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<ProductCreateRequest>,
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

    let default_supplier_id = match req.default_supplier_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad default_supplier_id").into_response(),
        },
        None => None,
    };
    let default_warehouse_id = match req.default_warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad default_warehouse_id").into_response(),
        },
        None => None,
    };
    let product_tags = req
        .product_tags
        .unwrap_or_default()
        .into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect();

    let product = match state
        .products_repo
        .create_product(ProductCreate {
            tenant_id,
            product_code: req.product_code.and_then(non_empty),
            name: req.name.trim().to_string(),
            alias: req.alias.and_then(non_empty),
            brand: req.brand.and_then(non_empty),
            category: req.category.and_then(non_empty),
            base_unit: req.base_unit.and_then(non_empty),
            second_unit: req.second_unit.and_then(non_empty),
            default_supplier_id,
            default_warehouse_id,
            primary_barcode: req.primary_barcode.and_then(non_empty),
            product_tags,
            remark: req.remark.and_then(non_empty),
            warning_stock: req.warning_stock.unwrap_or_else(|| serde_json::json!({})),
            batch_enabled: req.batch_enabled.unwrap_or(false),
            shelf_life_days: req.shelf_life_days,
            forbid_sale_days: req.forbid_sale_days,
            serial_enabled: req.serial_enabled.unwrap_or(false),
            serial_push_platform: req.serial_push_platform.unwrap_or(false),
            attrs: req.attrs.unwrap_or_else(|| serde_json::json!({})),
            allow_negative_stock: false,
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
        "PRODUCTS.CREATE",
        "PRODUCT",
        Some(product.product_id.to_string()),
        None,
        Some(serde_json::json!({
            "product_id": product.product_id,
            "name": product.name,
            "product_code": product.product_code,
            "alias": product.alias,
            "brand": product.brand,
            "category": product.category,
            "base_unit": product.base_unit,
            "second_unit": product.second_unit,
            "primary_barcode": product.primary_barcode,
            "batch_enabled": product.batch_enabled,
            "serial_enabled": product.serial_enabled,
        })),
    )
    .await;

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_PRODUCT_CREATED.to_string(),
            ref_type: Some("PRODUCT".to_string()),
            ref_id: Some(product.product_id.to_string()),
            metadata: Some(serde_json::json!({
                "product_id": product.product_id,
                "name": product.name.clone(),
                "product_code": product.product_code.clone(),
                "alias": product.alias.clone(),
                "brand": product.brand.clone(),
                "category": product.category.clone(),
                "base_unit": product.base_unit.clone(),
                "second_unit": product.second_unit.clone(),
                "primary_barcode": product.primary_barcode.clone(),
                "batch_enabled": product.batch_enabled,
                "serial_enabled": product.serial_enabled,
            })),
        })
        .await;

    (StatusCode::OK, Json(product)).into_response()
}

pub(crate) async fn update_product(
    State(state): State<AppState>,
    Path((tenant_id, product_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<ProductUpdateRequest>,
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
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let default_supplier_id = match req.default_supplier_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad default_supplier_id").into_response(),
        },
        None => None,
    };
    let default_warehouse_id = match req.default_warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad default_warehouse_id").into_response(),
        },
        None => None,
    };
    let product_tags = req.product_tags.map(|tags| {
        tags.into_iter()
            .map(|tag| tag.trim().to_string())
            .filter(|tag| !tag.is_empty())
            .collect::<Vec<String>>()
    });

    let product = match state
        .products_repo
        .update_product(
            tenant_id,
            product_id,
            ProductUpdate {
                product_code: req.product_code.and_then(non_empty),
                name: req.name.and_then(non_empty),
                alias: req.alias.and_then(non_empty),
                brand: req.brand.and_then(non_empty),
                category: req.category.and_then(non_empty),
                base_unit: req.base_unit.and_then(non_empty),
                second_unit: req.second_unit.and_then(non_empty),
                default_supplier_id,
                default_warehouse_id,
                primary_barcode: req.primary_barcode.and_then(non_empty),
                product_tags,
                remark: req.remark.and_then(non_empty),
                warning_stock: req.warning_stock,
                batch_enabled: req.batch_enabled,
                shelf_life_days: req.shelf_life_days,
                forbid_sale_days: req.forbid_sale_days,
                serial_enabled: req.serial_enabled,
                serial_push_platform: req.serial_push_platform,
                attrs: req.attrs,
                status: req.status.and_then(non_empty),
                allow_negative_stock: None,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("product not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "PRODUCTS.UPDATE",
        "PRODUCT",
        Some(product.product_id.to_string()),
        None,
        Some(serde_json::json!({
            "product_id": product.product_id,
            "name": product.name,
            "product_code": product.product_code,
            "alias": product.alias,
            "brand": product.brand,
            "category": product.category,
            "base_unit": product.base_unit,
            "second_unit": product.second_unit,
            "primary_barcode": product.primary_barcode,
            "batch_enabled": product.batch_enabled,
            "serial_enabled": product.serial_enabled,
        })),
    )
    .await;

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_PRODUCT_UPDATED.to_string(),
            ref_type: Some("PRODUCT".to_string()),
            ref_id: Some(product.product_id.to_string()),
            metadata: Some(serde_json::json!({
                "product_id": product.product_id,
                "name": product.name.clone(),
                "product_code": product.product_code.clone(),
                "alias": product.alias.clone(),
                "brand": product.brand.clone(),
                "category": product.category.clone(),
                "base_unit": product.base_unit.clone(),
                "second_unit": product.second_unit.clone(),
                "primary_barcode": product.primary_barcode.clone(),
                "batch_enabled": product.batch_enabled,
                "serial_enabled": product.serial_enabled,
            })),
        })
        .await;

    (StatusCode::OK, Json(product)).into_response()
}

pub(crate) async fn delete_product(
    State(state): State<AppState>,
    Path((tenant_id, product_id)): Path<(String, String)>,
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

    let product_id = match uuid::Uuid::parse_str(&product_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad product_id").into_response(),
    };

    let referenced = match sqlx::query_scalar::<_, i64>(
        "select count(*) from shared_item_refs where tenant_id = $1 and item_id = $2",
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(product_id)
    .fetch_one(state.db.pool())
    .await
    {
        Ok(count) => count,
        Err(error) if error.to_string().contains("shared_item_refs") => 0,
        Err(error) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response(),
    };
    if erp_domain::trade::item_removal_decision(referenced > 0, "DELETE").is_err() {
        return conflict("referenced item cannot be deleted").into_response();
    }

    let product = match state
        .products_repo
        .delete_product(tenant_id, product_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("product not found").into_response(),
    Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "PRODUCTS.DELETE",
        "PRODUCT",
        Some(product.product_id.to_string()),
        None,
        Some(serde_json::json!({
            "product_id": product.product_id,
            "name": product.name,
        })),
    )
    .await;

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    let _ = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_PRODUCT_DELETED.to_string(),
            ref_type: Some("PRODUCT".to_string()),
            ref_id: Some(product.product_id.to_string()),
            metadata: Some(serde_json::json!({
                "product_id": product.product_id,
                "name": product.name.clone(),
            })),
        })
        .await;

    (StatusCode::OK, Json(product)).into_response()
}

pub(crate) async fn list_product_categories(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ProductCategoryListQuery>,
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
    let keyword = query.keyword.and_then(non_empty);

    let repo = ProductCategoriesRepo::new(state.db.pool());
    let result = match repo
        .list_categories(
            tenant_id,
            ProductCategoryListFilters {
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

    (
        StatusCode::OK,
        Json(ProductCategoryListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn create_product_category(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProductCategoryCreateRequest>,
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

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name required").into_response();
    }

    let repo = ProductCategoriesRepo::new(state.db.pool());
    let category = match repo
        .create_category(ProductCategoryCreate {
            tenant_id,
            name: name.to_string(),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(category)).into_response()
}

pub(crate) async fn update_product_category(
    State(state): State<AppState>,
    Path((tenant_id, category_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProductCategoryUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let category_id = match uuid::Uuid::parse_str(&category_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad category_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let repo = ProductCategoriesRepo::new(state.db.pool());
    let category = match repo
        .update_category(
            tenant_id,
            category_id,
            ProductCategoryUpdate {
                name: req.name.and_then(non_empty),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("category not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(category)).into_response()
}

pub(crate) async fn delete_product_category(
    State(state): State<AppState>,
    Path((tenant_id, category_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let category_id = match uuid::Uuid::parse_str(&category_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad category_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let repo = ProductCategoriesRepo::new(state.db.pool());
    let category = match repo.delete_category(tenant_id, category_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("category not found").into_response(),
        Err(e) => {
            let message = e.to_string();
            if message.contains("category has products") {
                return conflict("category has products").into_response();
            }
            return (StatusCode::INTERNAL_SERVER_ERROR, message).into_response();
        }
    };

    (StatusCode::OK, Json(category)).into_response()
}

pub(crate) async fn list_product_brands(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ProductBrandListQuery>,
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
    let keyword = query.keyword.and_then(non_empty);

    let repo = ProductBrandsRepo::new(state.db.pool());
    let result = match repo
        .list_brands(
            tenant_id,
            ProductBrandListFilters {
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

    (
        StatusCode::OK,
        Json(ProductBrandListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn create_product_brand(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProductBrandCreateRequest>,
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

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name required").into_response();
    }

    let repo = ProductBrandsRepo::new(state.db.pool());
    let brand = match repo
        .create_brand(ProductBrandCreate {
            tenant_id,
            name: name.to_string(),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(brand)).into_response()
}

pub(crate) async fn update_product_brand(
    State(state): State<AppState>,
    Path((tenant_id, brand_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProductBrandUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let brand_id = match uuid::Uuid::parse_str(&brand_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad brand_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let repo = ProductBrandsRepo::new(state.db.pool());
    let brand = match repo
        .update_brand(
            tenant_id,
            brand_id,
            ProductBrandUpdate {
                name: req.name.and_then(non_empty),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("brand not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(brand)).into_response()
}

pub(crate) async fn delete_product_brand(
    State(state): State<AppState>,
    Path((tenant_id, brand_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let brand_id = match uuid::Uuid::parse_str(&brand_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad brand_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let repo = ProductBrandsRepo::new(state.db.pool());
    let brand = match repo.delete_brand(tenant_id, brand_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("brand not found").into_response(),
        Err(e) => {
            let message = e.to_string();
            if message.contains("brand has products") {
                return conflict("brand has products").into_response();
            }
            return (StatusCode::INTERNAL_SERVER_ERROR, message).into_response();
        }
    };

    (StatusCode::OK, Json(brand)).into_response()
}

pub(crate) async fn list_product_units(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ProductUnitListQuery>,
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
    let keyword = query.keyword.and_then(non_empty);

    let repo = ProductUnitsRepo::new(state.db.pool());
    let result = match repo
        .list_units(
            tenant_id,
            ProductUnitListFilters {
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

    (
        StatusCode::OK,
        Json(ProductUnitListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn create_product_unit(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProductUnitCreateRequest>,
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

    let name = req.name.trim();
    if name.is_empty() {
        return bad_request("name required").into_response();
    }

    let repo = ProductUnitsRepo::new(state.db.pool());
    let unit = match repo
        .create_unit(ProductUnitCreate {
            tenant_id,
            name: name.to_string(),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(unit)).into_response()
}

pub(crate) async fn update_product_unit(
    State(state): State<AppState>,
    Path((tenant_id, unit_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProductUnitUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let unit_id = match uuid::Uuid::parse_str(&unit_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad unit_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let repo = ProductUnitsRepo::new(state.db.pool());
    let unit = match repo
        .update_unit(
            tenant_id,
            unit_id,
            ProductUnitUpdate {
                name: req.name.and_then(non_empty),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("unit not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(unit)).into_response()
}

pub(crate) async fn list_product_warehouses(
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
    if let Err(resp) = ensure_permission(&ctx, "skus.read") {
        return resp.into_response();
    }

    let repo = ProductWarehousesRepo::new(state.db.pool());
    let items = match repo.list_product_warehouses(tenant_id, product_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(ProductWarehouseListResponse { items })).into_response()
}

pub(crate) async fn create_product_warehouse(
    State(state): State<AppState>,
    Path((tenant_id, product_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProductWarehouseCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let product_id = match uuid::Uuid::parse_str(&product_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad product_id").into_response(),
    };
    let warehouse_id = match uuid::Uuid::parse_str(&req.warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let repo = ProductWarehousesRepo::new(state.db.pool());
    let record = match repo
        .create_product_warehouse(ProductWarehouseCreate {
            tenant_id,
            product_id,
            warehouse_id,
            is_default: req.is_default.unwrap_or(false),
            allow_negative_stock: req.allow_negative_stock.unwrap_or(false),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(record)).into_response()
}

pub(crate) async fn update_product_warehouse(
    State(state): State<AppState>,
    Path((tenant_id, product_id, warehouse_id)): Path<(String, String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProductWarehouseUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let product_id = match uuid::Uuid::parse_str(&product_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad product_id").into_response(),
    };
    let warehouse_id = match uuid::Uuid::parse_str(&warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let repo = ProductWarehousesRepo::new(state.db.pool());
    let record = match repo
        .update_product_warehouse(
            tenant_id,
            product_id,
            warehouse_id,
            ProductWarehouseUpdate {
                is_default: req.is_default,
                allow_negative_stock: req.allow_negative_stock,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("product warehouse not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(record)).into_response()
}

pub(crate) async fn delete_product_warehouse(
    State(state): State<AppState>,
    Path((tenant_id, product_id, warehouse_id)): Path<(String, String, String)>,
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
    let warehouse_id = match uuid::Uuid::parse_str(&warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let repo = ProductWarehousesRepo::new(state.db.pool());
    let record = match repo
        .delete_product_warehouse(tenant_id, product_id, warehouse_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("product warehouse not found").into_response(),
        Err(e) => {
            let message = e.to_string();
            if message.contains("warehouse has inventory") {
                return conflict("warehouse has inventory").into_response();
            }
            if message.contains("warehouse has batch inventory") {
                return conflict("warehouse has batch inventory").into_response();
            }
            return (StatusCode::INTERNAL_SERVER_ERROR, message).into_response();
        }
    };

    (StatusCode::OK, Json(record)).into_response()
}

pub(crate) async fn list_product_kits(
    State(state): State<AppState>,
    Path((tenant_id, product_id)): Path<(String, String)>,
    Query(query): Query<ProductKitListQuery>,
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
    if let Err(resp) = ensure_permission(&ctx, "skus.read") {
        return resp.into_response();
    }

    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = KitsRepo::new(state.db.pool());
    let result = match repo
        .list_kits_by_product(
            tenant_id,
            KitProductListFilters {
                product_id,
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

pub(crate) async fn list_product_barcodes(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ProductBarcodeListQuery>,
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

    let repo = ProductBarcodesRepo::new(state.db.pool());
    let result = match repo
        .list_barcodes(
            tenant_id,
            ProductBarcodeListFilters {
                sku_id,
                barcode: query.barcode.and_then(non_empty),
                status: query.status.and_then(non_empty),
                level: query.level.and_then(non_empty),
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
        Json(ProductBarcodeListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn get_product_barcode(
    State(state): State<AppState>,
    Path((tenant_id, barcode_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let barcode_id = match uuid::Uuid::parse_str(&barcode_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad barcode_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.read") {
        return resp.into_response();
    }

    let repo = ProductBarcodesRepo::new(state.db.pool());
    let barcode = match repo.get_barcode(tenant_id, barcode_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("barcode not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(barcode)).into_response()
}

pub(crate) async fn create_product_barcode(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProductBarcodeCreateRequest>,
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
    let barcode = req.barcode.trim();
    if barcode.is_empty() {
        return bad_request("barcode required").into_response();
    }
    let level = req.level.trim();
    if level.is_empty() {
        return bad_request("level required").into_response();
    }
    let unit = req.unit.trim();
    if unit.is_empty() {
        return bad_request("unit required").into_response();
    }

    let repo = ProductBarcodesRepo::new(state.db.pool());
    let barcode = match repo
        .create_barcode(ProductBarcodeCreate {
            tenant_id,
            sku_id,
            barcode: barcode.to_string(),
            level: level.to_string(),
            unit: unit.to_string(),
            ratio: req.ratio.unwrap_or(1).max(1),
            is_default: req.is_default.unwrap_or(false),
            status: req
                .status
                .and_then(non_empty)
                .unwrap_or_else(|| "ACTIVE".to_string()),
            print_template: req.print_template.and_then(non_empty),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(barcode)).into_response()
}

pub(crate) async fn update_product_barcode(
    State(state): State<AppState>,
    Path((tenant_id, barcode_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProductBarcodeUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let barcode_id = match uuid::Uuid::parse_str(&barcode_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad barcode_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let repo = ProductBarcodesRepo::new(state.db.pool());
    let barcode = match repo
        .update_barcode(
            tenant_id,
            barcode_id,
            ProductBarcodeUpdate {
                barcode: req.barcode.and_then(non_empty),
                level: req.level.and_then(non_empty),
                unit: req.unit.and_then(non_empty),
                ratio: req.ratio,
                is_default: req.is_default,
                status: req.status.and_then(non_empty),
                print_template: req.print_template.map(|value| value.and_then(non_empty)),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("barcode not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(barcode)).into_response()
}

pub(crate) async fn delete_product_barcode(
    State(state): State<AppState>,
    Path((tenant_id, barcode_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let barcode_id = match uuid::Uuid::parse_str(&barcode_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad barcode_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let repo = ProductBarcodesRepo::new(state.db.pool());
    let barcode = match repo.delete_barcode(tenant_id, barcode_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("barcode not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(barcode)).into_response()
}

pub(crate) async fn list_product_unit_conversions(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ProductUnitConversionListQuery>,
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

    let repo = ProductUnitConversionsRepo::new(state.db.pool());
    let result = match repo
        .list_conversions(
            tenant_id,
            ProductUnitConversionListFilters {
                sku_id,
                scenario: query.scenario.and_then(non_empty),
                status: query.status.and_then(non_empty),
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
        Json(ProductUnitConversionListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn get_product_unit_conversion(
    State(state): State<AppState>,
    Path((tenant_id, conversion_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let conversion_id = match uuid::Uuid::parse_str(&conversion_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad conversion_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.read") {
        return resp.into_response();
    }

    let repo = ProductUnitConversionsRepo::new(state.db.pool());
    let conversion = match repo.get_conversion(tenant_id, conversion_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("conversion not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(conversion)).into_response()
}

pub(crate) async fn create_product_unit_conversion(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProductUnitConversionCreateRequest>,
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
    let base_unit = req.base_unit.trim();
    if base_unit.is_empty() {
        return bad_request("base_unit required").into_response();
    }
    let aux_unit = req.aux_unit.trim();
    if aux_unit.is_empty() {
        return bad_request("aux_unit required").into_response();
    }
    let scenario = req.scenario.trim();
    if scenario.is_empty() {
        return bad_request("scenario required").into_response();
    }

    let repo = ProductUnitConversionsRepo::new(state.db.pool());
    let conversion = match repo
        .create_conversion(ProductUnitConversionCreate {
            tenant_id,
            sku_id,
            base_unit: base_unit.to_string(),
            aux_unit: aux_unit.to_string(),
            ratio: req.ratio.unwrap_or(1).max(1),
            rounding: req
                .rounding
                .and_then(non_empty)
                .unwrap_or_else(|| "ROUND".to_string()),
            scenario: scenario.to_string(),
            status: req
                .status
                .and_then(non_empty)
                .unwrap_or_else(|| "ACTIVE".to_string()),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(conversion)).into_response()
}

pub(crate) async fn update_product_unit_conversion(
    State(state): State<AppState>,
    Path((tenant_id, conversion_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProductUnitConversionUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let conversion_id = match uuid::Uuid::parse_str(&conversion_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad conversion_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let repo = ProductUnitConversionsRepo::new(state.db.pool());
    let conversion = match repo
        .update_conversion(
            tenant_id,
            conversion_id,
            ProductUnitConversionUpdate {
                base_unit: req.base_unit.and_then(non_empty),
                aux_unit: req.aux_unit.and_then(non_empty),
                ratio: req.ratio,
                rounding: req.rounding.and_then(non_empty),
                scenario: req.scenario.and_then(non_empty),
                status: req.status.and_then(non_empty),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("conversion not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(conversion)).into_response()
}

pub(crate) async fn delete_product_unit_conversion(
    State(state): State<AppState>,
    Path((tenant_id, conversion_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let conversion_id = match uuid::Uuid::parse_str(&conversion_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad conversion_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let repo = ProductUnitConversionsRepo::new(state.db.pool());
    let conversion = match repo.delete_conversion(tenant_id, conversion_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("conversion not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(conversion)).into_response()
}

pub(crate) async fn list_product_channel_availability(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ProductChannelAvailabilityListQuery>,
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
    let channel_id = match query.channel_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad channel_id").into_response(),
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

    let repo = ProductChannelAvailabilityRepo::new(state.db.pool());
    let result = match repo
        .list_availability(
            tenant_id,
            ProductChannelAvailabilityListFilters {
                sku_id,
                channel_id,
                warehouse_id,
                status: query.status.and_then(non_empty),
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
        Json(ProductChannelAvailabilityListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn get_product_channel_availability(
    State(state): State<AppState>,
    Path((tenant_id, availability_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let availability_id = match uuid::Uuid::parse_str(&availability_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad availability_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.read") {
        return resp.into_response();
    }

    let repo = ProductChannelAvailabilityRepo::new(state.db.pool());
    let availability = match repo.get_availability(tenant_id, availability_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("availability not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(availability)).into_response()
}

pub(crate) async fn create_product_channel_availability(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProductChannelAvailabilityCreateRequest>,
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
    let channel_id = match req.channel_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad channel_id").into_response(),
        },
        None => None,
    };
    let warehouse_id = match req.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    let on_shelf_at = match req.on_shelf_at.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad on_shelf_at").into_response(),
        },
        None => None,
    };

    let repo = ProductChannelAvailabilityRepo::new(state.db.pool());
    let availability = match repo
        .create_availability(ProductChannelAvailabilityCreate {
            tenant_id,
            sku_id,
            channel_id,
            warehouse_id,
            status: req
                .status
                .and_then(non_empty)
                .unwrap_or_else(|| "ON".to_string()),
            available_qty: req.available_qty.unwrap_or(0),
            rule: req.rule.and_then(non_empty),
            on_shelf_at,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(availability)).into_response()
}

pub(crate) async fn update_product_channel_availability(
    State(state): State<AppState>,
    Path((tenant_id, availability_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProductChannelAvailabilityUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let availability_id = match uuid::Uuid::parse_str(&availability_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad availability_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let channel_id = match req.channel_id {
        Some(Some(raw)) => match uuid::Uuid::parse_str(&raw) {
            Ok(v) => Some(Some(v)),
            Err(_) => return bad_request("bad channel_id").into_response(),
        },
        Some(None) => Some(None),
        None => None,
    };
    let warehouse_id = match req.warehouse_id {
        Some(Some(raw)) => match uuid::Uuid::parse_str(&raw) {
            Ok(v) => Some(Some(v)),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        Some(None) => Some(None),
        None => None,
    };
    let on_shelf_at = match req.on_shelf_at {
        Some(Some(raw)) => match DateTime::parse_from_rfc3339(&raw) {
            Ok(v) => Some(Some(v.with_timezone(&Utc))),
            Err(_) => return bad_request("bad on_shelf_at").into_response(),
        },
        Some(None) => Some(None),
        None => None,
    };

    let repo = ProductChannelAvailabilityRepo::new(state.db.pool());
    let availability = match repo
        .update_availability(
            tenant_id,
            availability_id,
            ProductChannelAvailabilityUpdate {
                channel_id,
                warehouse_id,
                status: req.status.and_then(non_empty),
                available_qty: req.available_qty,
                rule: req.rule.map(|value| value.and_then(non_empty)),
                on_shelf_at,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("availability not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(availability)).into_response()
}

pub(crate) async fn delete_product_channel_availability(
    State(state): State<AppState>,
    Path((tenant_id, availability_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let availability_id = match uuid::Uuid::parse_str(&availability_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad availability_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let repo = ProductChannelAvailabilityRepo::new(state.db.pool());
    let availability = match repo.delete_availability(tenant_id, availability_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("availability not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(availability)).into_response()
}

pub(crate) async fn list_product_change_requests(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ProductChangeRequestListQuery>,
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

    let repo = ProductChangeRequestsRepo::new(state.db.pool());
    let result = match repo
        .list_requests(
            tenant_id,
            ProductChangeRequestListFilters {
                sku_id,
                status: query.status.and_then(non_empty),
                change_type: query.change_type.and_then(non_empty),
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
        Json(ProductChangeRequestListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn get_product_change_request(
    State(state): State<AppState>,
    Path((tenant_id, request_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let request_id = match uuid::Uuid::parse_str(&request_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad request_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.read") {
        return resp.into_response();
    }

    let repo = ProductChangeRequestsRepo::new(state.db.pool());
    let request = match repo.get_request(tenant_id, request_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("change request not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(request)).into_response()
}

pub(crate) async fn create_product_change_request(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProductChangeRequestCreateRequest>,
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
    if req.title.trim().is_empty() {
        return bad_request("title required").into_response();
    }
    if req.change_type.trim().is_empty() {
        return bad_request("change_type required").into_response();
    }
    if req.applicant.trim().is_empty() {
        return bad_request("applicant required").into_response();
    }

    let history = req.history.unwrap_or_default();
    let history = if history.is_empty() {
        vec![ProductChangeHistoryItem {
            time: Utc::now(),
            action: "CREATE".to_string(),
            actor: req.applicant.clone(),
            note: None,
        }]
    } else {
        history
    };

    let repo = ProductChangeRequestsRepo::new(state.db.pool());
    let request = match repo
        .create_request(ProductChangeRequestCreate {
            tenant_id,
            sku_id,
            title: req.title.trim().to_string(),
            change_type: req.change_type.trim().to_string(),
            status: req
                .status
                .and_then(non_empty)
                .unwrap_or_else(|| "DRAFT".to_string()),
            applicant: req.applicant.trim().to_string(),
            approver: req.approver.and_then(non_empty),
            reason: req.reason.and_then(non_empty),
            history,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(request)).into_response()
}

pub(crate) async fn update_product_change_request(
    State(state): State<AppState>,
    Path((tenant_id, request_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProductChangeRequestUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let request_id = match uuid::Uuid::parse_str(&request_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad request_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let repo = ProductChangeRequestsRepo::new(state.db.pool());
    let request = match repo
        .update_request(
            tenant_id,
            request_id,
            ProductChangeRequestUpdate {
                title: req.title.and_then(non_empty),
                change_type: req.change_type.and_then(non_empty),
                status: req.status.and_then(non_empty),
                approver: req.approver.map(|value| value.and_then(non_empty)),
                reason: req.reason.map(|value| value.and_then(non_empty)),
                history: req.history,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("change request not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(request)).into_response()
}

pub(crate) async fn delete_product_change_request(
    State(state): State<AppState>,
    Path((tenant_id, request_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let request_id = match uuid::Uuid::parse_str(&request_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad request_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let repo = ProductChangeRequestsRepo::new(state.db.pool());
    let request = match repo.delete_request(tenant_id, request_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("change request not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(request)).into_response()
}

pub(crate) async fn list_product_lifecycles(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ProductLifecycleListQuery>,
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

    let repo = ProductLifecyclesRepo::new(state.db.pool());
    let result = match repo
        .list_lifecycles(
            tenant_id,
            ProductLifecycleListFilters {
                sku_id,
                status: query.status.and_then(non_empty),
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
        Json(ProductLifecycleListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn get_product_lifecycle(
    State(state): State<AppState>,
    Path((tenant_id, lifecycle_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let lifecycle_id = match uuid::Uuid::parse_str(&lifecycle_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad lifecycle_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.read") {
        return resp.into_response();
    }

    let repo = ProductLifecyclesRepo::new(state.db.pool());
    let lifecycle = match repo.get_lifecycle(tenant_id, lifecycle_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("lifecycle not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(lifecycle)).into_response()
}

pub(crate) async fn create_product_lifecycle(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProductLifecycleCreateRequest>,
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

    let repo = ProductLifecyclesRepo::new(state.db.pool());
    let lifecycle = match repo
        .create_lifecycle(ProductLifecycleCreate {
            tenant_id,
            sku_id,
            status: req
                .status
                .and_then(non_empty)
                .unwrap_or_else(|| "ACTIVE".to_string()),
            inventory_qty: req.inventory_qty.unwrap_or(0),
            remark: req.remark.and_then(non_empty),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(lifecycle)).into_response()
}

pub(crate) async fn update_product_lifecycle(
    State(state): State<AppState>,
    Path((tenant_id, lifecycle_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<ProductLifecycleUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let lifecycle_id = match uuid::Uuid::parse_str(&lifecycle_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad lifecycle_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let repo = ProductLifecyclesRepo::new(state.db.pool());
    let lifecycle = match repo
        .update_lifecycle(
            tenant_id,
            lifecycle_id,
            ProductLifecycleUpdate {
                status: req.status.and_then(non_empty),
                inventory_qty: req.inventory_qty,
                remark: req.remark.map(|value| value.and_then(non_empty)),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("lifecycle not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(lifecycle)).into_response()
}

pub(crate) async fn delete_product_lifecycle(
    State(state): State<AppState>,
    Path((tenant_id, lifecycle_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let lifecycle_id = match uuid::Uuid::parse_str(&lifecycle_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad lifecycle_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "skus.write") {
        return resp.into_response();
    }

    let repo = ProductLifecyclesRepo::new(state.db.pool());
    let lifecycle = match repo.delete_lifecycle(tenant_id, lifecycle_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("lifecycle not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(lifecycle)).into_response()
}
