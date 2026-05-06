use super::*;

pub(crate) async fn list_permission_catalog(
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
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysPermissionRead.as_str()) {
        return resp.into_response();
    }

    let repo = PermissionsRepo::new(state.db.pool());
    let mut permissions = match repo.list_permissions(tenant_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    if permissions.is_empty() {
        let defaults = permission_catalog();
        if let Err(e) = repo.seed_permissions(tenant_id, &defaults).await {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
        }
        permissions = match repo.list_permissions(tenant_id).await {
            Ok(v) => v,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
    }

    (StatusCode::OK, Json(permissions)).into_response()
}
