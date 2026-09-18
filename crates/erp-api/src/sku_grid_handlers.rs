use super::*;

#[cfg(test)]
#[path = "sku_grid_handler_tests.rs"]
mod tests;

pub(crate) async fn query_skus_grid(
    State(state): State<AppState>,
    Path((tenant_id, product_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(query): Json<erp_grid_core::paged_query::PageQuery>,
) -> impl IntoResponse {
    let tenant = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let product = match uuid::Uuid::parse_str(&product_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad product_id").into_response(),
    };
    if let Err(response) = ensure_tenant_access(&ctx, tenant) {
        return response.into_response();
    }
    if let Err(response) = ensure_permission(&ctx, "skus.read") {
        return response.into_response();
    }
    if let Err(message) = erp_repo::sku_grid::validate(&query) {
        return bad_request(message).into_response();
    }

    match state.skus_repo.query_skus(tenant, product, &query).await {
        Ok(page) => (StatusCode::OK, Json(page)).into_response(),
        Err(error) => {
            tracing::error!(%error, "sku grid query failed");
            (StatusCode::INTERNAL_SERVER_ERROR, "SKU query failed").into_response()
        }
    }
}

pub(crate) async fn query_sku_ranges_grid(
    State(state): State<AppState>,
    Path((tenant_id, product_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(request): Json<erp_repo::sku_grid::SkuGridRangesRequest>,
) -> impl IntoResponse {
    let tenant = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let product = match uuid::Uuid::parse_str(&product_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad product_id").into_response(),
    };
    if let Err(response) = ensure_tenant_access(&ctx, tenant) {
        return response.into_response();
    }
    if let Err(response) = ensure_permission(&ctx, "skus.read") {
        return response.into_response();
    }
    if let Err(message) = erp_repo::sku_grid::validate_ranges(&request) {
        return bad_request(message).into_response();
    }
    match state
        .skus_repo
        .query_sku_ranges(tenant, product, &request)
        .await
    {
        Ok(page) => (StatusCode::OK, Json(page)).into_response(),
        Err(error) => {
            tracing::error!(%error, "SKU grid range query failed");
            (StatusCode::INTERNAL_SERVER_ERROR, "SKU range query failed").into_response()
        }
    }
}

pub(crate) async fn find_sku_cell(
    State(state): State<AppState>,
    Path((tenant_id, product_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(request): Json<erp_repo::product_grid_find::FindRequest>,
) -> impl IntoResponse {
    let tenant = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let product = match uuid::Uuid::parse_str(&product_id) {
        Ok(value) if !value.is_nil() => value,
        _ => return bad_request("bad product_id").into_response(),
    };
    if let Err(response) = ensure_tenant_access(&ctx, tenant) {
        return response.into_response();
    }
    if let Err(response) = ensure_permission(&ctx, "skus.read") {
        return response.into_response();
    }
    if let Err(message) = erp_repo::sku_grid_find::validate(&request) {
        return bad_request(message).into_response();
    }
    match erp_repo::sku_grid_find::find(state.db.pool(), tenant, product, &request).await {
        Ok(hit) => (StatusCode::OK, Json(serde_json::json!({"hit":hit}))).into_response(),
        Err(error) => {
            tracing::error!(%error, "sku grid find failed");
            (StatusCode::INTERNAL_SERVER_ERROR, "SKU find failed").into_response()
        }
    }
}

#[cfg(test)]
#[path = "sku_grid_find_handler_tests.rs"]
mod find_tests;
