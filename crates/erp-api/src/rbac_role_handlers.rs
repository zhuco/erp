use super::*;

pub(crate) async fn list_roles(
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
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysRoleRead.as_str()) {
        return resp.into_response();
    }

    let repo = RbacRepo::new(state.db.pool());
    let roles = match repo.list_roles(tenant_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(roles)).into_response()
}

pub(crate) async fn create_role(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<RoleCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysRoleWrite.as_str()) {
        return resp.into_response();
    }

    let repo = RbacRepo::new(state.db.pool());
    let role = match repo
        .create_role(RoleCreate {
            tenant_id,
            name: req.name.trim().to_string(),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    if let Some(perms) = req.permissions {
        let normalized: Vec<String> = perms
            .into_iter()
            .map(|p| normalize_permission_code(&p))
            .collect();
        if let Err(e) = repo
            .replace_role_permissions(tenant_id, role.role_id, &normalized)
            .await
        {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
        }
    }

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SYS.ROLE.CREATE",
        "role",
        Some(role.role_id.to_string()),
        None,
        Some(serde_json::to_value(&role).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(role)).into_response()
}

pub(crate) async fn update_role(
    State(state): State<AppState>,
    Path((tenant_id, role_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<RoleUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysRoleWrite.as_str()) {
        return resp.into_response();
    }
    let role_id = match RoleId::from_str(&role_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad role_id").into_response(),
    };

    let repo = RbacRepo::new(state.db.pool());
    let before = match repo.list_roles(tenant_id).await {
        Ok(roles) => roles
            .into_iter()
            .find(|role| role.role_id == role_id)
            .map(|role| serde_json::to_value(role).unwrap_or(Value::Null)),
        Err(_) => None,
    };

    let role = match repo
        .update_role(
            tenant_id,
            role_id,
            RoleUpdate {
                name: req.name.trim().to_string(),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("role not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SYS.ROLE.UPDATE",
        "role",
        Some(role.role_id.to_string()),
        before,
        Some(serde_json::to_value(&role).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(role)).into_response()
}

pub(crate) async fn list_role_permissions(
    State(state): State<AppState>,
    Path((tenant_id, role_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysRoleRead.as_str()) {
        return resp.into_response();
    }
    let role_id = match RoleId::from_str(&role_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad role_id").into_response(),
    };

    let repo = RbacRepo::new(state.db.pool());
    let perms = match repo.list_role_permissions(tenant_id, role_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    (StatusCode::OK, Json(perms)).into_response()
}

pub(crate) async fn update_role_permissions(
    State(state): State<AppState>,
    Path((tenant_id, role_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<RolePermissionsRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysRoleWrite.as_str()) {
        return resp.into_response();
    }
    let role_id = match RoleId::from_str(&role_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad role_id").into_response(),
    };

    let repo = RbacRepo::new(state.db.pool());
    let normalized: Vec<String> = req
        .permissions
        .into_iter()
        .map(|perm| normalize_permission_code(&perm))
        .collect();
    if let Err(e) = repo
        .replace_role_permissions(tenant_id, role_id, &normalized)
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SYS.ROLE.PERMISSIONS",
        "role",
        Some(role_id.to_string()),
        None,
        Some(Value::Array(
            normalized
                .iter()
                .map(|p| Value::String(p.clone()))
                .collect(),
        )),
    )
    .await;

    (StatusCode::OK, Json(normalized)).into_response()
}
