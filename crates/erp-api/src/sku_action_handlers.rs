use super::*;

pub(crate) async fn sku_action(
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
    if let Err(error) = erp_repo::sku_actions::validate(&command) {
        return sku_action_error(error);
    }
    match state
        .skus_repo
        .execute_grid_action(tenant, ctx.user_id, &command)
        .await
    {
        Ok(receipt) => (StatusCode::OK, Json(receipt)).into_response(),
        Err(error) => sku_action_error(error),
    }
}

pub(crate) async fn sku_action_receipt(
    State(state): State<AppState>,
    Path((tenant, operation)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant = match TenantId::from_str(&tenant) {
        Ok(tenant) => tenant,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let operation = match erp_grid_core::RowId::from_uuid(&operation) {
        Some(operation) => operation,
        None => return bad_request("bad operation_id").into_response(),
    };
    if let Err(response) = ensure_tenant_access(&ctx, tenant) {
        return response.into_response();
    }
    if let Err(response) = ensure_permission(&ctx, "skus.read") {
        return response.into_response();
    }
    match state
        .skus_repo
        .grid_action_receipt(tenant, ctx.user_id, operation)
        .await
    {
        Ok(Some(receipt)) => (StatusCode::OK, Json(receipt)).into_response(),
        Ok(None) => {
            not_found("Operation receipt not found; any resend must use the original identity")
                .into_response()
        }
        Err(error) => sku_action_error(error),
    }
}

fn sku_action_error(error: erp_repo::product_actions::ActionError) -> axum::response::Response {
    use erp_repo::product_actions::ActionError;
    match error {
        ActionError::Invalid(message) => bad_request(message).into_response(),
        ActionError::IdentityConflict => {
            (StatusCode::CONFLICT, "Operation identity conflict").into_response()
        }
        error => {
            tracing::error!(%error, "SKU action storage failed; caller must check receipt");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "Operation result unknown; check the original operation receipt",
            )
                .into_response()
        }
    }
}
