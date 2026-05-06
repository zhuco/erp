use super::*;

pub(crate) fn ensure_permission(
    ctx: &AuthContext,
    perm: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if ctx.is_admin {
        return Ok(());
    }
    let required = normalize_permission_code(perm);
    if ctx.perms.iter().any(|p| p == &required) {
        Ok(())
    } else {
        Err(forbidden("missing permission"))
    }
}

pub(crate) fn ensure_permission_any(
    ctx: &AuthContext,
    perms: &[&str],
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if ctx.is_admin {
        return Ok(());
    }
    if perms.iter().any(|perm| {
        let required = normalize_permission_code(perm);
        ctx.perms.iter().any(|p| p == &required)
    }) {
        Ok(())
    } else {
        Err(forbidden("missing permission"))
    }
}

pub(crate) fn ensure_admin(ctx: &AuthContext) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if ctx.is_admin {
        Ok(())
    } else {
        Err(forbidden("admin access required"))
    }
}

pub(crate) fn ensure_tenant_access(
    ctx: &AuthContext,
    tenant_id: TenantId,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if ctx.tenant_id == tenant_id || ctx.is_admin {
        Ok(())
    } else {
        Err(forbidden("tenant mismatch"))
    }
}

pub(crate) fn ensure_shop_access(
    ctx: &AuthContext,
    shop_id: ShopId,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if ctx.is_admin || ctx.shop_ids.is_empty() || ctx.shop_ids.contains(&shop_id) {
        Ok(())
    } else {
        Err(forbidden("shop access denied"))
    }
}

pub(crate) fn ensure_warehouse_access(
    ctx: &AuthContext,
    warehouse_id: uuid::Uuid,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if ctx.is_admin || ctx.warehouse_ids.is_empty() || ctx.warehouse_ids.contains(&warehouse_id) {
        Ok(())
    } else {
        Err(forbidden("warehouse access denied"))
    }
}

pub(crate) fn shop_scope_filter(ctx: &AuthContext) -> Option<Vec<ShopId>> {
    if ctx.is_admin || ctx.shop_ids.is_empty() {
        None
    } else {
        Some(ctx.shop_ids.clone())
    }
}

pub(crate) fn warehouse_scope_filter(ctx: &AuthContext) -> Option<Vec<uuid::Uuid>> {
    if ctx.is_admin || ctx.warehouse_ids.is_empty() {
        None
    } else {
        Some(ctx.warehouse_ids.clone())
    }
}

pub(crate) fn ensure_shop_scope_subset(
    ctx: &AuthContext,
    requested: &[ShopId],
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if ctx.is_admin || ctx.shop_ids.is_empty() {
        return Ok(());
    }
    for shop_id in requested {
        if !ctx.shop_ids.contains(shop_id) {
            return Err(forbidden("shop access denied"));
        }
    }
    Ok(())
}

pub(crate) fn ensure_uuid_scope_subset(
    ctx: &AuthContext,
    requested: &[uuid::Uuid],
    allowed: &[uuid::Uuid],
    label: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if ctx.is_admin || allowed.is_empty() {
        return Ok(());
    }
    for id in requested {
        if !allowed.contains(id) {
            return Err(forbidden(&format!("{label} access denied")));
        }
    }
    Ok(())
}

pub(crate) async fn pdd_only_mode_enabled(
    state: &AppState,
    tenant_id: TenantId,
) -> Result<bool, (StatusCode, Json<ErrorResponse>)> {
    if let Some(enabled) = feature_flags::pdd_only_mode_env() {
        return Ok(enabled);
    }

    let tenants_repo = TenantsRepo::new(state.db.pool());
    match tenants_repo.is_pdd_only_mode(tenant_id).await {
        Ok(v) => Ok(v),
        Err(e) => {
            warn!(tenant_id = %tenant_id, error = %e, "failed to load pdd-only mode");
            Ok(false)
        }
    }
}

pub(crate) async fn feature_flag_enabled(
    state: &AppState,
    tenant_id: TenantId,
    shop_id: Option<ShopId>,
    key: FeatureFlagKey,
) -> Result<bool, (StatusCode, Json<ErrorResponse>)> {
    let repo = FeatureFlagsRepo::new(state.db.pool());
    let flag = FeatureFlag::from_common_key(key);
    match repo.is_enabled(tenant_id, shop_id, flag).await {
        Ok(v) => Ok(v),
        Err(e) => {
            let shop_id_str = shop_id.map(|id| id.to_string());
            warn!(
                tenant_id = %tenant_id,
                shop_id = shop_id_str.as_deref(),
                feature = %key,
                error = %e,
                "failed to load feature flag"
            );
            Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "failed to load feature flag",
            ))
        }
    }
}

