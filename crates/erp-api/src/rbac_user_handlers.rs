use super::*;

#[derive(Debug, Deserialize)]
pub(crate) struct UserListQuery {
    email: Option<String>,
    status: Option<String>,
}

pub(crate) async fn list_users(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<UserListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysUserRead.as_str()) {
        return resp.into_response();
    }

    let repo = RbacRepo::new(state.db.pool());
    let users = match repo
        .list_users(
            tenant_id,
            UserListFilters {
                email: query.email.clone(),
                status: query.status.clone(),
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(users)).into_response()
}

pub(crate) async fn create_user(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<UserCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysUserWrite.as_str()) {
        return resp.into_response();
    }

    let password = req.password.trim();
    if password.is_empty() {
        return bad_request("password required").into_response();
    }
    let salt = argon2::password_hash::SaltString::generate(&mut rand::thread_rng());
    let hash = match argon2::Argon2::default().hash_password(password.as_bytes(), &salt) {
        Ok(v) => v.to_string(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "hash failed").into_response(),
    };

    let repo = RbacRepo::new(state.db.pool());
    let user = match repo
        .create_user(UserCreate {
            tenant_id,
            email: req.email.trim().to_string(),
            password_hash: hash,
            status: req.status.unwrap_or_else(|| "ACTIVE".to_string()),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    if let Some(role_ids) = req.role_ids {
        let ids = match parse_role_ids(&role_ids) {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
        if let Err(e) = repo.replace_user_roles(tenant_id, user.user_id, &ids).await {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
        }
    }

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SYS.USER.CREATE",
        "user",
        Some(user.user_id.to_string()),
        None,
        Some(serde_json::to_value(&user).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(user)).into_response()
}

pub(crate) async fn update_user(
    State(state): State<AppState>,
    Path((tenant_id, user_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<UserUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysUserWrite.as_str()) {
        return resp.into_response();
    }
    let user_id = match UserId::from_str(&user_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad user_id").into_response(),
    };

    let repo = RbacRepo::new(state.db.pool());
    let before = match repo
        .list_users(
            tenant_id,
            UserListFilters {
                email: None,
                status: None,
            },
        )
        .await
    {
        Ok(users) => users
            .into_iter()
            .find(|user| user.user_id == user_id)
            .map(|user| serde_json::to_value(user).unwrap_or(Value::Null)),
        Err(_) => None,
    };

    let password_hash = if let Some(password) = req.password.as_deref() {
        let password = password.trim();
        if password.is_empty() {
            return bad_request("password empty").into_response();
        }
        let salt = argon2::password_hash::SaltString::generate(&mut rand::thread_rng());
        match argon2::Argon2::default().hash_password(password.as_bytes(), &salt) {
            Ok(v) => Some(v.to_string()),
            Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "hash failed").into_response(),
        }
    } else {
        None
    };

    let user = match repo
        .update_user(
            tenant_id,
            user_id,
            UserUpdate {
                email: req.email.clone(),
                password_hash,
                status: req.status.clone(),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("user not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SYS.USER.UPDATE",
        "user",
        Some(user.user_id.to_string()),
        before,
        Some(serde_json::to_value(&user).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(user)).into_response()
}

pub(crate) async fn get_user_roles(
    State(state): State<AppState>,
    Path((tenant_id, user_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysUserRead.as_str()) {
        return resp.into_response();
    }
    let user_id = match UserId::from_str(&user_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad user_id").into_response(),
    };

    let repo = RbacRepo::new(state.db.pool());
    let roles = match repo.list_user_roles(tenant_id, user_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    (StatusCode::OK, Json(UserRolesResponse { roles })).into_response()
}

pub(crate) async fn update_user_roles(
    State(state): State<AppState>,
    Path((tenant_id, user_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<UserRolesRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysUserWrite.as_str()) {
        return resp.into_response();
    }
    let user_id = match UserId::from_str(&user_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad user_id").into_response(),
    };

    let role_ids = match parse_role_ids(&req.role_ids) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let repo = RbacRepo::new(state.db.pool());
    if let Err(e) = repo.replace_user_roles(tenant_id, user_id, &role_ids).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SYS.USER.ROLES",
        "user",
        Some(user_id.to_string()),
        None,
        Some(Value::Array(
            role_ids
                .iter()
                .map(|id| Value::String(id.to_string()))
                .collect(),
        )),
    )
    .await;

    (StatusCode::OK, Json(role_ids)).into_response()
}

pub(crate) async fn get_user_data_scopes(
    State(state): State<AppState>,
    Path((tenant_id, user_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysUserRead.as_str()) {
        return resp.into_response();
    }
    let user_id = match UserId::from_str(&user_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad user_id").into_response(),
    };

    let repo = DataScopesRepo::new(state.db.pool());
    let scopes = match repo.list_user_scopes(tenant_id, user_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = DataScopeResponse {
        shop_ids: scopes.shop_ids.iter().map(|id| id.to_string()).collect(),
        warehouse_ids: scopes
            .warehouse_ids
            .iter()
            .map(|id| id.to_string())
            .collect(),
        merchant_ids: scopes
            .merchant_ids
            .iter()
            .map(|id| id.to_string())
            .collect(),
        organization_ids: scopes
            .organization_ids
            .iter()
            .map(|id| id.to_string())
            .collect(),
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn update_user_data_scopes(
    State(state): State<AppState>,
    Path((tenant_id, user_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<DataScopeUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysUserWrite.as_str()) {
        return resp.into_response();
    }
    let user_id = match UserId::from_str(&user_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad user_id").into_response(),
    };

    let shop_ids = match parse_shop_ids(&req.shop_ids) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let warehouse_ids = match parse_uuid_list(&req.warehouse_ids) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let merchant_ids = match parse_uuid_list(&req.merchant_ids) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    let organization_ids = match parse_uuid_list(&req.organization_ids) {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if let Err(resp) = ensure_shop_scope_subset(&ctx, &shop_ids) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_uuid_scope_subset(&ctx, &merchant_ids, &ctx.merchant_ids, "merchant")
    {
        return resp.into_response();
    }
    if let Err(resp) = ensure_uuid_scope_subset(
        &ctx,
        &organization_ids,
        &ctx.organization_ids,
        "organization",
    ) {
        return resp.into_response();
    }

    let repo = DataScopesRepo::new(state.db.pool());
    if let Err(e) = repo.replace_user_shops(tenant_id, user_id, &shop_ids).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }
    if let Err(e) = repo
        .replace_user_warehouses(tenant_id, user_id, &warehouse_ids)
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }
    if let Err(e) = repo
        .replace_user_merchants(tenant_id, user_id, &merchant_ids)
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }
    if let Err(e) = repo
        .replace_user_organizations(tenant_id, user_id, &organization_ids)
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SYS.USER.DATA_SCOPE",
        "user",
        Some(user_id.to_string()),
        None,
        Some(serde_json::json!({
            "shop_ids": req.shop_ids,
            "warehouse_ids": req.warehouse_ids,
            "merchant_ids": req.merchant_ids,
            "organization_ids": req.organization_ids,
        })),
    )
    .await;

    (StatusCode::OK, Json("ok")).into_response()
}
