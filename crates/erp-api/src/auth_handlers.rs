use super::*;

#[derive(Debug, Deserialize)]
pub(crate) struct LoginRequest {
    tenant_id: String,
    email: String,
    password: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct LoginResponse {
    access_token: String,
    token_type: &'static str,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CurrentUserUpdateRequest {
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CurrentUserPasswordRequest {
    current_password: String,
    new_password: String,
}

pub(crate) async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&req.tenant_id) {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, "bad tenant_id").into_response(),
    };

    let auth_repo = AuthRepo::new(state.db.pool());
    let user = match auth_repo.find_user_by_email(tenant_id, &req.email).await {
        Ok(Some(v)) => v,
        Ok(None) => return (StatusCode::UNAUTHORIZED, "invalid credentials").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    if user.status != "ACTIVE" {
        return (StatusCode::UNAUTHORIZED, "user disabled").into_response();
    }

    let parsed_hash = match argon2::PasswordHash::new(&user.password_hash) {
        Ok(v) => v,
        Err(_) => return (StatusCode::UNAUTHORIZED, "invalid credentials").into_response(),
    };

    if argon2::Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return (StatusCode::UNAUTHORIZED, "invalid credentials").into_response();
    }

    let perms = match auth_repo.list_permissions(tenant_id, user.user_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let now = Utc::now().timestamp() as usize;
    let exp = now + 3600; // 1 hour
    let claims = Claims {
        sub: user.user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        perms,
        is_admin: false,
        iat: now,
        exp,
    };

    let token = match state.jwt.encode(&claims) {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    Json(LoginResponse {
        access_token: token,
        token_type: "Bearer",
        expires_in: 3600,
    })
    .into_response()
}

pub(crate) async fn get_current_user(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let repo = RbacRepo::new(state.db.pool());
    let user = match repo.get_user(ctx.tenant_id, ctx.user_id).await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return error_response(StatusCode::UNAUTHORIZED, "unauthorized", "user missing")
                .into_response();
        }
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let roles = match repo.list_user_roles(ctx.tenant_id, ctx.user_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = CurrentUserResponse {
        user,
        roles,
        permissions: ctx.perms.clone(),
        data_scopes: DataScopeResponse {
            shop_ids: ctx.shop_ids.iter().map(|id| id.to_string()).collect(),
            warehouse_ids: ctx.warehouse_ids.iter().map(|id| id.to_string()).collect(),
            merchant_ids: ctx.merchant_ids.iter().map(|id| id.to_string()).collect(),
            organization_ids: ctx
                .organization_ids
                .iter()
                .map(|id| id.to_string())
                .collect(),
        },
    };

    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn update_current_user_profile(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<CurrentUserUpdateRequest>,
) -> impl IntoResponse {
    let email = match req.email.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => return bad_request("email required").into_response(),
    };

    let repo = RbacRepo::new(state.db.pool());
    let before = match repo.get_user(ctx.tenant_id, ctx.user_id).await {
        Ok(Some(user)) => serde_json::to_value(user).ok(),
        Ok(None) => return not_found("user not found").into_response(),
        Err(_) => None,
    };

    let user = match repo
        .update_user(
            ctx.tenant_id,
            ctx.user_id,
            UserUpdate {
                email: Some(email),
                password_hash: None,
                status: None,
            },
        )
        .await
    {
        Ok(Some(user)) => user,
        Ok(None) => return not_found("user not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SYS.USER.PROFILE.UPDATE",
        "user",
        Some(user.user_id.to_string()),
        before,
        Some(serde_json::to_value(&user).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(user)).into_response()
}

pub(crate) async fn update_current_user_password(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<CurrentUserPasswordRequest>,
) -> impl IntoResponse {
    let current_password = req.current_password.trim();
    let new_password = req.new_password.trim();
    if current_password.is_empty() {
        return bad_request("current_password required").into_response();
    }
    if new_password.is_empty() {
        return bad_request("new_password required").into_response();
    }

    let auth_repo = AuthRepo::new(state.db.pool());
    let user_row = match auth_repo.get_user_by_id(ctx.tenant_id, ctx.user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => return not_found("user not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let parsed_hash = match argon2::PasswordHash::new(&user_row.password_hash) {
        Ok(v) => v,
        Err(_) => return (StatusCode::UNAUTHORIZED, "invalid credentials").into_response(),
    };
    if argon2::Argon2::default()
        .verify_password(current_password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return (StatusCode::UNAUTHORIZED, "invalid credentials").into_response();
    }

    let salt = argon2::password_hash::SaltString::generate(&mut rand::thread_rng());
    let hash = match argon2::Argon2::default().hash_password(new_password.as_bytes(), &salt) {
        Ok(v) => v.to_string(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "hash failed").into_response(),
    };

    let repo = RbacRepo::new(state.db.pool());
    let before = match repo.get_user(ctx.tenant_id, ctx.user_id).await {
        Ok(Some(user)) => serde_json::to_value(user).ok(),
        Ok(None) => return not_found("user not found").into_response(),
        Err(_) => None,
    };

    let user = match repo
        .update_user(
            ctx.tenant_id,
            ctx.user_id,
            UserUpdate {
                email: None,
                password_hash: Some(hash),
                status: None,
            },
        )
        .await
    {
        Ok(Some(user)) => user,
        Ok(None) => return not_found("user not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SYS.USER.PASSWORD.UPDATE",
        "user",
        Some(user.user_id.to_string()),
        before,
        Some(serde_json::to_value(&user).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(user)).into_response()
}
