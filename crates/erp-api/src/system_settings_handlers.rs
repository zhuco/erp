use super::*;

pub(crate) async fn list_settings(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<SettingsListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysSettingsRead.as_str()) {
        return resp.into_response();
    }

    let scope_id = match query.scope_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad scope_id").into_response(),
        },
        None => None,
    };

    let repo = SettingsRepo::new(state.db.pool());
    let settings = match repo
        .list_settings(
            tenant_id,
            SettingListFilters {
                scope_type: query.scope_type.clone(),
                scope_id,
                key: query.key.clone(),
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(settings)).into_response()
}

pub(crate) async fn upsert_setting(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<SettingUpsertRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysSettingsWrite.as_str()) {
        return resp.into_response();
    }

    let scope_id = match req.scope_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad scope_id").into_response(),
        },
        None => None,
    };

    let repo = SettingsRepo::new(state.db.pool());
    let before = match repo
        .list_settings(
            tenant_id,
            SettingListFilters {
                scope_type: Some(req.scope_type.clone()),
                scope_id,
                key: Some(req.key.clone()),
            },
        )
        .await
    {
        Ok(mut settings) => settings
            .pop()
            .map(|s| serde_json::to_value(s).unwrap_or(Value::Null)),
        Err(_) => None,
    };

    let setting = match repo
        .upsert_setting(SettingUpsert {
            tenant_id,
            scope_type: req.scope_type.to_uppercase(),
            scope_id,
            key: req.key.clone(),
            value_json: req.value,
            expected_version: req.expected_version,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => {
            if e.to_string().contains("version conflict") {
                return conflict("settings version conflict").into_response();
            }
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
        }
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "SYS.SETTINGS.UPDATE",
        "setting",
        Some(setting.setting_id.to_string()),
        before,
        Some(serde_json::to_value(&setting).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(setting)).into_response()
}
