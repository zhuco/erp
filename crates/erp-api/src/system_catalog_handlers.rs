use super::*;

pub(crate) async fn get_system_workflow_catalog(
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
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysSettingsRead.as_str()) {
        return resp.into_response();
    }

    let catalog = match load_system_workflow_catalog(state.db.pool(), tenant_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(catalog)).into_response()
}

pub(crate) async fn update_system_workflow_catalog(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<SystemWorkflowCatalogPayload>,
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

    let before = match load_system_workflow_catalog(state.db.pool(), tenant_id).await {
        Ok(v) => serde_json::to_value(v).ok(),
        Err(_) => None,
    };
    let value_json = match serde_json::to_value(&req) {
        Ok(v) => v,
        Err(_) => return bad_request("invalid workflow catalog").into_response(),
    };

    let repo = SettingsRepo::new(state.db.pool());
    let setting = match repo
        .upsert_setting(SettingUpsert {
            tenant_id,
            scope_type: "TENANT".to_string(),
            scope_id: None,
            key: SYSTEM_WORKFLOW_CATALOG_KEY.to_string(),
            value_json,
            expected_version: None,
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
        "SYS.WORKFLOWS.UPDATE",
        "setting",
        Some(setting.setting_id.to_string()),
        before,
        Some(serde_json::to_value(&req).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(req)).into_response()
}

pub(crate) async fn get_system_custom_fields_catalog(
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
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysSettingsRead.as_str()) {
        return resp.into_response();
    }

    let catalog = match load_system_custom_fields_catalog(state.db.pool(), tenant_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(catalog)).into_response()
}

pub(crate) async fn update_system_custom_fields_catalog(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<SystemCustomFieldsCatalogPayload>,
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

    let before = match load_system_custom_fields_catalog(state.db.pool(), tenant_id).await {
        Ok(v) => serde_json::to_value(v).ok(),
        Err(_) => None,
    };
    let value_json = match serde_json::to_value(&req) {
        Ok(v) => v,
        Err(_) => return bad_request("invalid custom fields catalog").into_response(),
    };

    let repo = SettingsRepo::new(state.db.pool());
    let setting = match repo
        .upsert_setting(SettingUpsert {
            tenant_id,
            scope_type: "TENANT".to_string(),
            scope_id: None,
            key: SYSTEM_CUSTOM_FIELDS_CATALOG_KEY.to_string(),
            value_json,
            expected_version: None,
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
        "SYS.CUSTOM_FIELDS.UPDATE",
        "setting",
        Some(setting.setting_id.to_string()),
        before,
        Some(serde_json::to_value(&req).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(req)).into_response()
}

pub(crate) async fn get_system_integrations_catalog(
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
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysSettingsRead.as_str()) {
        return resp.into_response();
    }

    let catalog = match load_system_integrations_catalog(state.db.pool(), tenant_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(catalog)).into_response()
}

pub(crate) async fn update_system_integrations_catalog(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<SystemIntegrationsCatalogPayload>,
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

    let before = match load_system_integrations_catalog(state.db.pool(), tenant_id).await {
        Ok(v) => serde_json::to_value(v).ok(),
        Err(_) => None,
    };
    let value_json = match serde_json::to_value(&req) {
        Ok(v) => v,
        Err(_) => return bad_request("invalid integrations catalog").into_response(),
    };

    let repo = SettingsRepo::new(state.db.pool());
    let setting = match repo
        .upsert_setting(SettingUpsert {
            tenant_id,
            scope_type: "TENANT".to_string(),
            scope_id: None,
            key: SYSTEM_INTEGRATIONS_CATALOG_KEY.to_string(),
            value_json,
            expected_version: None,
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
        "SYS.INTEGRATIONS.UPDATE",
        "setting",
        Some(setting.setting_id.to_string()),
        before,
        Some(serde_json::to_value(&req).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(req)).into_response()
}

pub(crate) async fn get_system_audit_reports_catalog(
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
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::SysSettingsRead.as_str()) {
        return resp.into_response();
    }

    let catalog = match load_system_audit_reports_catalog(state.db.pool(), tenant_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(catalog)).into_response()
}

pub(crate) async fn update_system_audit_reports_catalog(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<SystemAuditReportsCatalogPayload>,
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

    let before = match load_system_audit_reports_catalog(state.db.pool(), tenant_id).await {
        Ok(v) => serde_json::to_value(v).ok(),
        Err(_) => None,
    };
    let value_json = match serde_json::to_value(&req) {
        Ok(v) => v,
        Err(_) => return bad_request("invalid audit reports catalog").into_response(),
    };

    let repo = SettingsRepo::new(state.db.pool());
    let setting = match repo
        .upsert_setting(SettingUpsert {
            tenant_id,
            scope_type: "TENANT".to_string(),
            scope_id: None,
            key: SYSTEM_AUDIT_REPORTS_CATALOG_KEY.to_string(),
            value_json,
            expected_version: None,
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
        "SYS.AUDIT_REPORTS.UPDATE",
        "setting",
        Some(setting.setting_id.to_string()),
        before,
        Some(serde_json::to_value(&req).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(req)).into_response()
}
