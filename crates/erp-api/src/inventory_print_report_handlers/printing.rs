use super::*;

pub(crate) async fn list_print_devices(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<PrintDeviceListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "print_devices.read") {
        return resp.into_response();
    }

    let cursor = match query.cursor.as_deref() {
        Some(raw) => match PrintDeviceCursor::decode(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad cursor").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let result = match state
        .print_devices_repo
        .list_devices(
            tenant_id,
            PrintDeviceListFilters {
                fingerprint: query.fingerprint,
                status: query.status,
                cursor,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = ListResponse {
        data: result.items,
        next_cursor: result.next_cursor.map(|c| c.encode()),
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn list_print_mappings(
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
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::PrintMappingRead.as_str()) {
        return resp.into_response();
    }

    let repo = PrintMappingsRepo::new(state.db.pool());
    match repo.list_mappings(tenant_id).await {
        Ok(v) => (StatusCode::OK, Json(v)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    }
}

pub(crate) async fn upsert_print_mapping(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PrintMappingUpsertRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::PrintMappingWrite.as_str()) {
        return resp.into_response();
    }

    let doc_type = req.doc_type.trim();
    if doc_type.is_empty() {
        return bad_request("doc_type required").into_response();
    }
    let template_id = match uuid::Uuid::parse_str(req.template_id.trim()) {
        Ok(v) => v,
        Err(_) => return bad_request("bad template_id").into_response(),
    };

    let device_fingerprint = req
        .device_fingerprint
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if let Some(fingerprint) = device_fingerprint.as_deref() {
        let devices_repo = PrintDevicesRepoCore::new(state.db.pool());
        match devices_repo
            .get_device_by_fingerprint(tenant_id, fingerprint)
            .await
        {
            Ok(Some(_)) => {}
            Ok(None) => return bad_request("print device not found").into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }
    }

    let printer_name = req
        .printer_name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let repo = PrintMappingsRepo::new(state.db.pool());
    let mapping = match repo
        .upsert_mapping(PrintMappingUpsert {
            tenant_id,
            doc_type: doc_type.to_string(),
            template_id,
            device_fingerprint,
            printer_name,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(mapping)).into_response()
}

pub(crate) async fn delete_print_mapping(
    State(state): State<AppState>,
    Path((tenant_id, doc_type)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let doc_type = doc_type.trim();
    if doc_type.is_empty() {
        return bad_request("doc_type required").into_response();
    }
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, PermissionCode::PrintMappingWrite.as_str()) {
        return resp.into_response();
    }

    let repo = PrintMappingsRepo::new(state.db.pool());
    match repo.delete_mapping(tenant_id, doc_type).await {
        Ok(Some(v)) => (StatusCode::OK, Json(v)).into_response(),
        Ok(None) => not_found("print mapping not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    }
}

pub(crate) async fn register_print_device(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PrintDeviceRegisterRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "print_devices.write") {
        return resp.into_response();
    }

    let fingerprint = req.device_fingerprint.trim();
    if fingerprint.is_empty() {
        return bad_request("missing device_fingerprint").into_response();
    }
    let name = req
        .name
        .and_then(non_empty)
        .unwrap_or_else(|| fingerprint.to_string());
    let device_token = new_device_token();

    let is_default = match state.print_devices_repo.get_default_device(tenant_id).await {
        Ok(Some(_)) => false,
        Ok(None) => true,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let device = match state
        .print_devices_repo
        .create_device(PrintDeviceCreate {
            tenant_id,
            name,
            fingerprint: fingerprint.to_string(),
            status: req.status.unwrap_or_else(|| "ACTIVE".to_string()),
            is_default,
            device_token: device_token.clone(),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = PrintDeviceRegisterResponse {
        device_id: device.device_id,
        tenant_id: uuid::Uuid::from(device.tenant_id),
        name: device.name,
        fingerprint: device.fingerprint,
        status: device.status,
        is_default: device.is_default,
        device_token,
        created_at: device.created_at,
        updated_at: device.updated_at,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn create_print_device(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PrintDeviceCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "print_devices.write") {
        return resp.into_response();
    }

    let device = match state
        .print_devices_repo
        .create_device(PrintDeviceCreate {
            tenant_id,
            name: req.name,
            fingerprint: req.fingerprint,
            status: req.status.unwrap_or_else(|| "ACTIVE".to_string()),
            is_default: req.is_default.unwrap_or(false),
            device_token: new_device_token(),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(device)).into_response()
}

pub(crate) async fn update_print_device(
    State(state): State<AppState>,
    Path((tenant_id, device_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PrintDeviceUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "print_devices.write") {
        return resp.into_response();
    }

    let device_id = match uuid::Uuid::parse_str(&device_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad device_id").into_response(),
    };

    let device = match state
        .print_devices_repo
        .update_device(
            tenant_id,
            device_id,
            PrintDeviceUpdate {
                name: req.name,
                status: req.status,
                is_default: req.is_default,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("device not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(device)).into_response()
}

pub(crate) async fn list_print_templates(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<PrintTemplateListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "print_templates.read") {
        return resp.into_response();
    }

    let cursor = match query.cursor.as_deref() {
        Some(raw) => match PrintTemplateCursor::decode(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad cursor").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let shop_id = match query.shop_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };

    let result = match state
        .print_templates_repo
        .list_templates(
            tenant_id,
            PrintTemplateListFilters {
                shop_id,
                platform: query.platform,
                template_type: query.template_type,
                enabled: query.enabled,
                cursor,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = ListResponse {
        data: result.items,
        next_cursor: result.next_cursor.map(|c| c.encode()),
    };
    (StatusCode::OK, Json(response)).into_response()
}

#[allow(clippy::result_large_err)]
pub(crate) fn normalize_print_template_format(value: Option<String>) -> Result<String, Response> {
    let normalized = value
        .unwrap_or_else(|| "HTML".to_string())
        .trim()
        .to_uppercase();
    match normalized.as_str() {
        "HTML" | "ZPL" | "TEXT" => Ok(normalized),
        _ => Err(bad_request("unsupported content_format").into_response()),
    }
}

pub(crate) async fn create_print_template(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PrintTemplateCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "print_templates.write") {
        return resp.into_response();
    }

    let shop_id = match uuid::Uuid::parse_str(req.shop_id.trim()) {
        Ok(v) => v,
        Err(_) => return bad_request("bad shop_id").into_response(),
    };
    let version = req.version.unwrap_or(1);
    let content_format = match normalize_print_template_format(req.content_format) {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let template = match state
        .print_templates_repo
        .create_template(PrintTemplateCreate {
            tenant_id,
            shop_id,
            platform: req.platform,
            template_type: req.template_type,
            platform_template_id: req.platform_template_id,
            version,
            content_format,
            content: req.content.unwrap_or_else(|| serde_json::json!({})),
            enabled: req.enabled.unwrap_or(true),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let logs_repo = AuditLogsRepo::new(state.db.pool());
    let after_json = serde_json::to_value(&template).unwrap_or(serde_json::json!({}));
    if let Err(err) = logs_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "PRINT_TEMPLATE.CREATE".to_string(),
            entity_type: "print_template".to_string(),
            entity_id: Some(template.template_id.to_string()),
            before_json: None,
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("audit log failed: {err}"),
        )
            .into_response();
    }

    (StatusCode::OK, Json(template)).into_response()
}

pub(crate) async fn update_print_template(
    State(state): State<AppState>,
    Path((tenant_id, template_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<PrintTemplateUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "print_templates.write") {
        return resp.into_response();
    }

    let template_id = match uuid::Uuid::parse_str(&template_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad template_id").into_response(),
    };
    let content_format = match req.content_format {
        Some(raw) => match normalize_print_template_format(Some(raw)) {
            Ok(v) => Some(v),
            Err(resp) => return resp,
        },
        None => None,
    };

    let template = match state
        .print_templates_repo
        .update_template(
            tenant_id,
            template_id,
            PrintTemplateUpdate {
                platform_template_id: req.platform_template_id,
                content_format,
                content: req.content,
                enabled: req.enabled,
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("template not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let logs_repo = AuditLogsRepo::new(state.db.pool());
    let after_json = serde_json::to_value(&template).unwrap_or(serde_json::json!({}));
    if let Err(err) = logs_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "PRINT_TEMPLATE.UPDATE".to_string(),
            entity_type: "print_template".to_string(),
            entity_id: Some(template.template_id.to_string()),
            before_json: None,
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("audit log failed: {err}"),
        )
            .into_response();
    }

    (StatusCode::OK, Json(template)).into_response()
}

pub(crate) async fn get_print_template(
    State(state): State<AppState>,
    Path((tenant_id, template_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "print_templates.read") {
        return resp.into_response();
    }

    let template_id = match uuid::Uuid::parse_str(&template_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad template_id").into_response(),
    };

    let template = match state
        .print_templates_repo
        .get_template(tenant_id, template_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("template not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(template)).into_response()
}

pub(crate) async fn delete_print_template(
    State(state): State<AppState>,
    Path((tenant_id, template_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "print_templates.write") {
        return resp.into_response();
    }

    let template_id = match uuid::Uuid::parse_str(&template_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad template_id").into_response(),
    };

    let template = match state
        .print_templates_repo
        .delete_template(tenant_id, template_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("template not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let logs_repo = AuditLogsRepo::new(state.db.pool());
    let before_json = serde_json::to_value(&template).unwrap_or(serde_json::json!({}));
    if let Err(err) = logs_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "PRINT_TEMPLATE.DELETE".to_string(),
            entity_type: "print_template".to_string(),
            entity_id: Some(template.template_id.to_string()),
            before_json: Some(before_json),
            after_json: None,
            ip: None,
            ua: None,
        })
        .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("audit log failed: {err}"),
        )
            .into_response();
    }

    (StatusCode::OK, Json(template)).into_response()
}

pub(crate) async fn list_print_jobs(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<PrintJobListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "print_jobs.read") {
        return resp.into_response();
    }

    let job_id = match query.job_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad job_id").into_response(),
        },
        None => None,
    };
    let ref_id = match query.ref_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad ref_id").into_response(),
        },
        None => None,
    };
    let cursor = match query.cursor.as_deref() {
        Some(raw) => match PrintJobCursor::decode(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad cursor").into_response(),
        },
        None => None,
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let result = match state
        .print_jobs_repo
        .list_jobs(
            tenant_id,
            PrintJobListFilters {
                job_id,
                job_type: query.job_type,
                ref_type: query.ref_type,
                ref_id,
                status: query.status,
                cursor,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = ListResponse {
        data: result.items,
        next_cursor: result.next_cursor.map(|c| c.encode()),
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn attach_waybill_to_payload(
    state: &AppState,
    tenant_id: TenantId,
    payload: &mut Value,
) -> Result<(), Response> {
    let label: PrintShippingLabelPayload = match serde_json::from_value(payload.clone()) {
        Ok(v) => v,
        Err(_) => return Err(bad_request("invalid print payload").into_response()),
    };
    let order_id = match OrderId::from_str(&label.order_id) {
        Ok(v) => v,
        Err(_) => return Err(bad_request("bad order_id").into_response()),
    };
    let shop_id = match ShopId::from_str(&label.shop_id) {
        Ok(v) => v,
        Err(_) => return Err(bad_request("bad shop_id").into_response()),
    };

    let shipments_repo = ShipmentsRepo::new(state.db.pool());
    let (shipment_id, shipment) = match label.shipment_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => {
                let shipment = match shipments_repo.get_by_id(tenant_id, v).await {
                    Ok(v) => v,
                    Err(e) => {
                        return Err(
                            (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()
                        )
                    }
                };
                (v, shipment)
            }
            Err(_) => return Err(bad_request("bad shipment_id").into_response()),
        },
        None => {
            let shipment = match shipments_repo
                .get_by_order_id(tenant_id, uuid::Uuid::from(order_id))
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response())
                }
            };
            let shipment_id = shipment.as_ref().map(|s| s.shipment_id).unwrap_or_else(|| {
                uuid::Uuid::new_v5(&uuid::Uuid::from(tenant_id), label.order_id.as_bytes())
            });
            (shipment_id, shipment)
        }
    };

    let carrier_from_payload = label.carrier_code.clone().and_then(non_empty);
    let carrier_from_shipment = shipment.as_ref().and_then(|s| s.carrier_code.clone());
    let carrier_code = if let Some(code) = carrier_from_payload.or(carrier_from_shipment) {
        Some(code)
    } else {
        let shops_repo = ShopsRepo::new(state.db.pool());
        match shops_repo
            .get_default_carrier_code(tenant_id, shop_id)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response())
            }
        }
    };
    let Some(carrier_code) = carrier_code else {
        return Err(bad_request("carrier_code required").into_response());
    };

    let preferred_waybill = label
        .tracking_no
        .clone()
        .and_then(non_empty)
        .or_else(|| shipment.as_ref().and_then(|s| s.tracking_no.clone()));

    let waybills_repo = WaybillsRepo::new(state.db.pool());
    let reserved = match waybills_repo
        .reserve_waybill(
            tenant_id,
            uuid::Uuid::from(shop_id),
            &carrier_code,
            shipment_id,
            preferred_waybill.as_deref(),
        )
        .await
    {
        Ok(v) => v,
        Err(err) => {
            let message = err.to_string();
            if message.contains("no available waybill")
                || message.contains("shipment already reserved")
            {
                return Err(conflict("waybill not available").into_response());
            }
            return Err((StatusCode::INTERNAL_SERVER_ERROR, message).into_response());
        }
    };

    let obj = payload
        .as_object_mut()
        .ok_or_else(|| bad_request("invalid payload").into_response())?;
    obj.insert(
        "waybill_no".to_string(),
        Value::String(reserved.waybill_no.clone()),
    );
    if label.shipment_id.is_none() {
        obj.insert(
            "shipment_id".to_string(),
            Value::String(shipment_id.to_string()),
        );
    }
    if label
        .carrier_code
        .as_ref()
        .and_then(|v| non_empty(v.clone()))
        .is_none()
    {
        obj.insert(
            "carrier_code".to_string(),
            Value::String(carrier_code.clone()),
        );
    }
    if label
        .tracking_no
        .as_ref()
        .and_then(|v| non_empty(v.clone()))
        .is_none()
    {
        obj.insert(
            "tracking_no".to_string(),
            Value::String(reserved.waybill_no.clone()),
        );
    }

    Ok(())
}

pub(crate) async fn create_print_job(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<PrintJobCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "print_jobs.write") {
        return resp.into_response();
    }

    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::Printing).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        warn!(
            tenant_id = %tenant_id,
            "feature disabled: printing"
        );
        return feature_flag_forbidden(FeatureFlagKey::Printing).into_response();
    }

    let job_type = req.job_type.trim().to_string();
    if job_type.is_empty() {
        return bad_request("job_type required").into_response();
    }
    let ref_type = req.ref_type.trim().to_string();
    if ref_type.is_empty() {
        return bad_request("ref_type required").into_response();
    }
    let ref_id = match uuid::Uuid::parse_str(req.ref_id.trim()) {
        Ok(v) => v,
        Err(_) => return bad_request("bad ref_id").into_response(),
    };

    let template_id = match req.template_id.and_then(non_empty) {
        Some(raw) => match uuid::Uuid::parse_str(&raw) {
            Ok(v) => v,
            Err(_) => return bad_request("bad template_id").into_response(),
        },
        None => match state
            .print_templates_repo
            .get_default_template(tenant_id, &job_type, None)
            .await
        {
            Ok(Some(template)) => template.template_id,
            Ok(None) => return bad_request("default print template not found").into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        },
    };

    let local_agent_enabled = match feature_flag_enabled(
        &state,
        tenant_id,
        None,
        FeatureFlagKey::PrintLocalAgent,
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp.into_response(),
    };
    if !local_agent_enabled
        && req
            .device_fingerprint
            .as_deref()
            .map(|raw| !raw.trim().is_empty())
            .unwrap_or(false)
    {
        return feature_flag_forbidden(FeatureFlagKey::PrintLocalAgent).into_response();
    }

    let device_fingerprint = if !local_agent_enabled {
        None
    } else if let Some(raw) = req.device_fingerprint.as_deref() {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            let devices_repo = PrintDevicesRepoCore::new(state.db.pool());
            match devices_repo
                .get_device_by_fingerprint(tenant_id, trimmed)
                .await
            {
                Ok(Some(_)) => Some(trimmed.to_string()),
                Ok(None) => return bad_request("print device not found").into_response(),
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()
                }
            }
        }
    } else {
        match state.print_devices_repo.get_default_device(tenant_id).await {
            Ok(Some(device)) => Some(device.fingerprint),
            Ok(None) => None,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }
    };

    let mut payload = req.payload.unwrap_or_else(|| serde_json::json!({}));
    if job_type == "SHIPPING_LABEL" {
        if let Err(resp) = attach_waybill_to_payload(&state, tenant_id, &mut payload).await {
            return resp;
        }
    }

    let job = match state
        .print_jobs_repo
        .create_job(PrintJobCreate {
            tenant_id,
            requested_by_user_id: ctx.user_id,
            job_type,
            ref_type,
            ref_id,
            template_id,
            device_fingerprint,
            payload,
            trace_id: Some(req_ctx.trace_id.clone()),
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "PRINT_JOB.CREATE",
        "print_job",
        Some(job.job_id.to_string()),
        None,
        Some(serde_json::to_value(&job).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(job)).into_response()
}

pub(crate) async fn list_inventory_sync_job_events(
    State(state): State<AppState>,
    Path((tenant_id, job_id)): Path<(String, String)>,
    Query(query): Query<StockSyncJobEventListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let job_id = match uuid::Uuid::parse_str(&job_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad job_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "inventory.read") {
        return resp.into_response();
    }

    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(100).clamp(1, 500);
    let events = match StockSyncJobEventsRepo::new(state.db.pool())
        .list_events(
            tenant_id,
            job_id,
            query.status.and_then(non_empty),
            offset,
            limit,
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let items = events
        .into_iter()
        .map(|event| StockSyncJobEventResponse {
            event_id: event.event_id,
            status: event.status,
            attempt: event.attempt,
            payload: event.payload,
            created_at: event.created_at,
        })
        .collect();

    (
        StatusCode::OK,
        Json(StockSyncJobEventListResponse { items }),
    )
        .into_response()
}

pub(crate) async fn update_print_job(
    State(state): State<AppState>,
    Path((tenant_id, job_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<PrintJobUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "print_jobs.write") {
        return resp.into_response();
    }

    let job_id = match uuid::Uuid::parse_str(&job_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad job_id").into_response(),
    };

    let job = match state
        .print_jobs_repo
        .update_job(
            tenant_id,
            job_id,
            PrintJobUpdate {
                status: req.status,
                payload: req.payload,
                output_url: req.output_url,
                output_content: None,
                attempts: None,
                last_error: req.last_error,
                actor_user_id: Some(ctx.user_id),
            },
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("job not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    record_audit_log(
        &state,
        &ctx,
        &req_ctx,
        "PRINT_JOB.UPDATE",
        "print_job",
        Some(job.job_id.to_string()),
        None,
        Some(serde_json::to_value(&job).unwrap_or(Value::Null)),
    )
    .await;

    let logs_repo = AuditLogsRepo::new(state.db.pool());
    let after_json = serde_json::to_value(&job).unwrap_or(serde_json::json!({}));
    if let Err(err) = logs_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "PRINT_JOB.UPDATE".to_string(),
            entity_type: "print_job".to_string(),
            entity_id: Some(job.job_id.to_string()),
            before_json: None,
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("audit log failed: {err}"),
        )
            .into_response();
    }

    (StatusCode::OK, Json(job)).into_response()
}

pub(crate) async fn get_print_job(
    State(state): State<AppState>,
    Path((tenant_id, job_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "print_jobs.read") {
        return resp.into_response();
    }

    let job_id = match uuid::Uuid::parse_str(&job_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad job_id").into_response(),
    };

    let job = match state.print_jobs_repo.get_job(tenant_id, job_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("job not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(job)).into_response()
}

pub(crate) async fn get_print_job_output(
    State(state): State<AppState>,
    Path((tenant_id, job_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "print_jobs.read") {
        return resp.into_response();
    }

    let job_id = match uuid::Uuid::parse_str(&job_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad job_id").into_response(),
    };

    let output = match state
        .print_jobs_repo
        .get_job_output(tenant_id, job_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("job not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    if output.status != "DONE" {
        return conflict("print job not finished").into_response();
    }

    let content = match output.output_content {
        Some(value) => value,
        None => return not_found("print output missing").into_response(),
    };

    let format = output
        .content_format
        .unwrap_or_else(|| "HTML".to_string())
        .to_uppercase();
    let content_type = match format.as_str() {
        "ZPL" => "application/zpl",
        "TEXT" => "text/plain; charset=utf-8",
        _ => "text/html; charset=utf-8",
    };
    let mut resp = Response::new(Body::from(content));
    resp.headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    resp
}

pub(crate) async fn retry_print_job(
    State(state): State<AppState>,
    Path((tenant_id, job_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "print_jobs.write") {
        return resp.into_response();
    }

    let job_id = match uuid::Uuid::parse_str(&job_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad job_id").into_response(),
    };

    if let Err(err) = state
        .print_jobs_repo
        .retry_failed_job(tenant_id, job_id)
        .await
    {
        let message = err.to_string();
        if message.contains("print job not found") {
            return not_found("print job not found").into_response();
        }
        if message.contains("status not retryable") {
            return conflict("print job not retryable").into_response();
        }
        return (StatusCode::INTERNAL_SERVER_ERROR, message).into_response();
    }

    let job = match state.print_jobs_repo.get_job(tenant_id, job_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("job not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let logs_repo = AuditLogsRepo::new(state.db.pool());
    let after_json = serde_json::to_value(&job).unwrap_or(serde_json::json!({}));
    if let Err(err) = logs_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "PRINT_JOB.RETRY".to_string(),
            entity_type: "print_job".to_string(),
            entity_id: Some(job.job_id.to_string()),
            before_json: None,
            after_json: Some(after_json),
            ip: None,
            ua: None,
        })
        .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("audit log failed: {err}"),
        )
            .into_response();
    }

    (StatusCode::OK, Json(job)).into_response()
}

pub(crate) async fn dispatch_print_job(
    State(state): State<AppState>,
    Path((tenant_id, job_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "print_jobs.write") {
        return resp.into_response();
    }

    let job_id = match uuid::Uuid::parse_str(&job_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad job_id").into_response(),
    };

    if let Err(err) = state.print_jobs_repo.dispatch_job(tenant_id, job_id).await {
        let message = err.to_string();
        if message.contains("print job not found") {
            return not_found("print job not found").into_response();
        }
        if message.contains("status not dispatchable") {
            return conflict("print job not dispatchable").into_response();
        }
        return (StatusCode::INTERNAL_SERVER_ERROR, message).into_response();
    }

    let job = match state.print_jobs_repo.get_job(tenant_id, job_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("job not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(job)).into_response()
}

pub(crate) async fn list_print_agent_jobs(
    State(state): State<AppState>,
    Query(query): Query<PrintAgentJobQuery>,
    Extension(device_ctx): Extension<DeviceAuthContext>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(10).clamp(1, 50);
    let repo = PrintJobsRepoCore::new(state.db.pool());
    let jobs = match repo
        .claim_pending_jobs_for_device(device_ctx.tenant_id, &device_ctx.fingerprint, limit)
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let mut items = Vec::new();
    for job in jobs {
        let template_id = match job.template_id {
            Some(v) => v,
            None => {
                let _ = state
                    .print_jobs_repo
                    .update_job(
                        device_ctx.tenant_id,
                        job.job_id,
                        PrintJobUpdate {
                            status: Some("FAILED".to_string()),
                            payload: None,
                            output_url: None,
                            output_content: None,
                            attempts: None,
                            last_error: Some("template_id missing".to_string()),
                            actor_user_id: None,
                        },
                    )
                    .await;
                continue;
            }
        };

        let template = match state
            .print_templates_repo
            .get_template(device_ctx.tenant_id, template_id)
            .await
        {
            Ok(Some(v)) => v,
            Ok(None) => {
                let _ = state
                    .print_jobs_repo
                    .update_job(
                        device_ctx.tenant_id,
                        job.job_id,
                        PrintJobUpdate {
                            status: Some("FAILED".to_string()),
                            payload: None,
                            output_url: None,
                            output_content: None,
                            attempts: None,
                            last_error: Some("print template not found".to_string()),
                            actor_user_id: None,
                        },
                    )
                    .await;
                continue;
            }
            Err(err) => {
                warn!(error=%err, job_id=%job.job_id, "print template lookup failed");
                continue;
            }
        };

        let template_body = match template.content {
            Value::String(value) => value,
            other => other.to_string(),
        };
        let template_version = job.template_version.unwrap_or(template.version);

        items.push(PrintAgentJob {
            job_id: job.job_id,
            tenant_id: uuid::Uuid::from(device_ctx.tenant_id),
            job_type: job.job_type,
            ref_type: job.ref_type,
            ref_id: job.ref_id,
            template_id,
            template_version,
            content_format: template.content_format,
            template_content: template_body,
            payload: job.payload,
            attempts: job.attempts,
        });
    }

    (StatusCode::OK, Json(PrintAgentJobListResponse { items })).into_response()
}

pub(crate) async fn get_print_agent_device(
    State(state): State<AppState>,
    Extension(device_ctx): Extension<DeviceAuthContext>,
) -> impl IntoResponse {
    let repo = PrintDevicesRepoCore::new(state.db.pool());
    let device = match repo
        .get_device_by_fingerprint(device_ctx.tenant_id, &device_ctx.fingerprint)
        .await
    {
        Ok(Some(device)) => device,
        Ok(None) => return not_found("device not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    (StatusCode::OK, Json(device)).into_response()
}

pub(crate) async fn complete_print_agent_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
    Extension(device_ctx): Extension<DeviceAuthContext>,
    Json(req): Json<PrintAgentJobCompleteRequest>,
) -> impl IntoResponse {
    let job_id = match uuid::Uuid::parse_str(&job_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad job_id").into_response(),
    };

    let job = match state
        .print_jobs_repo
        .get_job(device_ctx.tenant_id, job_id)
        .await
    {
        Ok(Some(job)) => job,
        Ok(None) => return not_found("job not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    if job.device_fingerprint.as_deref() != Some(device_ctx.fingerprint.as_str()) {
        return forbidden("device mismatch").into_response();
    }

    let status = req.status.trim().to_uppercase();
    let (output_url, output_content, last_error) = match status.as_str() {
        "DONE" => {
            let content = match req.output_content {
                Some(value) if !value.trim().is_empty() => value,
                _ => return bad_request("output_content required").into_response(),
            };
            let url = format!(
                "/v1/tenants/{}/print-jobs/{}/output",
                device_ctx.tenant_id, job_id
            );
            (Some(url), Some(content), Some(String::new()))
        }
        "FAILED" => {
            let error = req.error.unwrap_or_else(|| "print failed".to_string());
            (None, None, Some(error))
        }
        _ => return bad_request("invalid status").into_response(),
    };

    let job = match state
        .print_jobs_repo
        .update_job(
            device_ctx.tenant_id,
            job_id,
            PrintJobUpdate {
                status: Some(status),
                payload: None,
                output_url,
                output_content,
                attempts: None,
                last_error,
                actor_user_id: None,
            },
        )
        .await
    {
        Ok(Some(job)) => job,
        Ok(None) => return not_found("job not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(job)).into_response()
}
