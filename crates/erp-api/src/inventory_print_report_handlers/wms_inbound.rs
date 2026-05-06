use super::*;

pub(crate) async fn list_inbounds(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<InboundDocListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.inbounds.read") {
        return resp.into_response();
    }

    let warehouse_id = match query.warehouse_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad warehouse_id").into_response(),
        },
        None => None,
    };
    let status = match query.status.as_deref() {
        Some(raw) => match normalize_inbound_status(raw) {
            Some(v) => Some(v),
            None => return bad_request("bad status").into_response(),
        },
        None => None,
    };
    let inbound_status = match query.inbound_status.as_deref() {
        Some(raw) => match InboundStatus::from_str(raw) {
            Ok(v) => Some(v.to_string()),
            Err(_) => return bad_request("bad inbound_status").into_response(),
        },
        None => None,
    };
    let receipt_status = match query.receipt_status.as_deref() {
        Some(raw) => match ReceiptStatus::from_str(raw) {
            Ok(v) => Some(v.to_string()),
            Err(_) => return bad_request("bad receipt_status").into_response(),
        },
        None => None,
    };
    let inbound_qc_status = match query.inbound_qc_status.as_deref() {
        Some(raw) => match InboundQcStatus::from_str(raw) {
            Ok(v) => Some(v.to_string()),
            Err(_) => return bad_request("bad inbound_qc_status").into_response(),
        },
        None => None,
    };
    let putaway_status = match query.putaway_status.as_deref() {
        Some(raw) => match PutawayStatus::from_str(raw) {
            Ok(v) => Some(v.to_string()),
            Err(_) => return bad_request("bad putaway_status").into_response(),
        },
        None => None,
    };
    let doc_type = match query.doc_type.as_deref() {
        Some(raw) => match normalize_inbound_doc_type(raw) {
            Some(v) => Some(v),
            None => return bad_request("bad doc_type").into_response(),
        },
        None => None,
    };
    let supplier_id = match query.supplier_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad supplier_id").into_response(),
        },
        None => None,
    };
    let sku_id = match query.sku_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad sku_id").into_response(),
        },
        None => None,
    };
    let created_from = match query.created_from.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad created_from").into_response(),
        },
        None => None,
    };
    let created_to = match query.created_to.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad created_to").into_response(),
        },
        None => None,
    };
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = InboundsRepo::new(state.db.pool());
    let result = match repo
        .list_docs(
            tenant_id,
            InboundDocListFilters {
                status,
                inbound_status,
                receipt_status,
                inbound_qc_status,
                putaway_status,
                doc_type,
                warehouse_id,
                doc_no: query.doc_no.and_then(non_empty),
                doc_no_keyword: query.doc_no_keyword.and_then(non_empty),
                ref_type: query.ref_type.and_then(non_empty),
                ref_id: query.ref_id.and_then(non_empty),
                supplier_id,
                sku_id,
                created_from,
                created_to,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = InboundDocListResponse {
        items: result
            .items
            .into_iter()
            .map(inbound_doc_summary_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn get_inbound_detail(
    State(state): State<AppState>,
    Path((tenant_id, doc_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let doc_id = match uuid::Uuid::parse_str(&doc_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad doc_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.inbounds.read") {
        return resp.into_response();
    }

    let repo = InboundsRepo::new(state.db.pool());
    let detail = match repo.get_detail(tenant_id, doc_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return not_found("inbound doc not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (StatusCode::OK, Json(inbound_doc_detail_response(detail))).into_response()
}

pub(crate) async fn audit_inbound(
    State(state): State<AppState>,
    Path((tenant_id, doc_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let doc_id = match uuid::Uuid::parse_str(&doc_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad doc_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.inbounds.write") {
        return resp.into_response();
    }

    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::InventoryWrite).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::InventoryWrite).into_response();
    }

    let repo = InboundsRepo::new(state.db.pool());
    let detail = match repo.audit_doc(tenant_id, doc_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("{e}")).into_response(),
    };

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_WMS_INBOUND_AUDITED.to_string(),
            ref_type: Some("INBOUND_DOC".to_string()),
            ref_id: Some(detail.doc.doc_id.to_string()),
            metadata: Some(serde_json::json!({
                "doc_no": detail.doc.doc_no,
                "warehouse_id": detail.doc.warehouse_id,
            })),
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    let audit_repo = AuditLogsRepo::new(state.db.pool());
    if let Err(e) = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(ctx.user_id),
            action: "WMS.INBOUND_AUDITED".to_string(),
            entity_type: "INBOUND_DOC".to_string(),
            entity_id: Some(detail.doc.doc_id.to_string()),
            before_json: None,
            after_json: Some(serde_json::to_value(&detail).unwrap_or(Value::Null)),
            ip: None,
            ua: None,
        })
        .await
    {
        warn!(error = %e, "audit log insert failed");
    }

    (StatusCode::OK, Json(inbound_doc_detail_response(detail))).into_response()
}

pub(crate) async fn create_inbound(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<InboundDocCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.inbounds.write") {
        return resp.into_response();
    }

    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::InventoryWrite).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::InventoryWrite).into_response();
    }

    let warehouse_id = match uuid::Uuid::parse_str(&req.warehouse_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad warehouse_id").into_response(),
    };
    let doc_type = match normalize_inbound_doc_type(&req.doc_type) {
        Some(v) => v,
        None => return bad_request("bad doc_type").into_response(),
    };
    let doc_no = req.doc_no.trim().to_string();
    if doc_no.is_empty() {
        return bad_request("doc_no is required").into_response();
    }

    let mut items = Vec::new();
    for item in &req.items {
        let sku_id = match uuid::Uuid::parse_str(&item.sku_id) {
            Ok(v) => v,
            Err(_) => return bad_request("bad sku_id").into_response(),
        };
        if item.planned_qty < 0 {
            return bad_request("planned_qty must be >= 0").into_response();
        }
        items.push(InboundDocItemCreate {
            sku_id,
            planned_qty: item.planned_qty,
        });
    }

    let repo = InboundsRepo::new(state.db.pool());
    let detail = match repo
        .create_doc(InboundDocCreate {
            doc_id: uuid::Uuid::new_v4(),
            tenant_id,
            warehouse_id,
            doc_type,
            doc_no,
            ref_type: req.ref_type.clone(),
            ref_id: req.ref_id.clone(),
            items,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_WMS_INBOUND_CREATED.to_string(),
            ref_type: Some("INBOUND_DOC".to_string()),
            ref_id: Some(detail.doc.doc_id.to_string()),
            metadata: Some(serde_json::json!({
                "doc_no": detail.doc.doc_no,
                "doc_type": detail.doc.doc_type,
                "warehouse_id": detail.doc.warehouse_id,
            })),
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    (StatusCode::OK, Json(inbound_doc_detail_response(detail))).into_response()
}

pub(crate) async fn receive_inbound(
    State(state): State<AppState>,
    Path((tenant_id, doc_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<InboundReceiveRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let doc_id = match uuid::Uuid::parse_str(&doc_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad doc_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.inbounds.write") {
        return resp.into_response();
    }

    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::InventoryWrite).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::InventoryWrite).into_response();
    }

    let require_audit =
        match fetch_tenant_setting_value(state.db.pool(), tenant_id, "inv.inbound_require_audit")
            .await
        {
            Ok(value) => value.as_ref().and_then(parse_bool_setting).unwrap_or(false),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };

    let repo = InboundsRepo::new(state.db.pool());
    if require_audit {
        let detail = match repo.get_detail(tenant_id, doc_id).await {
            Ok(Some(v)) => v,
            Ok(None) => return not_found("inbound doc not found").into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        if detail.doc.status == INBOUND_STATUS_DRAFT {
            return bad_request("inbound doc not audited").into_response();
        }
    }

    let mut items = Vec::new();
    for item in &req.items {
        let sku_id = match uuid::Uuid::parse_str(&item.sku_id) {
            Ok(v) => v,
            Err(_) => return bad_request("bad sku_id").into_response(),
        };
        let operation_id = match uuid::Uuid::parse_str(&item.operation_id) {
            Ok(v) => v,
            Err(_) => return bad_request("bad operation_id").into_response(),
        };
        if item.qty <= 0 {
            return bad_request("qty must be positive").into_response();
        }
        items.push(InboundReceiveItem {
            sku_id,
            qty: item.qty,
            operation_id,
        });
    }

    let detail = match repo
        .receive(InboundReceiveInput {
            tenant_id,
            doc_id,
            items,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("{e}")).into_response(),
    };

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_WMS_INBOUND_RECEIVED.to_string(),
            ref_type: Some("INBOUND_DOC".to_string()),
            ref_id: Some(detail.doc.doc_id.to_string()),
            metadata: Some(serde_json::json!({
                "doc_no": detail.doc.doc_no,
                "warehouse_id": detail.doc.warehouse_id,
            })),
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    (StatusCode::OK, Json(inbound_doc_detail_response(detail))).into_response()
}

pub(crate) async fn list_inbound_inspections(
    State(state): State<AppState>,
    Path((tenant_id, doc_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let doc_id = match uuid::Uuid::parse_str(&doc_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad doc_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.inbounds.read") {
        return resp.into_response();
    }

    let repo = InboundsRepo::new(state.db.pool());
    let items = match repo.list_inspections(tenant_id, doc_id).await {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(InboundInspectionListResponse { items }),
    )
        .into_response()
}

pub(crate) async fn inspect_inbound(
    State(state): State<AppState>,
    Path((tenant_id, doc_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<InboundInspectRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let doc_id = match uuid::Uuid::parse_str(&doc_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad doc_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.inbounds.write") {
        return resp.into_response();
    }

    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::InventoryWrite).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::InventoryWrite).into_response();
    }

    let mut items = Vec::new();
    for item in &req.items {
        let sku_id = match uuid::Uuid::parse_str(&item.sku_id) {
            Ok(v) => v,
            Err(_) => return bad_request("bad sku_id").into_response(),
        };
        let operation_id = match uuid::Uuid::parse_str(&item.operation_id) {
            Ok(v) => v,
            Err(_) => return bad_request("bad operation_id").into_response(),
        };
        if item.qty <= 0 {
            return bad_request("qty must be positive").into_response();
        }
        let normalized = item.result.trim().to_uppercase();
        if normalized != "PASSED" && normalized != "FAILED" && normalized != "PENDING" {
            return bad_request("result must be PASSED/FAILED/PENDING").into_response();
        }
        items.push(InboundInspectItem {
            sku_id,
            qty: item.qty,
            result: normalized,
            note: item.note.clone(),
            operation_id,
        });
    }

    let repo = InboundsRepo::new(state.db.pool());
    if let Err(e) = repo
        .inspect(InboundInspectInput {
            tenant_id,
            doc_id,
            items,
            operator_id: Some(uuid::Uuid::from(ctx.user_id)),
        })
        .await
    {
        return (StatusCode::BAD_REQUEST, format!("{e}")).into_response();
    }

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_WMS_INBOUND_INSPECTED.to_string(),
            ref_type: Some("INBOUND_DOC".to_string()),
            ref_id: Some(doc_id.to_string()),
            metadata: Some(serde_json::json!({ "items": req.items.len() })),
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    StatusCode::NO_CONTENT.into_response()
}

pub(crate) async fn putaway_inbound(
    State(state): State<AppState>,
    Path((tenant_id, doc_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<InboundPutawayRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let doc_id = match uuid::Uuid::parse_str(&doc_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad doc_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.inbounds.write") {
        return resp.into_response();
    }

    let enabled =
        match feature_flag_enabled(&state, tenant_id, None, FeatureFlagKey::InventoryWrite).await {
            Ok(v) => v,
            Err(resp) => return resp.into_response(),
        };
    if !enabled {
        return feature_flag_forbidden(FeatureFlagKey::InventoryWrite).into_response();
    }

    let require_audit =
        match fetch_tenant_setting_value(state.db.pool(), tenant_id, "inv.inbound_require_audit")
            .await
        {
            Ok(value) => value.as_ref().and_then(parse_bool_setting).unwrap_or(false),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };

    let settings_repo = SettingsRepo::new(state.db.pool());
    let require_location = match settings_repo
        .list_settings(
            tenant_id,
            SettingListFilters {
                scope_type: Some("TENANT".to_string()),
                scope_id: None,
                key: Some("wms.putaway_require_location".to_string()),
            },
        )
        .await
    {
        Ok(settings) => settings
            .first()
            .and_then(|setting| parse_bool_setting(&setting.value_json))
            .unwrap_or(false),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };
    let require_inspection = match settings_repo
        .list_settings(
            tenant_id,
            SettingListFilters {
                scope_type: Some("TENANT".to_string()),
                scope_id: None,
                key: Some("wms.inbound_require_inspection".to_string()),
            },
        )
        .await
    {
        Ok(settings) => settings
            .first()
            .and_then(|setting| parse_bool_setting(&setting.value_json))
            .unwrap_or(false),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let repo = InboundsRepo::new(state.db.pool());
    if require_audit {
        let detail = match repo.get_detail(tenant_id, doc_id).await {
            Ok(Some(v)) => v,
            Ok(None) => return not_found("inbound doc not found").into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        };
        if detail.doc.status == INBOUND_STATUS_DRAFT {
            return bad_request("inbound doc not audited").into_response();
        }
    }
    if require_inspection {
        match repo.has_unpassed_inspections(tenant_id, doc_id).await {
            Ok(true) => return bad_request("inbound doc not inspected").into_response(),
            Ok(false) => {}
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
        }
    }

    let mut items = Vec::new();
    for item in &req.items {
        let sku_id = match uuid::Uuid::parse_str(&item.sku_id) {
            Ok(v) => v,
            Err(_) => return bad_request("bad sku_id").into_response(),
        };
        let operation_id = match uuid::Uuid::parse_str(&item.operation_id) {
            Ok(v) => v,
            Err(_) => return bad_request("bad operation_id").into_response(),
        };
        let to_location_id = match item.to_location_id.as_deref() {
            Some(value) => match uuid::Uuid::parse_str(value) {
                Ok(v) => Some(v),
                Err(_) => return bad_request("bad to_location_id").into_response(),
            },
            None => None,
        };
        if require_location && to_location_id.is_none() {
            return bad_request("to_location_id required").into_response();
        }
        if item.qty <= 0 {
            return bad_request("qty must be positive").into_response();
        }
        items.push(InboundPutawayItem {
            sku_id,
            qty: item.qty,
            operation_id,
            from_area: item.from_area.clone(),
            to_location_id,
        });
    }

    let detail = match repo
        .putaway(InboundPutawayInput {
            tenant_id,
            doc_id,
            items,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("{e}")).into_response(),
    };

    let logs_repo = OperationLogsRepo::new(state.db.pool());
    if let Err(e) = logs_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(ctx.user_id),
            action: OP_ACTION_WMS_INBOUND_PUTAWAY.to_string(),
            ref_type: Some("INBOUND_DOC".to_string()),
            ref_id: Some(detail.doc.doc_id.to_string()),
            metadata: Some(serde_json::json!({
                "doc_no": detail.doc.doc_no,
                "warehouse_id": detail.doc.warehouse_id,
            })),
        })
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response();
    }

    (StatusCode::OK, Json(inbound_doc_detail_response(detail))).into_response()
}

pub(crate) async fn create_inbound_rejects(
    State(state): State<AppState>,
    Path((tenant_id, doc_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<InboundRejectCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let doc_id = match uuid::Uuid::parse_str(&doc_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad doc_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.inbounds.write") {
        return resp.into_response();
    }

    if req.items.is_empty() {
        return bad_request("items required").into_response();
    }

    let mut items = Vec::new();
    for item in &req.items {
        let sku_id = match uuid::Uuid::parse_str(&item.sku_id) {
            Ok(v) => v,
            Err(_) => return bad_request("bad sku_id").into_response(),
        };
        if item.qty <= 0 {
            return bad_request("qty must be positive").into_response();
        }
        items.push(InboundRejectCreateItem {
            sku_id,
            qty: item.qty,
            reason: item.reason.clone(),
        });
    }

    let repo = InboundRejectsRepo::new(state.db.pool());
    let inserted = match repo
        .create_rejects(InboundRejectCreateInput {
            tenant_id,
            doc_id,
            operator_id: Some(ctx.user_id),
            items,
        })
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("{e}")).into_response(),
    };

    let response = InboundRejectCreateResponse { inserted };
    (StatusCode::OK, Json(response)).into_response()
}

pub(crate) async fn list_inbound_rejects(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<InboundRejectListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "wms.inbounds.read") {
        return resp.into_response();
    }

    let sku_id = match query.sku_id.as_deref() {
        Some(raw) => match uuid::Uuid::parse_str(raw) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("bad sku_id").into_response(),
        },
        None => None,
    };
    let created_from = match query.created_from.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad created_from").into_response(),
        },
        None => None,
    };
    let created_to = match query.created_to.as_deref() {
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(v) => Some(v.with_timezone(&Utc)),
            Err(_) => return bad_request("bad created_to").into_response(),
        },
        None => None,
    };
    let offset = query.offset.unwrap_or(0).max(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let repo = InboundRejectsRepo::new(state.db.pool());
    let result = match repo
        .list_rejects(
            tenant_id,
            InboundRejectListFilters {
                doc_no_keyword: query.doc_no_keyword.and_then(non_empty),
                sku_id,
                created_from,
                created_to,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),
    };

    let response = InboundRejectListResponse {
        items: result
            .items
            .into_iter()
            .map(inbound_reject_response)
            .collect(),
        total: result.total,
    };
    (StatusCode::OK, Json(response)).into_response()
}