pub(crate) fn is_write_method(method: &Method) -> bool {
    matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    )
}

pub(crate) fn tenant_id_from_path(path: &str) -> Option<TenantId> {
    let mut segments = path.trim_matches('/').split('/');
    match (segments.next(), segments.next(), segments.next()) {
        (Some("v1"), Some("tenants"), Some(raw)) => TenantId::from_str(raw).ok(),
        _ => None,
    }
}

pub(crate) async fn read_only_enabled(
    state: &AppState,
    tenant_id: TenantId,
    shop_id: Option<ShopId>,
) -> Result<bool, (StatusCode, Json<ErrorResponse>)> {
    let repo = FeatureFlagsRepo::new(state.db.pool());
    match repo.is_read_only(tenant_id, shop_id).await {
        Ok(v) => Ok(v),
        Err(e) => {
            let shop_id_str = shop_id.map(|id| id.to_string());
            warn!(
                tenant_id = %tenant_id,
                shop_id = shop_id_str.as_deref(),
                error = %e,
                "failed to load read-only flag"
            );
            Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "failed to load read-only flag",
            ))
        }
    }
}

pub(crate) fn pdd_only_forbidden() -> (StatusCode, Json<ErrorResponse>) {
    error_response(
        StatusCode::FORBIDDEN,
        "pdd_only_mode",
        "pdd-only mode: platform not allowed",
    )
}

pub(crate) fn read_only_forbidden() -> (StatusCode, Json<ErrorResponse>) {
    error_response(
        StatusCode::FORBIDDEN,
        "feature_disabled",
        "read-only mode active",
    )
}

pub(crate) fn feature_flag_forbidden(key: FeatureFlagKey) -> (StatusCode, Json<ErrorResponse>) {
    error_response(
        StatusCode::FORBIDDEN,
        "feature_disabled",
        &format!("feature disabled: {key}"),
    )
}

pub(crate) fn non_empty(raw: impl AsRef<str>) -> Option<String> {
    let trimmed = raw.as_ref().trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub(crate) fn normalize_string_list(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        let normalized = trimmed.to_string();
        if seen.insert(normalized.clone()) {
            result.push(normalized);
        }
    }
    result
}

pub(crate) fn merge_auth_meta_json(
    base: Value,
    override_json: Option<Value>,
    patch: Value,
) -> Value {
    let mut map = match base {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    if let Some(Value::Object(override_map)) = override_json {
        for (key, value) in override_map {
            map.insert(key, value);
        }
    }
    if let Value::Object(patch_map) = patch {
        for (key, value) in patch_map {
            map.insert(key, value);
        }
    }
    Value::Object(map)
}

pub(crate) fn new_device_token() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub(crate) fn parse_shop_status(
    raw: Option<String>,
) -> Result<Option<String>, (StatusCode, Json<ErrorResponse>)> {
    let Some(status) = raw else {
        return Ok(None);
    };
    let normalized = status.trim().to_uppercase();
    if normalized.is_empty() {
        return Ok(None);
    }
    match normalized.as_str() {
        "AUTHORIZED" | "EXPIRED" | "DISABLED" => Ok(Some(normalized)),
        _ => Err(bad_request("invalid shop status")),
    }
}

pub(crate) fn parse_channel_status(
    raw: Option<String>,
) -> Result<Option<String>, (StatusCode, Json<ErrorResponse>)> {
    let Some(status) = raw else {
        return Ok(None);
    };
    let normalized = status.trim().to_uppercase();
    if normalized.is_empty() {
        return Ok(None);
    }
    match normalized.as_str() {
        "ACTIVE" | "DISABLED" => Ok(Some(normalized)),
        _ => Err(bad_request("invalid channel status")),
    }
}

pub(crate) fn bad_request(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    error_response(StatusCode::BAD_REQUEST, "bad_request", message)
}

pub(crate) fn forbidden(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    error_response(StatusCode::FORBIDDEN, "forbidden", message)
}

pub(crate) fn conflict(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    error_response(StatusCode::CONFLICT, "conflict", message)
}

pub(crate) fn not_found(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    error_response(StatusCode::NOT_FOUND, "not_found", message)
}

pub(crate) fn error_response(
    status: StatusCode,
    code: &str,
    message: &str,
) -> (StatusCode, Json<ErrorResponse>) {
    (
        status,
        Json(ErrorResponse {
            code: code.to_string(),
            message: message.to_string(),
            details: None,
            request_id: None,
        }),
    )
}
