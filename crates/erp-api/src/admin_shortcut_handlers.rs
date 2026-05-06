use super::*;

pub(crate) async fn admin_list_permission_catalog(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = ctx.tenant_id.to_string();
    list_permission_catalog(State(state), Path(tenant_id), Extension(ctx)).await
}

pub(crate) async fn admin_list_roles(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = ctx.tenant_id.to_string();
    list_roles(State(state), Path(tenant_id), Extension(ctx)).await
}

pub(crate) async fn admin_create_role(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<RoleCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = ctx.tenant_id.to_string();
    create_role(
        State(state),
        Path(tenant_id),
        Extension(ctx),
        Extension(req_ctx),
        Json(req),
    )
    .await
}

pub(crate) async fn admin_update_role(
    State(state): State<AppState>,
    Path(role_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<RoleUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = ctx.tenant_id.to_string();
    update_role(
        State(state),
        Path((tenant_id, role_id)),
        Extension(ctx),
        Extension(req_ctx),
        Json(req),
    )
    .await
}

pub(crate) async fn admin_list_role_permissions(
    State(state): State<AppState>,
    Path(role_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = ctx.tenant_id.to_string();
    list_role_permissions(State(state), Path((tenant_id, role_id)), Extension(ctx)).await
}

pub(crate) async fn admin_update_role_permissions(
    State(state): State<AppState>,
    Path(role_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<RolePermissionsRequest>,
) -> impl IntoResponse {
    let tenant_id = ctx.tenant_id.to_string();
    update_role_permissions(
        State(state),
        Path((tenant_id, role_id)),
        Extension(ctx),
        Extension(req_ctx),
        Json(req),
    )
    .await
}

pub(crate) async fn admin_list_users(
    State(state): State<AppState>,
    Query(query): Query<UserListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = ctx.tenant_id.to_string();
    list_users(State(state), Path(tenant_id), Query(query), Extension(ctx)).await
}

pub(crate) async fn admin_create_user(
    State(state): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<UserCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = ctx.tenant_id.to_string();
    create_user(
        State(state),
        Path(tenant_id),
        Extension(ctx),
        Extension(req_ctx),
        Json(req),
    )
    .await
}

pub(crate) async fn admin_update_user(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<UserUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = ctx.tenant_id.to_string();
    update_user(
        State(state),
        Path((tenant_id, user_id)),
        Extension(ctx),
        Extension(req_ctx),
        Json(req),
    )
    .await
}

pub(crate) async fn admin_get_user_roles(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = ctx.tenant_id.to_string();
    get_user_roles(State(state), Path((tenant_id, user_id)), Extension(ctx)).await
}

pub(crate) async fn admin_update_user_roles(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<UserRolesRequest>,
) -> impl IntoResponse {
    let tenant_id = ctx.tenant_id.to_string();
    update_user_roles(
        State(state),
        Path((tenant_id, user_id)),
        Extension(ctx),
        Extension(req_ctx),
        Json(req),
    )
    .await
}

pub(crate) async fn admin_get_user_data_scopes(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = ctx.tenant_id.to_string();
    get_user_data_scopes(State(state), Path((tenant_id, user_id)), Extension(ctx)).await
}

pub(crate) async fn admin_update_user_data_scopes(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<DataScopeUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = ctx.tenant_id.to_string();
    update_user_data_scopes(
        State(state),
        Path((tenant_id, user_id)),
        Extension(ctx),
        Extension(req_ctx),
        Json(req),
    )
    .await
}

pub(crate) async fn admin_list_audit_logs(
    State(state): State<AppState>,
    Query(query): Query<AuditLogListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = ctx.tenant_id.to_string();
    list_audit_logs(State(state), Path(tenant_id), Query(query), Extension(ctx)).await
}

pub(crate) async fn admin_list_dead_letters(
    State(state): State<AppState>,
    Query(query): Query<DeadLetterListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = ctx.tenant_id.to_string();
    list_dead_letters(State(state), Path(tenant_id), Query(query), Extension(ctx)).await
}

pub(crate) async fn admin_replay_dead_letter(
    State(state): State<AppState>,
    Path(dead_letter_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = ctx.tenant_id.to_string();
    replay_dead_letter(
        State(state),
        Path((tenant_id, dead_letter_id)),
        Extension(ctx),
        Extension(req_ctx),
    )
    .await
}
